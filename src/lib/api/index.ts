// ---------------------------------------------------------------------------
// Unified command layer. Inside Tauri, calls go to the Rust core via `invoke`.
// In a plain browser (dev server live preview), calls go to a rich mock so the
// whole UI stays usable. Every component imports from here — never from
// '@tauri-apps/api' directly — which keeps the boundary explicit.
// ---------------------------------------------------------------------------
import { isTauri } from '@tauri-apps/api/core';
import { mockCommands } from './mock';
import type {
  AiStatus,
  AppSettings,
  ClipboardItem,
  LicenseState,
  RecoveryEntry,
  ScanResult,
  WorkspaceProfile
} from './types';

export const inTauri = isTauri();

type Args = Record<string, unknown> | undefined;

async function call<T>(cmd: string, args?: Args): Promise<T> {
  if (inTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  const mock = mockCommands[cmd];
  if (!mock) throw new Error(`No mock for command "${cmd}"`);
  return mock(args) as Promise<T>;
}

// ----- window -------------------------------------------------------------

export async function hideWindow(): Promise<void> {
  if (inTauri) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().hide();
  } else {
    await mockCommands.hide_window();
  }
}

export async function startDragging(): Promise<void> {
  if (inTauri) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().startDragging();
  }
}

// ----- Smart Clipboard (Free) ----------------------------------------------

export const getClipboardHistory = (query?: string) =>
  call<ClipboardItem[]>('get_clipboard_history', { query });

export const copyToClipboard = (id: string) => call<void>('copy_to_clipboard', { id });

export const togglePin = (id: string, pinned: boolean) =>
  call<ClipboardItem | null>('toggle_pin_clipboard_item', { id, pinned });

export const deleteClipboardItem = (id: string) => call<void>('delete_clipboard_item', { id });

// ----- Text Time Machine (Free) --------------------------------------------

export const getRecoverableTexts = (query?: string) =>
  call<RecoveryEntry[]>('get_recoverable_texts', { query });

export const recoverText = (id: string) => call<string>('recover_text', { id });

export const deleteRecoveryEntry = (id: string) => call<void>('delete_recovery_entry', { id });

// ----- Licensing ------------------------------------------------------------

export const getLicenseState = () => call<LicenseState>('get_license_state');

export const activateLicense = (key: string) =>
  call<LicenseState>('activate_license', { key });

export const deactivateLicense = () => call<LicenseState>('deactivate_license');

// ----- Workspace Modes (Pro) ------------------------------------------------

export const getWorkspaceProfiles = () =>
  call<{ profiles: WorkspaceProfile[]; activeId: string | null }>('get_workspace_profiles');

export const applyWorkspaceProfile = (id: string) => call<{ ok: boolean }>('apply_workspace_profile', { id });

// ----- AI Declutterer (Pro) -------------------------------------------------

export const getAiStatus = () => call<AiStatus>('get_ai_status');

export const scanFolder = (path?: string) => call<ScanResult>('scan_folder', { path });

// ----- Settings --------------------------------------------------------------

export const getSettings = () => call<AppSettings>('get_settings');

export const saveSettings = (settings: AppSettings) =>
  call<AppSettings>('save_settings', { settings });

export * from './types';
