<script setup lang="ts">
import { computed } from 'vue';
import { useLibraryStore } from '../stores/libraryStore';
import { RefreshCw, XCircle, AlertCircle, Sparkles } from 'lucide-vue-next';

const store = useLibraryStore();

const newFilesCount = computed(() => {
  if (store.mode === 'browse') {
    return store.folderContent.files.filter((f) => f.is_new).length;
  }
  return store.searchResults.filter((f) => f.is_new).length;
});
</script>

<template>
  <footer class="h-8 border-t border-neutral-800 bg-neutral-950 px-4 flex items-center justify-between text-[11px] text-neutral-400 select-none shrink-0 z-20">
    <!-- Left: Status & Indexing Progress -->
    <div class="flex items-center gap-3">
      <div v-if="store.isScanning" class="flex items-center gap-2 text-indigo-400">
        <RefreshCw class="w-3 h-3 animate-spin" />
        <span>{{ store.scanProgress.current_status }}</span>
        <button
          @click="store.cancelScan"
          class="text-neutral-500 hover:text-red-400 flex items-center gap-0.5 ml-2 cursor-pointer"
        >
          <XCircle class="w-3 h-3" />
          <span>취소</span>
        </button>
      </div>

      <div v-else-if="store.scanProgress.last_error" class="flex items-center gap-1.5 text-amber-400">
        <AlertCircle class="w-3 h-3" />
        <span class="truncate max-w-md">{{ store.scanProgress.last_error }}</span>
      </div>

      <div v-else class="flex items-center gap-2 text-neutral-500">
        <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
        <span>정상 준비됨</span>
      </div>
    </div>

    <!-- Right: File & Folder Counts -->
    <div class="flex items-center gap-4">
      <template v-if="store.mode === 'browse'">
        <span>폴더 {{ store.folderContent.subfolders.length }}개</span>
        <span>파일 {{ store.folderContent.files.length }}권</span>
        <span v-if="newFilesCount > 0" class="flex items-center gap-1 text-emerald-400 font-medium">
          <Sparkles class="w-3 h-3" />
          NEW {{ newFilesCount }}권
        </span>
      </template>

      <template v-else>
        <span class="text-indigo-400 font-medium">검색 결과: {{ store.searchResults.length }}권</span>
        <span v-if="newFilesCount > 0" class="flex items-center gap-1 text-emerald-400 font-medium">
          <Sparkles class="w-3 h-3" />
          NEW {{ newFilesCount }}권
        </span>
      </template>
    </div>
  </footer>
</template>
