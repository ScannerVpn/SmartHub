//! License activation.
//!
//! Calls the licensing worker (which proxies Lemon Squeezy
//! `POST /v1/licenses/activate`), then stores:
//! - the full key → OS credential manager (`keyring`)
//! - instance id + masked key + validation timestamp → SQLite
//!
//! The full key never touches our database or log output.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use super::{mask_key, LicenseInfo, Tier, API_BASE};
use crate::core::db::{now, Db};

const KEY_SERVICE: &str = "app.smarthub.desktop";
const KEY_ACCOUNT: &str = "smarthub-license-key-v1";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivateResponse {
    activated: bool,
    #[serde(default)]
    license_key: Option<LicenseKeyObj>,
    #[serde(default)]
    instance: Option<InstanceObj>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LicenseKeyObj {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct InstanceObj {
    id: String,
}

/// Persist the resolved license parts.
fn store_license(db: &Db, key_masked: &str, instance_id: &str) -> Result<()> {
    db.kv_set("license.tier", "pro")?;
    db.kv_set("license.key_masked", key_masked)?;
    db.kv_set("license.instance_id", instance_id)?;
    db.kv_set("license.last_validated_at", &now().to_string())?;
    Ok(())
}

/// Reconstruct the license info from local state (used at startup and by
/// the `get_license_state` command). Does not hit the network.
pub fn load_local_state(db: &Db) -> LicenseInfo {
    let tier = match db.kv_get("license.tier").ok().flatten().as_deref() {
        Some("pro") => Tier::Pro,
        _ => Tier::Free,
    };
    let last_validated_at = db
        .kv_get("license.last_validated_at")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok());

    let grace = super::offline_grace::grace_remaining_secs(last_validated_at);
    LicenseInfo {
        tier,
        key_masked: db.kv_get("license.key_masked").ok().flatten(),
        grace_valid: grace.is_some(),
        grace_days_left: grace.map(|s| s / 86_400 + 1),
        instance_id: db.kv_get("license.instance_id").ok().flatten(),
        last_validated_at,
    }
}

/// Activate a key against the licensing API. Returns the new license info.
pub async fn activate(db: &Db, license_key: &str) -> Result<LicenseInfo> {
    let key = license_key.trim();
    if key.len() < 12 {
        return Err(anyhow!("That doesn't look like a license key."));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()?;

    let resp: ActivateResponse = client
        .post(format!("{API_BASE}/activate"))
        .json(&serde_json::json!({
            "license_key": key,
            "instance_name": hostname()
        }))
        .send()
        .await
        .context("licensing server unreachable")?
        .json()
        .await
        .context("bad response from licensing server")?;

    if !resp.activated {
        return Err(anyhow!(
            resp.error.unwrap_or_else(|| "License could not be activated".into())
        ));
    }
    let instance_id = resp
        .instance
        .map(|i| i.id)
        .or_else(|| resp.license_key.map(|k| k.id.to_string()))
        .ok_or_else(|| anyhow!("licensing server returned no instance id"))?;

    // Vault the full key; persist only the masked form + instance id.
    keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT)
        .context("open OS credential store")?
        .set_password(key)
        .context("store license key in OS vault")?;

    let masked = mask_key(key);
    store_license(db, &masked, &instance_id)?;

    Ok(load_local_state(db))
}

/// Read the vaulted key (needed by revalidation). `Ok(None)` = not activated.
pub fn vaulted_key() -> Option<String> {
    keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT)
        .ok()?
        .get_password()
        .ok()
}

/// Remove the device activation locally. The user can re-activate later;
/// server-side deactivation happens through the storefront portal.
pub fn deactivate(db: &Db) -> Result<LicenseInfo> {
    if let Ok(entry) = keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT) {
        let _ = entry.delete_credential();
    }
    for key in [
        "license.tier",
        "license.key_masked",
        "license.instance_id",
        "license.last_validated_at",
    ] {
        let _ = db.kv_set(key, "");
    }
    Ok(LicenseInfo::default())
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "smarthub-device".into())
}
