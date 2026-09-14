//! SmartHub — app entry & Tauri wiring.
//!
//! Responsibilities of this file:
//! - create the shared `AppState` (db, crypto option layer, services, license state)
//! - register all `tauri::command`s (the JS ↔ Rust contract — see `src/lib/api` on the frontend)
//! - set up the global hotkey, tray icon, and window focus behaviour
//!
//! Feature modules live under [`core`] and [`licensing`].

pub mod core;
pub mod licensing;

use std::sync::Arc;

use anyhow::Result;
use core::ai_engine::{AiEngine, AiStatus, ScanResultDto};
use core::clipboard::{ClipboardItemDto, ClipboardService};
use core::crypto::Crypto;
use core::db::{AppSettings, Db};
use core::hotkey;
use core::text_saver::{RecoveryEntryDto, TextSaver};
use core::workspace::{ProfilesDto, WorkspaceProfile, WorkspaceService};
use licensing::{LicenseInfo, Tier};
use parking_lot::RwLock;
use tauri::{AppHandle, Manager, State, WindowEvent};

/// Everything commands and services share. Cheap to clone (all Arc).
pub struct AppState {
    pub db: Arc<Db>,
    pub clipboard: Arc<ClipboardService>,
    pub text_saver: Arc<TextSaver>,
    pub workspace: Arc<WorkspaceService>,
    pub ai: Arc<AiEngine>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub text_capture_enabled: Arc<RwLock<bool>>,
    pub license: Arc<RwLock<LicenseInfo>>,
}

impl AppState {
    /// Effective Pro check — grace-aware. Every Pro command goes through this.
    fn require_pro(&self) -> Result<(), String> {
        let info = self.license.read();
        match licensing::offline_grace::effective_tier(&info) {
            Tier::Pro => Ok(()),
            Tier::Free => Err("pro_required".to_string()),
        }
    }
}

// ============================================================================
// Commands — Smart Clipboard (Free)
// ============================================================================

#[tauri::command]
fn get_clipboard_history(
    state: State<'_, AppState>,
    query: Option<String>,
) -> Result<Vec<ClipboardItemDto>, String> {
    state.clipboard.history(query).map_err(err_str)
}

#[tauri::command]
fn copy_to_clipboard(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.clipboard.copy(&id).map_err(err_str)
}

#[tauri::command]
fn toggle_pin_clipboard_item(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<Option<ClipboardItemDto>, String> {
    state.clipboard.set_pinned(&id, pinned).map_err(err_str)
}

#[tauri::command]
fn delete_clipboard_item(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.clipboard.delete(&id).map_err(err_str)
}

// ============================================================================
// Commands — Text Time Machine (Free)
// ============================================================================

#[tauri::command]
fn get_recoverable_texts(
    state: State<'_, AppState>,
    query: Option<String>,
) -> Result<Vec<RecoveryEntryDto>, String> {
    state.text_saver.list(query).map_err(err_str)
}

#[tauri::command]
fn recover_text(state: State<'_, AppState>, id: String) -> Result<String, String> {
    state.text_saver.recover(&id).map_err(err_str)
}

#[tauri::command]
fn delete_recovery_entry(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.text_saver.delete(&id).map_err(err_str)
}

// ============================================================================
// Commands — Licensing
// ============================================================================

#[tauri::command]
fn get_license_state(state: State<'_, AppState>) -> LicenseInfo {
    state.license.read().clone()
}

#[tauri::command]
async fn activate_license(state: State<'_, AppState>, key: String) -> Result<LicenseInfo, String> {
    let db = Arc::clone(&state.db);
    let info = licensing::activation::activate(&db, &key)
        .await
        .map_err(err_str)?;
    *state.license.write() = info.clone();
    Ok(info)
}

#[tauri::command]
fn deactivate_license(state: State<'_, AppState>) -> Result<LicenseInfo, String> {
    let info = licensing::activation::deactivate(&state.db).map_err(err_str)?;
    *state.license.write() = info.clone();
    Ok(info)
}

// ============================================================================
// Commands — Workspace Modes (Pro)
// ============================================================================

#[tauri::command]
fn get_workspace_profiles(state: State<'_, AppState>) -> Result<ProfilesDto, String> {
    state.workspace.list().map_err(err_str)
}

#[tauri::command]
fn apply_workspace_profile(state: State<'_, AppState>, id: String) -> Result<Vec<String>, String> {
    state.require_pro()?;
    state.workspace.apply(&id).map_err(err_str)
}

#[tauri::command]
fn save_workspace_profile(
    state: State<'_, AppState>,
    profile: WorkspaceProfile,
) -> Result<WorkspaceProfile, String> {
    state.require_pro()?;
    state.workspace.save(profile).map_err(err_str)
}

#[tauri::command]
fn delete_workspace_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.require_pro()?;
    state.workspace.delete(&id).map_err(err_str)
}

// ============================================================================
// Commands — AI Declutterer (Pro)
// ============================================================================

