//! SQLite persistence layer (rusqlite, bundled SQLite so no system deps).
//!
//! Layout:
//! - `clipboard_items` — Smart Clipboard history (content AES-256-GCM encrypted)
//! - `text_snapshots`  — Text Time Machine (hourly-purged, encrypted)
//! - `workspace_profiles` — Pro profiles, stored as JSON
//! - `kv`              — small key/value table (settings + license state)
//!
//! One `Connection` behind a `Mutex`; DB operations are short and the UI
//! is single-window, so contention is negligible. Heavy work is still
//! dispatched with `tokio::task::spawn_blocking` by the services.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Epoch seconds (UTC) — single time source for the whole app.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub hotkey: String,
    pub launch_at_login: bool,
    pub max_clipboard_items: i64,
    pub clipboard_monitoring: bool,
    pub text_capture_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Space".to_string(),
            launch_at_login: true,
            max_clipboard_items: 20, // Free tier cap (PRD) — Pro lifts it.
            clipboard_monitoring: true,
            text_capture_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardRow {
    pub id: String,
    pub kind: String,
    pub content: Vec<u8>, // ciphertext
    pub preview: String,
    pub link_title: Option<String>,
    pub hash: String,
    pub pinned: bool,
    pub created_at: i64,
    pub use_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRow {
    pub id: String,
    pub app: String,
    pub title: String,
    pub content: Vec<u8>, // ciphertext
    pub preview: String,
    pub created_at: i64,
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path).context("open sqlite")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.lock().execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_items (
                id          TEXT PRIMARY KEY,
                kind        TEXT NOT NULL,
                content     BLOB NOT NULL,
                preview     TEXT NOT NULL,
                link_title  TEXT,
                hash        TEXT NOT NULL,
                pinned      INTEGER NOT NULL DEFAULT 0,
                created_at  INTEGER NOT NULL,
                use_count   INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_clip_created ON clipboard_items(created_at DESC);

            CREATE TABLE IF NOT EXISTS text_snapshots (
                id          TEXT PRIMARY KEY,
                app         TEXT NOT NULL,
                title       TEXT NOT NULL,
                content     BLOB NOT NULL,
                preview     TEXT NOT NULL,
                created_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_text_created ON text_snapshots(created_at DESC);

            CREATE TABLE IF NOT EXISTS workspace_profiles (
                id       TEXT PRIMARY KEY,
                json     TEXT NOT NULL,
                builtin  INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS kv (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    // ---- kv / settings -----------------------------------------------------

    pub fn kv_get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM kv WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    pub fn kv_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.lock().execute(
            "INSERT INTO kv(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn settings(&self) -> Result<AppSettings> {
        let mut s = AppSettings::default();
        if let Some(json) = self.kv_get("settings")? {
            if let Ok(saved) = serde_json::from_str::<AppSettings>(&json) {
                s = saved;
            }
        }
        Ok(s)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        self.kv_set("settings", &serde_json::to_string(settings)?)
    }

    // ---- clipboard ----------------------------------------------------------

    pub fn clipboard_insert(&self, row: &ClipboardRow) -> Result<()> {
        self.conn.lock().execute(
            "INSERT OR REPLACE INTO clipboard_items
             (id, kind, content, preview, link_title, hash, pinned, created_at, use_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                row.id, row.kind, row.content, row.preview, row.link_title,
                row.hash, row.pinned as i64, row.created_at, row.use_count
            ],
        )?;
        Ok(())
    }

    /// List items, optionally filtered by a case-insensitive substring over
    /// preview / link title. Pinned first, then newest.
    pub fn clipboard_list(&self, query: Option<&str>, limit: i64) -> Result<Vec<ClipboardRow>> {
        let conn = self.conn.lock();
        let (sql, pattern) = match query.filter(|q| !q.trim().is_empty()) {
            Some(q) => (
                "SELECT id, kind, content, preview, link_title, hash, pinned, created_at, use_count
                 FROM clipboard_items
                 WHERE preview LIKE ?1 ESCAPE '\\' OR ifnull(link_title, '') LIKE ?1 ESCAPE '\\'
                 ORDER BY pinned DESC, created_at DESC LIMIT ?2",
                format!("%{}%", q.replace('%', "\\%").replace('_', "\\_")),
            ),
            None => (
                "SELECT id, kind, content, preview, link_title, hash, pinned, created_at, use_count
                 FROM clipboard_items
                 ORDER BY pinned DESC, created_at DESC LIMIT ?2",
                String::new(),
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = if pattern.is_empty() {
            stmt.query(params![limit])?
                .mapped(|r| map_clip(r))
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            stmt.query(params![pattern, limit])?
                .mapped(|r| map_clip(r))
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    pub fn clipboard_get(&self, id: &str) -> Result<Option<ClipboardRow>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, kind, content, preview, link_title, hash, pinned, created_at, use_count
             FROM clipboard_items WHERE id = ?1",
            params![id],
            |r| map_clip(r),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn clipboard_set_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clipboard_items SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        )?;
        Ok(())
    }

    pub fn clipboard_bump_usage(&self, id: &str) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clipboard_items SET use_count = use_count + 1, created_at = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
        Ok(())
    }

    pub fn clipboard_delete(&self, id: &str) -> Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clipboard_hash_exists(&self, hash: &str) -> Result<bool> {
        let conn = self.conn.lock();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE hash = ?1",
            params![hash],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Keep the newest `max` unpinned items.
    pub fn clipboard_trim(&self, max: i64) -> Result<()> {
        self.conn.lock().execute(
            "DELETE FROM clipboard_items WHERE pinned = 0 AND id NOT IN (
                 SELECT id FROM clipboard_items WHERE pinned = 0
                 ORDER BY created_at DESC LIMIT ?1
             )",
            params![max],
        )?;
        Ok(())
    }

    // ---- text snapshots -----------------------------------------------------

    pub fn text_insert(&self, row: &TextRow) -> Result<()> {
        self.conn.lock().execute(
            "INSERT OR REPLACE INTO text_snapshots (id, app, title, content, preview, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![row.id, row.app, row.title, row.content, row.preview, row.created_at],
        )?;
        Ok(())
    }

    /// Snapshots of the last `max_age_secs` (PRD: keep only the last hour).
    pub fn text_list(&self, query: Option<&str>, max_age_secs: i64) -> Result<Vec<TextRow>> {
        let cutoff = now() - max_age_secs;
        let conn = self.conn.lock();
        // ESCAPE '\' makes user input safe for LIKE patterns.
        let pattern = query
            .filter(|q| !q.trim().is_empty())
            .map(|q| format!("%{}%", q.replace('%', "\\%").replace('_', "\\_")))
            .unwrap_or_default();
        let mut stmt = conn.prepare(
            "SELECT id, app, title, content, preview, created_at FROM text_snapshots
             WHERE created_at >= ?1
               AND (?2 = '' OR preview LIKE ?2 ESCAPE '\\'
                    OR app LIKE ?2 ESCAPE '\\' OR title LIKE ?2 ESCAPE '\\')
             ORDER BY created_at DESC LIMIT 200",
        )?;
        let rows = stmt
            .query_map(params![cutoff, pattern], map_text)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn text_get(&self, id: &str) -> Result<Option<TextRow>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, app, title, content, preview, created_at FROM text_snapshots WHERE id = ?1",
            params![id],
            map_text,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn text_delete(&self, id: &str) -> Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM text_snapshots WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn text_purge(&self, max_age_secs: i64) -> Result<usize> {
        let cutoff = now() - max_age_secs;
        let n = self.conn.lock().execute(
            "DELETE FROM text_snapshots WHERE created_at < ?1",
            params![cutoff],
        )?;
        Ok(n)
    }

    // ---- workspace profiles -------------------------------------------------

    pub fn profiles_all(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, json FROM workspace_profiles ORDER BY rowid")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn profile_upsert(&self, id: &str, json: &str, builtin: bool) -> Result<()> {
        self.conn.lock().execute(
            "INSERT INTO workspace_profiles(id, json, builtin) VALUES(?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET json = excluded.json",
            params![id, json, builtin as i64],
        )?;
        Ok(())
    }

    pub fn profile_delete(&self, id: &str) -> Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM workspace_profiles WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn map_clip(r: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardRow> {
    Ok(ClipboardRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        content: r.get(2)?,
        preview: r.get(3)?,
        link_title: r.get(4)?,
        hash: r.get(5)?,
        pinned: r.get::<_, i64>(6)? != 0,
        created_at: r.get(7)?,
        use_count: r.get(8)?,
    })
}

fn map_text(r: &rusqlite::Row<'_>) -> rusqlite::Result<TextRow> {
    Ok(TextRow {
        id: r.get(0)?,
        app: r.get(1)?,
        title: r.get(2)?,
        content: r.get(3)?,
        preview: r.get(4)?,
        created_at: r.get(5)?,
    })
}

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for rusqlite::Result<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
