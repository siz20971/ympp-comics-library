<script setup lang="ts">
import { useLibraryStore } from '../stores/libraryStore';
import { ChevronRight, ArrowLeft, ArrowRight, ArrowUp, Folder, Star, Tag, Plus, X } from 'lucide-vue-next';

const store = useLibraryStore();
</script>

<template>
  <div class="h-12 border-b border-neutral-800/80 bg-neutral-900/40 px-4 flex items-center justify-between gap-4 text-xs">
    <!-- Breadcrumb items & Nav Buttons -->
    <div class="flex items-center gap-1 overflow-x-auto py-1 scrollbar-none">
      <!-- 뒤로 가기 (마우스 4번 / Alt+←) -->
      <button
        @click="store.goBack"
        :disabled="!store.canGoBack"
        class="flex items-center justify-center p-1 rounded hover:bg-neutral-800 disabled:opacity-30 disabled:pointer-events-none text-neutral-400 hover:text-neutral-100 transition cursor-pointer"
        title="뒤로 가기 (마우스 4번 / Alt+←)"
      >
        <ArrowLeft class="w-3.5 h-3.5" />
      </button>

      <!-- 앞으로 가기 (마우스 5번 / Alt+→) -->
      <button
        @click="store.goForward"
        :disabled="!store.canGoForward"
        class="flex items-center justify-center p-1 rounded hover:bg-neutral-800 disabled:opacity-30 disabled:pointer-events-none text-neutral-400 hover:text-neutral-100 transition cursor-pointer"
        title="앞으로 가기 (마우스 5번 / Alt+→)"
      >
        <ArrowRight class="w-3.5 h-3.5" />
      </button>

      <!-- 상위 폴더로 이동 (Alt+↑) -->
      <button
        @click="store.goToParent"
        :disabled="store.folderContent.parent_folder === null"
        class="flex items-center justify-center p-1 rounded hover:bg-neutral-800 disabled:opacity-30 disabled:pointer-events-none text-neutral-400 hover:text-neutral-100 transition cursor-pointer mr-1.5"
        title="상위 폴더로 이동 (Alt+↑)"
      >
        <ArrowUp class="w-3.5 h-3.5" />
      </button>

      <div class="h-3.5 w-[1px] bg-neutral-800 mr-1 shrink-0" />

      <template v-for="(item, index) in store.folderContent.breadcrumbs" :key="index">
        <ChevronRight v-if="index > 0" class="w-3.5 h-3.5 text-neutral-600 shrink-0" />
        <button
          @click="store.loadFolder(item.id)"
          class="flex items-center gap-1.5 px-2 py-1 rounded hover:bg-neutral-800 transition cursor-pointer font-medium truncate max-w-[180px]"
          :class="index === store.folderContent.breadcrumbs.length - 1 ? 'text-indigo-400 font-semibold' : 'text-neutral-400 hover:text-neutral-200'"
        >
          <Folder v-if="index === 0" class="w-3.5 h-3.5 text-indigo-400 shrink-0" />
          <span>{{ item.name }}</span>
        </button>
      </template>
    </div>

    <!-- Current Folder Info (Rating & Tags) if not root -->
    <div v-if="store.folderContent.current_folder && store.folderContent.current_folder.relative_path !== ''" class="flex items-center gap-3 shrink-0">
      <!-- Folder Rating -->
      <div class="flex items-center gap-1 text-neutral-400">
        <span class="text-[11px]">폴더 별점:</span>
        <div class="flex items-center">
          <button
            v-for="star in 5"
            :key="star"
            @click="store.updateFolderRating(store.folderContent.current_folder!.id, star === store.folderContent.current_folder!.rating ? 0 : star)"
            class="p-0.5 hover:scale-125 transition cursor-pointer"
            :title="`별점 ${star}점`"
          >
            <Star
              class="w-3.5 h-3.5"
              :class="star <= store.folderContent.current_folder!.rating ? 'text-amber-400 fill-amber-400' : 'text-neutral-600'"
            />
          </button>
        </div>
      </div>

      <!-- Folder Direct Tags -->
      <div class="flex items-center gap-1.5">
        <Tag class="w-3.5 h-3.5 text-indigo-400" />
        <div class="flex items-center gap-1">
          <span
            v-for="tag in store.folderContent.current_folder.direct_tags"
            :key="tag.id"
            class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-indigo-950/80 border border-indigo-800/60 text-indigo-300 text-[10px]"
          >
            #{{ tag.display_name }}
            <button
              @click="store.removeTagFromFolder(store.folderContent.current_folder!.id, tag.id)"
              class="hover:text-red-400 cursor-pointer"
            >
              <X class="w-2.5 h-2.5" />
            </button>
          </span>

          <button
            @click="
              store.openFolderTagPicker(
                store.folderContent.current_folder!.id,
                store.folderContent.current_folder!.name,
                store.folderContent.current_folder!.direct_tags
              )
            "
            class="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white text-[10px] transition cursor-pointer"
            title="폴더 태그 선택 / 추가 (하위 만화책에 자동 상속됨)"
          >
            <Plus class="w-3 h-3" />
            <span>태그</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
