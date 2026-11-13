import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * Converts a backend cover reference (`ymppcover://file-123.webp`,
 * `ymppcover://folder-45.webp?v=123`, or an existing `data:` / `http:` URL)
 * into a WebView-loadable image URL.
 */
export function resolveCoverUrl(raw: string | null | undefined): string | null {
  if (!raw) return null;
  if (raw.startsWith('ymppcover://')) {
    const rest = raw.slice('ymppcover://'.length);
    const qIdx = rest.indexOf('?');
    if (qIdx >= 0) {
      const file = rest.slice(0, qIdx);
      const query = rest.slice(qIdx);
      return convertFileSrc(file, 'ymppcover') + query;
    }
    return convertFileSrc(rest, 'ymppcover');
  }
  return raw;
}
