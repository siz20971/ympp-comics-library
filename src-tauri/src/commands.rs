use chrono::{Duration, Utc};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;

use crate::cover::CoverExtractor;
use crate::db::Database;
use crate::launcher::Launcher;
use crate::models::{
    AppSettings, ComicDetail, ComicFileItem, FolderContent, FolderDetail, FolderItem, Library,
    ScanProgress, SearchQuery, TagDetailItem, TagItem, UpdateCheckResult,
};
use crate::path_utils;
use crate::scanner::Scanner;
use crate::updater;

#[derive(Clone)]
pub struct LibraryContext {
    pub lib_root: PathBuf,
    pub cache_dir: PathBuf,
    pub db: Arc<Database>,
    pub cover_extractor: Arc<CoverExtractor>,
    pub scanner: Arc<Scanner>,
}

pub struct AppState {
    pub app_dir: PathBuf,
    pub settings_path: PathBuf,
    pub active_context: Arc<RwLock<Option<LibraryContext>>>,
    pub nav_generation: Arc<AtomicU64>,
}

impl AppState {
    pub async fn get_context(&self) -> Result<LibraryContext, String> {
        let guard = self.active_context.read().await;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "라이브러리가 선택되지 않았습니다. 폴더를 먼저 선택해주세요.".to_string())
    }

    pub async fn switch_library(&self, portable_or_abs_path: &str) -> Result<LibraryContext, String> {
        self.nav_generation.fetch_add(1, Ordering::AcqRel);
        let abs_lib_root = path_utils::resolve_to_absolute(portable_or_abs_path, &self.app_dir);

        let ctx = tokio::task::spawn_blocking(move || -> Result<LibraryContext, String> {
            if !abs_lib_root.exists() || !abs_lib_root.is_dir() {
                return Err(format!(
                    "라이브러리 디렉터리를 찾을 수 없습니다: {}",
                    abs_lib_root.display()
                ));
            }

            // Library folder uses .ymppCache for portable db and covers
            let legacy_cache_dir = abs_lib_root.join(".uhheuCache");
            let cache_dir = abs_lib_root.join(".ymppCache");
            if !cache_dir.exists() && legacy_cache_dir.exists() {
                let _ = std::fs::rename(&legacy_cache_dir, &cache_dir);
            }
            let covers_dir = cache_dir.join("covers");
            let db_path = cache_dir.join("comics_library.db");

            std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

            let db = Arc::new(Database::new(db_path).map_err(|e| e.to_string())?);
            let cover_extractor = Arc::new(CoverExtractor::new(covers_dir));
            let scanner = Arc::new(Scanner::new(db.clone(), cover_extractor.clone()));

            Ok(LibraryContext {
                lib_root: abs_lib_root,
                cache_dir,
                db,
                cover_extractor,
                scanner,
            })
        })
        .await
        .map_err(|e| e.to_string())??;

        let mut guard = self.active_context.write().await;
        *guard = Some(ctx.clone());
        drop(guard);

        // Warm cover cache index asynchronously in the background without blocking startup
        let cover_bg = ctx.cover_extractor.clone();
        tokio::task::spawn_blocking(move || {
            cover_bg.warm_existing_files();
        });

        Ok(ctx)
    }

    pub fn load_settings(&self) -> AppSettings {
        if self.settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.settings_path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        AppSettings {
            library_path: None,
            custom_viewer_path: None,
            auto_clear_new_on_open: true,
            thumbnail_size: 480,
            card_size: "md".to_string(),
            include_folder_tags_in_search: false,
            show_folder_new_badge: false,
            update_snooze_until: None,
            ignore_updates_permanently: false,
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        if let Some(parent) = self.settings_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        std::fs::write(&self.settings_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tauri::command]
pub async fn get_initial_state(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings = state.load_settings();
    let mut library: Option<Library> = None;

    if let Some(ref saved_path) = settings.library_path {
        if let Ok(ctx) = state.switch_library(saved_path).await {
            let lib_root_str = ctx.lib_root.to_string_lossy().to_string();
            if let Ok(mut lib) = ctx.db.get_or_create_library(&lib_root_str) {
                lib.display_path = Some(lib_root_str);
                library = Some(lib);
            }
        }
    }

    Ok(serde_json::json!({
        "settings": settings,
        "library": library,
    }))
}

#[tauri::command]
pub async fn set_library_path(path: String, state: State<'_, AppState>) -> Result<Library, String> {
    let target_path = Path::new(&path);
    let portable_rel_path = path_utils::to_relative_path(target_path, &state.app_dir);

    let mut settings = state.load_settings();
    settings.library_path = Some(portable_rel_path.clone());
    state.save_settings(&settings)?;

    let ctx = state.switch_library(&portable_rel_path).await?;
    let lib_root_str = ctx.lib_root.to_string_lossy().to_string();
    let mut lib = ctx.db.get_or_create_library(&lib_root_str).map_err(|e| e.to_string())?;
    lib.display_path = Some(lib_root_str);
    Ok(lib)
}

#[tauri::command]
pub async fn start_scan(path: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let ctx = match path {
        Some(p) => state.switch_library(&p).await?,
        None => state.get_context().await?,
    };

    let scanner = ctx.scanner.clone();
    let root_str = ctx.lib_root.to_string_lossy().to_string();

    tokio::task::spawn_blocking(move || {
        scanner.scan_library_blocking(&root_str)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, AppState>) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.scanner.cancel();
    Ok(())
}

#[tauri::command]
pub async fn get_scan_progress(state: State<'_, AppState>) -> Result<ScanProgress, String> {
    let ctx = state.get_context().await?;
    Ok(ctx.scanner.get_progress())
}

#[tauri::command]
pub async fn get_folder_content(
    folder_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<FolderContent, String> {
    let ctx = state.get_context().await?;
    state.nav_generation.fetch_add(1, Ordering::AcqRel);

    tokio::task::spawn_blocking(move || {
        let mut content = ctx
            .db
            .get_folder_content_bundle(folder_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "라이브러리가 설정되지 않았습니다.".to_string())?;

        if let Some(ref mut cf) = content.current_folder {
            if cf.cover_status == "ready" || ctx.cover_extractor.has_folder_cover(cf.id) {
                cf.cover_data_url = Some(CoverExtractor::folder_cover_uri(cf.id));
            }
        }

        let newly_ready_ids = ctx.cover_extractor.populate_folder_covers_uris(&mut content.subfolders);
        if !newly_ready_ids.is_empty() {
            let _ = ctx.db.batch_mark_folders_cover_ready(&newly_ready_ids);
        }

        ctx.cover_extractor.populate_file_covers_uris(&mut content.files);

        Ok(content)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn resolve_single_folder_cover(ctx: &LibraryContext, folder_id: i64) -> Option<String> {
    if let Some(url) = ctx.cover_extractor.get_folder_cover_uri_if_exists(folder_id) {
        let _ = ctx.db.update_folder_cover_status(folder_id, "ready");
        return Some(url);
    }
    if ctx.cover_extractor.is_known_empty_folder(folder_id) {
        return None;
    }

    let (rel_opt, first_comic_id, _) = ctx.db.get_folder_cover_candidates(folder_id).ok()?;
    let rel_path = rel_opt?;
    if rel_path.is_empty() {
        ctx.cover_extractor.mark_empty_folder(folder_id);
        let _ = ctx.db.update_folder_cover_status(folder_id, "empty");
        return None;
    }

    // 1. If there is a comic file inside this folder (or its descendants) with a ready cover, reuse it quickly
    if let Some(cid) = first_comic_id {
        if let Some(url) = ctx.cover_extractor.copy_comic_cover_to_folder(cid, folder_id) {
            let _ = ctx.db.update_folder_cover_status(folder_id, "ready");
            return Some(url);
        }
    }

    // 2. Otherwise check if the folder itself (or up to 2 levels of subdirectories) contains loose image files
    let full_folder_path = ctx.lib_root.join(&rel_path);
    if ctx
        .cover_extractor
        .extract_folder_cover_from_dir(folder_id, &full_folder_path, 480)
        .is_ok()
    {
        let _ = ctx.db.update_folder_cover_status(folder_id, "ready");
        return Some(CoverExtractor::folder_cover_uri(folder_id));
    }

    ctx.cover_extractor.mark_empty_folder(folder_id);
    let _ = ctx.db.update_folder_cover_status(folder_id, "empty");
    None
}

#[tauri::command]
pub async fn resolve_folder_covers(
    folder_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<(i64, String)>, String> {
    let ctx = state.get_context().await?;
    let nav_gen = state.nav_generation.clone();
    let expected_gen = nav_gen.load(Ordering::Acquire);

    tokio::task::spawn_blocking(move || {
        let mut resolved = Vec::new();
        for fid in folder_ids {
            // Abort remaining work immediately if the user navigated to another folder
            if nav_gen.load(Ordering::Acquire) != expected_gen {
                break;
            }
            if let Some(url) = resolve_single_folder_cover(&ctx, fid) {
                resolved.push((fid, url));
            }
        }
        Ok(resolved)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn refresh_item(
    item_type: String,
    id: i64,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let ctx = state.get_context().await?;
    let version = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    tokio::task::spawn_blocking(move || {
        match item_type.as_str() {
            "comic" => {
                ctx.cover_extractor.invalidate_file_cover(id);
                let _ = ctx.db.delete_cover_cache(id);

                if let Ok(Some(detail)) = ctx.db.get_comic_detail(id, &ctx.lib_root.to_string_lossy()) {
                    let zip_path = ctx.lib_root.join(&detail.relative_path);
                    if let Ok(cache_path) = ctx.cover_extractor.extract_and_cache(id, &zip_path, 480) {
                        let _ = ctx.db.upsert_cover_cache(
                            id,
                            &cache_path.to_string_lossy(),
                            &detail.modified_at,
                            "ready",
                            None,
                        );
                    }
                }
                if ctx.cover_extractor.has_file_cover(id) {
                    Ok(Some(CoverExtractor::file_cover_uri_versioned(id, version)))
                } else {
                    Ok(None)
                }
            }
            "folder" => {
                ctx.cover_extractor.invalidate_folder_cover(id);
                let _ = ctx.db.update_folder_cover_status(id, "unknown");
                if resolve_single_folder_cover(&ctx, id).is_some() {
                    Ok(Some(CoverExtractor::folder_cover_uri_versioned(id, version)))
                } else {
                    Ok(None)
                }
            }
            _ => Err("Invalid item type".to_string()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_comics(
    query: SearchQuery,
    state: State<'_, AppState>,
) -> Result<Vec<ComicFileItem>, String> {
    let ctx = state.get_context().await?;
    tokio::task::spawn_blocking(move || {
        let lib = ctx
            .db
            .get_latest_library()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "라이브러리가 설정되지 않았습니다.".to_string())?;

        let mut results = ctx.db.search_comics(lib.id, &query).map_err(|e| e.to_string())?;
        ctx.cover_extractor.populate_file_covers_uris(&mut results);

        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_folders(
    query: SearchQuery,
    state: State<'_, AppState>,
) -> Result<Vec<FolderItem>, String> {
    let ctx = state.get_context().await?;
    tokio::task::spawn_blocking(move || {
        let lib = ctx
            .db
            .get_latest_library()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "라이브러리가 설정되지 않았습니다.".to_string())?;

        let mut results = ctx.db.search_folders(lib.id, &query).map_err(|e| e.to_string())?;
        let newly_ready = ctx.cover_extractor.populate_folder_covers_uris(&mut results);
        if !newly_ready.is_empty() {
            let _ = ctx.db.batch_mark_folders_cover_ready(&newly_ready);
        }

        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_comic_detail(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<ComicDetail>, String> {
    let ctx = state.get_context().await?;
    tokio::task::spawn_blocking(move || {
        let lib_root_str = ctx.lib_root.to_string_lossy().to_string();
        let mut detail = ctx.db.get_comic_detail(file_id, &lib_root_str).map_err(|e| e.to_string())?;

        if let Some(ref mut d) = detail {
            d.cover_data_url = ctx.cover_extractor.get_file_cover_uri_if_exists(file_id);
        }

        Ok(detail)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_folder_detail(
    folder_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FolderDetail>, String> {
    let ctx = state.get_context().await?;
    tokio::task::spawn_blocking(move || {
        let lib_root_str = ctx.lib_root.to_string_lossy().to_string();
        let mut detail = ctx.db.get_folder_detail(folder_id, &lib_root_str).map_err(|e| e.to_string())?;
        if let Some(ref mut d) = detail {
            d.cover_data_url = resolve_single_folder_cover(&ctx, folder_id);
        }
        Ok(detail)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_comic_rating(
    file_id: i64,
    rating: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.update_comic_rating(file_id, rating).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_folder_rating(
    folder_id: i64,
    rating: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.update_folder_rating(folder_id, rating).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_comic_is_new(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let ctx = state.get_context().await?;
    ctx.db.toggle_comic_is_new(file_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_folder_is_new(
    folder_id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let ctx = state.get_context().await?;
    ctx.db.toggle_folder_is_new(folder_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_comic_comment(
    file_id: i64,
    comment: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.update_comic_comment(file_id, &comment).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_comic_tags(
    file_id: i64,
    tag_names: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    tokio::task::spawn_blocking(move || ctx.db.set_comic_tags(file_id, &tag_names).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_folder_tags(
    folder_id: i64,
    tag_names: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    tokio::task::spawn_blocking(move || ctx.db.set_folder_tags(folder_id, &tag_names).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_tag_to_comic(
    file_id: i64,
    tag_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.add_tag_to_comic(file_id, &tag_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_tag_from_comic(
    file_id: i64,
    tag_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.remove_tag_from_comic(file_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tag_to_folder(
    folder_id: i64,
    tag_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.add_tag_to_folder(folder_id, &tag_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_tag_from_folder(
    folder_id: i64,
    tag_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.remove_tag_from_folder(folder_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_comic_file(file_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let ctx = state.get_context().await?;
    let lib_root_str = ctx.lib_root.to_string_lossy().to_string();

    let detail = ctx
        .db
        .get_comic_detail(file_id, &lib_root_str)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "파일을 찾을 수 없습니다.".to_string())?;

    let settings = state.load_settings();

    let resolved_viewer = settings.custom_viewer_path.as_ref().map(|v| {
        path_utils::resolve_to_absolute(v, &state.app_dir)
            .to_string_lossy()
            .to_string()
    });

    Launcher::open_file(&detail.full_path, resolved_viewer.as_deref())?;

    if settings.auto_clear_new_on_open {
        ctx.db.mark_comic_as_read(file_id).ok();
    }

    Ok(())
}

#[tauri::command]
pub async fn open_folder_in_viewer(folder_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let ctx = state.get_context().await?;
    let lib_root_str = ctx.lib_root.to_string_lossy().to_string();

    let detail = ctx
        .db
        .get_folder_detail(folder_id, &lib_root_str)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "폴더를 찾을 수 없습니다.".to_string())?;

    let settings = state.load_settings();

    let resolved_viewer = settings.custom_viewer_path.as_ref().map(|v| {
        path_utils::resolve_to_absolute(v, &state.app_dir)
            .to_string_lossy()
            .to_string()
    });

    Launcher::open_file(&detail.full_path, resolved_viewer.as_deref())?;
    Ok(())
}

#[tauri::command]
pub async fn show_in_file_manager(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let clean = Launcher::normalize_windows_path(&path);
    let p = Path::new(&clean);
    let resolved = if p.is_absolute() && p.exists() {
        p.to_path_buf()
    } else if let Ok(ctx) = state.get_context().await {
        let candidate = ctx.lib_root.join(Path::new(&path));
        if candidate.exists() {
            candidate
        } else {
            let candidate_clean = ctx.lib_root.join(p);
            if candidate_clean.exists() {
                candidate_clean
            } else {
                path_utils::resolve_to_absolute(&clean, &state.app_dir)
            }
        }
    } else {
        path_utils::resolve_to_absolute(&clean, &state.app_dir)
    };

    Launcher::show_in_file_manager(&resolved.to_string_lossy())
}

#[tauri::command]
pub async fn get_cover_data_url(file_id: i64, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let ctx = state.get_context().await?;
    Ok(ctx.cover_extractor.get_cached_file_cover(file_id))
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> AppSettings {
    state.load_settings()
}

#[tauri::command]
pub fn save_app_settings(mut settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(ref v) = settings.custom_viewer_path {
        let rel_v = path_utils::to_relative_path(Path::new(v), &state.app_dir);
        settings.custom_viewer_path = Some(rel_v);
    }
    state.save_settings(&settings)
}

// --- Tag Management Commands ---
#[tauri::command]
pub async fn get_all_tags(state: State<'_, AppState>) -> Result<Vec<TagItem>, String> {
    let ctx = state.get_context().await?;
    ctx.db.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tags_with_counts(state: State<'_, AppState>) -> Result<Vec<TagDetailItem>, String> {
    let ctx = state.get_context().await?;
    ctx.db.get_tags_with_counts().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_tag(name: String, state: State<'_, AppState>) -> Result<TagItem, String> {
    let ctx = state.get_context().await?;
    ctx.db.get_or_create_tag(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tag(id: i64, new_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.update_tag(id, &new_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let ctx = state.get_context().await?;
    ctx.db.delete_tag(id).map_err(|e| e.to_string())
}

// --- Rebuild Database Command ---
#[tauri::command]
pub async fn rebuild_library_cache(state: State<'_, AppState>) -> Result<(), String> {
    let ctx = state.get_context().await?;

    // 1. Wipe database tables
    ctx.db.rebuild_database().map_err(|e| e.to_string())?;

    // 2. Clear cover cache directory
    ctx.cover_extractor.clear_memory_caches();
    if ctx.cache_dir.join("covers").exists() {
        if let Ok(entries) = std::fs::read_dir(ctx.cache_dir.join("covers")) {
            for entry in entries.flatten() {
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }

    // 3. Re-run scan
    let root_str = ctx.lib_root.to_string_lossy().to_string();
    let scanner = ctx.scanner.clone();

    tokio::task::spawn_blocking(move || {
        scanner.scan_library_blocking(&root_str)
    })
    .await
    .map_err(|e| e.to_string())?
}

// --- Auto-Update Commands ---
#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    updater::check_for_updates().await
}

#[tauri::command]
pub async fn download_and_apply_update(
    app: AppHandle,
    download_url: String,
    expected_size: Option<u64>,
) -> Result<(), String> {
    updater::download_and_apply_update(app, download_url, expected_size).await
}

#[tauri::command]
pub async fn snooze_update_week(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = state.load_settings();
    let until = Utc::now() + Duration::days(7);
    settings.update_snooze_until = Some(until.to_rfc3339());
    state.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub async fn ignore_update_forever(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = state.load_settings();
    settings.ignore_updates_permanently = true;
    state.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub async fn clear_update_suppression(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = state.load_settings();
    settings.update_snooze_until = None;
    settings.ignore_updates_permanently = false;
    state.save_settings(&settings)?;
    Ok(settings)
}
