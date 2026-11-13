use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: i64,
    pub root_path: String,
    pub display_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderItem {
    pub id: i64,
    pub library_id: i64,
    pub relative_path: String,
    pub name: String,
    pub parent_id: Option<i64>,
    pub rating: i32,
    pub is_new: bool,
    #[serde(default = "default_cover_status")]
    pub cover_status: String,
    pub cover_data_url: Option<String>,
    pub direct_tags: Vec<TagItem>,
}

fn default_cover_status() -> String {
    "unknown".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComicFileItem {
    pub id: i64,
    pub folder_id: i64,
    pub relative_path: String,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub modified_at: String,
    pub rating: i32,
    pub comment: String,
    pub is_new: bool,
    pub status: String,
    pub cover_cache_path: Option<String>,
    pub cover_data_url: Option<String>,
    pub direct_tags: Vec<TagItem>,
    pub inherited_tags: Vec<TagItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagItem {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDetailItem {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub is_system: bool,
    pub file_count: i64,
    pub folder_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderDetail {
    pub id: i64,
    pub library_id: i64,
    pub relative_path: String,
    pub full_path: String,
    pub name: String,
    pub parent_id: Option<i64>,
    pub rating: i32,
    pub is_new: bool,
    pub cover_data_url: Option<String>,
    pub direct_tags: Vec<TagItem>,
    pub subfolder_count: usize,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderContent {
    pub current_folder: Option<FolderItem>,
    pub parent_folder: Option<FolderItem>,
    pub subfolders: Vec<FolderItem>,
    pub files: Vec<ComicFileItem>,
    pub breadcrumbs: Vec<BreadcrumbItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadcrumbItem {
    pub id: Option<i64>,
    pub name: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComicDetail {
    pub id: i64,
    pub folder_id: i64,
    pub folder_name: String,
    pub folder_rating: i32,
    pub full_path: String,
    pub relative_path: String,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub modified_at: String,
    pub created_at: String,
    pub rating: i32,
    pub comment: String,
    pub is_new: bool,
    pub status: String,
    pub cover_data_url: Option<String>,
    pub direct_tags: Vec<TagItem>,
    pub inherited_tags: Vec<TagItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub keywords: Vec<String>,
    pub keyword_operator: String, // "AND" | "OR"
    pub tag_names: Vec<String>,
    pub tag_operator: String,     // "AND" | "OR"
    pub group_operator: String,   // "AND" | "OR"
    pub min_rating: Option<i32>,
    pub max_rating: Option<i32>,
    pub extension: Option<String>,
    pub sort_by: String,          // "name" | "rating" | "date" | "size"
    pub sort_order: String,       // "asc" | "desc"
    pub include_folder_tags: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub is_scanning: bool,
    pub current_status: String,
    pub total_files_found: usize,
    pub processed_files: usize,
    pub covers_extracted: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub library_path: Option<String>,
    pub custom_viewer_path: Option<String>,
    pub auto_clear_new_on_open: bool,
    pub thumbnail_size: u32,
    #[serde(default = "default_card_size")]
    pub card_size: String,
    #[serde(default = "default_include_folder_tags")]
    pub include_folder_tags_in_search: bool,
    #[serde(default = "default_show_folder_new_badge")]
    pub show_folder_new_badge: bool,
    #[serde(default)]
    pub update_snooze_until: Option<String>,
    #[serde(default)]
    pub ignore_updates_permanently: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseNoteEntry {
    pub version: String,
    pub title: String,
    pub published_at: String,
    pub body: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
    pub html_url: String,
    pub releases: Vec<ReleaseNoteEntry>,
    #[serde(default)]
    pub is_dev_build: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: f64,
    pub stage: String,
}

fn default_card_size() -> String {
    "md".to_string()
}

fn default_include_folder_tags() -> bool {
    false
}

fn default_show_folder_new_badge() -> bool {
    false
}
