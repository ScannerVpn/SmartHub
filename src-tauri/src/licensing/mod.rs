//! License lifecycle management (Lemon Squeezy-compatible API).
//!
//! Flow (see the PRD §4):
//! 1. User buys on the storefront → store issues a license key.
//! 2. The app calls our licensing API (`activate`) with the key.
//! 3. The key is stored in the OS credential manager; only a masked copy
//!    and the instance id are kept in the local database.
//! 4. Every 24 h the app revalidates; if the server is unreachable the
//!    7-day offline grace window keeps Pro active ([`offline_grace`]).
//!
//! The base URL points at the Cloudflare Worker in `server/workers`, which
//! proxies Lemon Squeezy so we can cache validations and track activations
//! in our own database — swap `API_BASE` if you self-host.

pub mod activation;
pub mod offline_grace;
pub mod validation;

use serde::{Deserialize, Serialize};

/// Worker endpoint that fronts the Lemon Squeezy license API.
pub const API_BASE: &str = "https://licensing.smarthub.app";

/// How often (seconds) the background task revalidates the license. PRD: 24 h.
pub const REVALIDATE_INTERVAL_SECS: u64 = 24 * 60 * 60;
/// Offline grace window (seconds). PRD: 7 days.
pub const GRACE_PERIOD_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "tier")]
pub enum Tier {
    Free,
    Pro,
}

/// What the frontend needs to render licensing state. Never contains
/// the full license key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseInfo {
    pub tier: Tier,
    pub key_masked: Option<String>,
    /// True when the last validation succeeded within the grace window
    /// (or we are currently online & validated).
    pub grace_valid: bool,
    /// Days left of the 7-day offline window, when running on cached validation.
    pub grace_days_left: Option<i64>,
    pub instance_id: Option<String>,
    /// Unix timestamp of the last successful validation.
    pub last_validated_at: Option<i64>,
}

impl Default for LicenseInfo {
    fn default() -> Self {
        Self {
            tier: Tier::Free,
            key_masked: None,
            grace_valid: false,
            grace_days_left: None,
            instance_id: None,
            last_validated_at: None,
        }
    }
}

/// Shorten a full key to `****-****-****-9F3A` for display.
pub fn mask_key(key: &str) -> String {
    let tail: String = key.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let tail = tail.chars().rev().take(4).collect::<Vec<_>>();
    let mut tail: String = tail.into_iter().rev().collect();
    tail.make_ascii_uppercase();
    format!("****-****-****-{tail}")
}
