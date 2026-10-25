use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use image::imageops::FilterType;
use image::ImageFormat;
use base64::prelude::*;
use crate::models::{ComicFileItem, FolderItem};

pub struct CoverExtractor {
    cache_dir: PathBuf,
    existing_files: RwLock<HashSet<String>>,
    data_url_cache: RwLock<HashMap<String, String>>,
    empty_folder_cache: RwLock<HashSet<i64>>,
}

impl CoverExtractor {
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).ok();
        let mut existing = HashSet::new();
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".webp") {
                        existing.insert(name.to_string());
                    }
                }
            }
        }
        Self {
            cache_dir,
            existing_files: RwLock::new(existing),
            data_url_cache: RwLock::new(HashMap::new()),
            empty_folder_cache: RwLock::new(HashSet::new()),
        }
    }

    pub fn get_cache_path(&self, file_id: i64) -> PathBuf {
        self.cache_dir.join(format!("{}.webp", file_id))
    }

    pub fn get_folder_cache_path(&self, folder_id: i64) -> PathBuf {
        self.cache_dir.join(format!("folder_{}.webp", folder_id))
    }

    pub fn has_file_cover(&self, file_id: i64) -> bool {
        let key = format!("{}.webp", file_id);
        self.existing_files
            .read()
            .map(|set| set.contains(&key))
            .unwrap_or(false)
    }

    pub fn has_folder_cover(&self, folder_id: i64) -> bool {
        let key = format!("folder_{}.webp", folder_id);
        self.existing_files
            .read()
            .map(|set| set.contains(&key))
            .unwrap_or(false)
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
        let url_key = format!("file_{}", file_id);
        if let Ok(mut cache) = self.data_url_cache.write() {
            cache.remove(&url_key);
        }
        let path = self.get_cache_path(file_id);
        let _ = std::fs::remove_file(path);
    }

    pub fn invalidate_folder_cover(&self, folder_id: i64) {
        let key = format!("folder_{}.webp", folder_id);
        if let Ok(mut ex) = self.existing_files.write() {
            ex.remove(&key);
        }
        let url_key = format!("folder_{}", folder_id);
        if let Ok(mut cache) = self.data_url_cache.write() {
            cache.remove(&url_key);
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
        if let Ok(mut map) = self.data_url_cache.write() {
            map.clear();
        }
        if let Ok(mut empty) = self.empty_folder_cache.write() {
            empty.clear();
        }
    }

    fn get_cached_by_key(&self, key: &str) -> Option<String> {
        if let Ok(cache) = self.data_url_cache.read() {
            if let Some(url) = cache.get(key) {
                return Some(url.clone());
            }
        }

        let exists = self
            .existing_files
            .read()
            .map(|set| set.contains(key))
            .unwrap_or(false);
        if !exists {
            return None;
        }

        let path = self.cache_dir.join(key);
        let bytes = std::fs::read(&path).ok()?;
        let b64 = BASE64_STANDARD.encode(&bytes);
        let url = format!("data:image/webp;base64,{}", b64);

        if let Ok(mut cache) = self.data_url_cache.write() {
            cache.insert(key.to_string(), url.clone());
        }
        Some(url)
    }

    pub fn get_cached_file_cover(&self, file_id: i64) -> Option<String> {
        let key = format!("{}.webp", file_id);
        self.get_cached_by_key(&key)
    }

    pub fn get_cached_folder_cover(&self, folder_id: i64) -> Option<String> {
        let key = format!("folder_{}.webp", folder_id);
        self.get_cached_by_key(&key)
    }

    pub fn populate_file_covers_parallel(&self, files: &mut [ComicFileItem]) {
        let mut to_load: Vec<(usize, String)> = Vec::new();
        {
            let cache = self.data_url_cache.read().ok();
            let existing = self.existing_files.read().ok();

            for (idx, file) in files.iter_mut().enumerate() {
                let key = format!("{}.webp", file.id);
                if let Some(ref c) = cache {
                    if let Some(url) = c.get(&key) {
                        file.cover_data_url = Some(url.clone());
                        continue;
                    }
                }
                if let Some(ref ex) = existing {
                    if ex.contains(&key) {
                        to_load.push((idx, key));
                    }
                }
            }
        }

        if to_load.is_empty() {
            return;
        }

        let loaded = self.read_keys_parallel(&to_load);
        if let Ok(mut cache) = self.data_url_cache.write() {
            for (idx, key, url) in loaded {
                cache.insert(key, url.clone());
                files[idx].cover_data_url = Some(url);
            }
        }
    }

    pub fn populate_folder_covers_parallel(&self, folders: &mut [FolderItem]) {
        let mut to_load: Vec<(usize, String)> = Vec::new();
        {
            let cache = self.data_url_cache.read().ok();
            let existing = self.existing_files.read().ok();

            for (idx, folder) in folders.iter_mut().enumerate() {
                let key = format!("folder_{}.webp", folder.id);
                if let Some(ref c) = cache {
                    if let Some(url) = c.get(&key) {
                        folder.cover_data_url = Some(url.clone());
                        continue;
                    }
                }
                if let Some(ref ex) = existing {
                    if ex.contains(&key) {
                        to_load.push((idx, key));
                    }
                }
            }
        }

        if to_load.is_empty() {
            return;
        }

        let loaded = self.read_keys_parallel(&to_load);
        if let Ok(mut cache) = self.data_url_cache.write() {
            for (idx, key, url) in loaded {
                cache.insert(key, url.clone());
                folders[idx].cover_data_url = Some(url);
            }
        }
    }

    fn read_keys_parallel(&self, items: &[(usize, String)]) -> Vec<(usize, String, String)> {
        let num_threads = 8.min(items.len()).max(1);
        let chunk_size = (items.len() + num_threads - 1) / num_threads;
        let cache_dir = &self.cache_dir;

        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for chunk in items.chunks(chunk_size) {
                handles.push(s.spawn(move || {
                    let mut out = Vec::with_capacity(chunk.len());
                    for (idx, key) in chunk {
                        let path = cache_dir.join(key);
                        if let Ok(bytes) = std::fs::read(&path) {
                            let b64 = BASE64_STANDARD.encode(&bytes);
                            let url = format!("data:image/webp;base64,{}", b64);
                            out.push((*idx, key.clone(), url));
                        }
                    }
                    out
                }));
            }
            let mut results = Vec::with_capacity(items.len());
            for h in handles {
                if let Ok(mut part) = h.join() {
                    results.append(&mut part);
                }
            }
            results
        })
    }

    pub fn extract_and_cache(
        &self,
        file_id: i64,
        zip_path: &Path,
        max_width: u32,
    ) -> Result<PathBuf, String> {
        let key = format!("{}.webp", file_id);
        let out_path = self.cache_dir.join(&key);
        if self.has_file_cover(file_id) || out_path.exists() {
            if let Ok(mut ex) = self.existing_files.write() {
                ex.insert(key);
            }
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
        if self.has_folder_cover(folder_id) || out_path.exists() {
            if let Ok(mut ex) = self.existing_files.write() {
                ex.insert(key);
            }
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
        let folder_key = format!("folder_{}.webp", folder_id);
        if let Some(url) = self.get_cached_by_key(&folder_key) {
            return Some(url);
        }

        let comic_path = self.get_cache_path(comic_file_id);
        let folder_path = self.get_folder_cache_path(folder_id);
        if comic_path.exists() {
            if std::fs::copy(&comic_path, &folder_path).is_ok() {
                if let Ok(mut ex) = self.existing_files.write() {
                    ex.insert(folder_key.clone());
                }
                return self.get_cached_by_key(&folder_key);
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

        let b64 = BASE64_STANDARD.encode(&raw_webp);
        let url = format!("data:image/webp;base64,{}", b64);

        if let Ok(mut ex) = self.existing_files.write() {
            ex.insert(key.to_string());
        }
        if let Ok(mut cache) = self.data_url_cache.write() {
            cache.insert(key.to_string(), url);
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
}
