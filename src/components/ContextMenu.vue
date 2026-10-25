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
  Sparkles,
  Check,
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
  const y = Math.min(props.target.y, window.innerHeight - 320);
  return {
    left: `${x}px`,
    top: `${y}px`,
  };
});

const submenuStyle = computed(() => {
  if (!props.target) return {};
  const isNearRightEdge = props.target.x > window.innerWidth - 380;
  return isNearRightEdge
    ? { right: '100%', marginRight: '4px', top: '0px' }
    : { left: '100%', marginLeft: '4px', top: '0px' };
});

const currentItemRating = computed(() => {
  if (props.target?.type === 'comic' && props.target.comic) {
    return props.target.comic.rating;
  }
  if (props.target?.type === 'folder' && props.target.folder) {
    return props.target.folder.rating;
  }
  return 0;
});

const currentItemIsNew = computed(() => {
  if (props.target?.type === 'comic' && props.target.comic) {
    return props.target.comic.is_new;
  }
  if (props.target?.type === 'folder' && props.target.folder) {
    return props.target.folder.is_new;
  }
  return false;
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

async function handleToggleNew() {
  if (props.target?.type === 'comic' && props.target.comic) {
    await store.toggleComicIsNew(props.target.comic.id);
  } else if (props.target?.type === 'folder' && props.target.folder) {
    await store.toggleFolderIsNew(props.target.folder.id);
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

const menuRef = ref<HTMLElement | null>(null);

function handleGlobalClick(e: MouseEvent) {
  if (menuRef.value && menuRef.value.contains(e.target as Node)) {
    return;
  }
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
    ref="menuRef"
    :style="positionStyle"
    class="fixed z-50 w-56 bg-neutral-900/95 backdrop-blur-md border border-neutral-700/80 rounded-xl shadow-2xl p-1.5 text-xs text-neutral-200 select-none animate-in fade-in zoom-in-95 duration-100"
    @click.stop
    @contextmenu.prevent.stop
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
      <!-- Item 1: 열기 -->
      <button
        @click="target.type === 'folder' ? handleEnterFolder() : handleOpenInViewer()"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer font-medium"
      >
        <FolderOpen v-if="target.type === 'folder'" class="w-4 h-4 text-indigo-400" />
        <Play v-else class="w-4 h-4 text-indigo-400 fill-indigo-400" />
        <span>{{ target.type === 'folder' ? '열기 (폴더 진입)' : '열기' }}</span>
      </button>

      <!-- Item 2: 뷰어로 열기 (폴더 및 파일 모두 지원) -->
      <button
        @click="handleOpenInViewer"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer font-medium"
      >
        <Play class="w-4 h-4 text-emerald-400 fill-emerald-400" />
        <span>{{ target.type === 'folder' ? '뷰어로 열기 (만화 뷰어)' : '뷰어로 열기' }}</span>
      </button>

      <div class="h-px bg-neutral-800 my-1"></div>

      <!-- Item 3: New 상태 토글하기 -->
      <button
        @click="handleToggleNew"
        class="w-full flex items-center justify-between px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <span class="flex items-center gap-2.5">
          <Sparkles class="w-4 h-4" :class="currentItemIsNew ? 'text-emerald-400' : 'text-neutral-400'" />
          <span>New 상태 토글하기</span>
        </span>
        <span
          class="text-[10px] font-bold px-1.5 py-0.5 rounded-full uppercase"
          :class="currentItemIsNew ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' : 'bg-neutral-800 text-neutral-400'"
        >
          {{ currentItemIsNew ? 'NEW ON' : 'OFF' }}
        </span>
      </button>

      <!-- Item 4: 별점 매기기 (0~5 하위 메뉴) -->
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
            <span>별점 매기기</span>
          </span>
          <div class="flex items-center gap-1.5">
            <span v-if="currentItemRating > 0" class="text-[11px] font-semibold text-amber-400">
              {{ currentItemRating }}점
            </span>
            <ChevronRight class="w-3.5 h-3.5 text-neutral-500" />
          </div>
        </button>

        <!-- Rating Submenu -->
        <div
          v-if="showRatingSubmenu"
          :style="submenuStyle"
          class="absolute w-38 bg-neutral-900/95 backdrop-blur-md border border-neutral-700/80 rounded-xl shadow-2xl p-1.5 space-y-0.5 animate-in fade-in zoom-in-95 duration-75 z-50"
        >
          <button
            v-for="star in [5, 4, 3, 2, 1]"
            :key="star"
            @click="handleSetRating(star)"
            class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
            :class="{ 'bg-neutral-800/60 font-semibold': currentItemRating === star }"
          >
            <span class="flex items-center gap-1 text-amber-400">
              <Star v-for="s in star" :key="s" class="w-3 h-3 fill-amber-400" />
            </span>
            <div class="flex items-center gap-1.5">
              <span class="text-[10px] text-neutral-400">{{ star }}점</span>
              <Check v-if="currentItemRating === star" class="w-3 h-3 text-emerald-400" />
            </div>
          </button>

          <div class="h-px bg-neutral-800 my-1"></div>

          <button
            @click="handleSetRating(0)"
            class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 text-left transition cursor-pointer"
            :class="{ 'bg-neutral-800/60 font-semibold': currentItemRating === 0 }"
          >
            <span>별점 해제 (0점)</span>
            <Check v-if="currentItemRating === 0" class="w-3 h-3 text-neutral-400" />
          </button>
        </div>
      </div>

      <div class="h-px bg-neutral-800 my-1"></div>

      <!-- Item 5: Show in Explorer -->
      <button
        @click="handleShowInExplorer"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <FolderSearch class="w-4 h-4 text-neutral-400" />
        <span>{{ target.type === 'comic' ? '파일 위치 열기' : '폴더 위치 열기' }}</span>
      </button>

      <!-- Item 6: Refresh (Thumbnail & Cache) -->
      <button
        @click="handleRefreshItem"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left transition cursor-pointer"
      >
        <RotateCw class="w-4 h-4 text-neutral-400" />
        <span>새로고침 (표지 재생성)</span>
      </button>

      <div class="h-px bg-neutral-800 my-1"></div>

      <!-- Item 7: Detail Info -->
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
