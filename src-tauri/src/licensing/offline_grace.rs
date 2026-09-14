//! Offline grace window (PRD §4): if the licensing server is unreachable,
//! Pro keeps working for 7 days after the last successful validation.
//! After that, the app falls back to Free until it can revalidate.

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::{AppHandle, Emitter};
use tokio::time::{interval, Duration};

use super::{LicenseInfo, Tier, GRACE_PERIOD_SECS, REVALIDATE_INTERVAL_SECS};
use crate::core::db::{now, Db};

/// Seconds left in the grace window, or `None` if it has fully expired.
pub fn grace_remaining_secs(last_validated_at: Option<i64>) -> Option<i64> {
    let last = last_validated_at?;
    let remaining = last + GRACE_PERIOD_SECS - now();
    (remaining > 0).then_some(remaining)
}

/// The effective tier right now: explicit Pro if validated recently enough,
/// even when the network is down.
pub fn effective_tier(info: &LicenseInfo) -> Tier {
    match info.tier {
        Tier::Free => Tier::Free,
        Tier::Pro => {
            if grace_remaining_secs(info.last_validated_at).is_some() {
                Tier::Pro
            } else {
                Tier::Free // grace expired — fallback to Free per the PRD
            }
        }
    }
}

/// Background task: revalidate every 24 h, then push the fresh state to the
/// UI. Runs forever; cheap (one request per day, tiny JSON).
pub fn spawn_revalidator(app: AppHandle, db: Arc<Db>, state: Arc<RwLock<LicenseInfo>>) {
    tauri::async_runtime::spawn(async move {
        let mut tick = interval(Duration::from_secs(REVALIDATE_INTERVAL_SECS));
        // Skip the first interval tick so we validate shortly *after* startup,
        // not before the UI has even shown.
        tick.tick().await;
        loop {
            // Apply the current local state through the grace filter first.
            let local = super::activation::load_local_state(&db);
            let mut effective = local.clone();
            effective.tier = effective_tier(&local);
            *state.write() = effective.clone();
            let _ = app.emit("license://changed", &effective);

            match super::validation::validate(&db).await {
                Ok((outcome, info)) => {
                    let mut info = info;
                    // Network failure keeps the locally-derived state.
                    if matches!(outcome, super::validation::ValidationOutcome::Unreachable) {
                        info.tier = effective_tier(&local);
                    }
                    *state.write() = info.clone();
                    let _ = app.emit("license://changed", &info);
                }
                Err(err) => {
                    #[cfg(debug_assertions)]
                    eprintln!("[license revalidator] {err:#}");
                    let _ = err;
                }
            }

            tick.tick().await;
        }
    });
}