#[tauri::command]
fn get_ai_status(state: State<'_, AppState>) -> AiStatus {
    state.ai.status()
}

#[tauri::command]
fn download_ai_model(state: State<'_, AppState>) -> Result<(), String> {
    state.require_pro()?;
    state.ai.download_model().map_err(err_str)
}

#[tauri::command]
async fn scan_folder(state: State<'_, AppState>, path: String) -> Result<ScanResultDto, String> {
    state.require_pro()?;
    let ai = Arc::clone(&state.ai);
    // Disk walk + hashing is blocking IO/CPU — keep it off the async runtime.
    tokio::task::spawn_blocking(move || ai.scan_folder(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err_str)
}

// ============================================================================
// Commands — Settings / Hotkey
// ============================================================================

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.read().clone()
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let old_hotkey = state.settings.read().hotkey.clone();
    state.db.save_settings(&settings).map_err(err_str)?;
    *state.text_capture_enabled.write() = settings.text_capture_enabled;

    // Hotkey change → re-register globally.
    if settings.hotkey != old_hotkey {
        hotkey::register(&app, &settings.hotkey).map_err(err_str)?;
    }
    *state.settings.write() = settings.clone();
    Ok(settings)
}

// ============================================================================
// App bootstrap
// ============================================================================

fn err_str(err: anyhow::Error) -> String {
    #[cfg(debug_assertions)]
    eprintln!("[smarthub] {err:#}");
    format!("{err:#}")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(core::hotkey::plugin_handler)
                .build(),
        )
        .setup(|app| {
            // ---- local, encrypted storage --------------------------------
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("smarthub"));
            let db = Arc::new(Db::open(&app_data.join("smarthub.sqlite"))?);
            let crypto = Arc::new(Crypto::load_or_create()?);

            // ---- services --------------------------------------------------
            let settings = Arc::new(RwLock::new(db.settings().unwrap_or_default()));
            let text_capture_enabled =
                Arc::new(RwLock::new(settings.read().text_capture_enabled));

            let clipboard = Arc::new(ClipboardService::new(Arc::clone(&db), Arc::clone(&crypto)));
            clipboard.start(Arc::clone(&settings));

            let text_saver = Arc::new(TextSaver::new(Arc::clone(&db), Arc::clone(&crypto)));
            text_saver.start(Arc::clone(&text_capture_enabled));

            let workspace = Arc::new(WorkspaceService::new(Arc::clone(&db)));
            workspace.seed_builtins().ok(); // non-fatal: profiles are re-seeded next launch

            let ai = Arc::new(AiEngine::new(&app_data));

            let license_state = Arc::new(RwLock::new(licensing::activation::load_local_state(&db)));

            app.manage(AppState {
                db: Arc::clone(&db),
                clipboard,
                text_saver,
                workspace,
                ai,
                settings: Arc::clone(&settings),
                text_capture_enabled,
                license: Arc::clone(&license_state),
            });

            // ---- global hotkey (Alt+Space) ---------------------------------
            let hotkey_str = settings.read().hotkey.clone();
            let initial = if hotkey_str.trim().is_empty() {
                hotkey::DEFAULT_HOTKEY
            } else {
                &hotkey_str
            };
            if let Err(err) = hotkey::register(app.handle(), initial) {
                // Conflicting hotkeys shouldn't kill the app — fall back to default.
                eprintln!("[hotkey] {err:#}; falling back to {}", hotkey::DEFAULT_HOTKEY);
                hotkey::register(app.handle(), hotkey::DEFAULT_HOTKEY).ok();
            }

            // ---- licensing: validate now + every 24 h ----------------------
            licensing::offline_grace::spawn_revalidator(
                app.handle().clone(),
                Arc::clone(&db),
                license_state,
            );

            // ---- tray ------------------------------------------------------
            build_tray(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Spotlight behaviour: clicking away dismisses the palette.
            if let WindowEvent::Focused(false) = event {
                #[cfg(not(debug_assertions))]
                window.hide().ok();
                #[cfg(debug_assertions)]
                let _ = window; // keep focus during `tauri dev`
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            copy_to_clipboard,
            toggle_pin_clipboard_item,
            delete_clipboard_item,
            get_recoverable_texts,
            recover_text,
            delete_recovery_entry,
            get_license_state,
            activate_license,
            deactivate_license,
            get_workspace_profiles,
            apply_workspace_profile,
            save_workspace_profile,
            delete_workspace_profile,
            get_ai_status,
            download_ai_model,
            scan_folder,
            get_settings,
            save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run SmartHub");
}

/// System tray with quick access (Show / Quit). Icon ships in `icons/`.
fn build_tray(app: &mut tauri::App) -> Result<(), tauri::Error> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    let show = MenuItemBuilder::with_id("show", "Open SmartHub  (Alt+Space)").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

    TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => core::hotkey::toggle_palette(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    ..
                }
            ) {
                core::hotkey::toggle_palette(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
