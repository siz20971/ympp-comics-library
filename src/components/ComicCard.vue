<script setup lang="ts">
import { onUnmounted } from 'vue';
import type { ComicFileItem } from '../types';
import { BookOpen, Star, Sparkles } from 'lucide-vue-next';

const props = defineProps<{
  comic: ComicFileItem;
  isSelected: boolean;
}>();

const emit = defineEmits<{
  (e: 'select', fileId: number): void;
  (e: 'open', fileId: number): void;
  (e: 'contextmenu', event: MouseEvent, comic: ComicFileItem): void;
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
    emit('select', props.comic.id);
  }, 220);
}

function handleDblClick() {
  if (clickTimer) {
    clearTimeout(clickTimer);
    clickTimer = null;
  }
  emit('open', props.comic.id);
}

onUnmounted(() => {
  if (clickTimer) {
    clearTimeout(clickTimer);
    clickTimer = null;
  }
});

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) {
    return (bytes / 1024).toFixed(0) + ' KB';
  }
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}
</script>

<template>
  <div
    @click="handleClick"
    @dblclick="handleDblClick"
    @contextmenu.prevent="emit('contextmenu', $event, comic)"
    class="group relative flex flex-col rounded-xl overflow-hidden bg-neutral-900/60 border transition duration-200 cursor-pointer select-none shadow-md hover:shadow-xl hover:shadow-indigo-500/10"
    :class="isSelected ? 'border-indigo-500 ring-2 ring-indigo-500/40 bg-neutral-800/80' : 'border-neutral-800 hover:border-neutral-700'"
  >
    <!-- Cover Aspect Container -->
    <div class="relative w-full aspect-[3/4] bg-neutral-950 flex items-center justify-center overflow-hidden">
      <!-- Thumbnail Image -->
      <img
        v-if="comic.cover_data_url"
        :src="comic.cover_data_url"
        :alt="comic.name"
        class="w-full h-full object-cover group-hover:scale-105 transition duration-300"
        loading="lazy"
      />
      <!-- Placeholder when no cover -->
      <div v-else class="flex flex-col items-center justify-center text-neutral-600 gap-2 p-4 text-center">
        <BookOpen class="w-10 h-10 stroke-[1.5]" />
        <span class="text-[10px] text-neutral-500 line-clamp-2">{{ comic.name }}</span>
      </div>

      <!-- New Badge (Phase 4 requirement) -->
      <div
        v-if="comic.is_new"
        class="absolute top-2 left-2 flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-emerald-500/90 text-white text-[9px] font-bold shadow-lg uppercase tracking-wider backdrop-blur"
      >
        <Sparkles class="w-2.5 h-2.5" />
        NEW
      </div>

      <!-- Rating Badge -->
      <div
        v-if="comic.rating > 0"
        class="absolute top-2 right-2 flex items-center gap-0.5 px-1.5 py-0.5 rounded-md bg-neutral-950/80 border border-amber-500/40 text-amber-400 text-[10px] font-bold backdrop-blur"
      >
        <Star class="w-2.5 h-2.5 fill-amber-400 text-amber-400" />
        {{ comic.rating }}
      </div>

      <!-- Format / Size pill -->
      <div class="absolute bottom-2 right-2 px-1.5 py-0.5 rounded bg-neutral-950/80 text-neutral-400 text-[9px] font-mono border border-neutral-800 backdrop-blur">
        {{ comic.extension.toUpperCase() }} · {{ formatSize(comic.size) }}
      </div>
    </div>

    <!-- Title & Tags Body -->
    <div class="p-2.5 flex flex-col gap-1.5 flex-1 justify-between bg-neutral-900/80">
      <div class="w-full">
        <h3
          class="text-xs font-semibold text-neutral-200 group-hover:text-white line-clamp-2 leading-snug"
          :title="comic.name"
        >
          {{ comic.name }}
        </h3>
      </div>

      <!-- Tags Preview: Direct (blue/indigo) vs Inherited (neutral/purple) -->
      <div class="flex flex-wrap items-center gap-1 overflow-hidden max-h-10">
        <!-- Direct Tags -->
        <span
          v-for="tag in comic.direct_tags.slice(0, 2)"
          :key="'d-' + tag.id"
          class="inline-flex items-center px-1.5 py-0.2 rounded bg-indigo-950/70 border border-indigo-800/60 text-indigo-300 text-[9px]"
          title="직접 지정 태그"
        >
          #{{ tag.display_name }}
        </span>

        <!-- Inherited Tags -->
        <span
          v-for="tag in comic.inherited_tags.slice(0, 2)"
          :key="'i-' + tag.id"
          class="inline-flex items-center px-1.5 py-0.2 rounded bg-neutral-800 border border-neutral-700 text-neutral-400 text-[9px]"
          title="상위 폴더 상속 태그"
        >
          #{{ tag.display_name }}
        </span>

        <span
          v-if="comic.direct_tags.length + comic.inherited_tags.length > 4"
          class="text-[9px] text-neutral-500"
        >
          +{{ comic.direct_tags.length + comic.inherited_tags.length - 4 }}
        </span>
      </div>
    </div>
  </div>
</template>
