use std::collections::HashSet;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use image::imageops::FilterType;
use image::ImageFormat;
use base64::prelude::*;
use rayon::prelude::*;
use crate::models::{ComicFileItem, FolderItem};

pub struct CoverExtractor {
    cache_dir: PathBuf,
    existing_files: RwLock<HashSet<String>>,
    existing_warmed: AtomicBool,
    empty_folder_cache: RwLock<HashSet<i64>>,
}

impl CoverExtractor {
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).ok();
        Self {
            cache_dir,
            existing_files: RwLock::new(HashSet::new()),
            existing_warmed: AtomicBool::new(false),
            empty_folder_cache: RwLock::new(HashSet::new()),
        }
    }

    /// Populates `existing_files` in the background so startup does not block on `read_dir`.
    pub fn warm_existing_files(&self) {
        let mut existing = HashSet::new();
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".webp") {
                        existing.insert(name.to_string());
                    }
                }
            }
        }
        if let Ok(mut set) = self.existing_files.write() {
            set.extend(existing);
        }
        self.existing_warmed.store(true, Ordering::Release);
    }

    pub fn file_cover_uri(file_id: i64) -> String {
        format!("ymppcover://file-{}.webp", file_id)
    }

    pub fn folder_cover_uri(folder_id: i64) -> String {
        format!("ymppcover://folder-{}.webp", folder_id)
    }

    pub fn file_cover_uri_versioned(file_id: i64, version: u128) -> String {
        format!("ymppcover://file-{}.webp?v={}", file_id, version)
    }

    pub fn folder_cover_uri_versioned(folder_id: i64, version: u128) -> String {
        format!("ymppcover://folder-{}.webp?v={}", folder_id, version)
    }

    pub fn get_cache_path(&self, file_id: i64) -> PathBuf {
        self.cache_dir.join(format!("{}.webp", file_id))
    }

    pub fn get_folder_cache_path(&self, folder_id: i64) -> PathBuf {
        self.cache_dir.join(format!("folder_{}.webp", folder_id))
    }

    fn has_cache_key(&self, key: &str) -> bool {
        if let Ok(set) = self.existing_files.read() {
            if set.contains(key) {
                return true;
            }
        }
        if self.existing_warmed.load(Ordering::Acquire) {
            return false;
        }
        if self.cache_dir.join(key).exists() {
            if let Ok(mut set) = self.existing_files.write() {
                set.insert(key.to_string());
            }
            return true;
        }
        false
    }

    pub fn has_file_cover(&self, file_id: i64) -> bool {
        self.has_cache_key(&format!("{}.webp", file_id))
    }

    pub fn has_folder_cover(&self, folder_id: i64) -> bool {
        self.has_cache_key(&format!("folder_{}.webp", folder_id))
    }

    pub fn get_file_cover_uri_if_exists(&self, file_id: i64) -> Option<String> {
        if self.has_file_cover(file_id) {
            Some(Self::file_cover_uri(file_id))
        } else {
            None
        }
    }

    pub fn get_folder_cover_uri_if_exists(&self, folder_id: i64) -> Option<String> {
        if self.has_folder_cover(folder_id) {
            Some(Self::folder_cover_uri(folder_id))
        } else {
            None
        }
    }

    pub fn is_known_empty_folder(&self, folder_id: i64) -> bool {
        self.empty_folder_cache
            .read()
            .map(|set| set.contains(&folder_id))
            .unwrap_or(false)
    }

    pub fn mark_empty_folder(&self, folder_id: i64) {
        if let Ok(mut set) = self.empty_folder_cache.write() {
            set.insert(folder_id);
        }
    }

    pub fn invalidate_file_cover(&self, file_id: i64) {
        let key = format!("{}.webp", file_id);
        if let Ok(mut ex) = self.existing_files.write() {
            ex.remove(&key);
        }
        let path = self.get_cache_path(file_id);
        let _ = std::fs::remove_file(path);
    }

    pub fn invalidate_folder_cover(&self, folder_id: i64) {
        let key = format!("folder_{}.webp", folder_id);
        if let Ok(mut ex) = self.existing_files.write() {
            ex.remove(&key);
        }
        if let Ok(mut empty) = self.empty_folder_cache.write() {
            empty.remove(&folder_id);
        }
        let path = self.get_folder_cache_path(folder_id);
        let _ = std::fs::remove_file(path);
    }

    pub fn clear_memory_caches(&self) {
        if let Ok(mut set) = self.existing_files.write() {
            set.clear();
        }
        if let Ok(mut empty) = self.empty_folder_cache.write() {
            empty.clear();
        }
        self.existing_warmed.store(true, Ordering::Release);
    }

    pub fn get_cached_file_cover(&self, file_id: i64) -> Option<String> {
        if !self.has_file_cover(file_id) {
            return None;
        }
        Self::read_as_data_url(&self.get_cache_path(file_id))
    }

    /// Reads raw WebP bytes for the custom `ymppcover` URI protocol (`file-<id>.webp` or `folder-<id>.webp`).
    pub fn read_cover_bytes_by_uri_name(&self, uri_name: &str) -> Option<Vec<u8>> {
        let disk_key = if let Some(rest) = uri_name.strip_prefix("file-") {
            let id_str = rest.strip_suffix(".webp")?;
            if id_str.is_empty() || !id_str.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            format!("{}.webp", id_str)
        } else if let Some(rest) = uri_name.strip_prefix("folder-") {
            let id_str = rest.strip_suffix(".webp")?;
            if id_str.is_empty() || !id_str.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            format!("folder_{}.webp", id_str)
        } else {
            return None;
        };

        let path = self.cache_dir.join(&disk_key);
        std::fs::read(&path).ok()
    }

    /// Populates lightweight `ymppcover://` URIs for comic files without reading `.webp` files or Base64 encoding.
    pub fn populate_file_covers_uris(&self, files: &mut [ComicFileItem]) {
        let warmed = self.existing_warmed.load(Ordering::Acquire);
        let existing_guard = if warmed { self.existing_files.read().ok() } else { None };

        for file in files.iter_mut() {
            if file.cover_cache_path.is_some() {
                file.cover_data_url = Some(Self::file_cover_uri(file.id));
                continue;
            }
            if let Some(ref set) = existing_guard {
                let key = format!("{}.webp", file.id);
                if set.contains(&key) {
                    file.cover_data_url = Some(Self::file_cover_uri(file.id));
                }
            }
        }
    }

    /// Populates lightweight `ymppcover://` URIs for folders and returns any `"unknown"` folder IDs
    /// whose cached `.webp` was confirmed to exist on disk so callers can persist `cover_status = 'ready'`.
    pub fn populate_folder_covers_uris(&self, folders: &mut [FolderItem]) -> Vec<i64> {
        let mut unknown_indices: Vec<usize> = Vec::new();

        for (idx, folder) in folders.iter_mut().enumerate() {
            match folder.cover_status.as_str() {
                "ready" => {
                    folder.cover_data_url = Some(Self::folder_cover_uri(folder.id));
                }
                "empty" => {
                    folder.cover_data_url = None;
                }
                _ => {
                    unknown_indices.push(idx);
                }
            }
        }

        if unknown_indices.is_empty() {
            return Vec::new();
        }

        let mut newly_ready_ids: Vec<i64> = Vec::new();

        if self.existing_warmed.load(Ordering::Acquire) {
            if let Ok(set) = self.existing_files.read() {
                for idx in unknown_indices {
                    let fid = folders[idx].id;
                    if set.contains(&format!("folder_{}.webp", fid)) {
                        folders[idx].cover_status = "ready".to_string();
                        folders[idx].cover_data_url = Some(Self::folder_cover_uri(fid));
                        newly_ready_ids.push(fid);
                    }
                }
            }
            return newly_ready_ids;
        }

        // Before background cache warming completes, check only the unknown folders in the current view in parallel
        let found_indices: Vec<usize> = unknown_indices
            .par_iter()
            .copied()
            .filter(|&idx| self.get_folder_cache_path(folders[idx].id).exists())
            .collect();

        if !found_indices.is_empty() {
            if let Ok(mut set) = self.existing_files.write() {
                for idx in &found_indices {
                    let fid = folders[*idx].id;
                    set.insert(format!("folder_{}.webp", fid));
                }
            }
            for idx in found_indices {
                let fid = folders[idx].id;
                folders[idx].cover_status = "ready".to_string();
                folders[idx].cover_data_url = Some(Self::folder_cover_uri(fid));
                newly_ready_ids.push(fid);
            }
        }

        newly_ready_ids
    }

    pub fn extract_and_cache(
        &self,
        file_id: i64,
        zip_path: &Path,
        max_width: u32,
    ) -> Result<PathBuf, String> {
        let key = format!("{}.webp", file_id);
        let out_path = self.cache_dir.join(&key);
        if self.has_file_cover(file_id) {
            return Ok(out_path);
        }

        let file = File::open(zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {}", e))?;

        // 1. Gather all image file names in the zip
        let mut image_entries: Vec<(usize, String)> = Vec::new();
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                if Self::is_supported_image(&name) {
                    image_entries.push((i, name));
                }
            }
        }

        if image_entries.is_empty() {
            return Err("No image found inside archive".to_string());
        }

        // 2. Find best candidate: explicit cover keyword first, otherwise natural first
        let best_index = Self::find_cover_index(&image_entries);

        // 3. Read only that entry to memory
        let mut entry = archive
            .by_index(best_index)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;

        let mut buffer = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to stream zip entry bytes: {}", e))?;

        self.encode_and_save_webp(&buffer, &key, &out_path, max_width)?;
        Ok(out_path)
    }

    /// Extracts a cover image for a folder that contains loose image files (or whose subdirectories contain images).
    pub fn extract_folder_cover_from_dir(
        &self,
        folder_id: i64,
        folder_path: &Path,
        max_width: u32,
    ) -> Result<PathBuf, String> {
        let key = format!("folder_{}.webp", folder_id);
        let out_path = self.cache_dir.join(&key);
        if self.has_folder_cover(folder_id) {
            return Ok(out_path);
        }

        let best_img_path = Self::find_best_image_in_dir(folder_path, 2)
            .ok_or_else(|| "No loose images found in folder".to_string())?;

        let buffer = std::fs::read(&best_img_path)
            .map_err(|e| format!("Failed to read folder image: {}", e))?;

        self.encode_and_save_webp(&buffer, &key, &out_path, max_width)?;
        Ok(out_path)
    }

    /// Copies an existing comic cover to act as the folder cover.
    pub fn copy_comic_cover_to_folder(&self, comic_file_id: i64, folder_id: i64) -> Option<String> {
        if self.has_folder_cover(folder_id) {
            return Some(Self::folder_cover_uri(folder_id));
        }

        let comic_path = self.get_cache_path(comic_file_id);
        let folder_path = self.get_folder_cache_path(folder_id);
        if comic_path.exists() {
            if std::fs::copy(&comic_path, &folder_path).is_ok() {
                let folder_key = format!("folder_{}.webp", folder_id);
                if let Ok(mut ex) = self.existing_files.write() {
                    ex.insert(folder_key);
                }
                return Some(Self::folder_cover_uri(folder_id));
            }
        }
        None
    }

    fn encode_and_save_webp(
        &self,
        buffer: &[u8],
        key: &str,
        out_path: &Path,
        max_width: u32,
    ) -> Result<(), String> {
        let img = image::load_from_memory(buffer).map_err(|e| format!("Failed to decode image: {}", e))?;

        let thumb = if img.width() > max_width {
            img.resize(max_width, max_width * 2, FilterType::Triangle)
        } else {
            img
        };

        let mut webp_bytes = Cursor::new(Vec::new());
        thumb
            .write_to(&mut webp_bytes, ImageFormat::WebP)
            .map_err(|e| format!("Failed to encode webp thumbnail: {}", e))?;
        let raw_webp = webp_bytes.into_inner();

        std::fs::write(out_path, &raw_webp)
            .map_err(|e| format!("Failed to write cache file: {}", e))?;

        if let Ok(mut ex) = self.existing_files.write() {
            ex.insert(key.to_string());
        }
        Ok(())
    }

    fn find_best_image_in_dir(dir: &Path, max_depth: usize) -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        let mut images: Vec<(usize, String, PathBuf)> = Vec::new();
        let mut subdirs: Vec<(String, PathBuf)> = Vec::new();

        for (idx, entry_res) in entries.enumerate() {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            let name = entry.file_name().to_string_lossy().to_string();
            if file_type.is_file() {
                if Self::is_supported_image(&name) {
                    images.push((idx, name, entry.path()));
                }
            } else if file_type.is_dir() && max_depth > 0 {
                if name != ".ymppCache" && name != ".uhheuCache" && name != ".git" && name != "$RECYCLE.BIN" {
                    subdirs.push((name, entry.path()));
                }
            }
        }

        if !images.is_empty() {
            let indexed: Vec<(usize, String)> = images
                .iter()
                .enumerate()
                .map(|(i, (_, name, _))| (i, name.clone()))
                .collect();
            let best_i = Self::find_cover_index(&indexed);
            return Some(images[best_i].2.clone());
        }

        if max_depth > 0 && !subdirs.is_empty() {
            subdirs.sort_by(|a, b| natural_cmp(&a.0, &b.0));
            for (_, subdir_path) in subdirs.into_iter().take(5) {
                if let Some(found) = Self::find_best_image_in_dir(&subdir_path, max_depth - 1) {
                    return Some(found);
                }
            }
        }

        None
    }

    pub fn is_supported_image(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".png")
            || lower.ends_with(".webp")
            || lower.ends_with(".bmp")
            || lower.ends_with(".gif")
    }

    fn find_cover_index(entries: &[(usize, String)]) -> usize {
        // Priority keywords
        let cover_keywords = ["cover", "folder", "title", "000.", "001."];

        for kw in &cover_keywords {
            for (idx, name) in entries {
                let file_name = Path::new(name)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(name)
                    .to_lowercase();
                if file_name.contains(kw) {
                    return *idx;
                }
            }
        }

        // Fallback: natural string sorting by file name
        let mut sorted = entries.to_vec();
        sorted.sort_by(|a, b| {
            let name_a = Path::new(&a.1).file_name().and_then(|f| f.to_str()).unwrap_or(&a.1);
            let name_b = Path::new(&b.1).file_name().and_then(|f| f.to_str()).unwrap_or(&b.1);
            natural_cmp(name_a, name_b)
        });

        sorted[0].0
    }

    pub fn read_as_data_url(path: &Path) -> Option<String> {
        if !path.exists() {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        let b64 = BASE64_STANDARD.encode(&bytes);
        Some(format!("data:image/webp;base64,{}", b64))
    }
}

