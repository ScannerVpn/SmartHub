//! Global hotkey — Alt+Space by default, user-configurable.
//!
//! Built on `tauri-plugin-global-shortcut` (itself on the `global-hotkey`
//! crate, per the PRD). The hotkey toggles the palette: hidden → show,
//! center, focus, and tell the UI to reset to the home panel via the
//! `palette://opened` event.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub const DEFAULT_HOTKEY: &str = "Alt+Space";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyConfig {
    pub shortcut: String,
}

/// Called from the global shortcut handler (and exposed for tests).
pub fn toggle_palette(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => {
                let _ = win.hide();
            }
            _ => {
                let _ = win.center();
                let _ = win.show();
                let _ = win.set_focus();
                // The UI listens for this to reset navigation + focus search.
                let _ = app.emit("palette://opened", ());
            }
        }
    }
}

/// (Re)register a single shortcut string like "Alt+Space".
pub fn register(app: &AppHandle, shortcut: &str) -> Result<()> {
    let parsed =
        Shortcut::from_str(shortcut).with_context(|| format!("invalid shortcut: {shortcut}"))?;

    let manager = app.global_shortcut();
    // Swap atomically-ish: unregister everything we own, then register.
    manager.unregister_all().ok();
    manager
        .register(parsed)
        .with_context(|| format!("shortcut {shortcut} is already taken by another app"))?;
    Ok(())
}

/// Wire the handler; called once during app setup. The plugin dispatches on
/// the accelerator event and we toggle the palette window on key-down only.
pub fn plugin_handler(
    app: &AppHandle,
    _shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    use tauri_plugin_global_shortcut::ShortcutState;
    if event.state() == ShortcutState::Pressed {
        toggle_palette(app);
    }
}
