<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useVirtualizer } from '@tanstack/vue-virtual';
import { useLibraryStore } from './stores/libraryStore';
import TopNav from './components/TopNav.vue';
import BreadcrumbNav from './components/BreadcrumbNav.vue';
import FolderCard from './components/FolderCard.vue';
import ComicCard from './components/ComicCard.vue';
import DetailPanel from './components/DetailPanel.vue';
import SettingsModal from './components/SettingsModal.vue';
import TagPickerModal from './components/TagPickerModal.vue';
import UpdateModal from './components/UpdateModal.vue';
import StatusBar from './components/StatusBar.vue';
import ContextMenu, { type ContextMenuTarget } from './components/ContextMenu.vue';
import { FolderOpen, SearchX, BookOpen, Layers, Play, FolderSearch } from 'lucide-vue-next';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type { ComicFileItem, FolderItem } from './types';

const store = useLibraryStore();

const contextMenuTarget = ref<ContextMenuTarget | null>(null);

// Virtualized grid state
const scrollContainerRef = ref<HTMLElement | null>(null);
const contentWidth = ref(900);
let resizeObserver: ResizeObserver | null = null;

const GRID_GAP = 16;

const columnCount = computed(() => {
  return Math.max(1, Math.floor((contentWidth.value + GRID_GAP) / (store.cardMinWidth + GRID_GAP)));
});

const estimatedCardRowHeight = computed(() => {
  const cols = columnCount.value;
  const cardWidth = Math.max(100, (contentWidth.value - (cols - 1) * GRID_GAP) / cols);
  return Math.round(cardWidth * (4 / 3) + 84);
});

type VirtualRow =
  | { kind: 'section-header'; key: string; icon: 'layers' | 'book'; title: string; count: number }
  | { kind: 'spacer'; key: string }
  | { kind: 'folder-row'; key: string; items: FolderItem[] }
  | { kind: 'comic-row'; key: string; items: ComicFileItem[] };

function pushChunkedRows<T>(
  rows: VirtualRow[],
  items: T[],
  cols: number,
  kind: 'folder-row' | 'comic-row',
  prefix: string
) {
  for (let i = 0; i < items.length; i += cols) {
    const slice = items.slice(i, i + cols);
    if (kind === 'folder-row') {
      rows.push({ kind: 'folder-row', key: `${prefix}-${i}`, items: slice as FolderItem[] });
    } else {
      rows.push({ kind: 'comic-row', key: `${prefix}-${i}`, items: slice as ComicFileItem[] });
    }
  }
}

const virtualRows = computed<VirtualRow[]>(() => {
  if (!store.library) return [];
  const cols = columnCount.value;
  const rows: VirtualRow[] = [];

  if (store.mode === 'search') {
    const folders = store.searchFolderResults;
    const comics = store.searchResults;
    if (folders.length > 0) {
      rows.push({ kind: 'section-header', key: 's-hdr-f', icon: 'layers', title: '폴더 검색 결과', count: folders.length });
      pushChunkedRows(rows, folders, cols, 'folder-row', `sf-${cols}`);
      if (comics.length > 0) {
        rows.push({ kind: 'spacer', key: 's-spacer' });
      }
    }
    if (comics.length > 0) {
      if (folders.length > 0) {
        rows.push({ kind: 'section-header', key: 's-hdr-c', icon: 'book', title: '만화책 파일', count: comics.length });
      }
      pushChunkedRows(rows, comics, cols, 'comic-row', `sc-${cols}`);
    }
  } else {
    const folders = store.folderContent.subfolders;
    const comics = store.folderContent.files;
    if (folders.length > 0) {
      rows.push({ kind: 'section-header', key: 'b-hdr-f', icon: 'layers', title: '하위 폴더', count: folders.length });
      pushChunkedRows(rows, folders, cols, 'folder-row', `bf-${store.currentFolderId ?? 'root'}-${cols}`);
      if (comics.length > 0) {
        rows.push({ kind: 'spacer', key: 'b-spacer' });
      }
    }
    if (comics.length > 0) {
      rows.push({ kind: 'section-header', key: 'b-hdr-c', icon: 'book', title: '만화책 목록', count: comics.length });
      pushChunkedRows(rows, comics, cols, 'comic-row', `bc-${store.currentFolderId ?? 'root'}-${cols}`);
    }
  }

  return rows;
});

