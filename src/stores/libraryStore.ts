import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  Library,
  FolderContent,
  FolderDetail,
  FolderItem,
  ComicDetail,
  ComicFileItem,
  SearchQuery,
  ScanProgress,
  AppSettings,
  TagItem,
  TagDetailItem,
  SelectableTagOption,
  TagPickerContext,
  UpdateCheckResult,
  UpdateDownloadProgress,
} from '../types';

export function cleanTagDisplay(raw: string): string {
  return raw.trim().replace(/^#+/, '').trim();
}

export function normalizeTagKey(raw: string): string {
  return cleanTagDisplay(raw).toLowerCase();
}

export const useLibraryStore = defineStore('library', () => {
  // State
  const library = ref<Library | null>(null);
  const settings = ref<AppSettings>({
    library_path: null,
    custom_viewer_path: null,
    auto_clear_new_on_open: true,
    thumbnail_size: 480,
    card_size: 'md',
    include_folder_tags_in_search: false,
    show_folder_new_badge: false,
    update_snooze_until: null,
    ignore_updates_permanently: false,
  });

  const cardMinWidth = computed(() => {
    switch (settings.value.card_size) {
      case 'xs':
        return 110;
      case 'sm':
        return 140;
      case 'md':
        return 180;
      case 'lg':
        return 230;
      case 'xl':
        return 300;
      default:
        return 180;
    }
  });

  const mode = ref<'browse' | 'search'>('browse');
  const currentFolderId = ref<number | null>(null);
  const folderContent = ref<FolderContent>({
    current_folder: null,
    parent_folder: null,
    subfolders: [],
    files: [],
    breadcrumbs: [],
  });

  // Navigation History Stacks
  const historyBackStack = ref<(number | null)[]>([]);
  const historyForwardStack = ref<(number | null)[]>([]);

  const canGoBack = computed(() => {
    return historyBackStack.value.length > 0 || (folderContent.value.parent_folder !== null && mode.value === 'browse');
  });

  const canGoForward = computed(() => {
    return historyForwardStack.value.length > 0;
  });

  const searchResults = ref<ComicFileItem[]>([]);
  const searchFolderResults = ref<FolderItem[]>([]);

  // Selection state
  const selectedComicId = ref<number | null>(null);
  const selectedComicDetail = ref<ComicDetail | null>(null);
  const isLoadingComicDetail = ref(false);

  const selectedFolderId = ref<number | null>(null);
  const selectedFolderDetail = ref<FolderDetail | null>(null);
  const isLoadingFolderDetail = ref(false);

  const allTags = ref<TagItem[]>([]);
  const tagsWithCounts = ref<TagDetailItem[]>([]);

  // Modal & Tag Picker states
  const isSettingsOpen = ref(false);
  const settingsInitialTab = ref<'general' | 'tags'>('general');
  const tagPickerContext = ref<TagPickerContext | null>(null);

  // Auto-update states
  const updateResult = ref<UpdateCheckResult | null>(null);
  const isUpdateModalOpen = ref(false);
  const isCheckingUpdate = ref(false);
  const updateCheckError = ref<string | null>(null);
  const isDownloadingUpdate = ref(false);
  const updateProgress = ref<UpdateDownloadProgress | null>(null);
  const updateError = ref<string | null>(null);
  let updateUnlisten: UnlistenFn | null = null;

  const isAutoUpdateSuppressed = computed(() => {
    if (settings.value.ignore_updates_permanently) return true;
    if (settings.value.update_snooze_until) {
      const snoozeTime = Date.parse(settings.value.update_snooze_until);
      if (!Number.isNaN(snoozeTime) && snoozeTime > Date.now()) {
        return true;
      }
    }
    return false;
  });

  const availableTags = computed<SelectableTagOption[]>(() => {
    const map = new Map<string, SelectableTagOption>();

    for (const tag of allTags.value) {
      const key = normalizeTagKey(tag.display_name || tag.name);
      if (!key) continue;
      map.set(key, {
        key,
        display_name: tag.display_name,
        id: tag.id,
      });
    }

    return Array.from(map.values()).sort((a, b) =>
      a.display_name.localeCompare(b.display_name, 'ko')
    );
  });

  const searchQuery = ref<SearchQuery>({
    keywords: [],
    keyword_operator: 'AND',
    tag_names: [],
    tag_operator: 'AND',
    group_operator: 'AND',
    min_rating: null,
    max_rating: null,
    extension: null,
    sort_by: 'name',
    sort_order: 'asc',
  });

  const rawKeywordInput = ref('');
  const rawTagInput = ref('');
  const selectedSearchTags = ref<string[]>([]);

  const scanProgress = ref<ScanProgress>({
    is_scanning: false,
    current_status: '준비 완료',
    total_files_found: 0,
    processed_files: 0,
    covers_extracted: 0,
    last_error: null,
  });

  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let currentNavId = 0;

  // Actions
  async function init() {
    try {
      const state = await invoke<{ settings: AppSettings; library: Library | null }>('get_initial_state');
      if (state.settings) {
        settings.value = {
          ...settings.value,
          ...state.settings,
          update_snooze_until: state.settings.update_snooze_until ?? null,
          ignore_updates_permanently: state.settings.ignore_updates_permanently ?? false,
        };
      }
      // Run mandatory startup update check asynchronously (does not block initial library rendering)
      void checkForUpdates(true);

      if (state.library) {
        library.value = state.library;
        await Promise.all([loadFolder(null), loadTags()]);
      }
    } catch (err) {
      console.error('Failed to init app state:', err);
    }
  }

  async function loadTags() {
    try {
      allTags.value = await invoke<TagItem[]>('get_all_tags');
    } catch (err) {
      console.error('Failed to load tags:', err);
    }
  }

  async function loadTagsWithCounts() {
    try {
      tagsWithCounts.value = await invoke<TagDetailItem[]>('get_tags_with_counts');
    } catch (err) {
      console.error('Failed to load tags with counts:', err);
    }
  }

  async function setLibraryPath(path: string) {
    try {
      const lib = await invoke<Library>('set_library_path', { path });
      library.value = lib;
      settings.value.library_path = lib.root_path;
      clearSelection();
      historyBackStack.value = [];
      historyForwardStack.value = [];
      await startScan();
      await loadFolder(null);
    } catch (err) {
      console.error('Failed to set library path:', err);
      throw err;
    }
  }

  async function loadFolder(
    folderId: number | null,
    preserveSelection: boolean = false,
    isHistoryNav: boolean = false
  ) {
    // 이미 해당 폴더가 로드되어 화면에 표시 중인지 판별 (초기 로드 상태가 아닐 때만 유효)
    const isAlreadyLoaded = mode.value === 'browse' && (
      (folderId === null && (folderContent.value.current_folder === null || folderContent.value.current_folder.relative_path === '') && folderContent.value.breadcrumbs.length > 0) ||
      (folderId !== null && folderContent.value.current_folder?.id === folderId)
    );

    // 동일 폴더 재진입 방지 (새로고침이거나 히스토리 이동인 경우는 통과)
    if (!isHistoryNav && !preserveSelection && isAlreadyLoaded) {
      return;
    }

    // 일반 이동 시 히스토리 스택에 이전 위치 기록 (최초 로드가 아닌 이미 브라우징 중일 때만)
    if (!isHistoryNav && library.value && folderContent.value.breadcrumbs.length > 0) {
      if (currentFolderId.value !== folderId || mode.value === 'search') {
        historyBackStack.value.push(currentFolderId.value);
        historyForwardStack.value = [];
      }
    }

    const navId = ++currentNavId;
    try {
      currentFolderId.value = folderId;
      const content = await invoke<FolderContent>('get_folder_content', { folderId });
      if (navId !== currentNavId) return;

      folderContent.value = content;
      mode.value = 'browse';
      if (!preserveSelection) {
        clearSelection();
      }

      // Asynchronously resolve missing folder covers in background chunks (skip known empty folders)
      const missingCoverIds = content.subfolders
        .filter((f) => !f.cover_data_url && f.cover_status !== 'empty')
        .map((f) => f.id);

      if (missingCoverIds.length > 0) {
        resolveMissingFolderCovers(missingCoverIds, navId);
      }
    } catch (err) {
      console.error('Failed to load folder:', err);
    }
  }

  async function resolveMissingFolderCovers(folderIds: number[], navId: number) {
    const chunkSize = 16;
    for (let i = 0; i < folderIds.length; i += chunkSize) {
      if (navId !== currentNavId) return;
      const chunk = folderIds.slice(i, i + chunkSize);
      try {
        const resolved = await invoke<[number, string][]>('resolve_folder_covers', { folderIds: chunk });
        if (navId !== currentNavId) return;

        const resolvedMap = new Map<number, string>(resolved);
        for (const sub of folderContent.value.subfolders) {
          const url = resolvedMap.get(sub.id);
          if (url) {
            sub.cover_data_url = url;
          }
        }
        for (const sf of searchFolderResults.value) {
          const url = resolvedMap.get(sf.id);
          if (url) {
            sf.cover_data_url = url;
          }
        }
        if (selectedFolderDetail.value && resolvedMap.has(selectedFolderDetail.value.id)) {
          selectedFolderDetail.value.cover_data_url = resolvedMap.get(selectedFolderDetail.value.id)!;
        }
      } catch (err) {
        console.error('Failed to resolve folder covers:', err);
      }
    }
  }

  async function goBack() {
    if (historyBackStack.value.length > 0) {
      const prevFolderId = historyBackStack.value.pop()!;
      historyForwardStack.value.push(currentFolderId.value);
      await loadFolder(prevFolderId, false, true);
      return true;
    }
    // 히스토리 스택이 비어 있어도 상위 폴더가 있으면 상위 폴더로 이동
    if (folderContent.value.parent_folder !== null && mode.value === 'browse') {
      await loadFolder(folderContent.value.parent_folder.id);
      return true;
    }
    return false;
  }

  async function goForward() {
    if (historyForwardStack.value.length > 0) {
      const nextFolderId = historyForwardStack.value.pop()!;
      historyBackStack.value.push(currentFolderId.value);
      await loadFolder(nextFolderId, false, true);
      return true;
    }
    return false;
  }

  async function goToParent() {
    if (folderContent.value.parent_folder !== null) {
      await loadFolder(folderContent.value.parent_folder.id);
    }
  }

  function clearSelection() {
    selectedComicId.value = null;
    selectedComicDetail.value = null;
    selectedFolderId.value = null;
    selectedFolderDetail.value = null;
  }

  async function startScan() {
    try {
      scanProgress.value.is_scanning = true;
      scanProgress.value.current_status = '스캔 시작 중...';

      startProgressPolling();

      await invoke('start_scan', { path: null });
      await loadTags();
      if (mode.value === 'browse') {
        await loadFolder(currentFolderId.value);
      } else {
        await executeSearch();
      }
    } catch (err) {
      console.error('Scan error:', err);
      scanProgress.value.last_error = String(err);
    } finally {
      stopProgressPolling();
      await fetchScanProgress();
    }
  }

  function cancelScan() {
    invoke('cancel_scan').catch(console.error);
  }

  async function fetchScanProgress() {
    try {
      const p = await invoke<ScanProgress>('get_scan_progress');
      scanProgress.value = p;
      if (!p.is_scanning && pollTimer) {
        stopProgressPolling();
      }
    } catch (err) {
      console.error('Failed to fetch progress:', err);
    }
  }

  function startProgressPolling() {
    stopProgressPolling();
    pollTimer = setInterval(fetchScanProgress, 500);
  }

  function stopProgressPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function executeSearch() {
    try {
      const kw = rawKeywordInput.value
        .split(' ')
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      searchQuery.value.keywords = kw;

      const typedTags = rawTagInput.value
        .split(' ')
        .map((s) => cleanTagDisplay(s))
        .filter((s) => s.length > 0);

      const tgMap = new Map<string, string>();
      for (const t of [...selectedSearchTags.value, ...typedTags]) {
        const clean = cleanTagDisplay(t);
        const key = clean.toLowerCase();
        if (key && !tgMap.has(key)) {
          tgMap.set(key, clean);
        }
      }
      const tg = Array.from(tgMap.values());
      searchQuery.value.tag_names = tg;
      searchQuery.value.include_folder_tags = settings.value.include_folder_tags_in_search;

      if (kw.length === 0 && tg.length === 0 && searchQuery.value.min_rating === null && !searchQuery.value.extension) {
        searchResults.value = [];
        searchFolderResults.value = [];
        mode.value = 'browse';
        return;
      }

      mode.value = 'search';
      clearSelection();

      const searchId = ++currentNavId;
      const comicsPromise = invoke<ComicFileItem[]>('search_comics', {
        query: searchQuery.value,
      });

      if (settings.value.include_folder_tags_in_search) {
        const foldersPromise = invoke<FolderItem[]>('search_folders', {
          query: searchQuery.value,
        });
        const [comics, folders] = await Promise.all([comicsPromise, foldersPromise]);
        if (searchId !== currentNavId) return;
        searchResults.value = comics;
        searchFolderResults.value = folders;

        const missingCoverIds = folders
          .filter((f) => !f.cover_data_url && f.cover_status !== 'empty')
          .map((f) => f.id);
        if (missingCoverIds.length > 0) {
          resolveMissingFolderCovers(missingCoverIds, searchId);
        }
      } else {
        const comics = await comicsPromise;
        if (searchId !== currentNavId) return;
        searchResults.value = comics;
        searchFolderResults.value = [];
      }
    } catch (err) {
      console.error('Failed to search comics:', err);
    }
  }

  function clearSearch() {
    rawKeywordInput.value = '';
    rawTagInput.value = '';
    selectedSearchTags.value = [];
    searchQuery.value.keywords = [];
    searchQuery.value.tag_names = [];
    searchQuery.value.min_rating = null;
    searchQuery.value.extension = null;
    searchResults.value = [];
    searchFolderResults.value = [];
    mode.value = 'browse';
    clearSelection();
  }

  async function setSearchTags(tags: string[]) {
    const seen = new Set<string>();
    const cleaned: string[] = [];
    for (const raw of tags) {
      const display = cleanTagDisplay(raw);
      const key = display.toLowerCase();
      if (key && !seen.has(key)) {
        seen.add(key);
        cleaned.push(display);
      }
    }
    selectedSearchTags.value = cleaned;
    rawTagInput.value = '';
    await executeSearch();
  }

  async function removeSearchTag(tagName: string) {
    const targetKey = normalizeTagKey(tagName);
    selectedSearchTags.value = selectedSearchTags.value.filter(
      (t) => normalizeTagKey(t) !== targetKey
    );
    await executeSearch();
  }

  async function selectComic(fileId: number, force: boolean = false) {
    if (!force && selectedComicId.value === fileId && selectedComicDetail.value) {
      return;
    }
    selectedFolderId.value = null;
    selectedFolderDetail.value = null;

    selectedComicId.value = fileId;
    isLoadingComicDetail.value = true;
    try {
      selectedComicDetail.value = await invoke<ComicDetail | null>('get_comic_detail', { fileId });
    } catch (err) {
      console.error('Failed to get comic detail:', err);
    } finally {
      isLoadingComicDetail.value = false;
    }
  }

  async function selectFolder(folderId: number, force: boolean = false) {
    if (!force && selectedFolderId.value === folderId && selectedFolderDetail.value) {
      return;
    }
    selectedComicId.value = null;
    selectedComicDetail.value = null;

    selectedFolderId.value = folderId;
    isLoadingFolderDetail.value = true;
    try {
      selectedFolderDetail.value = await invoke<FolderDetail | null>('get_folder_detail', { folderId });
    } catch (err) {
      console.error('Failed to get folder detail:', err);
    } finally {
      isLoadingFolderDetail.value = false;
    }
  }

  let lastActionKey = '';
  let lastActionTime = 0;
  const ACTION_COOLDOWN_MS = 500;

  function canTriggerAction(key: string): boolean {
    const now = Date.now();
    if (lastActionKey === key && now - lastActionTime < ACTION_COOLDOWN_MS) {
      return false;
    }
    lastActionKey = key;
    lastActionTime = now;
    return true;
  }

  async function openComic(fileId: number) {
    if (!canTriggerAction(`comic_${fileId}`)) return;
    try {
      await invoke('open_comic_file', { fileId });

      if (settings.value.auto_clear_new_on_open) {
        if (selectedComicDetail.value && selectedComicDetail.value.id === fileId) {
          selectedComicDetail.value.is_new = false;
        }
        const fileInFolder = folderContent.value.files.find((f) => f.id === fileId);
        if (fileInFolder) fileInFolder.is_new = false;
        const fileInSearch = searchResults.value.find((f) => f.id === fileId);
        if (fileInSearch) fileInSearch.is_new = false;
      }
    } catch (err) {
      console.error('Failed to open comic file:', err);
      alert('만화책 열기 실패: ' + err);
    }
  }

  async function openFolderInViewer(folderId: number) {
    if (!canTriggerAction(`folder_viewer_${folderId}`)) return;
    try {
      await invoke('open_folder_in_viewer', { folderId });
    } catch (err) {
      console.error('Failed to open folder in viewer:', err);
      alert('만화 뷰어로 폴더 열기 실패: ' + err);
    }
  }

  async function showInFileManager(path: string) {
    if (!canTriggerAction(`file_manager_${path}`)) return;
    try {
      await invoke('show_in_file_manager', { path });
    } catch (err) {
      console.error('Failed to show in file manager:', err);
      alert('탐색기 열기 실패: ' + err);
    }
  }

  async function refreshItem(itemType: 'comic' | 'folder', id: number) {
    try {
      const newCoverUrl = await invoke<string | null>('refresh_item', { itemType, id });

      if (itemType === 'comic') {
        if (selectedComicDetail.value && selectedComicDetail.value.id === id) {
          selectedComicDetail.value.cover_data_url = newCoverUrl;
        }
        const fileItem = folderContent.value.files.find((f) => f.id === id);
        if (fileItem) fileItem.cover_data_url = newCoverUrl;

        const searchItem = searchResults.value.find((f) => f.id === id);
        if (searchItem) searchItem.cover_data_url = newCoverUrl;
      } else {
        if (selectedFolderDetail.value && selectedFolderDetail.value.id === id) {
          selectedFolderDetail.value.cover_data_url = newCoverUrl;
        }
        const subItem = folderContent.value.subfolders.find((f) => f.id === id);
        if (subItem) subItem.cover_data_url = newCoverUrl;

        const searchFItem = searchFolderResults.value.find((f) => f.id === id);
        if (searchFItem) searchFItem.cover_data_url = newCoverUrl;

        if (folderContent.value.current_folder && folderContent.value.current_folder.id === id) {
          folderContent.value.current_folder.cover_data_url = newCoverUrl;
        }
      }
    } catch (err) {
      console.error('Failed to refresh item:', err);
      alert('항목 새로고침 실패: ' + err);
    }
  }

  async function updateComicRating(fileId: number, rating: number) {
    try {
      await invoke('update_comic_rating', { fileId, rating });
      if (selectedComicDetail.value && selectedComicDetail.value.id === fileId) {
        selectedComicDetail.value.rating = rating;
      }
      const item = folderContent.value.files.find((f) => f.id === fileId);
      if (item) item.rating = rating;
      const sItem = searchResults.value.find((f) => f.id === fileId);
      if (sItem) sItem.rating = rating;
    } catch (err) {
      console.error('Failed to update comic rating:', err);
    }
  }

  async function updateFolderRating(folderId: number, rating: number) {
    try {
      await invoke('update_folder_rating', { folderId, rating });
      if (folderContent.value.current_folder && folderContent.value.current_folder.id === folderId) {
        folderContent.value.current_folder.rating = rating;
      }
      if (selectedFolderDetail.value && selectedFolderDetail.value.id === folderId) {
        selectedFolderDetail.value.rating = rating;
      }
      const sub = folderContent.value.subfolders.find((f) => f.id === folderId);
      if (sub) sub.rating = rating;
      const searchFItem = searchFolderResults.value.find((f) => f.id === folderId);
      if (searchFItem) searchFItem.rating = rating;
    } catch (err) {
      console.error('Failed to update folder rating:', err);
    }
  }

  async function toggleComicIsNew(comicId: number) {
    try {
      const updatedIsNew = await invoke<boolean>('toggle_comic_is_new', { fileId: comicId });
      const fileInFolder = folderContent.value.files.find((f) => f.id === comicId);
      if (fileInFolder) fileInFolder.is_new = updatedIsNew;
      const fileInSearch = searchResults.value.find((f) => f.id === comicId);
      if (fileInSearch) fileInSearch.is_new = updatedIsNew;
      if (selectedComicDetail.value && selectedComicDetail.value.id === comicId) {
        selectedComicDetail.value.is_new = updatedIsNew;
      }
      return updatedIsNew;
    } catch (err) {
      console.error('Failed to toggle comic is_new:', err);
      throw err;
    }
  }

  async function toggleFolderIsNew(folderId: number) {
    try {
      const updatedIsNew = await invoke<boolean>('toggle_folder_is_new', { folderId });
      const folderInSub = folderContent.value.subfolders.find((f) => f.id === folderId);
      if (folderInSub) folderInSub.is_new = updatedIsNew;
      const folderInSearch = searchFolderResults.value.find((f) => f.id === folderId);
      if (folderInSearch) folderInSearch.is_new = updatedIsNew;
      if (selectedFolderDetail.value && selectedFolderDetail.value.id === folderId) {
        selectedFolderDetail.value.is_new = updatedIsNew;
      }
      return updatedIsNew;
    } catch (err) {
      console.error('Failed to toggle folder is_new:', err);
      throw err;
    }
  }

  async function updateComicComment(fileId: number, comment: string) {
    try {
      await invoke('update_comic_comment', { fileId, comment });
      if (selectedComicDetail.value && selectedComicDetail.value.id === fileId) {
        selectedComicDetail.value.comment = comment;
      }
      const item = folderContent.value.files.find((f) => f.id === fileId);
      if (item) item.comment = comment;
    } catch (err) {
      console.error('Failed to update comment:', err);
    }
  }

  async function addTagToComic(fileId: number, tagName: string) {
    if (!tagName.trim()) return;
    try {
      await invoke('add_tag_to_comic', { fileId, tagName: tagName.trim() });
      await selectComic(fileId, true);
      if (selectedComicDetail.value) {
        const fileItem = folderContent.value.files.find((f) => f.id === fileId);
        if (fileItem) fileItem.direct_tags = selectedComicDetail.value.direct_tags;
        const searchItem = searchResults.value.find((f) => f.id === fileId);
        if (searchItem) searchItem.direct_tags = selectedComicDetail.value.direct_tags;
      }
      await loadTags();
    } catch (err) {
      console.error('Failed to add tag:', err);
    }
  }

  async function removeTagFromComic(fileId: number, tagId: number) {
    try {
      await invoke('remove_tag_from_comic', { fileId, tagId });
      await selectComic(fileId, true);
      if (selectedComicDetail.value) {
        const fileItem = folderContent.value.files.find((f) => f.id === fileId);
        if (fileItem) fileItem.direct_tags = selectedComicDetail.value.direct_tags;
        const searchItem = searchResults.value.find((f) => f.id === fileId);
        if (searchItem) searchItem.direct_tags = selectedComicDetail.value.direct_tags;
      }
      await loadTags();
    } catch (err) {
      console.error('Failed to remove tag:', err);
    }
  }

  async function addTagToFolder(folderId: number, tagName: string) {
    if (!tagName.trim()) return;
    try {
      await invoke('add_tag_to_folder', { folderId, tagName: tagName.trim() });
      if (selectedFolderId.value === folderId) {
        await selectFolder(folderId, true);
      }
      // If a comic is currently selected, refresh its inherited tags immediately
      if (selectedComicId.value) {
        await selectComic(selectedComicId.value, true);
      }
      if (mode.value === 'search') {
        await executeSearch();
      } else {
        await loadFolder(currentFolderId.value, true);
      }
      await loadTags();
    } catch (err) {
      console.error('Failed to add folder tag:', err);
    }
  }

  async function removeTagFromFolder(folderId: number, tagId: number) {
    try {
      await invoke('remove_tag_from_folder', { folderId, tagId });
      if (selectedFolderId.value === folderId) {
        await selectFolder(folderId, true);
      }
      // If a comic is currently selected, refresh its inherited tags immediately
      if (selectedComicId.value) {
        await selectComic(selectedComicId.value, true);
      }
      if (mode.value === 'search') {
        await executeSearch();
      } else {
        await loadFolder(currentFolderId.value, true);
      }
      await loadTags();
    } catch (err) {
      console.error('Failed to remove folder tag:', err);
    }
  }

  // Tag Management Actions
  async function createTag(name: string): Promise<{ created: boolean; duplicate: boolean; tag: TagItem | null }> {
    const clean = cleanTagDisplay(name);
    const key = clean.toLowerCase();
    if (!clean) {
      return { created: false, duplicate: false, tag: null };
    }
    try {
      const existing = allTags.value.find(
        (t) => normalizeTagKey(t.display_name || t.name) === key
      );
      if (existing) {
        return { created: false, duplicate: true, tag: existing };
      }

      const tag = await invoke<TagItem>('create_tag', { name: clean });
      await loadTags();
      await loadTagsWithCounts();
      return { created: true, duplicate: false, tag };
    } catch (err) {
      console.error('Failed to create tag:', err);
      alert('태그 생성 실패: ' + err);
      throw err;
    }
  }

  async function updateTag(id: number, newName: string) {
    const clean = cleanTagDisplay(newName);
    if (!clean) return;
    try {
      const existing = allTags.value.find(
        (t) => t.id !== id && normalizeTagKey(t.display_name || t.name) === clean.toLowerCase()
      );
      if (existing) {
        alert(`이미 존재하는 태그 '#${existing.display_name}'입니다.`);
        return;
      }

      await invoke('update_tag', { id, newName: clean });
      await loadTags();
      await loadTagsWithCounts();
      if (selectedComicId.value) await selectComic(selectedComicId.value, true);
      if (selectedFolderId.value) await selectFolder(selectedFolderId.value, true);
      if (mode.value === 'browse') {
        await loadFolder(currentFolderId.value, true);
      } else {
        await executeSearch();
      }
    } catch (err) {
      console.error('Failed to update tag:', err);
      alert('태그 수정 실패: ' + err);
    }
  }

  async function deleteTag(id: number) {
    try {
      await invoke('delete_tag', { id });
      await loadTags();
      await loadTagsWithCounts();
      if (selectedComicId.value) await selectComic(selectedComicId.value, true);
      if (selectedFolderId.value) await selectFolder(selectedFolderId.value, true);
      if (mode.value === 'browse') {
        await loadFolder(currentFolderId.value, true);
      } else {
        await executeSearch();
      }
    } catch (err) {
      console.error('Failed to delete tag:', err);
      alert('태그 삭제 실패: ' + err);
    }
  }

  async function ensureTagExists(
    rawName: string
  ): Promise<{ created: boolean; duplicate: boolean; tag: SelectableTagOption | null }> {
    const clean = cleanTagDisplay(rawName);
    const key = clean.toLowerCase();
    if (!key) {
      return { created: false, duplicate: false, tag: null };
    }

    const existing = availableTags.value.find((t) => t.key === key);
    if (existing) {
      return { created: false, duplicate: true, tag: existing };
    }

    const res = await createTag(clean);
    if (!res.tag) {
      return { created: false, duplicate: false, tag: null };
    }
    return {
      created: res.created,
      duplicate: res.duplicate,
      tag: {
        key: normalizeTagKey(res.tag.display_name || res.tag.name),
        display_name: res.tag.display_name,
        id: res.tag.id,
      },
    };
  }

  async function setComicTags(fileId: number, tagNames: string[]) {
    try {
      await invoke('set_comic_tags', { fileId, tagNames });
      if (selectedComicId.value === fileId) {
        await selectComic(fileId, true);
      }
      // Update inline card tags immediately if refreshed detail isn't open
      const updatedDetail =
        selectedComicId.value === fileId
          ? selectedComicDetail.value
          : await invoke<ComicDetail | null>('get_comic_detail', { fileId });
      if (updatedDetail) {
        const fileItem = folderContent.value.files.find((f) => f.id === fileId);
        if (fileItem) fileItem.direct_tags = updatedDetail.direct_tags;
        const searchItem = searchResults.value.find((f) => f.id === fileId);
        if (searchItem) searchItem.direct_tags = updatedDetail.direct_tags;
      }
      await loadTags();
    } catch (err) {
      console.error('Failed to set comic tags:', err);
    }
  }

  async function setFolderTags(folderId: number, tagNames: string[]) {
    try {
      await invoke('set_folder_tags', { folderId, tagNames });
      if (selectedFolderId.value === folderId) {
        await selectFolder(folderId, true);
      }
      if (selectedComicId.value) {
        await selectComic(selectedComicId.value, true);
      }
      if (mode.value === 'search') {
        await executeSearch();
      } else {
        await loadFolder(currentFolderId.value, true);
      }
      await loadTags();
    } catch (err) {
      console.error('Failed to set folder tags:', err);
    }
  }

  function openSettingsModal(tab: 'general' | 'tags' = 'general') {
    settingsInitialTab.value = tab;
    isSettingsOpen.value = true;
  }

  function closeSettingsModal() {
    isSettingsOpen.value = false;
  }

  function openSearchTagPicker() {
    tagPickerContext.value = {
      mode: 'search',
      initialTags: [...selectedSearchTags.value],
    };
  }

  function openComicTagPicker(comicId: number, title: string, currentTags: TagItem[]) {
    tagPickerContext.value = {
      mode: 'comic',
      targetId: comicId,
      targetTitle: title,
      initialTags: currentTags.map((t) => t.display_name),
    };
  }

  function openFolderTagPicker(folderId: number, title: string, currentTags: TagItem[]) {
    tagPickerContext.value = {
      mode: 'folder',
      targetId: folderId,
      targetTitle: title,
      initialTags: currentTags.map((t) => t.display_name),
    };
  }

  async function openFolderTagPickerById(folderId: number, fallbackTitle: string) {
    try {
      const detail = await invoke<FolderDetail | null>('get_folder_detail', { folderId });
      if (detail) {
        openFolderTagPicker(detail.id, detail.name, detail.direct_tags);
      } else {
        openFolderTagPicker(folderId, fallbackTitle, []);
      }
    } catch (err) {
      console.error('Failed to load folder detail for tag picker:', err);
    }
  }

  function closeTagPicker() {
    tagPickerContext.value = null;
  }

  async function applyTagPickerSelection(selectedTagNames: string[]) {
    const ctx = tagPickerContext.value;
    if (!ctx) return;
    tagPickerContext.value = null;

    if (ctx.mode === 'search') {
      await setSearchTags(selectedTagNames);
    } else if (ctx.mode === 'comic' && ctx.targetId !== undefined) {
      await setComicTags(ctx.targetId, selectedTagNames);
    } else if (ctx.mode === 'folder' && ctx.targetId !== undefined) {
      await setFolderTags(ctx.targetId, selectedTagNames);
    }
  }

  // Database Rebuild Action
  async function rebuildDatabase() {
    try {
      scanProgress.value.is_scanning = true;
      scanProgress.value.current_status = 'DB 초기화 및 완전 재구축 중...';
      startProgressPolling();

      await invoke('rebuild_library_cache');
      clearSelection();
      await loadTags();
      await loadFolder(null);
    } catch (err) {
      console.error('Failed to rebuild database:', err);
      alert('DB 재구축 실패: ' + err);
    } finally {
      stopProgressPolling();
      await fetchScanProgress();
    }
  }

  async function saveSettings(newSettings: AppSettings) {
    try {
      await invoke('save_app_settings', { settings: newSettings });
      settings.value = newSettings;
    } catch (err) {
      console.error('Failed to save settings:', err);
    }
  }

  async function checkForUpdates(isStartup: boolean = false): Promise<UpdateCheckResult | null> {
    if (isCheckingUpdate.value) return updateResult.value;
    isCheckingUpdate.value = true;
    updateCheckError.value = null;

    try {
      const res = await invoke<UpdateCheckResult>('check_for_updates');
      updateResult.value = res;

      if (res.has_update) {
        if (!isStartup || !isAutoUpdateSuppressed.value) {
          isUpdateModalOpen.value = true;
        }
      }
      return res;
    } catch (err) {
      const msg = typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
      updateCheckError.value = msg;
      if (isStartup) {
        console.warn('Startup update check skipped/failed:', msg);
      } else {
        console.error('Manual update check failed:', msg);
      }
      return null;
    } finally {
      isCheckingUpdate.value = false;
    }
  }

  function openUpdateModal() {
    if (updateResult.value?.has_update) {
      updateError.value = null;
      isUpdateModalOpen.value = true;
    }
  }

  function closeUpdateModal() {
    if (isDownloadingUpdate.value) return;
    isUpdateModalOpen.value = false;
  }

  async function snoozeUpdateWeek() {
    try {
      const updated = await invoke<AppSettings>('snooze_update_week');
      settings.value = updated;
      isUpdateModalOpen.value = false;
    } catch (err) {
      console.error('Failed to snooze update:', err);
    }
  }

  async function ignoreUpdateForever() {
    try {
      const updated = await invoke<AppSettings>('ignore_update_forever');
      settings.value = updated;
      isUpdateModalOpen.value = false;
    } catch (err) {
      console.error('Failed to ignore updates forever:', err);
    }
  }

  async function clearUpdateSuppression() {
    try {
      const updated = await invoke<AppSettings>('clear_update_suppression');
      settings.value = updated;
    } catch (err) {
      console.error('Failed to reset update suppression:', err);
    }
  }

  async function downloadAndApplyUpdate() {
    if (!updateResult.value?.download_url || isDownloadingUpdate.value) return;

    isDownloadingUpdate.value = true;
    updateError.value = null;
    updateProgress.value = {
      downloaded: 0,
      total: updateResult.value.asset_size ?? null,
      percent: 0,
      stage: 'downloading',
    };

    try {
      if (updateUnlisten) {
        updateUnlisten();
        updateUnlisten = null;
      }
      updateUnlisten = await listen<UpdateDownloadProgress>('update://progress', (event) => {
        updateProgress.value = event.payload;
      });

      await invoke('download_and_apply_update', {
        downloadUrl: updateResult.value.download_url,
        expectedSize: updateResult.value.asset_size ?? null,
      });
    } catch (err) {
      const msg = typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
      updateError.value = msg;
      isDownloadingUpdate.value = false;
    } finally {
      if (updateUnlisten) {
        updateUnlisten();
        updateUnlisten = null;
      }
    }
  }

  const isScanning = computed(() => scanProgress.value.is_scanning);

  return {
    library,
    settings,
    cardMinWidth,
    mode,
    currentFolderId,
    folderContent,
    historyBackStack,
    historyForwardStack,
    canGoBack,
    canGoForward,
    searchResults,
    searchFolderResults,
    selectedComicId,
    selectedComicDetail,
    isLoadingComicDetail,
    selectedFolderId,
    selectedFolderDetail,
    isLoadingFolderDetail,
    searchQuery,
    rawKeywordInput,
    rawTagInput,
    selectedSearchTags,
    scanProgress,
    isScanning,
    allTags,
    tagsWithCounts,
    availableTags,
    isSettingsOpen,
    settingsInitialTab,
    tagPickerContext,
    updateResult,
    isUpdateModalOpen,
    isCheckingUpdate,
    updateCheckError,
    isDownloadingUpdate,
    updateProgress,
    updateError,
    isAutoUpdateSuppressed,
    init,
    setLibraryPath,
    loadFolder,
    goBack,
    goForward,
    goToParent,
    clearSelection,
    startScan,
    cancelScan,
    executeSearch,
    clearSearch,
    setSearchTags,
    removeSearchTag,
    selectComic,
    selectFolder,
    openComic,
    openFolderInViewer,
    refreshItem,
    showInFileManager,
    updateComicRating,
    updateFolderRating,
    toggleComicIsNew,
    toggleFolderIsNew,
    updateComicComment,
    setComicTags,
    setFolderTags,
    addTagToComic,
    removeTagFromComic,
    addTagToFolder,
    removeTagFromFolder,
    loadTagsWithCounts,
    createTag,
    updateTag,
    deleteTag,
    ensureTagExists,
    openSettingsModal,
    closeSettingsModal,
    openSearchTagPicker,
    openComicTagPicker,
    openFolderTagPicker,
    openFolderTagPickerById,
    closeTagPicker,
    applyTagPickerSelection,
    rebuildDatabase,
    saveSettings,
    checkForUpdates,
    openUpdateModal,
    closeUpdateModal,
    snoozeUpdateWeek,
    ignoreUpdateForever,
    clearUpdateSuppression,
    downloadAndApplyUpdate,
  };
});
