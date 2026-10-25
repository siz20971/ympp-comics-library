<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useLibraryStore } from '../stores/libraryStore';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  X,
  Folder,
  FolderOpen,
  Save,
  MonitorPlay,
  Sparkles,
  Tag,
  Trash2,
  Edit2,
  Plus,
  RotateCcw,
  AlertTriangle,
  Sliders,
} from 'lucide-vue-next';

import type { CardSizeMode } from '../types';

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const store = useLibraryStore();

const activeTab = ref<'general' | 'tags'>('general');

// General settings
const customViewer = ref(store.settings.custom_viewer_path || '');
const autoClearNew = ref(store.settings.auto_clear_new_on_open);
const showFolderNewBadge = ref(store.settings.show_folder_new_badge ?? false);
const cardSize = ref<CardSizeMode>(store.settings.card_size || 'md');
const includeFolderTagsInSearch = ref(store.settings.include_folder_tags_in_search ?? false);

const cardSizeOptions: { value: CardSizeMode; label: string; width: number }[] = [
  { value: 'xs', label: '가장 작게', width: 110 },
  { value: 'sm', label: '작게', width: 140 },
  { value: 'md', label: '기본', width: 180 },
  { value: 'lg', label: '크게', width: 230 },
  { value: 'xl', label: '가장 크게', width: 300 },
];

const cardSizeLabels: Record<CardSizeMode, string> = {
  xs: '가장 작게 (110px)',
  sm: '작게 (140px)',
  md: '기본 (180px)',
  lg: '크게 (230px)',
  xl: '가장 크게 (300px)',
};

// Tag management
const newTagName = ref('');
const editingTagId = ref<number | null>(null);
const editingTagName = ref('');

// Rebuild DB confirm
const showRebuildConfirm = ref(false);
const isRebuilding = ref(false);

onMounted(() => {
  store.loadTagsWithCounts();
});

async function selectCustomViewer() {
  try {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      filters: [
        {
          name: '실행 프로그램',
          extensions: ['exe', 'bat', 'cmd'],
        },
      ],
      title: '만화책 뷰어 프로그램 선택',
    });
    if (selected && typeof selected === 'string') {
      customViewer.value = selected;
    }
  } catch (err) {
    console.error('Failed to pick viewer exe:', err);
  }
}

async function handleSaveGeneral() {
  await store.saveSettings({
    library_path: store.settings.library_path,
    custom_viewer_path: customViewer.value.trim() ? customViewer.value.trim() : null,
    auto_clear_new_on_open: autoClearNew.value,
    thumbnail_size: 480,
    card_size: cardSize.value,
    include_folder_tags_in_search: includeFolderTagsInSearch.value,
    show_folder_new_badge: showFolderNewBadge.value,
  });
  if (store.mode === 'search') {
    await store.executeSearch();
  }
  emit('close');
}

// Tag Management Functions
async function handleCreateTag() {
  if (newTagName.value.trim()) {
    await store.createTag(newTagName.value.trim());
    newTagName.value = '';
  }
}

function startEditTag(tag: { id: number; display_name: string }) {
  editingTagId.value = tag.id;
  editingTagName.value = tag.display_name;
}

async function saveEditTag(tagId: number) {
  if (editingTagName.value.trim()) {
    await store.updateTag(tagId, editingTagName.value.trim());
    editingTagId.value = null;
    editingTagName.value = '';
  }
}

async function handleDeleteTag(tagId: number, tagName: string) {
  if (confirm(`'#${tagName}' 태그를 정말 삭제하시겠습니까?\n이 태그가 지정된 모든 만화책과 폴더에서 제거됩니다.`)) {
    await store.deleteTag(tagId);
  }
}

// DB Rebuild Function
async function handleExecuteRebuild() {
  showRebuildConfirm.value = false;
  isRebuilding.value = true;
  try {
    await store.rebuildDatabase();
    alert('데이터베이스 및 표지 캐시 재구축이 완료되었습니다.');
    emit('close');
  } catch (err) {
    alert('재구축 실패: ' + err);
  } finally {
    isRebuilding.value = false;
  }
}
</script>

