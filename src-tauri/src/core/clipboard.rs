//! Smart Clipboard (Free tier).
//!
//! A lightweight polling watcher (300 ms) records text/link/code/color items
//! into encrypted SQLite storage. Polling with `arboard` is cross-platform;
//! on Windows a `clipboard-win` event listener would shave a few ms of
//! latency — the service is behind a trait boundary so that swap is local.
//!
//! Pinned items survive trimming; the free tier keeps the newest N items
//! (default 20, configurable).

use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::time::{interval, Duration};
use uuid::Uuid;

use super::crypto::Crypto;
use super::db::{now, AppSettings, ClipboardRow, Db};

const POLL_INTERVAL: Duration = Duration::from_millis(300);
const PREVIEW_LEN: usize = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemDto {
    pub id: String,
    pub kind: String, // "text" | "link" | "code" | "image" | "color"
    pub content: String,
    pub preview: String,
    pub link_title: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
    pub use_count: i64,
}

pub struct ClipboardService {
    db: Arc<Db>,
    crypto: Arc<Crypto>,
    /// Hash of the last item we set ourselves, so `copy_to_clipboard`
    /// doesn't immediately re-record what the user just recalled.
    self_hash: Mutex<Option<String>>,
    last_seen_hash: Mutex<Option<String>>,
}

impl ClipboardService {
    pub fn new(db: Arc<Db>, crypto: Arc<Crypto>) -> Self {
        Self {
            db,
            crypto,
            self_hash: Mutex::new(None),
            last_seen_hash: Mutex::new(None),
        }
    }

    /// Background watcher task. Runs for the lifetime of the app.
    pub fn start(self: &Arc<Self>, settings: Arc<parking_lot::RwLock<AppSettings>>) {
        let svc = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let mut tick = interval(POLL_INTERVAL);
            loop {
                tick.tick().await;
                let monitoring = settings.read().clipboard_monitoring;
                if !monitoring {
                    continue;
                }
                if let Err(err) = svc.poll_once().await {
                    // Clipboard contention is normal (e.g. while another app
                    // holds it open); log and carry on.
                    tracing_warn(&err);
                }
            }
        });
    }

    async fn poll_once(&self) -> Result<()> {
        // arboard isn't Send-safe to hold across awaits — do a quick blocking read.
        let text = tokio::task::spawn_blocking(|| -> Result<Option<String>> {
            let mut cb = arboard::Clipboard::new().context("open clipboard")?;
            Ok(cb.get_text().ok().filter(|s| !s.trim().is_empty()))
        })
        .await??;

        let Some(text) = text else { return Ok(()) };
        let hash = hex_sha256(text.as_bytes());

        // Skip unchanged content and items we put there ourselves.
        if self.last_seen_hash.lock().as_deref() == Some(&hash) {
            return Ok(());
        }
        *self.last_seen_hash.lock() = Some(hash.clone());
        if self.self_hash.lock().as_deref() == Some(&hash) {
            *self.self_hash.lock() = None; // consume the sentinel once
            return Ok(());
        }
        if self.db.clipboard_hash_exists(&hash)? {
            return Ok(()); // already in history
        }

        let kind = classify(&text);
        let link_title = if kind == "link" {
            fetch_page_title(&text).await.ok().flatten()
        } else {
            None
        };

        let row = ClipboardRow {
            id: Uuid::new_v4().to_string(),
            kind: kind.to_string(),
            content: self.crypto.encrypt_str(&text)?,
            preview: preview_of(&text, PREVIEW_LEN),
            link_title,
            hash,
            pinned: false,
            created_at: now(),
            use_count: 1,
        };
        self.db.clipboard_insert(&row)?;

        let max = self.db.settings()?.max_clipboard_items;
        self.db.clipboard_trim(max)?;
        Ok(())
    }

    fn to_dto(&self, row: ClipboardRow) -> Result<ClipboardItemDto> {
        Ok(ClipboardItemDto {
            id: row.id,
            kind: row.kind,
            content: self.crypto.decrypt_str(&row.content)?,
            preview: row.preview,
            link_title: row.link_title,
            pinned: row.pinned,
            created_at: row.created_at,
            use_count: row.use_count,
        })
    }

    // ---- commands ----------------------------------------------------------

    pub fn history(&self, query: Option<String>) -> Result<Vec<ClipboardItemDto>> {
        let rows = self.db.clipboard_list(query.as_deref(), 500)?;
        rows.into_iter().map(|r| self.to_dto(r)).collect()
    }

    pub fn copy(&self, id: &str) -> Result<()> {
        let row = self
            .db
            .clipboard_get(id)?
            .ok_or_else(|| anyhow::anyhow!("clipboard item not found"))?;
        let content = self.crypto.decrypt_str(&row.content)?;
        *self.self_hash.lock() = Some(hex_sha256(content.as_bytes()));
        // Setting the OS clipboard is a blocking call: keep it off the runtime.
        let text = content.clone();
        std::thread::spawn(move || {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(text);
            }
        });
        self.db.clipboard_bump_usage(id)?;
        Ok(())
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<Option<ClipboardItemDto>> {
        self.db.clipboard_set_pinned(id, pinned)?;
        match self.db.clipboard_get(id)? {
            Some(row) => Ok(Some(self.to_dto(row)?)),
            None => Ok(None),
        }
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.clipboard_delete(id)
    }
}

// ---- classification & helpers ----------------------------------------------

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn classify(text: &str) -> &'static str {
    let t = text.trim();
    if (t.starts_with("http://") || t.starts_with("https://")) && !t.contains(char::is_whitespace) {
        "link"
    } else if is_css_color(t) {
        "color"
    } else if looks_like_code(t) {
        "code"
    } else {
        "text"
    }
}

fn is_css_color(t: &str) -> bool {
    let hex = t.strip_prefix('#').unwrap_or(t);
    (hex.len() == 6 || hex.len() == 3 || hex.len() == 8)
        && hex.chars().all(|c| c.is_ascii_hexdigit())
        && t.starts_with('#')
}

fn looks_like_code(t: &str) -> bool {
    let markers = ["->", "=>", "::", "fn ", "let ", "const ", "import ", "def ", "<?php", "#!/", "};"];
    let newline = t.lines().count() >= 2;
    newline && markers.iter().any(|m| t.contains(m))
}

fn preview_of(text: &str, len: usize) -> String {
    let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(len).collect()
}

/// Resolve a URL to its <title>, best-effort with a strict timeout so a slow
/// site never blocks the watcher. Only the <title> tag is parsed — no full
/// HTML parsing, no JavaScript, minimal surface.
async fn fetch_page_title(url: &str) -> Result<Option<String>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .user_agent("SmartHub/0.1 (+https://smarthub.app)")
        .build()?;
    let body = client.get(url).send().await?.text().await?;
    let lower = body.to_lowercase();
    let start = lower.find("<title>").map(|i| i + 7);
    let end = lower.find("</title>");
    if let (Some(s), Some(e)) = (start, end) {
        if e > s {
            let title = body[s..e].trim();
            if !title.is_empty() {
                return Ok(Some(title.chars().take(140).collect()));
            }
        }
    }
    Ok(None)
}

/// `log`-style warning without pulling a logging dependency for one call site.
fn tracing_warn(err: &anyhow::Error) {
    #[cfg(debug_assertions)]
    eprintln!("[clipboard watcher] {err:#}");
    let _ = err;
}