const virtualizerOptions = computed(() => ({
  count: virtualRows.value.length,
  getScrollElement: () => scrollContainerRef.value,
  estimateSize: (index: number) => {
    const row = virtualRows.value[index];
    if (!row) return 300;
    if (row.kind === 'section-header') return 32;
    if (row.kind === 'spacer') return 20;
    return estimatedCardRowHeight.value;
  },
  overscan: 3,
  getItemKey: (index: number) => virtualRows.value[index]?.key ?? index,
}));

const rowVirtualizer = useVirtualizer(virtualizerOptions);

function measureVirtualRow(el: unknown) {
  if (el instanceof Element) {
    rowVirtualizer.value.measureElement(el);
  }
}

watch(
  [() => store.currentFolderId, () => store.mode],
  () => {
    if (scrollContainerRef.value) {
      scrollContainerRef.value.scrollTop = 0;
    }
  }
);

function handleMouseUp(e: MouseEvent) {
  if (!store.library) return;
  // Mouse 4: Back button
  if (e.button === 3) {
    e.preventDefault();
    e.stopPropagation();
    store.goBack();
  }
  // Mouse 5: Forward button
  else if (e.button === 4) {
    e.preventDefault();
    e.stopPropagation();
    store.goForward();
  }
}

function handleMouseDown(e: MouseEvent) {
  // Prevent default browser/webview history navigation on Mouse 4 & 5
  if (e.button === 3 || e.button === 4) {
    e.preventDefault();
    e.stopPropagation();
  }
}

function handleKeyDown(e: KeyboardEvent) {
  if (!store.library) return;
  const target = e.target as HTMLElement | null;
  const isInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);

  if (e.altKey && e.key === 'ArrowLeft') {
    e.preventDefault();
    store.goBack();
  } else if (e.altKey && e.key === 'ArrowRight') {
    e.preventDefault();
    store.goForward();
  } else if (e.altKey && e.key === 'ArrowUp') {
    e.preventDefault();
    store.goToParent();
  } else if (!isInput && e.key === 'Backspace') {
    e.preventDefault();
    store.goBack();
  }
}

onMounted(() => {
  store.init();
  window.addEventListener('mouseup', handleMouseUp, { capture: true });
  window.addEventListener('mousedown', handleMouseDown, { capture: true });
  window.addEventListener('keydown', handleKeyDown);

  if (scrollContainerRef.value) {
    contentWidth.value = Math.max(200, scrollContainerRef.value.clientWidth - 48);
    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        contentWidth.value = Math.max(200, entry.contentRect.width - 48);
      }
    });
    resizeObserver.observe(scrollContainerRef.value);
  }
});

onUnmounted(() => {
  window.removeEventListener('mouseup', handleMouseUp, { capture: true });
  window.removeEventListener('mousedown', handleMouseDown, { capture: true });
  window.removeEventListener('keydown', handleKeyDown);
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
});

async function handlePickLibrary() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: '만화책 라이브러리 폴더 선택',
    });
    if (selected && typeof selected === 'string') {
      await store.setLibraryPath(selected);
    }
  } catch (err) {
    console.error('Failed to select directory:', err);
  }
}

function handleComicContextMenu(e: MouseEvent, comic: ComicFileItem) {
  e.preventDefault();
  e.stopPropagation();
  contextMenuTarget.value = {
    type: 'comic',
    comic,
    x: e.clientX,
    y: e.clientY,
  };
}

function handleFolderContextMenu(e: MouseEvent, folder: FolderItem) {
  e.preventDefault();
  e.stopPropagation();
  contextMenuTarget.value = {
    type: 'folder',
    folder,
    x: e.clientX,
    y: e.clientY,
  };
}

function handleBackgroundContextMenu(e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (!store.library) return;
  contextMenuTarget.value = {
    type: 'background',
    x: e.clientX,
    y: e.clientY,
  };
}
</script>

