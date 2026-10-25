use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

use crate::cover::CoverExtractor;
use crate::db::Database;
use crate::models::ScanProgress;

pub struct Scanner {
    db: Arc<Database>,
    cover_extractor: Arc<CoverExtractor>,
    progress: Arc<Mutex<ScanProgress>>,
    is_cancelled: Arc<AtomicBool>,
}

impl Scanner {
    pub fn new(db: Arc<Database>, cover_extractor: Arc<CoverExtractor>) -> Self {
        Self {
            db,
            cover_extractor,
            progress: Arc::new(Mutex::new(ScanProgress {
                is_scanning: false,
                current_status: "대기 중".to_string(),
                total_files_found: 0,
                processed_files: 0,
                covers_extracted: 0,
                last_error: None,
            })),
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_progress(&self) -> ScanProgress {
        let p = self.progress.lock().unwrap();
        p.clone()
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn scan_library_blocking(&self, root_path_str: &str) -> Result<(), String> {
        let root = Path::new(root_path_str);
        if !root.exists() || !root.is_dir() {
            let err = format!("라이브러리 경로가 존재하지 않거나 디렉터리가 아닙니다: {}", root_path_str);
            self.set_error(&err);
            return Err(err);
        }

        self.is_cancelled.store(false, Ordering::SeqCst);
        {
            let mut p = self.progress.lock().unwrap();
            p.is_scanning = true;
            p.current_status = "라이브러리 등록 및 초기화 중...".to_string();
            p.total_files_found = 0;
            p.processed_files = 0;
            p.covers_extracted = 0;
            p.last_error = None;
        }

        let lib = self.db.get_or_create_library(root_path_str).map_err(|e| e.to_string())?;

        // 1. Gather all directories and zip/cbz files
        {
            let mut p = self.progress.lock().unwrap();
            p.current_status = "파일 시스템 스캔 중...".to_string();
        }

        let mut discovered_files: Vec<(PathBuf, String, String, i64, String)> = Vec::new();
        let mut discovered_folders: Vec<(PathBuf, String, String)> = Vec::new();

        // Register root folder itself as relative_path = ""
        let root_folder_id = self.db.upsert_folder(lib.id, "", "Root", None).map_err(|e| e.to_string())?;

        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Exclude .ymppCache, .uhheuCache and .git directories from scan
                if e.file_type().is_dir() && (name == ".ymppCache" || name == ".uhheuCache" || name == ".git" || name == "$RECYCLE.BIN") {
                    return false;
                }
                true
            });

        for entry_res in walker {
            if self.is_cancelled.load(Ordering::SeqCst) {
                let mut p = self.progress.lock().unwrap();
                p.current_status = "스캔이 취소되었습니다.".to_string();
                p.is_scanning = false;
                return Ok(());
            }

            let entry = match entry_res {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Scan warning: {}", err);
                    continue;
                }
            };

            let path = entry.path();
            if path == root {
                continue;
            }

            let rel_path = match path.strip_prefix(root) {
                Ok(rp) => rp.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };

            let file_name = entry.file_name().to_string_lossy().to_string();

            if entry.file_type().is_dir() {
                discovered_folders.push((path.to_path_buf(), rel_path, file_name));
            } else if entry.file_type().is_file() {
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if ext == "zip" || ext == "cbz" {
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
                    let mod_time = metadata
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                    discovered_files.push((path.to_path_buf(), rel_path, file_name, size, mod_time));
                }
            }
        }

        // Sort folders by path depth so parents are created before children
        discovered_folders.sort_by_key(|f| f.1.matches('/').count());

        // 2. Upsert folders in database
        {
            let mut p = self.progress.lock().unwrap();
            p.current_status = format!("{}개 폴더 구조 인덱싱 중...", discovered_folders.len());
        }

        let mut upserted_folders: Vec<(i64, PathBuf)> = Vec::with_capacity(discovered_folders.len());
        for (folder_path, rel_path, name) in &discovered_folders {
            let parent_rel = if let Some(last_slash) = rel_path.rfind('/') {
                &rel_path[..last_slash]
            } else {
                ""
            };

            let parent_id = if parent_rel.is_empty() {
                Some(root_folder_id)
            } else {
                let p_stmt = self.db.conn().map_err(|e| e.to_string())?;
                let mut stmt = p_stmt
                    .prepare("SELECT id FROM folders WHERE library_id = ?1 AND relative_path = ?2")
                    .map_err(|e| e.to_string())?;
                stmt.query_row(rusqlite::params![lib.id, parent_rel], |r| r.get(0)).ok()
            };

            let fid = self.db.upsert_folder(lib.id, rel_path, name, parent_id).map_err(|e| e.to_string())?;
            upserted_folders.push((fid, folder_path.clone()));
        }

        // 3. Upsert files and prepare cover extraction queue
        let total_files = discovered_files.len();
        {
            let mut p = self.progress.lock().unwrap();
            p.total_files_found = total_files;
            p.current_status = format!("{}개 만화책 파일 메타데이터 등록 중...", total_files);
        }

        let mut needs_cover_files: Vec<(i64, PathBuf, String)> = Vec::new();

        for (idx, (full_path, rel_path, name, size, mod_time)) in discovered_files.into_iter().enumerate() {
            if self.is_cancelled.load(Ordering::SeqCst) {
                break;
            }

            let parent_rel = if let Some(last_slash) = rel_path.rfind('/') {
                &rel_path[..last_slash]
            } else {
                ""
            };

            let folder_id = if parent_rel.is_empty() {
                root_folder_id
            } else {
                let conn = self.db.conn().map_err(|e| e.to_string())?;
                let mut stmt = conn
                    .prepare("SELECT id FROM folders WHERE library_id = ?1 AND relative_path = ?2")
                    .map_err(|e| e.to_string())?;
                stmt.query_row(rusqlite::params![lib.id, parent_rel], |r| r.get(0))
                    .unwrap_or(root_folder_id)
            };

            let ext = full_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            let (file_id, file_changed) = self
                .db
                .upsert_comic_file(folder_id, &rel_path, &name, &ext, size, &mod_time)
                .map_err(|e| e.to_string())?;

            let cache_exists = self.cover_extractor.has_file_cover(file_id);

            if file_changed || !cache_exists {
                needs_cover_files.push((file_id, full_path, mod_time));
            }

            if idx % 100 == 0 || idx + 1 == total_files {
                let mut p = self.progress.lock().unwrap();
                p.processed_files = idx + 1;
            }
        }

        // 4. Extract covers
        let total_covers = needs_cover_files.len();
        {
            let mut p = self.progress.lock().unwrap();
            p.current_status = format!("표지 썸네일 추출 중 (0/{})...", total_covers);
        }

        for (c_idx, (file_id, zip_path, mod_time)) in needs_cover_files.into_iter().enumerate() {
            if self.is_cancelled.load(Ordering::SeqCst) {
                break;
            }

            match self.cover_extractor.extract_and_cache(file_id, &zip_path, 480) {
                Ok(cache_path) => {
                    self.db
                        .upsert_cover_cache(file_id, &cache_path.to_string_lossy(), &mod_time, "ready", None)
                        .ok();
                }
                Err(err) => {
                    self.db
                        .upsert_cover_cache(file_id, "", &mod_time, "failed", Some(&err))
                        .ok();
                }
            }

            if c_idx % 10 == 0 || c_idx + 1 == total_covers {
                let mut p = self.progress.lock().unwrap();
                p.covers_extracted = c_idx + 1;
                p.current_status = format!("표지 썸네일 추출 중 ({}/{})...", c_idx + 1, total_covers);
            }
        }

        // 5. Extract folder covers (for image-only comic folders and series folders)
        for (fid, folder_path) in upserted_folders {
            if self.is_cancelled.load(Ordering::SeqCst) {
                break;
            }
            if !self.cover_extractor.has_folder_cover(fid) {
                if self.cover_extractor.extract_folder_cover_from_dir(fid, &folder_path, 480).is_err() {
                    if let Ok((_, Some(cid), _)) = self.db.get_folder_cover_candidates(fid) {
                        self.cover_extractor.copy_comic_cover_to_folder(cid, fid);
                    }
                }
            }
        }

        self.db.update_library_scan_time(lib.id).map_err(|e| e.to_string())?;

        {
            let mut p = self.progress.lock().unwrap();
            p.is_scanning = false;
            p.current_status = "인덱싱 완료".to_string();
        }

        Ok(())
    }

    fn set_error(&self, err: &str) {
        let mut p = self.progress.lock().unwrap();
        p.is_scanning = false;
        p.last_error = Some(err.to_string());
        p.current_status = format!("오류 발생: {}", err);
    }
}
