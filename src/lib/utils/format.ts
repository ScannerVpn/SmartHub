// ---------------------------------------------------------------------------
// Formatting helpers shared by panels.
// ---------------------------------------------------------------------------

const units = ['B', 'KB', 'MB', 'GB', 'TB'];

export function fmtBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  let b = bytes;
  let u = 0;
  while (b >= 1024 && u < units.length - 1) {
    b /= 1024;
    u += 1;
  }
  return `${b >= 10 || u === 0 ? Math.round(b) : b.toFixed(1)} ${units[u]}`;
}

/** "just now", "5m ago", "2h ago", "Mon 14:02" relative timestamps. */
export function fmtRelative(epochSeconds: number): string {
  const diff = Math.max(0, Date.now() / 1000 - epochSeconds);
  if (diff < 45) return 'just now';
  if (diff < 3600) return `${Math.round(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.round(diff / 3600)}h ago`;
  const d = new Date(epochSeconds * 1000);
  return d.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' });
}

export function truncate(s: string, n: number): string {
  const oneLine = s.replace(/\s+/g, ' ').trim();
  return oneLine.length <= n ? oneLine : oneLine.slice(0, n - 1) + '…';
}
