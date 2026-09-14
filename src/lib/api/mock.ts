// ---------------------------------------------------------------------------
// Browser mock for the live preview / `npm run dev` outside of Tauri.
// Mirrors the Rust command surface with believable latency and sample data,
// so the full UI can be exercised without the native backend.
// ---------------------------------------------------------------------------
import type {
  AppSettings,
  ClipboardItem,
  LicenseState,
  RecoveryEntry,
  ScanResult,
  WorkspaceProfile
} from './types';

const now = Math.floor(Date.now() / 1000);
const m = (mins: number) => now - mins * 60;

let clipSeq = 0;
const clip = (i: Partial<ClipboardItem>): ClipboardItem => ({
  id: `clip-${++clipSeq}`,
  kind: 'text',
  content: '',
  preview: '',
  linkTitle: null,
  pinned: false,
  createdAt: m(5),
  useCount: 1,
  ...i
});

const clipboardSeed: ClipboardItem[] = [
  clip({
    kind: 'link',
    content: 'https://github.com/tauri-apps/tauri',
    preview: 'github.com/tauri-apps/tauri',
    linkTitle: 'Build smaller, faster, and more secure desktop apps | Tauri',
    pinned: true,
    createdAt: m(2),
    useCount: 9
  }),
  clip({
    kind: 'code',
    content: `let win = app.get_webview_window("main").unwrap();\nwin.show()?;\nwin.set_focus()?;`,
    preview: 'win.show()?; win.set_focus()?;',
    pinned: true,
    createdAt: m(14),
    useCount: 5
  }),
  clip({
    kind: 'text',
    content: 'Your data never leaves your device. Zero telemetry. 100% offline.',
    preview: 'Your data never leaves your device. Zero telemetry…',
    createdAt: m(26),
    useCount: 3
  }),
  clip({
    kind: 'color',
    content: '#6366F1',
    preview: '#6366F1',
    createdAt: m(41),
    useCount: 2
  }),
  clip({
    kind: 'image',
    content:
      'data:image/svg+xml;utf8,' +
      encodeURIComponent(
        `<svg xmlns="http://www.w3.org/2000/svg" width="240" height="120"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#6366F1"/><stop offset="1" stop-color="#8B5CF6"/></linearGradient></defs><rect width="240" height="120" rx="12" fill="url(#g)"/><text x="120" y="66" font-family="system-ui" font-size="14" fill="white" text-anchor="middle">Screenshot_2026-09-14.png</text></svg>`
      ),
    preview: 'Screenshot_2026-09-14.png (240×120)',
    createdAt: m(55)
  }),
  clip({
    kind: 'text',
    content: 'npm run tauri build',
    preview: 'npm run tauri build',
    createdAt: m(72),
    useCount: 4
  }),
  clip({
    kind: 'text',
    content: 'دیتای کاربر فقط روی دستگاه خودش می‌مونه — هیچ تله‌متری‌ای وجود نداره.',
    preview: 'دیتای کاربر فقط روی دستگاه خودش می‌مونه…',
    createdAt: m(95)
  }),
  clip({
    kind: 'link',
    content: 'https://lemonsqueezy.com/features/merchant-of-record',
    preview: 'lemonsqueezy.com/features/merchant-of-record',
    linkTitle: 'Merchant of Record — Lemon Squeezy',
    createdAt: m(130)
  })
];

const recoverySeed: RecoveryEntry[] = [
  {
    id: 'rec-1',
    app: 'msedge.exe',
    title: 'Gmail — New Message',
    content:
      'Hi Sara, thanks for the review! I pushed the fix for the hotkey conflict and the clipboard watcher is now fully async. Can you check the 0.1.0 build on your Windows machine?',
    preview: 'Hi Sara, thanks for the review! I pushed the fix for the hotkey…',
    createdAt: m(9)
  },
  {
    id: 'rec-2',
    app: 'Code.exe',
    title: 'lib.rs — SmartHub',
    content:
      'AppState is shared across all commands via Tauri managed state. The clipboard watcher owns its own connection, settings come from sqlite.',
    preview: 'AppState is shared across all commands via Tauri managed state…',
    createdAt: m(18)
  },
  {
    id: 'rec-3',
    app: 'Notion.exe',
    title: 'Weekly plan — Notion',
    content:
      'Mon: ship MVP build. Tue: record the 30s demo. Wed: draft Product Hunt copy. Thu: reach out to 10 dev-tool YouTubers.',
    preview: 'Mon: ship MVP build. Tue: record the 30s demo…',
    createdAt: m(33)
  },
  {
    id: 'rec-4',
    app: 'Telegram.exe',
    title: 'Ali (product chat)',
    content:
      'الان دقیقاً همون چیزی که لازم داشتم بود — نجات متن‌های تایپ‌شده واقعاً کار منو می‌گیره. کی نسخه پرو میدی بیرون؟',
    preview: 'الان دقیقاً همون چیزی که لازم داشتم بود — نجات متن‌های…',
    createdAt: m(47)
  }
];

