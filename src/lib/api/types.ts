// ---------------------------------------------------------------------------
// Shared types between the Rust core (typed as JSON-serializable structs) and
// the Svelte UI. Keep these in sync with `src-tauri/src`.
// ---------------------------------------------------------------------------

export type ClipboardKind = 'text' | 'link' | 'code' | 'image' | 'color';

export interface ClipboardItem {
  id: string;
  kind: ClipboardKind;
  /** Full content. Images carry a data-URL thumbnail instead. */
  content: string;
  /** Short single-line preview shown in the list. */
  preview: string;
  /** For links: the resolved page title, if fetch succeeded. */
  linkTitle: string | null;
  pinned: boolean;
  /** Unix epoch seconds. */
  createdAt: number;
  useCount: number;
}

export interface RecoveryEntry {
  id: string;
  /** Process/app the text was typed in, e.g. "Code.exe". */
  app: string;
  /** Window title at capture time. */
  title: string;
  content: string;
  preview: string;
  createdAt: number;
}

export type LicenseTier = 'free' | 'pro';

export interface LicenseState {
  tier: LicenseTier;
  /** Masked license key, e.g. "SMAR-****-****-9F3A" — never the full key. */
  keyMasked: string | null;
  /** True while the offline grace window is still valid. */
  graceValid: boolean;
  /** Days remaining in the 7-day offline grace window (when offline). */
  graceDaysLeft: number | null;
}

export interface HotkeyConfig {
  /** Tauri shortcut syntax, e.g. "Alt+Space". */
  shortcut: string;
}

export type ProfileActionKind =
  | 'launch_app'
  | 'kill_app'
  | 'set_wallpaper'
  | 'set_volume'
  | 'mute'
  | 'do_not_disturb'
  | 'run_command';

export interface ProfileAction {
  kind: ProfileActionKind;
  /** Path/command/name/level depending on `kind`. */
  value: string;
}

export interface WorkspaceProfile {
  id: string;
  name: string;
  icon: string;
  actions: ProfileAction[];
  builtin: boolean;
}

export type AiStatus =
  | { state: 'not_downloaded' }
  | { state: 'downloading'; progress: number }
  | { state: 'ready'; model: string }
  | { state: 'error'; message: string };

export interface DeclutterSuggestion {
  id: string;
  path: string;
  reason: 'duplicate' | 'stale' | 'installer' | 'large_archive';
  detail: string;
  sizeBytes: number;
  category: string;
}

export interface ScanResult {
  folder: string;
  fileCount: number;
  totalBytes: number;
  suggestions: DeclutterSuggestion[];
  categories: Record<string, number>;
}

export interface AppSettings {
  hotkey: string;
  launchAtLogin: boolean;
  maxClipboardItems: number;
  clipboardMonitoring: boolean;
  textCaptureEnabled: boolean;
}
