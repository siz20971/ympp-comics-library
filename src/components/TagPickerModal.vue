<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { cleanTagDisplay, normalizeTagKey, useLibraryStore } from '../stores/libraryStore';
import {
  X,
  Tag,
  Plus,
  Search,
  Check,
  Settings,
  AlertCircle,
  CheckCircle2,
} from 'lucide-vue-next';

const store = useLibraryStore();

const filterQuery = ref('');
const newTagInput = ref('');
const selectedMap = ref<Map<string, string>>(new Map());
const feedback = ref<{ type: 'info' | 'success' | 'warn'; message: string } | null>(null);
const isSubmitting = ref(false);

const ctx = computed(() => store.tagPickerContext);

const modalTitle = computed(() => {
  if (!ctx.value) return '';
  if (ctx.value.mode === 'search') return '검색할 태그 선택';
  if (ctx.value.mode === 'folder') return '폴더 태그 등록 / 편집';
  return '작품 태그 등록 / 편집';
});

const modalSubtitle = computed(() => {
  if (!ctx.value) return '';
  if (ctx.value.mode === 'search') {
    return '등록된 태그 목록에서 다중 선택하거나 새 태그를 추가할 수 있습니다.';
  }
  if (ctx.value.mode === 'folder') {
    return `${ctx.value.targetTitle || '폴더'} · 지정된 태그는 하위 만화책에 자동 상속됩니다.`;
  }
  return ctx.value.targetTitle || '선택한 만화책';
});

watch(
  ctx,
  (next) => {
    filterQuery.value = '';
    newTagInput.value = '';
    feedback.value = null;
    const map = new Map<string, string>();
    if (next) {
      for (const raw of next.initialTags) {
        const display = cleanTagDisplay(raw);
        const key = display.toLowerCase();
        if (key && !map.has(key)) {
          map.set(key, display);
        }
      }
      if (store.library && store.allTags.length === 0) {
        store.loadTagsWithCounts();
      }
    }
    selectedMap.value = map;
  },
  { immediate: true }
);

const selectedList = computed(() => Array.from(selectedMap.value.entries()));

const filteredOptions = computed(() => {
  const q = normalizeTagKey(filterQuery.value);
  if (!q) return store.availableTags;
  return store.availableTags.filter(
    (t) => t.key.includes(q) || t.display_name.toLowerCase().includes(q)
  );
});

function toggleTag(key: string, displayName: string) {
  const next = new Map(selectedMap.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.set(key, displayName);
  }
  selectedMap.value = next;
}

function clearSelected() {
  selectedMap.value = new Map();
}

async function handleAddNewTag() {
  const raw = newTagInput.value;
  const clean = cleanTagDisplay(raw);
  if (!clean) return;
  if (!store.library) {
    feedback.value = {
      type: 'warn',
      message: '라이브러리 폴더를 먼저 선택해주세요.',
    };
    return;
  }

  try {
    const res = await store.ensureTagExists(clean);
    if (!res.tag) return;

    const next = new Map(selectedMap.value);
    next.set(res.tag.key, res.tag.display_name);
    selectedMap.value = next;
    newTagInput.value = '';

    if (res.duplicate) {
      feedback.value = {
        type: 'warn',
        message: `이미 존재하는 태그 '#${res.tag.display_name}'이(가) 있어 중복 추가하지 않고 선택했습니다.`,
      };
    } else {
      feedback.value = {
        type: 'success',
        message: `새 태그 '#${res.tag.display_name}'이(가) 추가 및 선택되었습니다.`,
      };
    }
  } catch (err) {
    feedback.value = {
      type: 'warn',
      message: `태그 추가 실패: ${err}`,
    };
  }
}

async function handleConfirm() {
  if (isSubmitting.value) return;
  isSubmitting.value = true;
  try {
    const chosen = Array.from(selectedMap.value.values());
    await store.applyTagPickerSelection(chosen);
  } finally {
    isSubmitting.value = false;
  }
}

