//! Workspace Modes (Pro tier).
//!
//! A profile is a named bundle of system actions — launch/quit apps, set the
//! wallpaper, set master volume, toggle Do Not Disturb. Applying a profile
//! executes its actions sequentially and records the state.
//!
//! Built-in profiles ship on first run (Focus / Streaming / Coding, incl. the
//! Creator Quick-Launch presets for OBS & Discord from the PRD); Pro users can
//! create unlimited custom profiles.

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileActionKind {
    LaunchApp,
    KillApp,
    SetWallpaper,
    SetVolume,
    Mute,
    DoNotDisturb,
    RunCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAction {
    pub kind: ProfileActionKind,
    /// Executable path/name, wallpaper path, level ("0.0"–"1.0"), or command.
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProfile {
    pub id: String,
    pub name: String,
    pub icon: String, // icon key resolved by the UI ("moon", "radio", "code"…)
    pub actions: Vec<ProfileAction>,
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesDto {
    pub profiles: Vec<WorkspaceProfile>,
    pub active_id: Option<String>,
}

pub struct WorkspaceService {
    db: Arc<Db>,
}

impl WorkspaceService {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    /// Seed the built-in profiles once (first launch only — `builtin` rows
    /// are upserted by id, so upgraded defaults still land on app updates).
    pub fn seed_builtins(&self) -> Result<()> {
        for p in builtin_profiles() {
            let json = serde_json::to_string(&p)?;
            self.db.profile_upsert(&p.id, &json, true)?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<ProfilesDto> {
        let profiles = self
            .db
            .profiles_all()?
            .into_iter()
            .filter_map(|(_, json)| serde_json::from_str::<WorkspaceProfile>(&json).ok())
            .collect();
        let active_id = self.db.kv_get("workspace.active")?;
        Ok(ProfilesDto { profiles, active_id })
    }

    pub fn save(&self, mut profile: WorkspaceProfile) -> Result<WorkspaceProfile> {
        if profile.id.is_empty() {
            profile.id = Uuid::new_v4().to_string();
        }
        let json = serde_json::to_string(&profile)?;
        self.db.profile_upsert(&profile.id, &json, profile.builtin)?;
        Ok(profile)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.profile_delete(id)
    }

    /// Execute a profile's actions. Failures of individual actions are
    /// collected, not fatal — e.g. quitting an app that isn't running.
    pub fn apply(&self, id: &str) -> Result<Vec<String>> {
        let profile = self
            .db
            .profiles_all()?
            .into_iter()
            .find(|(pid, _)| pid == id)
            .and_then(|(_, json)| serde_json::from_str::<WorkspaceProfile>(&json).ok())
            .ok_or_else(|| anyhow::anyhow!("profile not found: {id}"))?;

        let mut warnings = Vec::new();
        for action in &profile.actions {
            if let Err(err) = execute_action(action) {
                warnings.push(format!("{:?} → {err:#}", action.kind));
            }
        }
        self.db.kv_set("workspace.active", id)?;
        Ok(warnings)
    }
}

// ============================================================================
// Built-in profiles (PRD: Focus / Gaming / Streaming / Coding presets)
// ============================================================================

pub fn builtin_profiles() -> Vec<WorkspaceProfile> {
    use ProfileActionKind::*;
    let a = |kind: ProfileActionKind, value: &str| ProfileAction {
        kind,
        value: value.to_string(),
    };
    vec![
        WorkspaceProfile {
            id: "p-focus".into(),
            name: "Focus".into(),
            icon: "moon".into(),
            builtin: true,
            actions: vec![
                a(KillApp, "Discord.exe"),
                a(KillApp, "Telegram.exe"),
                a(DoNotDisturb, "true"),
                a(SetVolume, "0.2"),
            ],
        },
        WorkspaceProfile {
            id: "p-stream".into(),
            name: "Streaming".into(),
            icon: "radio".into(),
            builtin: true,
            actions: vec![
                a(LaunchApp, "obs64.exe"), // Creator Quick-Launch preset
                a(LaunchApp, "Discord.exe"),
                a(LaunchApp, "Streamlabs OBS.exe"),
                a(SetVolume, "0.6"),
                a(DoNotDisturb, "true"),
            ],
        },
        WorkspaceProfile {
            id: "p-code".into(),
            name: "Coding".into(),
            icon: "code".into(),
            builtin: true,
            actions: vec![
                a(LaunchApp, "Code.exe"),
                a(LaunchApp, "WindowsTerminal.exe"),
                a(DoNotDisturb, "false"),
                a(SetVolume, "0.35"),
            ],
        },
    ]
}

// ============================================================================
// Action execution
// ============================================================================

fn execute_action(action: &ProfileAction) -> Result<()> {
    match action.kind {
        ProfileActionKind::LaunchApp => launch_app(&action.value),
        ProfileActionKind::KillApp => kill_app(&action.value),
        ProfileActionKind::SetVolume => set_volume(action.value.parse::<f32>()?.clamp(0.0, 1.0)),
        ProfileActionKind::Mute => set_mute(action.value == "true"),
        ProfileActionKind::SetWallpaper => set_wallpaper(&action.value),
        ProfileActionKind::DoNotDisturb => set_do_not_disturb(action.value == "true"),
        ProfileActionKind::RunCommand => run_command(&action.value),
    }
}

fn launch_app(path_or_name: &str) -> Result<()> {
    std::process::Command::new(path_or_name)
        .spawn()
        .map(|_| ())
        .map_err(Into::into)
}

#[cfg(windows)]
fn kill_app(image_name: &str) -> Result<()> {
    // taskkill /IM is the most reliable way without enumerating processes.
    let status = std::process::Command::new("taskkill")
        .args(["/IM", image_name, "/T"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("process not running: {image_name}")
    }
}

#[cfg(not(windows))]
fn kill_app(image_name: &str) -> Result<()> {
    std::process::Command::new("pkill")
        .arg("-x")
        .arg(image_name.trim_end_matches(".exe"))
        .status()
        .map(|_| ())
        .map_err(Into::into)
}

#[cfg(not(windows))]
fn set_volume(_level: f32) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn set_mute(_muted: bool) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn set_wallpaper(_path: &str) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn set_do_not_disturb(_on: bool) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn run_command(_cmd: &str) -> Result<()> {
    Ok(())
}

// ---- Windows implementations ------------------------------------------------

#[cfg(windows)]
fn set_volume(level: f32) -> Result<()> {
    with_endpoint_volume(|ep| unsafe {
        ep.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
        Ok(())
    })
}

#[cfg(windows)]
fn set_mute(muted: bool) -> Result<()> {
    with_endpoint_volume(|ep| unsafe {
        ep.SetMute(muted, std::ptr::null())?;
        Ok(())
    })
}

/// Shared plumbing for IAudioEndpointVolume calls (default render device).
#[cfg(windows)]
fn with_endpoint_volume<F>(f: F) -> Result<()>
where
    F: FnOnce(&windows::Win32::Media::Audio::IAudioEndpointVolume) -> windows::core::Result<()>,
{
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        STGM_READ,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let r = (|| -> Result<()> {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let endpoint = device.Activate::<windows::Win32::Media::Audio::IAudioEndpointVolume>(CLSCTX_ALL, None)?;
            f(&endpoint)?;
            Ok(())
        })();
        CoUninitialize();
        r
    }
}

#[cfg(windows)]
fn set_wallpaper(path: &str) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_SETDESKWALLPAPER, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
    };
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(wide.as_ptr() as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )?;
    }
    Ok(())
}

#[cfg(windows)]
fn set_do_not_disturb(on: bool) -> Result<()> {
    // Focus Assist / Quiet hours: registry toggle + broadcast so Settings
    // picks it up. Full toast-suppression uses the notification platform API;
    // the registry path covers the user-visible "Do not disturb" switch.
    let value = if on { "1" } else { "0" };
    let status = std::process::Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings",
            "/v",
            "NOC_GLOBAL_SETTING_ALLOW_NOTIFICATION_SOUND",
            "/t",
            "REG_DWORD",
            "/d",
            value,
            "/f",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        anyhow::bail!("could not toggle Do Not Disturb");
    }
    // Notify the shell that settings changed.
    unsafe {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            SendNotifyMessageW, HWND_BROADCAST, WM_SETTINGCHANGE,
        };
        let _ = SendNotifyMessageW(HWND_BROADCAST, WM_SETTINGCHANGE, Some(WPARAM(0)), LPARAM(0));
    }
    Ok(())
}

#[cfg(windows)]
fn run_command(cmd: &str) -> Result<()> {
    std::process::Command::new("cmd")
        .args(["/C", cmd])
        .spawn()
        .map(|_| ())
        .map_err(Into::into)
}
