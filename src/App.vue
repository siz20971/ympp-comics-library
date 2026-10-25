<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { useLibraryStore } from './stores/libraryStore';
import TopNav from './components/TopNav.vue';
import BreadcrumbNav from './components/BreadcrumbNav.vue';
import FolderCard from './components/FolderCard.vue';
import ComicCard from './components/ComicCard.vue';
import DetailPanel from './components/DetailPanel.vue';
import SettingsModal from './components/SettingsModal.vue';
import StatusBar from './components/StatusBar.vue';
import ContextMenu, { type ContextMenuTarget } from './components/ContextMenu.vue';
import { FolderOpen, SearchX, BookOpen, Layers, Play, FolderSearch } from 'lucide-vue-next';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type { ComicFileItem, FolderItem } from './types';

const store = useLibraryStore();
const isSettingsOpen = ref(false);

const contextMenuTarget = ref<ContextMenuTarget | null>(null);

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
});

onUnmounted(() => {
  window.removeEventListener('mouseup', handleMouseUp, { capture: true });
  window.removeEventListener('mousedown', handleMouseDown, { capture: true });
  window.removeEventListener('keydown', handleKeyDown);
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
    <TopNav @open-settings="isSettingsOpen = true" />

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
          class="flex-1 overflow-y-auto p-6 scroll-smooth"
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

          <!-- 2. Search Mode Results View -->
          <template v-else-if="store.mode === 'search'">
            <div
              v-if="store.searchResults.length === 0 && store.searchFolderResults.length === 0"
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

            <div v-else class="space-y-8 pb-12">
              <!-- Folders (Loose-image comics / series) -->
              <div v-if="store.searchFolderResults.length > 0">
                <h3 class="text-xs font-bold text-neutral-400 uppercase tracking-wider mb-3 flex items-center gap-2">
                  <Layers class="w-3.5 h-3.5 text-indigo-400" />
                  폴더 검색 결과 ({{ store.searchFolderResults.length }})
                </h3>
                <div
                  class="grid gap-4"
                  :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${store.cardMinWidth}px, 1fr))` }"
                >
                  <FolderCard
                    v-for="sub in store.searchFolderResults"
                    :key="sub.id"
                    :folder="sub"
                    :is-selected="store.selectedFolderId === sub.id"
                    @select-folder="store.selectFolder"
                    @open-folder="store.loadFolder"
                    @contextmenu="handleFolderContextMenu"
                  />
                </div>
              </div>

              <!-- Comic Files -->
              <div v-if="store.searchResults.length > 0">
                <h3
                  v-if="store.searchFolderResults.length > 0"
                  class="text-xs font-bold text-neutral-400 uppercase tracking-wider mb-3 flex items-center gap-2"
                >
                  <BookOpen class="w-3.5 h-3.5 text-indigo-400" />
                  만화책 파일 ({{ store.searchResults.length }})
                </h3>
                <div
                  class="grid gap-4"
                  :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${store.cardMinWidth}px, 1fr))` }"
                >
                  <ComicCard
                    v-for="comic in store.searchResults"
                    :key="comic.id"
                    :comic="comic"
                    :is-selected="store.selectedComicId === comic.id"
                    @select="store.selectComic"
                    @open="store.openComic"
                    @contextmenu="handleComicContextMenu"
                  />
                </div>
              </div>
            </div>
          </template>

          <!-- 3. Browse Mode View -->
          <template v-else>
            <!-- Subfolders Section -->
            <div v-if="store.folderContent.subfolders.length > 0" class="mb-8">
              <h3 class="text-xs font-bold text-neutral-400 uppercase tracking-wider mb-3 flex items-center gap-2">
                <Layers class="w-3.5 h-3.5 text-indigo-400" />
                하위 폴더 ({{ store.folderContent.subfolders.length }})
              </h3>
              <div
                class="grid gap-4"
                :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${store.cardMinWidth}px, 1fr))` }"
              >
                <FolderCard
                  v-for="sub in store.folderContent.subfolders"
                  :key="sub.id"
                  :folder="sub"
                  :is-selected="store.selectedFolderId === sub.id"
                  @select-folder="store.selectFolder"
                  @open-folder="store.loadFolder"
                  @contextmenu="handleFolderContextMenu"
                />
              </div>
            </div>

            <!-- Comic Files Section -->
            <div v-if="store.folderContent.files.length > 0">
              <h3 class="text-xs font-bold text-neutral-400 uppercase tracking-wider mb-3 flex items-center gap-2">
                <BookOpen class="w-3.5 h-3.5 text-indigo-400" />
                만화책 목록 ({{ store.folderContent.files.length }})
              </h3>
              <div
                class="grid gap-4 pb-12"
                :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${store.cardMinWidth}px, 1fr))` }"
              >
                <ComicCard
                  v-for="comic in store.folderContent.files"
                  :key="comic.id"
                  :comic="comic"
                  :is-selected="store.selectedComicId === comic.id"
                  @select="store.selectComic"
                  @open="store.openComic"
                  @contextmenu="handleComicContextMenu"
                />
              </div>
            </div>

            <!-- Empty Folder Notification -->
            <div
              v-if="store.folderContent.subfolders.length === 0 && store.folderContent.files.length === 0"
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
          </template>
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
      v-if="isSettingsOpen"
      @close="isSettingsOpen = false"
    />

    <!-- Custom Right Click Context Menu -->
    <ContextMenu
      :target="contextMenuTarget"
      @close="contextMenuTarget = null"
      @open-settings="isSettingsOpen = true"
    />
  </div>
</template>
