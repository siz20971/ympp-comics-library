pub mod commands;
pub mod cover;
pub mod db;
pub mod launcher;
pub mod models;
pub mod path_utils;
pub mod scanner;

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use commands::AppState;
use tauri::Manager;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_dir = path_utils::get_app_dir();
    let settings_path = app_dir.join("settings.json");

    let app_state = AppState {
        app_dir,
        settings_path,
        active_context: Arc::new(RwLock::new(None)),
        nav_generation: Arc::new(AtomicU64::new(0)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .register_asynchronous_uri_scheme_protocol("ymppcover", |ctx, request, responder| {
            let app_handle = ctx.app_handle().clone();
            let uri_path = request.uri().path().trim_start_matches('/').to_string();

            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                let lib_ctx = state.active_context.read().await.clone();

                let bytes_opt = if let Some(c) = lib_ctx {
                    tokio::task::spawn_blocking(move || {
                        c.cover_extractor.read_cover_bytes_by_uri_name(&uri_path)
                    })
                    .await
                    .ok()
                    .flatten()
                } else {
                    None
                };

                let response = match bytes_opt {
                    Some(bytes) => tauri::http::Response::builder()
                        .status(200)
                        .header("Content-Type", "image/webp")
                        .header("Cache-Control", "public, max-age=86400")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(bytes)
                        .unwrap(),
                    None => tauri::http::Response::builder()
                        .status(404)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Vec::new())
                        .unwrap(),
                };
                responder.respond(response);
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_initial_state,
            commands::set_library_path,
            commands::start_scan,
            commands::cancel_scan,
            commands::get_scan_progress,
            commands::get_folder_content,
            commands::resolve_folder_covers,
            commands::refresh_item,
            commands::search_comics,
            commands::search_folders,
            commands::get_comic_detail,
            commands::get_folder_detail,
            commands::update_comic_rating,
            commands::update_folder_rating,
            commands::toggle_comic_is_new,
            commands::toggle_folder_is_new,
            commands::update_comic_comment,
            commands::set_comic_tags,
            commands::set_folder_tags,
            commands::add_tag_to_comic,
            commands::remove_tag_from_comic,
            commands::add_tag_to_folder,
            commands::remove_tag_from_folder,
            commands::open_comic_file,
            commands::open_folder_in_viewer,
            commands::show_in_file_manager,
            commands::get_cover_data_url,
            commands::get_app_settings,
            commands::save_app_settings,
            commands::get_all_tags,
            commands::get_tags_with_counts,
            commands::create_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::rebuild_library_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
