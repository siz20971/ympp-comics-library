<script setup lang="ts">
import { ref, watch } from 'vue';
import { useLibraryStore } from '../stores/libraryStore';
import {
  X,
  Play,
  Star,
  Tag,
  Plus,
  Folder,
  Calendar,
  HardDrive,
  FileText,
  Sparkles,
  FolderOpen,
  FolderSearch,
  Layers,
  BookOpen,
  Info,
  RotateCw,
} from 'lucide-vue-next';

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const store = useLibraryStore();

// Comic states
const newDirectTag = ref('');
const newInheritedTag = ref('');
const isEditingComment = ref(false);
const commentDraft = ref('');

// Folder states
const newFolderTag = ref('');

watch(
  () => store.selectedComicDetail,
  (detail) => {
    if (detail) {
      commentDraft.value = detail.comment;
    }
  },
  { immediate: true }
);

function handleAddComicTag() {
  if (newDirectTag.value.trim() && store.selectedComicDetail) {
    store.addTagToComic(store.selectedComicDetail.id, newDirectTag.value.trim());
    newDirectTag.value = '';
  }
}

function handleAddInheritedTag() {
  if (newInheritedTag.value.trim() && store.selectedComicDetail) {
    store.addTagToFolder(store.selectedComicDetail.folder_id, newInheritedTag.value.trim());
    newInheritedTag.value = '';
  }
}

function handleAddFolderTag() {
  if (newFolderTag.value.trim() && store.selectedFolderDetail) {
    store.addTagToFolder(store.selectedFolderDetail.id, newFolderTag.value.trim());
    newFolderTag.value = '';
  }
}

function handleSaveComment() {
  if (store.selectedComicDetail) {
    store.updateComicComment(store.selectedComicDetail.id, commentDraft.value);
    isEditingComment.value = false;
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) {
    return (bytes / 1024).toFixed(0) + ' KB';
  }
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr);
    return d.toLocaleString('ko-KR', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return dateStr;
  }
}
</script>