<template>
  <div class="fixed inset-0 bg-black/75 backdrop-blur-sm flex items-center justify-center z-50 p-4">
    <div class="w-full max-w-2xl bg-neutral-900 border border-neutral-800 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
      <!-- Header with Tabs -->
      <div class="h-14 border-b border-neutral-800 px-6 flex items-center justify-between shrink-0 bg-neutral-950/60">
        <div class="flex items-center gap-1">
          <button
            @click="activeTab = 'general'"
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition cursor-pointer"
            :class="activeTab === 'general' ? 'bg-neutral-800 text-indigo-400 border border-neutral-700/80 shadow' : 'text-neutral-400 hover:text-white'"
          >
            <Sliders class="w-3.5 h-3.5" />
            일반 설정
          </button>
          <button
            @click="activeTab = 'tags'; store.loadTagsWithCounts()"
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition cursor-pointer"
            :class="activeTab === 'tags' ? 'bg-neutral-800 text-indigo-400 border border-neutral-700/80 shadow' : 'text-neutral-400 hover:text-white'"
          >
            <Tag class="w-3.5 h-3.5" />
            저장된 태그 관리 ({{ store.tagsWithCounts.length }})
          </button>
        </div>

        <div class="flex items-center gap-2">
          <span class="px-2 py-0.5 rounded-full bg-indigo-950/80 border border-indigo-800/60 text-indigo-300 text-[10px] font-mono font-semibold">
            v0.1.0
          </span>
          <button
            @click="emit('close')"
            class="p-1 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-white transition cursor-pointer"
          >
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Tab 1: General Settings -->
      <div v-if="activeTab === 'general'" class="p-6 space-y-6 text-xs text-neutral-300 overflow-y-auto flex-1">
        <!-- Library Path -->
        <div class="space-y-2">
          <label class="font-semibold text-neutral-200 block">라이브러리 경로</label>
          <div class="space-y-1.5">
            <div class="flex items-center gap-2">
              <span class="text-neutral-500 w-16 shrink-0">실제 경로:</span>
              <input
                type="text"
                readonly
                :value="store.library?.display_path || store.library?.root_path || '설정되지 않음'"
                class="flex-1 bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-neutral-300 font-mono text-xs focus:outline-none"
              />
            </div>
            <div class="flex items-center gap-2">
              <span class="text-neutral-500 w-16 shrink-0">상대 저장:</span>
              <input
                type="text"
                readonly
                :value="store.library?.root_path || '설정되지 않음'"
                class="flex-1 bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-1.5 text-indigo-300 font-mono text-xs focus:outline-none"
              />
            </div>
          </div>
          <p class="text-[11px] text-neutral-500">
            상단 바의 폴더 버튼을 클릭하여 언제든지 라이브러리 폴더를 변경할 수 있습니다.
          </p>
        </div>

        <!-- Custom Viewer -->
        <div class="space-y-2">
          <label class="font-semibold text-neutral-200 flex items-center gap-1.5">
            <MonitorPlay class="w-4 h-4 text-indigo-400" />
            외부 뷰어 실행 프로그램 (.exe)
          </label>
          <div class="flex items-center gap-2">
            <input
              v-model="customViewer"
              type="text"
              placeholder="비워둘 경우 OS 기본 연결 프로그램으로 실행됩니다."
              class="flex-1 bg-neutral-950 border border-neutral-800 rounded-lg px-3 py-2 text-white font-mono text-xs focus:border-indigo-500 focus:outline-none"
            />
            <button
              @click="selectCustomViewer"
              class="px-3 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 rounded-lg border border-neutral-700 transition cursor-pointer flex items-center gap-1.5"
            >
              <FolderOpen class="w-3.5 h-3.5" />
              찾아보기
            </button>
          </div>
          <p class="text-[11px] text-neutral-500">
            예: Honeyview, NeeView, Bandiview 등 선호하는 뷰어 경로를 직접 지정할 수 있습니다.
          </p>
        </div>

        <!-- Card Item Size (5 stages) -->
        <div class="space-y-3 p-4 rounded-xl bg-neutral-950 border border-neutral-800/80">
          <div class="flex items-center justify-between">
            <label class="font-semibold text-neutral-200 flex items-center gap-1.5">
              <Sliders class="w-4 h-4 text-indigo-400" />
              카드 항목 크기 (5단계)
            </label>
            <span class="text-xs font-semibold text-indigo-400">
              {{ cardSizeLabels[cardSize] }}
            </span>
          </div>

          <div class="grid grid-cols-5 gap-2">
            <button
              v-for="opt in cardSizeOptions"
              :key="opt.value"
              type="button"
              @click="cardSize = opt.value"
              class="py-2 px-1 rounded-lg border text-center transition cursor-pointer flex flex-col items-center gap-1"
              :class="cardSize === opt.value
                ? 'bg-indigo-600/30 border-indigo-500 text-white font-bold ring-1 ring-indigo-500'
                : 'bg-neutral-900 border-neutral-800 text-neutral-400 hover:text-neutral-200 hover:border-neutral-700'"
            >
              <span class="text-xs">{{ opt.label }}</span>
              <span class="text-[10px] text-neutral-500">{{ opt.width }}px</span>
            </button>
          </div>
          <p class="text-[11px] text-neutral-500">
            리스트 뷰에 표시되는 폴더 및 만화책 카드의 너비를 5단계로 조절합니다.
          </p>
        </div>

        <!-- Auto Clear New Policy -->
        <div class="p-3.5 rounded-xl bg-neutral-950 border border-neutral-800/80 flex items-start gap-3">
          <input
            v-model="autoClearNew"
            id="auto-new"
            type="checkbox"
            class="mt-0.5 rounded border-neutral-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
          />
          <label for="auto-new" class="cursor-pointer space-y-0.5">
            <span class="font-semibold text-white flex items-center gap-1">
              <Sparkles class="w-3.5 h-3.5 text-emerald-400" />
              만화책 열람 시 'NEW' 상태 자동 해제
            </span>
            <p class="text-[11px] text-neutral-400">
              만화책을 열 때 신규 배지(NEW)를 자동으로 읽음 처리합니다.
            </p>
          </label>
        </div>

        <!-- Show Folder NEW Badge -->
        <div class="p-3.5 rounded-xl bg-neutral-950 border border-neutral-800/80 flex items-start gap-3">
          <input
            v-model="showFolderNewBadge"
            id="show-folder-new"
            type="checkbox"
            class="mt-0.5 rounded border-neutral-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
          />
          <label for="show-folder-new" class="cursor-pointer space-y-0.5">
            <span class="font-semibold text-white flex items-center gap-1.5">
              <Folder class="w-3.5 h-3.5 text-indigo-400" />
              폴더 항목에 'NEW' 배지 표시
            </span>
            <p class="text-[11px] text-neutral-400">
              새로 추가된 폴더에도 만화책과 동일하게 'NEW' 배지를 표시합니다. 비활성화 시 폴더에는 NEW 배지가 표시되지 않습니다. (기본값: 꺼짐)
            </p>
          </label>
        </div>

        <!-- Include Folder in Search Policy -->
        <div class="p-3.5 rounded-xl bg-neutral-950 border border-neutral-800/80 flex items-start gap-3">
          <input
            v-model="includeFolderTagsInSearch"
            id="include-folder-tags"
            type="checkbox"
            class="mt-0.5 rounded border-neutral-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
          />
          <label for="include-folder-tags" class="cursor-pointer space-y-0.5">
            <span class="font-semibold text-white flex items-center gap-1.5">
              <FolderOpen class="w-3.5 h-3.5 text-indigo-400" />
              검색 결과에 폴더 자체도 포함 (낱개 이미지 만화책)
            </span>
            <p class="text-[11px] text-neutral-400">
              폴더 자체가 낱개 이미지 만화책인 경우를 위해, 활성화 시 검색 조건(태그/이름/별점)에 일치하는 폴더 자체를 검색 결과에 함께 표시합니다. 비활성화 시에는 상속 태그를 포함한 만화책 파일들만 표시됩니다. (기본값: 꺼짐)
            </p>
          </label>
        </div>

        <!-- .ymppCache Portable Storage Info -->
        <div class="space-y-1 p-3.5 rounded-xl bg-neutral-950/80 border border-neutral-800 text-[11px] text-neutral-400">
          <p class="text-indigo-400 font-semibold mb-1 flex items-center gap-1.5">
            📁 라이브러리 내장 캐시 (.ymppCache)
          </p>
          <p>· 데이터베이스: <code class="text-neutral-200">&lt;라이브러리&gt;/.ymppCache/comics_library.db</code></p>
          <p>· 표지 캐시: <code class="text-neutral-200">&lt;라이브러리&gt;/.ymppCache/covers/</code></p>
          <p>· 만화책 폴더를 다른 외장하드나 PC로 옮겨도 메타데이터와 표지 캐시가 그대로 유지됩니다.</p>
        </div>

        <!-- Rebuild Database Section -->
        <div class="p-4 rounded-xl border border-red-900/40 bg-red-950/20 space-y-3">
          <div class="flex items-center justify-between">
            <div>
              <h4 class="font-bold text-red-400 flex items-center gap-1.5">
                <AlertTriangle class="w-4 h-4" />
                DB 및 표지 캐시 완전 재구축
              </h4>
              <p class="text-[11px] text-neutral-400 mt-0.5">
                현재 라이브러리의 인덱스, 태그 및 캐시를 완전히 초기화하고 파일 시스템을 처음부터 다시 스캔합니다.
              </p>
            </div>
            <button
              @click="showRebuildConfirm = true"
              :disabled="isRebuilding || !store.library"
              class="px-3.5 py-2 rounded-lg bg-red-600/90 hover:bg-red-600 text-white font-medium text-xs flex items-center gap-1.5 transition cursor-pointer disabled:opacity-50 shrink-0"
            >
              <RotateCcw class="w-3.5 h-3.5" :class="{ 'animate-spin': isRebuilding }" />
              DB 완전 재구축
            </button>
          </div>
        </div>

        <!-- Build Version Info -->
        <div class="flex items-center justify-between text-[11px] text-neutral-500 pt-3 border-t border-neutral-800/80">
          <span>YMPP Comics Library</span>
          <span class="font-mono text-neutral-400">빌드 버전: v0.1.0</span>
        </div>
      </div>

      <!-- Tab 2: Tag Management -->
      <div v-else-if="activeTab === 'tags'" class="p-6 space-y-4 text-xs text-neutral-300 overflow-y-auto flex-1">
        <!-- New Tag Registration Form -->
        <div class="flex items-center gap-2 p-3 bg-neutral-950 rounded-xl border border-neutral-800">
          <input
            v-model="newTagName"
            @keydown.enter="handleCreateTag"
            type="text"
            placeholder="등록할 새 태그 이름 입력 (예: 일상물, 명작)..."
            class="flex-1 bg-neutral-900 border border-neutral-800 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:border-indigo-500"
          />
          <button
            @click="handleCreateTag"
            class="px-3.5 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-xs flex items-center gap-1.5 transition cursor-pointer"
          >
            <Plus class="w-3.5 h-3.5" />
            태그 등록
          </button>
        </div>

        <!-- Tags Table / List -->
        <div class="rounded-xl border border-neutral-800 overflow-hidden bg-neutral-950">
          <div class="grid grid-cols-12 bg-neutral-900/80 px-4 py-2.5 font-semibold text-neutral-400 text-[11px] border-b border-neutral-800">
            <div class="col-span-5">태그명</div>
            <div class="col-span-2 text-center">만화책 수</div>
            <div class="col-span-2 text-center">폴더 수</div>
            <div class="col-span-3 text-right">관리</div>
          </div>

          <div v-if="store.tagsWithCounts.length === 0" class="p-8 text-center text-neutral-500 text-xs">
            등록된 태그가 없습니다.
          </div>

          <div
            v-for="tag in store.tagsWithCounts"
            :key="tag.id"
            class="grid grid-cols-12 items-center px-4 py-2.5 border-b border-neutral-900 hover:bg-neutral-900/40 transition text-xs"
          >
            <!-- Tag Name (or edit input) -->
            <div class="col-span-5 flex items-center gap-2">
              <template v-if="editingTagId === tag.id">
                <input
                  v-model="editingTagName"
                  @keydown.enter="saveEditTag(tag.id)"
                  type="text"
                  class="bg-neutral-900 border border-indigo-500 rounded px-2 py-1 text-xs text-white focus:outline-none"
                  autoFocus
                />
                <button
                  @click="saveEditTag(tag.id)"
                  class="px-2 py-1 rounded bg-indigo-600 text-white text-[10px] cursor-pointer"
                >
                  저장
                </button>
                <button
                  @click="editingTagId = null"
                  class="px-2 py-1 rounded bg-neutral-800 text-neutral-400 text-[10px] cursor-pointer"
                >
                  취소
                </button>
              </template>
              <template v-else>
                <span class="inline-flex items-center px-2 py-0.5 rounded bg-indigo-950 border border-indigo-800 text-indigo-300 font-medium text-xs">
                  #{{ tag.display_name }}
                </span>
              </template>
            </div>

            <!-- File Count -->
            <div class="col-span-2 text-center text-neutral-300 font-mono">
              {{ tag.file_count }}권
            </div>

            <!-- Folder Count -->
            <div class="col-span-2 text-center text-neutral-300 font-mono">
              {{ tag.folder_count }}개
            </div>

            <!-- Actions -->
            <div class="col-span-3 flex items-center justify-end gap-1.5">
              <button
                v-if="editingTagId !== tag.id"
                @click="startEditTag(tag)"
                class="p-1.5 rounded hover:bg-neutral-800 text-neutral-400 hover:text-indigo-400 transition cursor-pointer"
                title="태그 이름 수정"
              >
                <Edit2 class="w-3.5 h-3.5" />
              </button>
              <button
                @click="handleDeleteTag(tag.id, tag.display_name)"
                class="p-1.5 rounded hover:bg-neutral-800 text-neutral-400 hover:text-red-400 transition cursor-pointer"
                title="태그 삭제"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>

        <!-- Build Version Info -->
        <div class="flex items-center justify-between text-[11px] text-neutral-500 pt-3 border-t border-neutral-800/80">
          <span>YMPP Comics Library</span>
          <span class="font-mono text-neutral-400">빌드 버전: v0.1.0</span>
        </div>
      </div>

      <!-- Footer for General Settings Tab -->
      <div v-if="activeTab === 'general'" class="h-14 border-t border-neutral-800 px-6 flex items-center justify-end gap-2 bg-neutral-900/50 shrink-0">
        <button
          @click="emit('close')"
          class="px-4 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs cursor-pointer transition"
        >
          취소
        </button>
        <button
          @click="handleSaveGeneral"
          class="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-xs flex items-center gap-1.5 cursor-pointer shadow-lg shadow-indigo-600/30 transition"
        >
          <Save class="w-3.5 h-3.5" />
          설정 저장
        </button>
      </div>

      <!-- Footer for Tags Tab -->
      <div v-else class="h-14 border-t border-neutral-800 px-6 flex items-center justify-between bg-neutral-900/50 shrink-0 text-xs text-neutral-400">
        <span>* 태그 이름을 수정하면 연결된 모든 만화책과 폴더의 태그가 즉시 변경됩니다.</span>
        <button
          @click="emit('close')"
          class="px-4 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs cursor-pointer transition"
        >
          닫기
        </button>
      </div>
    </div>

    <!-- Rebuild Confirm Dialog Modal -->
    <div
      v-if="showRebuildConfirm"
      class="fixed inset-0 bg-black/80 flex items-center justify-center z-60 p-4"
    >
      <div class="w-full max-w-md bg-neutral-900 border border-red-800 rounded-2xl shadow-2xl p-6 space-y-4 text-xs">
        <div class="flex items-center gap-3 text-red-400">
          <AlertTriangle class="w-6 h-6 shrink-0" />
          <h4 class="text-sm font-bold text-white">데이터베이스를 완전히 재구축하시겠습니까?</h4>
        </div>
        <p class="text-neutral-300 leading-relaxed">
          라이브러리의 <code class="text-amber-300">.ymppCache</code> 내의 데이터베이스와 표지 캐시를 모두 비우고 처음부터 전체 재스캔을 진행합니다.
          <br /><br />
          수동으로 등록한 코멘트, 별점, 태그 정보가 초기화될 수 있습니다. 계속 진행하시겠습니까?
        </p>
        <div class="flex justify-end gap-2 pt-2">
          <button
            @click="showRebuildConfirm = false"
            class="px-4 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs cursor-pointer"
          >
            취소
          </button>
          <button
            @click="handleExecuteRebuild"
            class="px-4 py-2 rounded-lg bg-red-600 hover:bg-red-500 text-white font-semibold text-xs cursor-pointer shadow-lg shadow-red-600/30"
          >
            확인 및 재구축 실행
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
