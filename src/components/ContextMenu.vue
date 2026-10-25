<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { useLibraryStore } from '../stores/libraryStore';
import type { ComicFileItem, FolderItem } from '../types';
import {
  Play,
  FolderOpen,
  Star,
  FolderSearch,
  Info,
  ChevronRight,
  RotateCw,
  Settings,
} from 'lucide-vue-next';

export interface ContextMenuTarget {
  type: 'comic' | 'folder' | 'background';
  comic?: ComicFileItem;
  folder?: FolderItem;
  x: number;
  y: number;
}

const props = defineProps<{
  target: ContextMenuTarget | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'open-settings'): void;
}>();

const store = useLibraryStore();
const showRatingSubmenu = ref(false);

const positionStyle = computed(() => {
  if (!props.target) return {};
  const x = Math.min(props.target.x, window.innerWidth - 220);
  const y = Math.min(props.target.y, window.innerHeight - 280);
  return {
    left: `${x}px`,
    top: `${y}px`,
  };
});

function handleOpenInViewer() {
  if (props.target?.type === 'comic' && props.target.comic) {
    store.openComic(props.target.comic.id);
  } else if (props.target?.type === 'folder' && props.target.folder) {
    store.openFolderInViewer(props.target.folder.id);
  }
  emit('close');
}

function handleEnterFolder() {
  if (props.target?.type === 'folder' && props.target.folder) {
    store.loadFolder(props.target.folder.id);
  }
  emit('close');
}

function handleSelect() {
  if (props.target?.type === 'comic' && props.target.comic) {
    store.selectComic(props.target.comic.id);
  } else if (props.target?.type === 'folder' && props.target.folder) {
    store.selectFolder(props.target.folder.id);
  }
  emit('close');
}

function handleSetRating(rating: number) {
  if (props.target?.type === 'comic' && props.target.comic) {
    store.updateComicRating(props.target.comic.id, rating);
  } else if (props.target?.type === 'folder' && props.target.folder) {
    store.updateFolderRating(props.target.folder.id, rating);
  }
  emit('close');
}

function handleShowInExplorer() {
  if (props.target?.type === 'comic' && props.target.comic) {
    store.showInFileManager(props.target.comic.relative_path);
  } else if (props.target?.type === 'folder' && props.target.folder) {
    store.showInFileManager(props.target.folder.relative_path || '');
  }
  emit('close');
}

function handleRefreshItem() {
  if (props.target?.type === 'comic' && props.target.comic) {
    store.refreshItem('comic', props.target.comic.id);
  } else if (props.target?.type === 'folder' && props.target.folder) {
    store.refreshItem('folder', props.target.folder.id);
  }
  emit('close');
}

function handleRefreshCurrentFolder() {
  store.loadFolder(store.currentFolderId, true);
  emit('close');
}

function handleStartScan() {
  store.startScan();
  emit('close');
}

function handleOpenSettings() {
  emit('open-settings');
  emit('close');
}

function handleGlobalClick() {
  emit('close');
}

onMounted(() => {
  window.addEventListener('click', handleGlobalClick);
  window.addEventListener('contextmenu', handleGlobalClick);
});

onUnmounted(() => {
  window.removeEventListener('click', handleGlobalClick);
  window.removeEventListener('contextmenu', handleGlobalClick);
});
</script>

<template>
  <div
    v-if="target"
    :style="positionStyle"
    class="fixed z-50 w-52 bg-neutral-900/95 backdrop-blur-md border border-neutral-700/80 rounded-xl shadow-2xl p-1.5 text-xs text-neutral-200 select-none animate-in fade-in zoom-in-95 duration-100"
    @click.stop
  >
    <!-- Background (Empty space) Menu -->
    <template v-if="target.type === 'background'">
      <button
        @click="handleRefreshCurrentFolder"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer font-medium"
      >
        <RotateCw class="w-4 h-4 text-indigo-400" />
        <span>새로고침 (현재 폴더)</span>
      </button>

      <button
        @click="handleStartScan"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <FolderOpen class="w-4 h-4 text-neutral-400" />
        <span>라이브러리 전체 스캔</span>
      </button>

      <div class="h-px bg-neutral-800 my-1"></div>

      <button
        @click="handleOpenSettings"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <Settings class="w-4 h-4 text-neutral-400" />
        <span>설정</span>
      </button>
    </template>

    <!-- Comic / Folder Item Menu -->
    <template v-else>
      <!-- Item 1: Open in Viewer (both Comic and Folder) -->
      <button
        @click="handleOpenInViewer"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer font-medium"
      >
        <Play class="w-4 h-4 text-indigo-400 fill-indigo-400" />
        <span>{{ target.type === 'comic' ? '만화책 열기' : '만화 뷰어로 열기' }}</span>
      </button>

      <!-- Item 2: Enter Folder (Folder only) -->
      <button
        v-if="target.type === 'folder'"
        @click="handleEnterFolder"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <FolderOpen class="w-4 h-4 text-neutral-400" />
        <span>폴더 열기 (진입)</span>
      </button>

      <!-- Item 3: Rating with Submenu -->
      <div
        class="relative"
        @mouseenter="showRatingSubmenu = true"
        @mouseleave="showRatingSubmenu = false"
      >
        <button
          class="w-full flex items-center justify-between px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
          :class="{ 'bg-neutral-800': showRatingSubmenu }"
        >
          <span class="flex items-center gap-2.5">
            <Star class="w-4 h-4 text-amber-400 fill-amber-400" />
            <span>별점 편집</span>
          </span>
          <ChevronRight class="w-3.5 h-3.5 text-neutral-500" />
        </button>

        <!-- Rating Submenu -->
        <div
          v-if="showRatingSubmenu"
          class="absolute left-full top-0 ml-1 w-36 bg-neutral-900/95 backdrop-blur-md border border-neutral-700/80 rounded-xl shadow-2xl p-1.5 space-y-0.5 animate-in fade-in zoom-in-95 duration-75"
        >
          <button
            v-for="star in [5, 4, 3, 2, 1]"
            :key="star"
            @click="handleSetRating(star)"
            class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
          >
            <span class="flex items-center gap-1.5 text-amber-400">
              <Star v-for="s in star" :key="s" class="w-3 h-3 fill-amber-400" />
            </span>
            <span class="text-[10px] text-neutral-400">{{ star }}점</span>
          </button>

          <div class="h-px bg-neutral-800 my-1"></div>

          <button
            @click="handleSetRating(0)"
            class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 text-left transition cursor-pointer"
          >
            <span>별점 취소 (0점)</span>
          </button>
        </div>
      </div>

      <!-- Item 4: Show in Explorer -->
      <button
        @click="handleShowInExplorer"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <FolderSearch class="w-4 h-4 text-neutral-400" />
        <span>{{ target.type === 'comic' ? '파일 위치 열기' : '폴더 위치 열기' }}</span>
      </button>

      <!-- Item 5: Refresh (Thumbnail & Cache) -->
      <button
        @click="handleRefreshItem"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <RotateCw class="w-4 h-4 text-neutral-400" />
        <span>새로고침 (표지 재생성)</span>
      </button>

      <div class="h-px bg-neutral-800 my-1"></div>

      <!-- Item 6: Detail Info -->
      <button
        @click="handleSelect"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 text-left transition cursor-pointer"
      >
        <Info class="w-4 h-4" />
        <span>상세 정보 보기</span>
      </button>
    </template>
  </div>
</template>