<template>
  <aside
    class="w-80 md:w-96 border-l border-neutral-800 bg-neutral-900/95 backdrop-blur flex flex-col h-full z-10 select-none shadow-2xl shrink-0"
  >
    <!-- Header -->
    <div class="h-12 border-b border-neutral-800 px-4 flex items-center justify-between gap-2 shrink-0">
      <span class="text-xs font-semibold text-neutral-300 flex items-center gap-1.5">
        <template v-if="store.selectedFolderDetail">
          <Folder class="w-4 h-4 text-indigo-400" />
          <span>폴더 상세 정보</span>
        </template>
        <template v-else-if="store.selectedComicDetail">
          <BookOpen class="w-4 h-4 text-indigo-400" />
          <span>만화책 상세 정보</span>
        </template>
        <template v-else>
          <Info class="w-4 h-4 text-neutral-500" />
          <span class="text-neutral-400">상세 정보</span>
        </template>
      </span>
      <button
        v-if="store.selectedComicDetail || store.selectedFolderDetail"
        @click="emit('close')"
        class="p-1 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-white transition cursor-pointer"
        title="선택 해제"
      >
        <X class="w-4 h-4" />
      </button>
    </div>

    <!-- Scrollable Content -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      <!-- ================= FOLDER DETAIL VIEW ================= -->
      <template v-if="store.selectedFolderDetail">
        <!-- Folder Cover or Icon Box -->
        <div
          v-if="store.selectedFolderDetail.cover_data_url"
          class="w-full aspect-[3/4] bg-neutral-950 rounded-xl overflow-hidden border border-neutral-800 relative shadow-inner"
        >
          <img
            :src="store.selectedFolderDetail.cover_data_url"
            :alt="store.selectedFolderDetail.name"
            class="w-full h-full object-contain"
          />
          <div
            v-if="store.settings.show_folder_new_badge && store.selectedFolderDetail.is_new"
            class="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-500 text-white text-[10px] font-bold shadow z-10"
          >
            <Sparkles class="w-3 h-3" />
            신규 (NEW)
          </div>
          <div
            class="absolute left-2 flex items-center gap-1 px-2 py-0.5 rounded-md bg-neutral-950/80 border border-neutral-700/60 text-indigo-300 text-[10px] font-semibold backdrop-blur"
            :class="(store.settings.show_folder_new_badge && store.selectedFolderDetail.is_new) ? 'top-8' : 'top-2'"
          >
            <Folder class="w-3 h-3 text-indigo-400" />
            폴더
          </div>
          <button
            @click="store.refreshItem('folder', store.selectedFolderDetail.id)"
            class="absolute top-2 right-2 p-1.5 rounded-lg bg-neutral-950/80 hover:bg-neutral-800 border border-neutral-800 text-neutral-400 hover:text-white backdrop-blur transition cursor-pointer"
            title="폴더 표지 새로고침 (리프레시)"
          >
            <RotateCw class="w-3.5 h-3.5" />
          </button>
        </div>

        <div
          v-else
          class="w-full aspect-[4/3] bg-neutral-950 rounded-xl flex flex-col items-center justify-center border border-neutral-800 shadow-inner p-4 text-center relative"
        >
          <div
            v-if="store.settings.show_folder_new_badge && store.selectedFolderDetail.is_new"
            class="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-500 text-white text-[10px] font-bold shadow z-10"
          >
            <Sparkles class="w-3 h-3" />
            신규 (NEW)
          </div>
          <button
            @click="store.refreshItem('folder', store.selectedFolderDetail.id)"
            class="absolute top-2 right-2 p-1.5 rounded-lg bg-neutral-900/80 hover:bg-neutral-800 border border-neutral-800 text-neutral-400 hover:text-white transition cursor-pointer"
            title="폴더 표지 새로고침 (리프레시)"
          >
            <RotateCw class="w-3.5 h-3.5" />
          </button>
          <Folder class="w-16 h-16 text-indigo-500 mb-2" />
          <h2 class="text-sm font-bold text-white break-words line-clamp-2 px-2">
            {{ store.selectedFolderDetail.name }}
          </h2>
          <span class="text-[11px] text-neutral-500 mt-1 font-mono break-all px-2">
            {{ store.selectedFolderDetail.relative_path || '루트 폴더' }}
          </span>
        </div>

        <!-- Action Buttons -->
        <div class="space-y-2">
          <button
            @click="store.openFolderInViewer(store.selectedFolderDetail.id)"
            class="w-full py-2.5 px-4 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-xs flex items-center justify-center gap-2 transition duration-200 shadow-lg shadow-indigo-600/30 cursor-pointer"
          >
            <Play class="w-4 h-4 fill-white" />
            만화 뷰어로 폴더 열기
          </button>
          <button
            @click="store.loadFolder(store.selectedFolderDetail.id)"
            class="w-full py-2 px-4 rounded-xl bg-neutral-800 hover:bg-neutral-700 text-neutral-200 font-medium text-xs flex items-center justify-center gap-2 transition border border-neutral-700 cursor-pointer"
          >
            <FolderOpen class="w-3.5 h-3.5 text-indigo-400" />
            폴더 열기 (진입)
          </button>
          <button
            @click="store.showInFileManager(store.selectedFolderDetail.relative_path || '')"
            class="w-full py-2 px-4 rounded-xl bg-neutral-800 hover:bg-neutral-700 text-neutral-200 font-medium text-xs flex items-center justify-center gap-2 transition border border-neutral-700 cursor-pointer"
          >
            <FolderSearch class="w-3.5 h-3.5" />
            탐색기에서 폴더 위치 열기
          </button>
        </div>

        <!-- Title & Basic Info if has cover -->
        <div v-if="store.selectedFolderDetail.cover_data_url" class="space-y-1">
          <h2 class="text-sm font-bold text-white break-words leading-snug">
            {{ store.selectedFolderDetail.name }}
          </h2>
          <span class="text-[11px] text-neutral-500 font-mono break-all block">
            {{ store.selectedFolderDetail.relative_path || '루트 폴더' }}
          </span>
        </div>

        <!-- Folder Rating -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-xs font-semibold text-neutral-300">폴더 별점</span>
            <div class="flex items-center gap-0.5">
              <button
                v-for="s in 5"
                :key="s"
                @click="store.updateFolderRating(store.selectedFolderDetail.id, s === store.selectedFolderDetail.rating ? 0 : s)"
                class="p-0.5 hover:scale-125 transition cursor-pointer"
              >
                <Star
                  class="w-4 h-4"
                  :class="s <= store.selectedFolderDetail.rating ? 'text-amber-400 fill-amber-400' : 'text-neutral-700'"
                />
              </button>
              <span class="text-xs font-bold text-amber-400 ml-1.5">{{ store.selectedFolderDetail.rating }}점</span>
            </div>
          </div>
          <p class="text-[10px] text-neutral-500">
            * 폴더 별점은 하위 파일에 상속되지 않는 독립된 평가입니다.
          </p>
        </div>

        <!-- Folder Tags (Inherited to children) -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
              <Tag class="w-3.5 h-3.5 text-indigo-400" />
              폴더 직접 태그
            </span>
            <span class="text-[10px] text-emerald-400 font-medium">하위 파일에 자동 상속</span>
          </div>

          <div class="flex flex-wrap items-center gap-1.5">
            <span
              v-for="tag in store.selectedFolderDetail.direct_tags"
              :key="tag.id"
              class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-indigo-950 border border-indigo-800 text-indigo-300 text-xs"
            >
              #{{ tag.display_name }}
              <button
                @click="store.removeTagFromFolder(store.selectedFolderDetail.id, tag.id)"
                class="hover:text-red-400 cursor-pointer"
              >
                <X class="w-3 h-3" />
              </button>
            </span>

            <div class="flex items-center gap-1 w-full mt-1">
              <input
                v-model="newFolderTag"
                @keydown.enter="handleAddFolderTag"
                type="text"
                placeholder="+ 폴더 태그 추가... (Enter)"
                class="flex-1 bg-neutral-900 border border-neutral-800 focus:border-indigo-500 rounded px-2 py-1 text-xs text-white focus:outline-none"
              />
              <button
                @click="handleAddFolderTag"
                class="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-indigo-400 hover:text-white cursor-pointer"
              >
                <Plus class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>

        <!-- Folder Stats -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-2 text-xs text-neutral-300">
          <div class="flex items-center justify-between">
            <span class="text-neutral-500 flex items-center gap-1.5"><Layers class="w-3.5 h-3.5" /> 하위 폴더:</span>
            <span class="font-semibold">{{ store.selectedFolderDetail.subfolder_count }}개</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-neutral-500 flex items-center gap-1.5"><BookOpen class="w-3.5 h-3.5" /> 만화책 파일:</span>
            <span class="font-semibold">{{ store.selectedFolderDetail.file_count }}권</span>
          </div>
        </div>
      </template>

      <!-- ================= COMIC FILE DETAIL VIEW ================= -->
      <template v-else-if="store.selectedComicDetail">
        <!-- Cover Preview -->
        <div class="w-full aspect-[3/4] bg-neutral-950 rounded-xl overflow-hidden border border-neutral-800 relative shadow-inner">
          <img
            v-if="store.selectedComicDetail.cover_data_url"
            :src="store.selectedComicDetail.cover_data_url"
            :alt="store.selectedComicDetail.name"
            class="w-full h-full object-contain"
          />
          <div v-else class="w-full h-full flex flex-col items-center justify-center text-neutral-600 gap-2">
            <FileText class="w-12 h-12 stroke-[1.5]" />
            <span class="text-xs">표지 이미지 없음</span>
          </div>

          <div
            v-if="store.selectedComicDetail.is_new"
            class="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-500 text-white text-[10px] font-bold shadow"
          >
            <Sparkles class="w-3 h-3" />
            신규 (NEW)
          </div>

          <button
            @click="store.refreshItem('comic', store.selectedComicDetail.id)"
            class="absolute top-2 right-2 p-1.5 rounded-lg bg-neutral-950/80 hover:bg-neutral-800 border border-neutral-800 text-neutral-400 hover:text-white backdrop-blur transition cursor-pointer"
            title="만화책 표지 새로고침 (리프레시)"
          >
            <RotateCw class="w-3.5 h-3.5" />
          </button>
        </div>

        <!-- Action Buttons -->
        <div class="space-y-2">
          <button
            @click="store.openComic(store.selectedComicDetail.id)"
            class="w-full py-2.5 px-4 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-xs flex items-center justify-center gap-2 transition duration-200 shadow-lg shadow-indigo-600/30 cursor-pointer"
          >
            <Play class="w-4 h-4 fill-white" />
            만화책 열기 (뷰어 실행)
          </button>
          <button
            @click="store.showInFileManager(store.selectedComicDetail.relative_path)"
            class="w-full py-2 px-4 rounded-xl bg-neutral-800 hover:bg-neutral-700 text-neutral-200 font-medium text-xs flex items-center justify-center gap-2 transition border border-neutral-700 cursor-pointer"
          >
            <FolderSearch class="w-3.5 h-3.5" />
            탐색기에서 파일 위치 열기
          </button>
        </div>

        <!-- Title & Basic Info -->
        <div class="space-y-1">
          <h2 class="text-sm font-bold text-white break-words leading-snug">
            {{ store.selectedComicDetail.name }}
          </h2>
          <div class="flex items-center gap-2 text-[11px] text-neutral-400">
            <span class="font-mono uppercase font-semibold text-indigo-400">{{ store.selectedComicDetail.extension }}</span>
            <span>·</span>
            <span>{{ formatBytes(store.selectedComicDetail.size) }}</span>
          </div>
        </div>

        <!-- Rating Section -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-xs font-semibold text-neutral-300">파일 별점</span>
            <div class="flex items-center gap-0.5">
              <button
                v-for="s in 5"
                :key="s"
                @click="store.updateComicRating(store.selectedComicDetail.id, s === store.selectedComicDetail.rating ? 0 : s)"
                class="p-0.5 hover:scale-125 transition cursor-pointer"
              >
                <Star
                  class="w-4 h-4"
                  :class="s <= store.selectedComicDetail.rating ? 'text-amber-400 fill-amber-400' : 'text-neutral-700'"
                />
              </button>
              <span class="text-xs font-bold text-amber-400 ml-1.5">{{ store.selectedComicDetail.rating }}점</span>
            </div>
          </div>

          <div class="flex items-center justify-between text-[11px] text-neutral-400 pt-1 border-t border-neutral-900">
            <span class="flex items-center gap-1">
              <Folder class="w-3 h-3 text-neutral-500" />
              상위 폴더 별점
            </span>
            <span class="font-medium text-neutral-300">
              {{ store.selectedComicDetail.folder_rating > 0 ? `${store.selectedComicDetail.folder_rating}점` : '미평가' }}
            </span>
          </div>
        </div>

        <!-- Tags Section: Direct vs Inherited -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-3">
          <!-- Direct Tags -->
          <div class="space-y-1.5">
            <div class="flex items-center justify-between">
              <span class="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
                <Tag class="w-3.5 h-3.5 text-indigo-400" />
                직접 지정 태그
              </span>
              <span class="text-[10px] text-neutral-500">파일 고유 태그</span>
            </div>

            <div class="flex flex-wrap items-center gap-1.5">
              <span
                v-for="tag in store.selectedComicDetail.direct_tags"
                :key="tag.id"
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-indigo-950 border border-indigo-800 text-indigo-300 text-xs"
              >
                #{{ tag.display_name }}
                <button
                  @click="store.removeTagFromComic(store.selectedComicDetail.id, tag.id)"
                  class="hover:text-red-400 cursor-pointer"
                >
                  <X class="w-3 h-3" />
                </button>
              </span>

              <!-- Tag Input -->
              <div class="flex items-center gap-1 w-full mt-1">
                <input
                  v-model="newDirectTag"
                  @keydown.enter="handleAddComicTag"
                  type="text"
                  placeholder="+ 새 태그 추가... (Enter)"
                  class="flex-1 bg-neutral-900 border border-neutral-800 focus:border-indigo-500 rounded px-2 py-1 text-xs text-white focus:outline-none"
                />
                <button
                  @click="handleAddComicTag"
                  class="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-indigo-400 hover:text-white cursor-pointer"
                >
                  <Plus class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>

          <!-- Inherited Tags -->
          <div class="space-y-1.5 pt-2 border-t border-neutral-900">
            <div class="flex items-center justify-between">
              <span class="text-xs font-semibold text-neutral-300 flex items-center gap-1.5">
                <Folder class="w-3.5 h-3.5 text-indigo-400" />
                상속된 태그 (상위 폴더)
              </span>
              <span class="text-[10px] text-emerald-400 font-medium">폴더 지정 태그</span>
            </div>

            <div class="flex flex-wrap items-center gap-1.5">
              <span
                v-for="tag in store.selectedComicDetail.inherited_tags"
                :key="tag.id"
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-neutral-900 border border-neutral-800 text-neutral-300 text-xs"
                title="상위 폴더에 지정된 태그입니다. 삭제 시 상위 폴더에서 제거됩니다."
              >
                #{{ tag.display_name }}
                <button
                  @click="store.removeTagFromFolder(store.selectedComicDetail.folder_id, tag.id)"
                  class="hover:text-red-400 cursor-pointer"
                  title="상위 폴더에서 태그 삭제"
                >
                  <X class="w-3 h-3" />
                </button>
              </span>
              <p
                v-if="store.selectedComicDetail.inherited_tags.length === 0"
                class="text-[11px] text-neutral-600 italic"
              >
                상속된 태그 없음
              </p>

              <!-- Tag Input for Folder Tag from Comic View -->
              <div class="flex items-center gap-1 w-full mt-1">
                <input
                  v-model="newInheritedTag"
                  @keydown.enter="handleAddInheritedTag"
                  type="text"
                  placeholder="+ 상위 폴더에 태그 추가... (Enter)"
                  class="flex-1 bg-neutral-900 border border-neutral-800 focus:border-indigo-500 rounded px-2 py-1 text-xs text-white focus:outline-none"
                />
                <button
                  @click="handleAddInheritedTag"
                  class="p-1 rounded bg-neutral-800 hover:bg-neutral-700 text-indigo-400 hover:text-white cursor-pointer"
                  title="상위 폴더에 태그 추가"
                >
                  <Plus class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Comment Section -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-xs font-semibold text-neutral-300">코멘트 / 메모</span>
            <button
              v-if="!isEditingComment"
              @click="isEditingComment = true"
              class="text-[11px] text-indigo-400 hover:text-indigo-300 cursor-pointer"
            >
              편집
            </button>
          </div>

          <div v-if="isEditingComment" class="space-y-2">
            <textarea
              v-model="commentDraft"
              rows="3"
              placeholder="이 만화책에 대한 감상이나 메모를 작성하세요..."
              class="w-full bg-neutral-900 border border-neutral-800 focus:border-indigo-500 rounded p-2 text-xs text-white focus:outline-none resize-none"
            ></textarea>
            <div class="flex justify-end gap-2">
              <button
                @click="isEditingComment = false; commentDraft = store.selectedComicDetail.comment"
                class="px-2.5 py-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-400 text-xs cursor-pointer"
              >
                취소
              </button>
              <button
                @click="handleSaveComment"
                class="px-2.5 py-1 rounded bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-medium cursor-pointer"
              >
                저장
              </button>
            </div>
          </div>

          <p v-else class="text-xs text-neutral-400 whitespace-pre-wrap min-h-[2.5rem] bg-neutral-900/60 p-2 rounded border border-neutral-900">
            {{ store.selectedComicDetail.comment || '등록된 코멘트가 없습니다.' }}
          </p>
        </div>

        <!-- File Meta Info -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800/80 space-y-1.5 text-[11px] text-neutral-400 font-mono">
          <div class="flex items-start justify-between gap-2">
            <span class="text-neutral-500 flex items-center gap-1 shrink-0"><Folder class="w-3 h-3" /> 폴더:</span>
            <span class="text-neutral-300 truncate text-right">{{ store.selectedComicDetail.folder_name }}</span>
          </div>
          <div class="flex items-start justify-between gap-2">
            <span class="text-neutral-500 flex items-center gap-1 shrink-0"><Calendar class="w-3 h-3" /> 수정일:</span>
            <span class="text-neutral-300">{{ formatDate(store.selectedComicDetail.modified_at) }}</span>
          </div>
          <div class="flex items-start justify-between gap-2">
            <span class="text-neutral-500 flex items-center gap-1 shrink-0"><HardDrive class="w-3 h-3" /> 전체 경로:</span>
            <span class="text-neutral-300 break-all text-right font-sans text-[10px]">{{ store.selectedComicDetail.full_path }}</span>
          </div>
        </div>
      </template>

      <!-- ================= EMPTY PLACEHOLDER VIEW ================= -->
      <template v-else>
        <div class="h-full flex flex-col items-center justify-center text-center p-6 text-neutral-500 gap-3 min-h-[300px]">
          <div class="w-14 h-14 rounded-2xl bg-neutral-950 border border-neutral-800 flex items-center justify-center text-neutral-600 shadow-inner">
            <Info class="w-6 h-6 stroke-[1.5]" />
          </div>
          <div class="space-y-1">
            <p class="text-xs font-medium text-neutral-400">선택된 항목 없음</p>
            <p class="text-[11px] text-neutral-600 leading-relaxed">
              폴더나 만화책을 클릭하면<br />태그, 별점, 상세 정보가 여기에 표시됩니다.
            </p>
          </div>
        </div>
      </template>
    </div>
  </aside>
</template>