// Simple natural comparison helper (handles embedded numbers like 1, 2, 10)
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(c1), Some(c2)) => {
                if c1.is_ascii_digit() && c2.is_ascii_digit() {
                    let mut num_a: u64 = 0;
                    while let Some(d) = a_chars.peek() {
                        if let Some(digit) = d.to_digit(10) {
                            num_a = num_a.saturating_mul(10).saturating_add(digit as u64);
                            a_chars.next();
                        } else {
                            break;
                        }
                    }

                    let mut num_b: u64 = 0;
                    while let Some(d) = b_chars.peek() {
                        if let Some(digit) = d.to_digit(10) {
                            num_b = num_b.saturating_mul(10).saturating_add(digit as u64);
                            b_chars.next();
                        } else {
                            break;
                        }
                    }

                    if num_a != num_b {
                        return num_a.cmp(&num_b);
                    }
                } else {
                    let ord = c1.to_ascii_lowercase().cmp(&c2.to_ascii_lowercase());
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                    a_chars.next();
                    b_chars.next();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sorting() {
        assert_eq!(natural_cmp("1.jpg", "2.jpg"), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp("2.jpg", "10.jpg"), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp("page_09.png", "page_10.png"), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp("Cover.jpg", "cover.jpg"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_find_cover_priority() {
        let entries = vec![
            (0, "002.jpg".to_string()),
            (1, "001.jpg".to_string()),
            (2, "cover_art.jpg".to_string()),
        ];
        // Priority keyword 'cover' must win over '001.jpg'
        let best = CoverExtractor::find_cover_index(&entries);
        assert_eq!(best, 2);

        let entries_no_kw = vec![
            (0, "p10.jpg".to_string()),
            (1, "p2.jpg".to_string()),
            (2, "p1.jpg".to_string()),
        ];
        // Natural sort first must be p1.jpg (index 2)
        let best_nat = CoverExtractor::find_cover_index(&entries_no_kw);
        assert_eq!(best_nat, 2);
    }

    #[test]
    fn test_read_cover_bytes_by_uri_name_validation() {
        let temp_dir = std::env::temp_dir().join(format!(
            "test_cover_uri_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("12.webp"), b"webp-file-12").unwrap();
        std::fs::write(temp_dir.join("folder_34.webp"), b"webp-folder-34").unwrap();

        let extractor = CoverExtractor::new(temp_dir.clone());
        assert_eq!(extractor.read_cover_bytes_by_uri_name("file-12.webp"), Some(b"webp-file-12".to_vec()));
        assert_eq!(extractor.read_cover_bytes_by_uri_name("folder-34.webp"), Some(b"webp-folder-34".to_vec()));
        assert_eq!(extractor.read_cover_bytes_by_uri_name("../12.webp"), None);
        assert_eq!(extractor.read_cover_bytes_by_uri_name("file-abc.webp"), None);
        assert_eq!(extractor.read_cover_bytes_by_uri_name("file-.webp"), None);
        assert_eq!(extractor.read_cover_bytes_by_uri_name("other.webp"), None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_populate_folder_covers_uris_and_warm() {
        let temp_dir = std::env::temp_dir().join(format!(
            "test_cover_warm_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("folder_10.webp"), b"f10").unwrap();
        std::fs::write(temp_dir.join("folder_11.webp"), b"f11").unwrap();

        let extractor = CoverExtractor::new(temp_dir.clone());
        let mut folders = vec![
            FolderItem {
                id: 10,
                library_id: 1,
                relative_path: "a".to_string(),
                name: "a".to_string(),
                parent_id: Some(1),
                rating: 0,
                is_new: false,
                cover_status: "ready".to_string(),
                cover_data_url: None,
                direct_tags: Vec::new(),
            },
            FolderItem {
                id: 11,
                library_id: 1,
                relative_path: "b".to_string(),
                name: "b".to_string(),
                parent_id: Some(1),
                rating: 0,
                is_new: false,
                cover_status: "unknown".to_string(),
                cover_data_url: None,
                direct_tags: Vec::new(),
            },
            FolderItem {
                id: 12,
                library_id: 1,
                relative_path: "c".to_string(),
                name: "c".to_string(),
                parent_id: Some(1),
                rating: 0,
                is_new: false,
                cover_status: "empty".to_string(),
                cover_data_url: None,
                direct_tags: Vec::new(),
            },
        ];

        // Before warming, unknown folder 11 should be discovered on disk
        let newly_ready = extractor.populate_folder_covers_uris(&mut folders);
        assert_eq!(newly_ready, vec![11]);
        assert_eq!(folders[0].cover_data_url.as_deref(), Some("ymppcover://folder-10.webp"));
        assert_eq!(folders[1].cover_data_url.as_deref(), Some("ymppcover://folder-11.webp"));
        assert_eq!(folders[1].cover_status, "ready");
        assert_eq!(folders[2].cover_data_url, None);

        // Warm in background and verify O(1) lookup
        extractor.warm_existing_files();
        assert!(extractor.has_folder_cover(10));
        assert!(extractor.has_folder_cover(11));
        assert!(!extractor.has_folder_cover(12));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
