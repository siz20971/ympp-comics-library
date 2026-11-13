<script setup lang="ts">
import { computed } from 'vue';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  Sparkles,
  Download,
  Clock,
  BellOff,
  X,
  ExternalLink,
  AlertTriangle,
  ArrowRight,
  Loader2,
  Package,
} from 'lucide-vue-next';
import { useLibraryStore } from '../stores/libraryStore';

const store = useLibraryStore();

const result = computed(() => store.updateResult);
const isBusy = computed(() => store.isDownloadingUpdate);

const progressPercent = computed(() => {
  const p = store.updateProgress;
  if (!p) return 0;
  if (p.stage === 'verifying' || p.stage === 'replacing' || p.stage === 'restarting') {
    return 100;
  }
  if (typeof p.percent === 'number' && p.percent > 0) {
    return Math.min(100, Math.round(p.percent));
  }
  if (!p.total || p.total <= 0) return 0;
  return Math.min(100, Math.round((p.downloaded / p.total) * 100));
});

const stageLabel = computed(() => {
  const p = store.updateProgress;
  if (!p) return '';
  switch (p.stage) {
    case 'downloading':
      return `업데이트 다운로드 중... (${formatBytes(p.downloaded)}${p.total ? ' / ' + formatBytes(p.total) : ''})`;
    case 'verifying':
      return '실행 파일 무결성 검증 중...';
    case 'replacing':
      return '프로그램 교체 중...';
    case 'restarting':
      return '새 버전으로 재시작하는 중...';
    default:
      return '처리 중...';
  }
});

function formatBytes(bytes: number | null | undefined): string {
  if (!bytes || bytes <= 0) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  return `${mb.toFixed(2)} MB`;
}

function formatDate(iso: string): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString('ko-KR', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  });
}

type ParsedLine =
  | { kind: 'h1' | 'h2' | 'h3'; text: string }
  | { kind: 'bullet'; text: string; indent: number }
  | { kind: 'hr' }
  | { kind: 'blank' }
  | { kind: 'p'; text: string };

function parseReleaseLines(body: string): ParsedLine[] {
  const rawLines = (body || '').replace(/\r\n/g, '\n').split('\n');
  const parsed: ParsedLine[] = [];

  for (const raw of rawLines) {
    const trimmed = raw.trim();
    if (!trimmed) {
      parsed.push({ kind: 'blank' });
      continue;
    }
    if (/^[-*_]{3,}$/.test(trimmed)) {
      parsed.push({ kind: 'hr' });
      continue;
    }
    if (trimmed.startsWith('### ')) {
      parsed.push({ kind: 'h3', text: trimmed.slice(4).trim() });
      continue;
    }
    if (trimmed.startsWith('## ')) {
      parsed.push({ kind: 'h2', text: trimmed.slice(3).trim() });
      continue;
    }
    if (trimmed.startsWith('# ')) {
      parsed.push({ kind: 'h1', text: trimmed.slice(2).trim() });
      continue;
    }
    const bulletMatch = raw.match(/^(\s*)[-*+]\s+(.*)$/);
    if (bulletMatch) {
      const indent = Math.min(2, Math.floor(bulletMatch[1].length / 2));
      parsed.push({ kind: 'bullet', text: bulletMatch[2].trim(), indent });
      continue;
    }
    parsed.push({ kind: 'p', text: trimmed });
  }

  return parsed;
}