function handleOpenSettingsTags() {
  store.closeTagPicker();
  store.openSettingsModal('tags');
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && ctx.value) {
    e.stopPropagation();
    store.closeTagPicker();
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div
    v-if="ctx"
    class="fixed inset-0 bg-black/75 backdrop-blur-sm flex items-center justify-center z-50 p-4"
    @click.self="store.closeTagPicker()"
  >
    <div
      class="w-full max-w-xl bg-neutral-900 border border-neutral-800 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]"
      @click.stop
    >
      <!-- Header -->
      <div class="px-5 py-3.5 border-b border-neutral-800 bg-neutral-950/60 flex items-center justify-between gap-3 shrink-0">
        <div class="min-w-0">
          <h3 class="text-sm font-bold text-white flex items-center gap-2">
            <Tag class="w-4 h-4 text-indigo-400 shrink-0" />
            <span>{{ modalTitle }}</span>
          </h3>
          <p class="text-[11px] text-neutral-400 truncate mt-0.5" :title="modalSubtitle">
            {{ modalSubtitle }}
          </p>
        </div>

        <div class="flex items-center gap-1.5 shrink-0">
          <button
            type="button"
            @click="handleOpenSettingsTags"
            class="px-2.5 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white text-[11px] flex items-center gap-1 border border-neutral-700/60 transition cursor-pointer"
            title="설정에서 태그 및 커스텀 태그 관리 열기"
          >
            <Settings class="w-3.5 h-3.5 text-indigo-400" />
            <span class="hidden sm:inline">태그 관리</span>
          </button>
          <button
            type="button"
            @click="store.closeTagPicker()"
            class="p-1.5 rounded-lg hover:bg-neutral-800 text-neutral-400 hover:text-white transition cursor-pointer"
          >
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Body -->
      <div class="p-5 space-y-4 overflow-y-auto flex-1 text-xs">
        <!-- Add New Tag Input -->
        <div class="space-y-1.5">
          <label class="text-[11px] font-semibold text-neutral-300 block">
            새 태그 추가 (기존 태그와 이름이 같으면 중복 추가되지 않고 자동 선택됩니다)
          </label>
          <div class="flex items-center gap-2">
            <input
              v-model="newTagInput"
              @keydown.enter.prevent="handleAddNewTag"
              type="text"
              placeholder="새 태그 이름 입력... (예: 일상물, 완결)"
              class="flex-1 bg-neutral-950 border border-neutral-800 focus:border-indigo-500 rounded-lg px-3 py-2 text-xs text-white placeholder-neutral-500 focus:outline-none"
            />
            <button
              type="button"
              @click="handleAddNewTag"
              :disabled="!cleanTagDisplay(newTagInput)"
              class="px-3.5 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white font-medium text-xs flex items-center gap-1.5 transition cursor-pointer shrink-0"
            >
              <Plus class="w-3.5 h-3.5" />
              추가
            </button>
          </div>

          <div
            v-if="feedback"
            class="flex items-center gap-1.5 px-3 py-2 rounded-lg text-[11px] border"
            :class="
              feedback.type === 'success'
                ? 'bg-emerald-950/50 border-emerald-800/70 text-emerald-300'
                : 'bg-amber-950/50 border-amber-800/70 text-amber-300'
            "
          >
            <CheckCircle2 v-if="feedback.type === 'success'" class="w-3.5 h-3.5 shrink-0" />
            <AlertCircle v-else class="w-3.5 h-3.5 shrink-0" />
            <span>{{ feedback.message }}</span>
          </div>
        </div>

        <!-- Selected Tags Summary -->
        <div class="p-3 rounded-xl bg-neutral-950 border border-neutral-800 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-[11px] font-semibold text-neutral-300">
              선택된 태그 ({{ selectedList.length }}개)
            </span>
            <button
              v-if="selectedList.length > 0"
              type="button"
              @click="clearSelected"
              class="text-[11px] text-neutral-400 hover:text-red-400 transition cursor-pointer"
            >
              선택 모두 해제
            </button>
          </div>

          <div v-if="selectedList.length === 0" class="text-[11px] text-neutral-500 italic">
            아래 목록에서 태그를 클릭하여 다중 선택할 수 있습니다.
          </div>

          <div v-else class="flex flex-wrap gap-1.5 max-h-24 overflow-y-auto">
            <button
              v-for="[key, display] in selectedList"
              :key="key"
              type="button"
              @click="toggleTag(key, display)"
              class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-indigo-600/30 border border-indigo-500/70 text-indigo-200 text-xs hover:bg-red-950/60 hover:border-red-700 hover:text-red-200 transition cursor-pointer"
              title="클릭하여 선택 해제"
            >
              <span>#{{ display }}</span>
              <X class="w-3 h-3" />
            </button>
          </div>
        </div>

        <!-- Existing Tags Filter + Multi-select List -->
        <div class="space-y-2">
          <div class="flex items-center justify-between gap-2">
            <span class="text-[11px] font-semibold text-neutral-300">
              전체 태그 목록 ({{ store.availableTags.length }})
            </span>
            <div class="relative w-56">
              <Search class="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
              <input
                v-model="filterQuery"
                type="text"
                placeholder="태그 목록 내 검색..."
                class="w-full bg-neutral-950 border border-neutral-800 focus:border-indigo-500 rounded-lg pl-8 pr-7 py-1.5 text-[11px] text-neutral-100 placeholder-neutral-500 focus:outline-none"
              />
              <button
                v-if="filterQuery"
                type="button"
                @click="filterQuery = ''"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-500 hover:text-neutral-200 cursor-pointer"
              >
                <X class="w-3 h-3" />
              </button>
            </div>
          </div>

          <div
            v-if="filteredOptions.length === 0"
            class="p-8 rounded-xl border border-neutral-800 bg-neutral-950 text-center text-neutral-500 text-xs"
          >
            {{
              store.availableTags.length === 0
                ? '아직 등록된 태그가 없습니다. 위 입력칸에서 새 태그를 추가해 보세요.'
                : '검색어와 일치하는 태그가 없습니다.'
            }}
          </div>

          <div
            v-else
            class="grid grid-cols-2 sm:grid-cols-3 gap-1.5 max-h-60 overflow-y-auto p-2 rounded-xl border border-neutral-800 bg-neutral-950"
          >
            <button
              v-for="tag in filteredOptions"
              :key="tag.key"
              type="button"
              @click="toggleTag(tag.key, tag.display_name)"
              class="flex items-center justify-between gap-1.5 px-2.5 py-2 rounded-lg border text-left transition cursor-pointer"
              :class="
                selectedMap.has(tag.key)
                  ? 'bg-indigo-600/25 border-indigo-500 text-white font-semibold'
                  : 'bg-neutral-900/70 border-neutral-800 text-neutral-300 hover:border-neutral-700 hover:bg-neutral-900'
              "
            >
              <div class="flex items-center gap-1.5 min-w-0">
                <span
                  class="w-4 h-4 rounded flex items-center justify-center border shrink-0 transition"
                  :class="
                    selectedMap.has(tag.key)
                      ? 'bg-indigo-600 border-indigo-500 text-white'
                      : 'border-neutral-700 bg-neutral-950 text-transparent'
                  "
                >
                  <Check class="w-3 h-3" />
                </span>
                <span class="truncate text-xs">#{{ tag.display_name }}</span>
              </div>
            </button>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-5 py-3.5 border-t border-neutral-800 bg-neutral-900/60 flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div v-if="ctx.mode === 'search'" class="flex items-center gap-2 text-xs">
          <span class="text-neutral-400 text-[11px]">태그 결합:</span>
          <div class="inline-flex rounded-md bg-neutral-950 p-0.5 border border-neutral-800">
            <button
              type="button"
              @click="store.searchQuery.tag_operator = 'AND'"
              :class="store.searchQuery.tag_operator === 'AND' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
              class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
            >
              AND
            </button>
            <button
              type="button"
              @click="store.searchQuery.tag_operator = 'OR'"
              :class="store.searchQuery.tag_operator === 'OR' ? 'bg-indigo-600 text-white' : 'text-neutral-400 hover:text-neutral-200'"
              class="px-2 py-0.5 rounded text-[11px] font-semibold cursor-pointer"
            >
              OR
            </button>
          </div>
        </div>
        <div v-else class="text-[11px] text-neutral-500">
          다중 선택 후 저장 버튼을 누르면 반영됩니다.
        </div>

        <div class="flex items-center gap-2 ml-auto">
          <button
            type="button"
            @click="store.closeTagPicker()"
            class="px-3.5 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs transition cursor-pointer"
          >
            취소
          </button>
          <button
            type="button"
            @click="handleConfirm"
            :disabled="isSubmitting"
            class="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white font-medium text-xs transition shadow-lg shadow-indigo-600/30 cursor-pointer"
          >
            {{
              ctx.mode === 'search'
                ? `선택 태그로 검색 (${selectedList.length})`
                : `태그 저장 (${selectedList.length})`
            }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