<template>
  <div
    class="h-screen w-screen flex flex-col bg-neutral-950 text-neutral-100 overflow-hidden font-sans"
    @contextmenu.prevent.stop="handleBackgroundContextMenu"
  >
    <!-- Top Navigation -->
    <TopNav @open-settings="store.openSettingsModal('general')" />

    <!-- Main Workspace (Center + Right Detail) -->
    <div class="flex-1 flex overflow-hidden relative">
      <!-- Center Content Area -->
      <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
        <!-- Breadcrumbs (Browse Mode) -->
        <BreadcrumbNav v-if="store.mode === 'browse' && store.library" />

        <!-- Search Mode Header -->
        <div
          v-else-if="store.mode === 'search'"
          class="h-12 border-b border-neutral-800 bg-neutral-900/60 px-6 flex items-center justify-between text-xs text-neutral-300"
        >
          <div class="flex items-center gap-2">
            <span class="font-semibold text-white">검색 모드</span>
            <span class="text-neutral-500">·</span>
            <span>
              검색 결과:
              <template v-if="store.searchFolderResults.length > 0">
                폴더 <strong class="text-indigo-400 font-semibold">{{ store.searchFolderResults.length }}</strong>개,
              </template>
              만화책 <strong class="text-indigo-400 font-semibold">{{ store.searchResults.length }}</strong>권 발견
            </span>
          </div>
          <button
            @click="store.clearSearch"
            class="px-2.5 py-1 rounded bg-neutral-800 hover:bg-neutral-700 text-indigo-400 hover:text-indigo-300 transition cursor-pointer"
          >
            탐색 모드로 돌아가기
          </button>
        </div>

        <!-- Scrollable Grid Content -->
        <div
          ref="scrollContainerRef"
          class="flex-1 overflow-y-auto p-6"
          @contextmenu.prevent.stop="handleBackgroundContextMenu"
        >
          <!-- 1. No Library Selected View -->
          <div
            v-if="!store.library"
            class="h-full flex flex-col items-center justify-center text-center p-8 max-w-md mx-auto space-y-4"
          >
            <div class="w-20 h-20 rounded-2xl bg-indigo-950/60 border border-indigo-800/40 flex items-center justify-center text-indigo-400 shadow-2xl shadow-indigo-600/20">
              <FolderOpen class="w-10 h-10" />
            </div>
            <div class="space-y-1">
              <h2 class="text-lg font-bold text-white">만화책 라이브러리 폴더를 지정하세요</h2>
              <p class="text-xs text-neutral-400 leading-relaxed">
                로컬 디스크 또는 WebDAV 네트워크 드라이브의 만화책 폴더를 선택하면 내부에 <code class="text-indigo-400">.ymppCache</code>를 생성하여 DB와 표지를 독립 보존합니다.
              </p>
            </div>
            <button
              @click="handlePickLibrary"
              class="px-5 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-xs flex items-center gap-2 transition duration-200 shadow-lg shadow-indigo-600/30 cursor-pointer"
            >
              <FolderOpen class="w-4 h-4" />
              라이브러리 폴더 선택
            </button>
          </div>

          <!-- 2. Search Mode Empty View -->
          <div
            v-else-if="store.mode === 'search' && store.searchResults.length === 0 && store.searchFolderResults.length === 0"
            class="h-full flex flex-col items-center justify-center text-neutral-500 gap-3 py-16"
          >
            <SearchX class="w-12 h-12 stroke-[1.5]" />
            <p class="text-xs">일치하는 만화책 또는 폴더가 없습니다.</p>
            <button
              @click="store.clearSearch"
              class="px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-xs text-neutral-300 cursor-pointer"
            >
              검색 조건 초기화
            </button>
          </div>

          <!-- 3. Browse Mode Empty Folder Notification -->
          <div
            v-else-if="store.mode === 'browse' && store.folderContent.subfolders.length === 0 && store.folderContent.files.length === 0"
            class="h-64 flex flex-col items-center justify-center text-neutral-500 gap-3"
          >
            <BookOpen class="w-10 h-10 stroke-[1.5]" />
            <p class="text-xs">이 폴더에 지원되는 만화책 압축 파일(ZIP/CBZ)이 없습니다.</p>
            <div
              v-if="store.folderContent.current_folder && store.folderContent.current_folder.relative_path !== ''"
              class="flex items-center gap-2 mt-1"
            >
              <button
                @click="store.openFolderInViewer(store.folderContent.current_folder.id)"
                class="px-3.5 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium flex items-center gap-2 shadow-lg shadow-indigo-600/20 transition cursor-pointer"
              >
                <Play class="w-3.5 h-3.5 fill-white" />
                만화 뷰어로 현재 폴더 열기
              </button>
              <button
                @click="store.showInFileManager(store.folderContent.current_folder.relative_path || '')"
                class="px-3 py-2 rounded-xl bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium flex items-center gap-1.5 transition border border-neutral-700 cursor-pointer"
              >
                <FolderSearch class="w-3.5 h-3.5" />
                탐색기에서 열기
              </button>
            </div>
          </div>

          <!-- 4. Virtualized Grid (Browse & Search Results) -->
          <div
            v-else
            class="relative w-full"
            :style="{ height: `${rowVirtualizer.getTotalSize() + 48}px` }"
          >
            <div
              v-for="vRow in rowVirtualizer.getVirtualItems()"
              :key="String(vRow.key)"
              :data-index="vRow.index"
              :ref="measureVirtualRow"
              class="absolute top-0 left-0 w-full"
              :style="{ transform: `translateY(${vRow.start}px)` }"
            >
              <template v-if="virtualRows[vRow.index]">
                <!-- Section Header -->
                <h3
                  v-if="virtualRows[vRow.index].kind === 'section-header'"
                  class="text-xs font-bold text-neutral-400 uppercase tracking-wider pb-3 flex items-center gap-2"
                >
                  <Layers
                    v-if="(virtualRows[vRow.index] as { icon: string }).icon === 'layers'"
                    class="w-3.5 h-3.5 text-indigo-400"
                  />
                  <BookOpen v-else class="w-3.5 h-3.5 text-indigo-400" />
                  {{ (virtualRows[vRow.index] as { title: string }).title }} ({{ (virtualRows[vRow.index] as { count: number }).count }})
                </h3>

                <!-- Spacer -->
                <div v-else-if="virtualRows[vRow.index].kind === 'spacer'" class="h-5" />

                <!-- Folder Row -->
                <div
                  v-else-if="virtualRows[vRow.index].kind === 'folder-row'"
                  class="grid gap-4 pb-4"
                  :style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))` }"
                >
                  <FolderCard
                    v-for="sub in (virtualRows[vRow.index] as { items: FolderItem[] }).items"
                    :key="sub.id"
                    :folder="sub"
                    :is-selected="store.selectedFolderId === sub.id"
                    @select-folder="store.selectFolder"
                    @open-folder="store.loadFolder"
                    @contextmenu="handleFolderContextMenu"
                  />
                </div>

                <!-- Comic File Row -->
                <div
                  v-else-if="virtualRows[vRow.index].kind === 'comic-row'"
                  class="grid gap-4 pb-4"
                  :style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))` }"
                >
                  <ComicCard
                    v-for="comic in (virtualRows[vRow.index] as { items: ComicFileItem[] }).items"
                    :key="comic.id"
                    :comic="comic"
                    :is-selected="store.selectedComicId === comic.id"
                    @select="store.selectComic"
                    @open="store.openComic"
                    @contextmenu="handleComicContextMenu"
                  />
                </div>
              </template>
            </div>
          </div>
        </div>
      </main>

      <!-- Right Detail Panel (Permanently visible) -->
      <DetailPanel
        @close="store.clearSelection()"
      />
    </div>

    <!-- Status Bar -->
    <StatusBar />

    <!-- Settings Modal -->
    <SettingsModal
      v-if="store.isSettingsOpen"
      :initial-tab="store.settingsInitialTab"
      @close="store.closeSettingsModal()"
    />

    <!-- Dedicated Tag Picker Popup -->
    <TagPickerModal />

    <!-- Auto-Update Modal -->
    <UpdateModal />

    <!-- Custom Right Click Context Menu -->
    <ContextMenu
      :target="contextMenuTarget"
      @close="contextMenuTarget = null"
      @open-settings="store.openSettingsModal('general')"
    />
  </div>
</template>
