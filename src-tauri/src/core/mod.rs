//! Public module layout for the SmartHub core.
//!
//! - [`db`]          — SQLite persistence (clipboard history, settings, license state)
//! - [`crypto`]      — AES-256-GCM encryption-at-rest with an OS-vaulted key
//! - [`clipboard`]   — Smart Clipboard module (Free tier)
//! - [`text_saver`]  — Text Time Machine module (Free tier)
//! - [`hotkey`]      — Global hotkey registration (Alt+Space) and palette toggling
//! - [`workspace`]   — Workspace Modes module (Pro tier)
//! - [`ai_engine`]   — On-device model management + file declutter heuristics (Pro tier)

pub mod ai_engine;
pub mod clipboard;
pub mod crypto;
pub mod db;
pub mod hotkey;
pub mod text_saver;
pub mod workspace;
