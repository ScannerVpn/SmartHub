//! Text Time Machine (Free tier).
//!
//! Snapshots the text of the currently focused editable control every ~1.5 s
//! (when it changed and the field has settled), keeps snapshots for one hour,
//! and lets the user recover anything they lost to a crash, a stray Ctrl+W,
//! or a form that ate their message.
//!
//! Windows implementation uses UI Automation via `windows-rs`
//! (Win32_UI_Accessibility). Password fields (`IsPassword`) are never read.
//! Non-Windows targets compile a no-op stub so the rest of the app still
//! works — macOS support would use the AXUIElement API behind the same trait.

use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::time::{interval, Duration};
use uuid::Uuid;

use super::crypto::Crypto;
use super::db::{now, Db, TextRow};

/// PRD: keep rescued text only for the last hour.
pub const RETENTION_SECS: i64 = 60 * 60;
const POLL_INTERVAL: Duration = Duration::from_millis(1500);
const PREVIEW_LEN: usize = 140;
/// Ignore tiny transient states (e.g. a single partially-typed char in a
/// widget that churns); anything substantive is captured once it settles.
const MIN_CONTENT_LEN: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntryDto {
    pub id: String,
    pub app: String,
    pub title: String,
    pub content: String,
    pub preview: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct FocusedText {
    pub app: String,
    pub window_title: String,
    pub text: String,
    /// True when the focused control is a password field.
    pub is_password: bool,
}

pub struct TextSaver {
    db: Arc<Db>,
    crypto: Arc<Crypto>,
    /// Last snapshot signature: (app, title, content-hash-ish). Simple
    /// string comparison is fine — snapshots are already debounced.
    last_signature: Mutex<Option<String>>,
}

impl TextSaver {
    pub fn new(db: Arc<Db>, crypto: Arc<Crypto>) -> Self {
        Self {
            db,
            crypto,
            last_signature: Mutex::new(None),
        }
    }

    pub fn start(self: &Arc<Self>, enabled: Arc<parking_lot::RwLock<bool>>) {
        let svc = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let mut tick = interval(POLL_INTERVAL);
            loop {
                tick.tick().await;
                if !*enabled.read() {
                    continue;
                }
                if let Err(err) = svc.poll_once().await {
                    #[cfg(debug_assertions)]
                    eprintln!("[text saver] {err:#}");
                    let _ = err;
                }
            }
        });
    }

    async fn poll_once(&self) -> Result<()> {
        // UIA is COM and blocking — run it on the blocking thread pool;
        // `??` unwraps both the JoinError and the inner anyhow::Result.
        let captured = tokio::task::spawn_blocking(capture_focused_text).await??;

        // Purge hourly even when nothing changed, so old secrets don't linger.
        self.db.text_purge(RETENTION_SECS).ok();

        let Some(f) = captured else { return Ok(()) };
        if f.is_password || f.text.len() < MIN_CONTENT_LEN {
            return Ok(());
        }

        let signature = format!("{}|{}|{}", f.app, f.window_title, f.text.len());
        let last = self.last_signature.lock().clone();
        if last.as_deref() == Some(&signature) {
            return Ok(()); // settled already recorded
        }
        *self.last_signature.lock() = Some(signature);

        let row = TextRow {
            id: Uuid::new_v4().to_string(),
            app: f.app,
            title: f.window_title,
            content: self.crypto.encrypt_str(&f.text)?,
            preview: {
                let one: String = f.text.split_whitespace().collect::<Vec<_>>().join(" ");
                one.chars().take(PREVIEW_LEN).collect()
            },
            created_at: now(),
        };
        self.db.text_insert(&row)?;
        Ok(())
    }

    // ---- commands -----------------------------------------------------------

    pub fn list(&self, query: Option<String>) -> Result<Vec<RecoveryEntryDto>> {
        self.db
            .text_list(query.as_deref(), RETENTION_SECS)?
            .into_iter()
            .map(|r| {
                Ok(RecoveryEntryDto {
                    id: r.id,
                    app: r.app,
                    title: r.title,
                    content: self.crypto.decrypt_str(&r.content)?,
                    preview: r.preview,
                    created_at: r.created_at,
                })
            })
            .collect()
    }

    /// Copy the rescued text back to the clipboard; the user pastes wherever.
    pub fn recover(&self, id: &str) -> Result<String> {
        let row = self
            .db
            .text_get(id)?
            .ok_or_else(|| anyhow::anyhow!("snapshot not found"))?;
        let text = self.crypto.decrypt_str(&row.content)?;
        let to_copy = text.clone();
        std::thread::spawn(move || {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(to_copy);
            }
        });
        Ok(text)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.text_delete(id)
    }
}

// ============================================================================
// Windows implementation — UI Automation over windows-rs
// ============================================================================

#[cfg(windows)]
fn capture_focused_text() -> Result<Option<FocusedText>> {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationValuePattern, UIA_IsPasswordPropertyId,
        UIA_ValuePatternId,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };

    // UIA requires an initialized COM apartment on this thread.
    unsafe {
        // Ignore RPC_E_CHANGED_MODE — the pool thread may already be MTA;
        // UIA calls below still work there for element reads.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let result = (|| -> Result<Option<FocusedText>> {
        unsafe {
            let automation: IUIAutomation =
                windows::Win32::System::Com::CoCreateInstance(&CUIAutomation, None, windows::Win32::System::Com::CLSCTX_INPROC_SERVER)?;
            let element = automation.GetFocusedElement()?;

            // Skip password controls unconditionally (privacy: never capture them).
            // windows-core 0.58 VARIANT exposes typed TryFrom conversions.
            let variant = element
                .GetCurrentPropertyValue(UIA_IsPasswordPropertyId)
                .unwrap_or_default();
            let is_password = bool::try_from(&variant).unwrap_or(false);
            if is_password {
                return Ok(None);
            }

            // Prefer the ValuePattern (editable controls); fall back to Name.
            let text = match element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
                Ok(value_pattern) => value_pattern.CurrentValue()?.to_string(),
                Err(_) => element.CurrentName()?.to_string(),
            };
            if text.trim().is_empty() {
                return Ok(None);
            }

            // Foreground window metadata for context in the recovery list.
            let hwnd = GetForegroundWindow();
            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let window_title = String::from_utf16_lossy(&title_buf[..len as usize]);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let app = process_name(pid).unwrap_or_else(|| format!("pid {pid}"));

            Ok(Some(FocusedText {
                app,
                window_title,
                text,
                is_password,
            }))
        }
    })();

    unsafe { CoUninitialize() };
    result
}

/// Resolve a process id to its image name (e.g. "Code.exe") for display.
#[cfg(windows)]
fn process_name(pid: u32) -> Option<String> {
    let tasklist = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&tasklist.stdout);
    let name = stdout.trim().trim_matches('"').split("\",\"").next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

// ============================================================================
// Non-Windows stub
// ============================================================================

#[cfg(not(windows))]
fn capture_focused_text() -> Result<Option<FocusedText>> {
    // TODO(macOS): AXUIElementCopyAttributeValue(AXFocusedUIElement, kAXValueAttribute).
    Ok(None)
}