const profilesSeed: WorkspaceProfile[] = [
  {
    id: 'p-focus',
    name: 'Focus',
    icon: 'moon',
    builtin: true,
    actions: [
      { kind: 'kill_app', value: 'discord.exe' },
      { kind: 'kill_app', value: 'Telegram.exe' },
      { kind: 'do_not_disturb', value: 'true' },
      { kind: 'set_volume', value: '0.2' },
      { kind: 'launch_app', value: 'Code.exe' }
    ]
  },
  {
    id: 'p-stream',
    name: 'Streaming',
    icon: 'radio',
    builtin: true,
    actions: [
      { kind: 'launch_app', value: 'obs64.exe' },
      { kind: 'launch_app', value: 'Discord.exe' },
      { kind: 'set_volume', value: '0.6' },
      { kind: 'do_not_disturb', value: 'true' }
    ]
  },
  {
    id: 'p-code',
    name: 'Coding',
    icon: 'code',
    builtin: true,
    actions: [
      { kind: 'launch_app', value: 'Code.exe' },
      { kind: 'launch_app', value: 'WindowsTerminal.exe' },
      { kind: 'do_not_disturb', value: 'false' },
      { kind: 'set_volume', value: '0.35' }
    ]
  }
];

const scanSeed: ScanResult = {
  folder: 'C:\\Users\\you\\Downloads',
  fileCount: 412,
  totalBytes: 8_372_000_000,
  categories: { documents: 96, images: 150, videos: 8, archives: 34, installers: 22, code: 102 },
  suggestions: [
    {
      id: 'sug-1',
      path: 'Downloads\\SmartHub-Setup (2).msi',
      reason: 'duplicate',
      detail: 'کپی دقیق SmartHub-Setup.msi — یک نسخه کافی است',
      sizeBytes: 14_155_776,
      category: 'installers'
    },
    {
      id: 'sug-2',
      path: 'Downloads\\node_setup_x64.exe',
      reason: 'stale',
      detail: '۱۲۴ روز بدون استفاده — نصب‌کننده معمولاً یک‌بار لازم می‌شود',
      sizeBytes: 31_654_912,
      category: 'installers'
    },
    {
      id: 'sug-3',
      path: 'Downloads\\photos_backup_final_FINAL.zip',
      reason: 'large_archive',
      detail: '۱/۸ گیگ — آرشیو قدیمی، کاندید انتقال به فضای ابری یا حذف',
      sizeBytes: 1_935_000_000,
      category: 'archives'
    },
    {
      id: 'sug-4',
      path: 'Downloads\\report_draft_copy.docx',
      reason: 'duplicate',
      detail: '۹۹٪ شباهت محتوایی با report_draft.docx',
      sizeBytes: 84_123,
      category: 'documents'
    },
    {
      id: 'sug-5',
      path: 'Desktop\\tmp_scan_old.zip',
      reason: 'stale',
      detail: '۳۱۰ روز بدون باز شدن',
      sizeBytes: 402_101_248,
      category: 'archives'
    }
  ]
};

