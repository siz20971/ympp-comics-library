<script setup lang="ts">
import { ref } from 'vue';
import { useLibraryStore } from '../stores/libraryStore';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  FolderOpen,
  RefreshCw,
  Search,
  SlidersHorizontal,
  Settings,
  X,
  Star,
  Tag,
} from 'lucide-vue-next';

const emit = defineEmits<{
  (e: 'open-settings'): void;
}>();

const store = useLibraryStore();
const showFilterMenu = ref(false);

async function handleSelectFolder() {
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
    console.error('Failed to open folder picker:', err);
  }
}

function handleSearchKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    store.executeSearch();
  }
}
</script>

<template>
  <header class="h-16 border-b border-neutral-800 bg-neutral-900/90 backdrop-blur px-4 flex items-center justify-between gap-4 z-20">
    <!-- Left: Logo & Library Picker -->
    <div class="flex items-center gap-3 shrink-0">
      <div class="flex items-center gap-2">
        <div class="w-8 h-8 rounded-lg bg-indigo-600 flex items-center justify-center font-bold text-white shadow-lg shadow-indigo-600/30">
          U
        </div>
        <span class="font-bold text-neutral-100 hidden sm:inline tracking-tight">Comics Viewer</span>
      </div>

      <button
        @click="handleSelectFolder"
        class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium border border-neutral-700/60 transition cursor-pointer"
        :title="store.library?.display_path ? `실제 경로: ${store.library.display_path}\n(상대 저장: ${store.library.root_path})` : '라이브러리 폴더 변경'"
      >
        <FolderOpen class="w-4 h-4 text-indigo-400" />
        <span class="max-w-[220px] truncate">
          {{ store.library ? (store.library.display_path || store.library.root_path) : '라이브러리 선택...' }}
        </span>
        <span
          v-if="store.library && store.library.root_path.startsWith('.')"
          class="px-1.5 py-0.2 rounded bg-indigo-950 text-indigo-300 text-[10px] border border-indigo-800/60 font-mono"
          title="실행파일 기준 상대 경로로 연동 중"
        >
          상대경로
        </span>
      </button>

      <button
        v-if="store.library"
        @click="store.startScan"
        :disabled="store.isScanning"
        class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 disabled:opacity-50 text-neutral-300 text-xs font-medium border border-neutral-700/60 transition cursor-pointer"
        title="수동 인덱싱 새로고침"
      >
        <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin text-indigo-400': store.isScanning }" />
        <span class="hidden md:inline">{{ store.isScanning ? '스캔 중' : '동기화' }}</span>
      </button>
    </div>

    <!-- Center: Search Controls -->
    <div class="flex-1 max-w-2xl flex items-center gap-2">
      <div class="relative flex-1">
        <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-neutral-400" />
        <input
          v-model="store.rawKeywordInput"
          @keydown="handleSearchKeydown"
          type="text"
          placeholder="파일명/폴더명 검색... (Enter)"
          class="w-full bg-neutral-950 border border-neutral-800 focus:border-indigo-500 focus:outline-none rounded-lg pl-9 pr-8 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 transition"
        />
        <button
          v-if="store.rawKeywordInput"
          @click="store.rawKeywordInput = ''; store.executeSearch()"
          class="absolute right-2.5 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-200"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>

      <div class="relative w-44 hidden md:block">
        <Tag class="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-400" />
        <input
          v-model="store.rawTagInput"
          @keydown="handleSearchKeydown"
          type="text"
          placeholder="#태그 검색..."
          class="w-full bg-neutral-950 border border-neutral-800 focus:border-indigo-500 focus:outline-none rounded-lg pl-8 pr-7 py-1.5 text-xs text-neutral-100 placeholder-neutral-500 transition"
        />
        <button
          v-if="store.rawTagInput"
          @click="store.rawTagInput = ''; store.executeSearch()"
          class="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-200"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>

      <button
        @click="store.executeSearch"
        class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-xs font-medium transition cursor-pointer shadow-sm shadow-indigo-600/30"
      >
        검색
      </button>

      <button
        @click="showFilterMenu = !showFilterMenu"
        class="p-1.5 rounded-lg border border-neutral-700/60 bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition cursor-pointer relative"
        :class="{ 'border-indigo-500 text-indigo-400': showFilterMenu || store.mode === 'search' }"
        title="검색 상세 옵션"
      >
        <SlidersHorizontal class="w-4 h-4" />
      </button>

      <button
        v-if="store.mode === 'search'"
        @click="store.clearSearch"
        class="px-2.5 py-1.5 bg-neutral-800 hover:bg-neutral-700 text-neutral-300 rounded-lg text-xs font-medium border border-neutral-700/60 transition cursor-pointer"
        title="탐색 모드로 돌아가기"
      >
        초기화
      </button>
    </div>

    <!-- Right: Settings -->
    <div class="flex items-center gap-2">
      <button
        @click="emit('open-settings')"
        class="p-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition cursor-pointer border border-neutral-700/60"
        title="환경 설정"
      >
        <Settings class="w-4 h-4" />
      </button>
    </div>
  </header>

  <!-- Filter Dropdown Options -->
  <div
    v-if="showFilterMenu"
    class="border-b border-neutral-800 bg-neutral-900/95 backdrop-blur px-6 py-3 flex flex-wrap items-center gap-6 text-xs text-neutral-300 shadow-xl animate-in slide-in-from-top-2 duration-150"
  >
    <!-- Operators -->
    <div class="flex items-center gap-2">
      <span class="text-neutral-400">키워드 결합:</span>
      <div class="inline-flex rounded-md bg-neutral-950 p-0.5 border border-neutral-800">
        <button
          @click="store.searchQuery.keyword_operator = 'AND'; store.executeSearch()"
          :class="store.searchQuery.keyword_operator === 'AND' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
          class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
        >
          AND
        </button>
        <button
          @click="store.searchQuery.keyword_operator = 'OR'; store.executeSearch()"
          :class="store.searchQuery.keyword_operator === 'OR' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
          class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
        >
          OR
        </button>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <span class="text-neutral-400">태그 결합:</span>
      <div class="inline-flex rounded-md bg-neutral-950 p-0.5 border border-neutral-800">
        <button
          @click="store.searchQuery.tag_operator = 'AND'; store.executeSearch()"
          :class="store.searchQuery.tag_operator === 'AND' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
          class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
        >
          AND
        </button>
        <button
          @click="store.searchQuery.tag_operator = 'OR'; store.executeSearch()"
          :class="store.searchQuery.tag_operator === 'OR' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
          class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
        >
          OR
        </button>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <span class="text-neutral-400">키워드와 태그 간:</span>
      <div class="inline-flex rounded-md bg-neutral-950 p-0.5 border border-neutral-800">
        <button
          @click="store.searchQuery.group_operator = 'AND'; store.executeSearch()"
          :class="store.searchQuery.group_operator === 'AND' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
          class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
        >
          AND
        </button>
        <button
          @click="store.searchQuery.group_operator = 'OR'; store.executeSearch()"
          :class="store.searchQuery.group_operator === 'OR' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
          class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
        >
          OR
        </button>
      </div>
    </div>

    <!-- Rating Filter -->
    <div class="flex items-center gap-2">
      <span class="text-neutral-400 flex items-center gap-1">
        <Star class="w-3.5 h-3.5 text-amber-400 fill-amber-400" /> 최소 별점:
      </span>
      <select
        v-model="store.searchQuery.min_rating"
        @change="store.executeSearch"
        class="bg-neutral-950 border border-neutral-800 rounded px-2 py-0.5 text-xs text-neutral-200 focus:outline-none focus:border-indigo-500 cursor-pointer"
      >
        <option :value="null">전체</option>
        <option :value="1">★ 1 이상</option>
        <option :value="2">★ 2 이상</option>
        <option :value="3">★ 3 이상</option>
        <option :value="4">★ 4 이상</option>
        <option :value="5">★ 5 만점</option>
      </select>
    </div>

    <!-- Sort By -->
    <div class="flex items-center gap-2">
      <span class="text-neutral-400">정렬:</span>
      <select
        v-model="store.searchQuery.sort_by"
        @change="store.executeSearch"
        class="bg-neutral-950 border border-neutral-800 rounded px-2 py-0.5 text-xs text-neutral-200 focus:outline-none focus:border-indigo-500 cursor-pointer"
      >
        <option value="name">이름순</option>
        <option value="rating">별점순</option>
        <option value="date">최근 추가순</option>
        <option value="size">용량순</option>
      </select>
      <select
        v-model="store.searchQuery.sort_order"
        @change="store.executeSearch"
        class="bg-neutral-950 border border-neutral-800 rounded px-2 py-0.5 text-xs text-neutral-200 focus:outline-none focus:border-indigo-500 cursor-pointer"
      >
        <option value="asc">오름차순</option>
        <option value="desc">내림차순</option>
      </select>
    </div>
  </div>
</template>
