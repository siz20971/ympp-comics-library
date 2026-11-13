export interface Library {
  id: number;
  root_path: string;
  display_path?: string | null;
  created_at: string;
  updated_at: string;
  last_scanned_at: string | null;
}

export interface TagItem {
  id: number;
  name: string;
  display_name: string;
  is_system: boolean;
}

export interface TagDetailItem {
  id: number;
  name: string;
  display_name: string;
  is_system: boolean;
  file_count: number;
  folder_count: number;
}

export interface FolderItem {
  id: number;
  library_id: number;
  relative_path: string;
  name: string;
  parent_id: number | null;
  rating: number;
  is_new: boolean;
  cover_status?: string;
  cover_data_url: string | null;
  direct_tags: TagItem[];
}

export interface FolderDetail {
  id: number;
  library_id: number;
  relative_path: string;
  full_path: string;
  name: string;
  parent_id: number | null;
  rating: number;
  is_new: boolean;
  cover_data_url: string | null;
  direct_tags: TagItem[];
  subfolder_count: number;
  file_count: number;
}

export interface ComicFileItem {
  id: number;
  folder_id: number;
  relative_path: string;
  name: string;
  extension: string;
  size: number;
  modified_at: string;
  rating: number;
  comment: string;
  is_new: boolean;
  status: string;
  cover_cache_path: string | null;
  cover_data_url: string | null;
  direct_tags: TagItem[];
  inherited_tags: TagItem[];
}

export interface BreadcrumbItem {
  id: number | null;
  name: string;
  relative_path: string;
}

export interface FolderContent {
  current_folder: FolderItem | null;
  parent_folder: FolderItem | null;
  subfolders: FolderItem[];
  files: ComicFileItem[];
  breadcrumbs: BreadcrumbItem[];
}

export interface ComicDetail {
  id: number;
  folder_id: number;
  folder_name: string;
  folder_rating: number;
  full_path: string;
  relative_path: string;
  name: string;
  extension: string;
  size: number;
  modified_at: string;
  created_at: string;
  rating: number;
  comment: string;
  is_new: boolean;
  status: string;
  cover_data_url: string | null;
  direct_tags: TagItem[];
  inherited_tags: TagItem[];
}

export interface SearchQuery {
  keywords: string[];
  keyword_operator: 'AND' | 'OR';
  tag_names: string[];
  tag_operator: 'AND' | 'OR';
  group_operator: 'AND' | 'OR';
  min_rating: number | null;
  max_rating: number | null;
  extension: string | null;
  sort_by: 'name' | 'rating' | 'date' | 'size';
  sort_order: 'asc' | 'desc';
  include_folder_tags?: boolean;
}

export interface ScanProgress {
  is_scanning: boolean;
  current_status: string;
  total_files_found: number;
  processed_files: number;
  covers_extracted: number;
  last_error: string | null;
}

export type CardSizeMode = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

export interface AppSettings {
  library_path: string | null;
  custom_viewer_path: string | null;
  auto_clear_new_on_open: boolean;
  thumbnail_size: number;
  card_size: CardSizeMode;
  include_folder_tags_in_search: boolean;
  show_folder_new_badge: boolean;
  update_snooze_until?: string | null;
  ignore_updates_permanently?: boolean;
}

export interface ReleaseNoteEntry {
  version: string;
  title: string;
  published_at: string;
  body: string;
  html_url: string;
}

export interface UpdateCheckResult {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  download_url: string | null;
  asset_name: string | null;
  asset_size: number | null;
  html_url: string;
  releases: ReleaseNoteEntry[];
  is_dev_build: boolean;
}

export interface UpdateDownloadProgress {
  downloaded: number;
  total: number | null;
  percent: number;
  stage: string;
}

export interface SelectableTagOption {
  key: string;
  display_name: string;
  id: number;
}

export interface TagPickerContext {
  mode: 'search' | 'comic' | 'folder';
  targetId?: number;
  targetTitle?: string;
  initialTags: string[];
}