function stripInlineMarkdown(text: string): string {
  return text.replace(/\*\*(.*?)\*\*/g, '$1').replace(/`([^`]+)`/g, '$1');
}

async function openExternalRelease(url?: string | null) {
  if (!url) return;
  try {
    await openUrl(url);
  } catch (err) {
    console.error('Failed to open release URL:', err);
  }
}

async function handleIgnoreForever() {
  if (isBusy.value) return;
  await store.ignoreUpdateForever();
}

async function handleSnoozeWeek() {
  if (isBusy.value) return;
  await store.snoozeUpdateWeek();
}

async function handleStartUpdate() {
  if (isBusy.value) return;
  await store.downloadAndApplyUpdate();
}
</script>

<template>
  <div
    v-if="store.isUpdateModalOpen && result"
    class="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4"
  >
    <div
      class="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl overflow-hidden"
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-neutral-800 flex items-center justify-between bg-neutral-950/70">
        <div class="flex items-center gap-3">
          <div class="p-2 rounded-xl bg-indigo-500/15 border border-indigo-500/30 text-indigo-400">
            <Sparkles class="w-5 h-5" />
          </div>
          <div>
            <h2 class="text-sm font-bold text-neutral-100">새 버전 업데이트 안내</h2>
            <div class="flex items-center gap-2 mt-0.5 text-xs">
              <span class="font-mono text-neutral-400">{{ result.current_version }}</span>
              <ArrowRight class="w-3.5 h-3.5 text-neutral-500" />
              <span class="font-mono font-semibold text-emerald-400 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/30">
                {{ result.latest_version }}
              </span>
              <span v-if="result.asset_size" class="text-[11px] text-neutral-500 flex items-center gap-1 ml-1">
                <Package class="w-3 h-3" />
                {{ formatBytes(result.asset_size) }}
              </span>
            </div>
          </div>
        </div>

        <button
          @click="store.closeUpdateModal()"
          :disabled="isBusy"
          class="p-1.5 rounded-lg text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 transition cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          title="닫기 (이번 실행만 닫기)"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Body: Full Release Notes -->
      <div class="p-6 overflow-y-auto flex-1 space-y-4 text-xs text-neutral-300">
        <div
          v-if="result.is_dev_build"
          class="p-3 rounded-xl bg-amber-950/40 border border-amber-800/60 text-amber-200 flex items-start gap-2.5"
        >
          <AlertTriangle class="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
          <div class="space-y-1">
            <p class="font-semibold">개발(cargo dev / target) 환경에서 실행 중입니다.</p>
            <p class="text-[11px] text-amber-300/90 leading-relaxed">
              개발 바이너리 보호를 위해 자동 자가 교체가 비활성화되어 있습니다. GitHub 릴리즈 페이지에서 확인하거나 릴리즈 exe로 실행해주세요.
            </p>
          </div>
        </div>

        <div
          v-else-if="!result.download_url"
          class="p-3 rounded-xl bg-amber-950/40 border border-amber-800/60 text-amber-200 flex items-start gap-2.5"
        >
          <AlertTriangle class="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
          <div class="space-y-1">
            <p class="font-semibold">이 릴리즈에 YMPPComicsLibrary.exe 첨부 파일이 아직 없습니다.</p>
            <p class="text-[11px] text-amber-300/90">
              GitHub 릴리즈 페이지에서 직접 다운로드할 수 있습니다.
            </p>
          </div>
        </div>

        <!-- Aggregated Release Notes -->
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <h3 class="text-[11px] font-semibold text-neutral-400 uppercase tracking-wider">
              변경 사항 전체 보기 ({{ result.releases.length }}개 릴리즈)
            </h3>
            <button
              v-if="result.html_url"
              type="button"
              @click="openExternalRelease(result.html_url)"
              class="text-[11px] text-indigo-400 hover:text-indigo-300 flex items-center gap-1 cursor-pointer"
            >
              GitHub에서 보기
              <ExternalLink class="w-3 h-3" />
            </button>
          </div>

          <div
            v-for="(rel, idx) in result.releases"
            :key="rel.version"
            class="p-4 rounded-xl bg-neutral-950/80 border border-neutral-800 space-y-2.5"
          >
            <div class="flex items-center justify-between gap-2 pb-2 border-b border-neutral-800/80">
              <div class="flex items-center gap-2 min-w-0">
                <span
                  class="px-2 py-0.5 rounded font-mono text-[11px] font-semibold"
                  :class="
                    idx === 0
                      ? 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30'
                      : 'bg-neutral-800 text-neutral-300 border border-neutral-700'
                  "
                >
                  {{ rel.version }}
                </span>
                <span class="font-semibold text-neutral-100 truncate">
                  {{ rel.title || rel.version }}
                </span>
              </div>
              <div class="flex items-center gap-2 shrink-0">
                <span v-if="rel.published_at" class="text-[11px] text-neutral-500">
                  {{ formatDate(rel.published_at) }}
                </span>
                <button
                  v-if="rel.html_url"
                  type="button"
                  @click="openExternalRelease(rel.html_url)"
                  class="text-neutral-500 hover:text-indigo-400 cursor-pointer"
                  title="릴리즈 노트 웹에서 열기"
                >
                  <ExternalLink class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            <div class="space-y-1 text-[12px] leading-relaxed text-neutral-300">
              <template v-if="rel.body && rel.body.trim()">
                <template v-for="(line, lIdx) in parseReleaseLines(rel.body)" :key="lIdx">
                  <div v-if="line.kind === 'blank'" class="h-1.5" />
                  <hr v-else-if="line.kind === 'hr'" class="border-neutral-800 my-2" />
                  <h4 v-else-if="line.kind === 'h1'" class="text-sm font-bold text-neutral-100 pt-1">
                    {{ stripInlineMarkdown(line.text) }}
                  </h4>
                  <h5 v-else-if="line.kind === 'h2'" class="text-[13px] font-bold text-indigo-300 pt-1">
                    {{ stripInlineMarkdown(line.text) }}
                  </h5>
                  <h6 v-else-if="line.kind === 'h3'" class="text-xs font-semibold text-neutral-200 pt-0.5">
                    {{ stripInlineMarkdown(line.text) }}
                  </h6>
                  <div
                    v-else-if="line.kind === 'bullet'"
                    class="flex items-start gap-2"
                    :style="{ paddingLeft: `${line.indent * 14}px` }"
                  >
                    <span class="text-indigo-400 select-none mt-0.5">•</span>
                    <span class="flex-1 break-words">{{ stripInlineMarkdown(line.text) }}</span>
                  </div>
                  <p v-else class="break-words text-neutral-300">
                    {{ stripInlineMarkdown(line.text) }}
                  </p>
                </template>
              </template>
              <p v-else class="text-neutral-500 italic">
                등록된 릴리즈 변경 내역이 없습니다.
              </p>
            </div>
          </div>
        </div>

        <!-- Download Progress -->
        <div
          v-if="isBusy || store.updateProgress"
          class="p-3.5 rounded-xl bg-neutral-950 border border-indigo-500/30 space-y-2"
        >
          <div class="flex items-center justify-between text-[11px]">
            <span class="text-indigo-300 font-medium flex items-center gap-1.5">
              <Loader2 class="w-3.5 h-3.5 animate-spin" />
              {{ stageLabel }}
            </span>
            <span class="font-mono text-neutral-300">{{ progressPercent }}%</span>
          </div>
          <div class="w-full h-2 rounded-full bg-neutral-800 overflow-hidden">
            <div
              class="h-full bg-gradient-to-r from-indigo-500 to-emerald-400 transition-all duration-150"
              :style="{ width: `${progressPercent}%` }"
            />
          </div>
        </div>

        <!-- Error Message -->
        <div
          v-if="store.updateError"
          class="p-3.5 rounded-xl bg-red-950/40 border border-red-800/60 text-red-200 space-y-2"
        >
          <div class="flex items-start gap-2">
            <AlertTriangle class="w-4 h-4 text-red-400 shrink-0 mt-0.5" />
            <div class="space-y-1 flex-1 min-w-0">
              <p class="font-semibold text-red-300">업데이트 적용에 실패했습니다.</p>
              <p class="text-[11px] text-red-200/90 break-all">{{ store.updateError }}</p>
            </div>
          </div>
          <button
            v-if="result.html_url"
            type="button"
            @click="openExternalRelease(result.html_url)"
            class="px-3 py-1.5 rounded-lg bg-red-900/50 hover:bg-red-800/60 text-red-100 text-[11px] font-medium inline-flex items-center gap-1.5 cursor-pointer"
          >
            <ExternalLink class="w-3.5 h-3.5" />
            수동 다운로드를 위해 릴리즈 페이지 열기
          </button>
        </div>
      </div>

      <!-- Footer: 3 User Choices -->
      <div class="px-6 py-4 border-t border-neutral-800 bg-neutral-950/70 flex flex-wrap items-center justify-between gap-2">
        <button
          type="button"
          @click="handleIgnoreForever"
          :disabled="isBusy"
          class="px-3 py-2 rounded-lg text-xs text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/80 transition flex items-center gap-1.5 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          title="앞으로 실행 시 자동 업데이트 알림을 표시하지 않습니다 (설정에서 다시 확인할 수 있습니다)"
        >
          <BellOff class="w-3.5 h-3.5" />
          계속 무시하기
        </button>

        <div class="flex items-center gap-2 ml-auto">
          <button
            type="button"
            @click="handleSnoozeWeek"
            :disabled="isBusy"
            class="px-3.5 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium border border-neutral-700/70 transition flex items-center gap-1.5 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Clock class="w-3.5 h-3.5 text-neutral-400" />
            일주일 업데이트 보지 않기
          </button>

          <button
            type="button"
            @click="handleStartUpdate"
            :disabled="isBusy || !result.download_url || result.is_dev_build"
            class="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold transition flex items-center gap-1.5 shadow-sm shadow-indigo-600/30 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Loader2 v-if="isBusy" class="w-3.5 h-3.5 animate-spin" />
            <Download v-else class="w-3.5 h-3.5" />
            업데이트 하기
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