// Mock state (mutated by commands to feel alive)
const state = {
  settings: {
    hotkey: 'Alt+Space',
    launchAtLogin: true,
    maxClipboardItems: 20,
    clipboardMonitoring: true,
    textCaptureEnabled: true
  } as AppSettings,
  license: {
    tier: 'free',
    keyMasked: null,
    graceValid: false,
    graceDaysLeft: null
  } as LicenseState,
  profiles: structuredClone(profilesSeed),
  activeProfile: null as string | null
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function matches(query: string | undefined, item: ClipboardItem) {
  if (!query) return true;
  const q = query.toLowerCase();
  return (
    item.content.toLowerCase().includes(q) ||
    item.preview.toLowerCase().includes(q) ||
    (item.linkTitle ?? '').toLowerCase().includes(q)
  );
}

/** Mock implementation of every Rust command the UI can invoke. */
export const mockCommands: Record<string, (args?: Record<string, unknown>) => Promise<unknown>> = {
  // ---- Smart Clipboard -------------------------------------------------
  async get_clipboard_history(a) {
    await sleep(120);
    const q = a?.query as string | undefined;
    return clipboardSeed
      .filter((c) => matches(q, c))
      .sort((x, y) => Number(y.pinned) - Number(x.pinned) || y.createdAt - x.createdAt)
      .slice(0, state.settings.maxClipboardItems);
  },

  async copy_to_clipboard(a) {
    await sleep(40);
    const item = clipboardSeed.find((c) => c.id === a?.id);
    if (item && typeof navigator !== 'undefined' && navigator.clipboard && item.kind !== 'image') {
      navigator.clipboard.writeText(item.content).catch(() => {});
    }
    if (item) item.useCount += 1;
    return null;
  },

  async toggle_pin_clipboard_item(a) {
    await sleep(30);
    const item = clipboardSeed.find((c) => c.id === a?.id);
    if (item) item.pinned = !!a?.pinned;
    return item;
  },

  async delete_clipboard_item(a) {
    await sleep(30);
    const i = clipboardSeed.findIndex((c) => c.id === a?.id);
    if (i >= 0) clipboardSeed.splice(i, 1);
    return null;
  },

  // ---- Text Time Machine ----------------------------------------------
  async get_recoverable_texts(a) {
    await sleep(120);
    const q = ((a?.query as string | undefined) ?? '').toLowerCase();
    return recoverySeed
      .filter(
        (r) =>
          !q ||
          r.content.toLowerCase().includes(q) ||
          r.app.toLowerCase().includes(q) ||
          r.title.toLowerCase().includes(q)
      )
      .sort((x, y) => y.createdAt - x.createdAt);
  },

  async recover_text(a) {
    await sleep(40);
    const entry = recoverySeed.find((r) => r.id === a?.id);
    if (entry && navigator.clipboard) {
      navigator.clipboard.writeText(entry.content).catch(() => {});
    }
    return entry?.content ?? '';
  },

  async delete_recovery_entry(a) {
    await sleep(30);
    const i = recoverySeed.findIndex((r) => r.id === a?.id);
    if (i >= 0) recoverySeed.splice(i, 1);
    return null;
  },

  // ---- Licensing --------------------------------------------------------
  async get_license_state() {
    await sleep(60);
    return { ...state.license };
  },

  async activate_license(a) {
    await sleep(700);
    const key = String(a?.key ?? '').trim();
    if (key.length < 12) throw new Error('Invalid license key format');
    state.license = {
      tier: 'pro',
      keyMasked: `****-****-****-${key.slice(-4).toUpperCase()}`,
      graceValid: true,
      graceDaysLeft: 7
    };
    return { ...state.license };
  },

  async deactivate_license() {
    await sleep(150);
    state.license = { tier: 'free', keyMasked: null, graceValid: false, graceDaysLeft: null };
    return { ...state.license };
  },

  // ---- Workspace Modes --------------------------------------------------
  async get_workspace_profiles() {
    await sleep(80);
    return { profiles: state.profiles, activeId: state.activeProfile };
  },

  async apply_workspace_profile(a) {
    if (state.license.tier !== 'pro') throw new Error('pro_required');
    await sleep(500);
    state.activeProfile = String(a?.id);
    return { ok: true };
  },

  async save_workspace_profile(a) {
    if (state.license.tier !== 'pro') throw new Error('pro_required');
    await sleep(120);
    return a?.profile;
  },

  async delete_workspace_profile(a) {
    if (state.license.tier !== 'pro') throw new Error('pro_required');
    await sleep(60);
    state.profiles = state.profiles.filter((p) => p.id !== a?.id);
    return null;
  },

  // ---- AI File Declutterer ---------------------------------------------
  async get_ai_status() {
    await sleep(60);
    // Browser preview: model not downloaded, heuristics still available
    return { state: 'not_downloaded' };
  },

  async scan_folder(a) {
    await sleep(900);
    return { ...scanSeed, folder: (a?.path as string) || scanSeed.folder };
  },

  // ---- Settings ---------------------------------------------------------
  async get_settings() {
    await sleep(40);
    return { ...state.settings };
  },

  async save_settings(a) {
    await sleep(60);
    Object.assign(state.settings, a?.settings ?? {});
    return { ...state.settings };
  },

  // ---- Window ------------------------------------------------------------
  async hide_window() {
    // In the browser preview there is no window to hide — no-op.
    return null;
  }
};
