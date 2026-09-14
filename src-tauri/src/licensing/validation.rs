//! Periodic license validation (every 24 h) with network-failure tolerance.
//!
//! Distinct error classes matter here:
//! - **Server says "invalid"** (key revoked/refunded/banned) → drop to Free.
//! - **Network/transport failure** → keep Pro; the offline grace window in
//!   [`super::offline_grace`] decides when to fall back.

use anyhow::Result;
use serde::Deserialize;

use super::{LicenseInfo, Tier, API_BASE};
use crate::core::db::{now, Db};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidateResponse {
    valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationOutcome {
    /// Server reachable and the license is valid.
    Valid,
    /// Server reachable and the license is definitively invalid.
    Invalid,
    /// Couldn't reach the server — outcome unknown, use grace.
    Unreachable,
}

pub async fn validate(db: &Db) -> Result<(ValidationOutcome, LicenseInfo)> {
    let Some(key) = super::activation::vaulted_key() else {
        return Ok((ValidationOutcome::Invalid, LicenseInfo::default()));
    };
    let instance_id = db.kv_get("license.instance_id").ok().flatten();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()?;

    let send = client
        .post(format!("{API_BASE}/validate"))
        .json(&serde_json::json!({
            "license_key": key,
            "instance_id": instance_id
        }))
        .send()
        .await;

    let resp = match send {
        Ok(r) => r,
        Err(_) => return Ok((ValidationOutcome::Unreachable, current(db))),
    };
    let body: ValidateResponse = match resp.json().await {
        Ok(b) => b,
        Err(_) => return Ok((ValidationOutcome::Unreachable, current(db))),
    };

    if body.valid {
        db.kv_set("license.tier", "pro")?;
        db.kv_set("license.last_validated_at", &now().to_string())?;
        Ok((ValidationOutcome::Valid, super::activation::load_local_state(db)))
    } else {
        // Definitively invalid — server explicitly said no.
        let _ = db.kv_set("license.tier", "free");
        Ok((ValidationOutcome::Invalid, super::activation::load_local_state(db)))
    }
}

fn current(db: &Db) -> LicenseInfo {
    let mut info = super::activation::load_local_state(db);
    if info.tier == Tier::Free {
        info.grace_valid = false;
    }
    info
}
