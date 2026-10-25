<script setup lang="ts">
import { onUnmounted } from 'vue';
import type { FolderItem } from '../types';
import { Folder, Star } from 'lucide-vue-next';

const props = defineProps<{
  folder: FolderItem;
  isSelected?: boolean;
}>();

const emit = defineEmits<{
  (e: 'select-folder', folderId: number): void;
  (e: 'open-folder', folderId: number): void;
  (e: 'contextmenu', event: MouseEvent, folder: FolderItem): void;
}>();

let clickTimer: ReturnType<typeof setTimeout> | null = null;

function handleClick() {
  if (clickTimer) {
    clearTimeout(clickTimer);
    clickTimer = null;
    return;
  }
  clickTimer = setTimeout(() => {
    clickTimer = null;
    emit('select-folder', props.folder.id);
  }, 220);
}

function handleDblClick() {
  if (clickTimer) {
    clearTimeout(clickTimer);
    clickTimer = null;
  }
  emit('open-folder', props.folder.id);
}

onUnmounted(() => {
  if (clickTimer) {
    clearTimeout(clickTimer);
    clickTimer = null;
  }
});
</script>

<template>
  <div
    @click="handleClick"
    @dblclick="handleDblClick"
    @contextmenu.prevent="emit('contextmenu', $event, folder)"
    class="group relative flex flex-col rounded-xl overflow-hidden bg-neutral-900/60 border transition duration-200 cursor-pointer select-none shadow-md hover:shadow-xl hover:shadow-indigo-500/10"
    :class="isSelected ? 'border-indigo-500 ring-2 ring-indigo-500/40 bg-neutral-800/80' : 'border-neutral-800 hover:border-neutral-700'"
  >
    <!-- Cover Aspect Container -->
    <div class="relative w-full aspect-[3/4] bg-neutral-950 flex items-center justify-center overflow-hidden">
      <!-- Thumbnail Image -->
      <img
        v-if="folder.cover_data_url"
        :src="folder.cover_data_url"
        :alt="folder.name"
        class="w-full h-full object-cover group-hover:scale-105 transition duration-300"
        loading="lazy"
      />
      <!-- Fallback Folder Graphic when no thumbnail -->
      <div v-else class="w-full h-full flex flex-col items-center justify-center text-indigo-400/70 bg-gradient-to-b from-neutral-900/80 to-neutral-950 p-4 gap-2">
        <div class="w-14 h-14 rounded-2xl bg-indigo-950/40 border border-indigo-900/40 flex items-center justify-center group-hover:scale-110 group-hover:text-indigo-300 transition duration-200 shadow-inner">
          <Folder class="w-8 h-8 text-indigo-400 group-hover:text-indigo-300" />
        </div>
      </div>

      <!-- Folder Pill Badge (Top Left) -->
      <div
        class="absolute top-2 left-2 flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-neutral-950/80 border border-neutral-800 text-indigo-300 text-[10px] font-semibold backdrop-blur"
      >
        <Folder class="w-3 h-3 text-indigo-400 fill-indigo-400/40" />
        폴더
      </div>

      <!-- Rating Badge (Top Right) -->
      <div
        v-if="folder.rating > 0"
        class="absolute top-2 right-2 flex items-center gap-0.5 px-1.5 py-0.5 rounded-md bg-neutral-950/80 border border-amber-500/40 text-amber-400 text-[10px] font-bold backdrop-blur"
      >
        <Star class="w-2.5 h-2.5 fill-amber-400 text-amber-400" />
        {{ folder.rating }}
      </div>
    </div>

    <!-- Title & Tags Body -->
    <div class="p-2.5 flex flex-col gap-1.5 flex-1 justify-between bg-neutral-900/80">
      <div class="w-full">
        <h3
          class="text-xs font-semibold text-neutral-200 group-hover:text-white line-clamp-2 leading-snug"
          :title="folder.name"
        >
          {{ folder.name }}
        </h3>
      </div>

      <!-- Tags preview -->
      <div v-if="folder.direct_tags.length > 0" class="flex items-center gap-1 mt-1 overflow-hidden max-w-full">
        <span
          v-for="t in folder.direct_tags.slice(0, 2)"
          :key="t.id"
          class="px-1.5 py-0.5 rounded bg-indigo-950/70 border border-indigo-800/40 text-[9px] text-indigo-300 truncate"
        >
          #{{ t.display_name }}
        </span>
        <span v-if="folder.direct_tags.length > 2" class="text-[9px] text-neutral-500">
          +{{ folder.direct_tags.length - 2 }}
        </span>
      </div>
    </div>
  </div>
</template>
