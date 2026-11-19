use crate::{Engine, model::Result, paths};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, State, ipc::Channel};
use tauri_plugin_updater::{Update, UpdaterExt};

pub const RELEASES: &str = "https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases";
const MAX_DOWNLOAD: u64 = 128 * 1024 * 1024;

#[derive(Default)]
pub struct AppUpdates(tokio::sync::Mutex<Session>);
#[derive(Default)]
struct Session {
    candidate: Option<Update>,
    downloaded: Option<Vec<u8>>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateAction {
    Status,
    Preference { enabled: bool },
    Check,
    Download { version: String },
    Install { version: String, confirmed: bool },
    OpenReleases,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    bytes: u64,
    total: Option<u64>,
}

pub fn startup_preference(store: &crate::storage::Store) -> Result<bool> {
    let value: Option<String> = store
        .connection
        .query_row(
            "SELECT value FROM state WHERE key='app_update_startup'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    value
        .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
        .unwrap_or(Ok(true))
}

pub fn validate_release(current: &str, version: &str, url: &str) -> Result<()> {
    let current = semver::Version::parse(current).map_err(|_| "Invalid current app version.")?;
    let next = semver::Version::parse(version).map_err(|_| "Invalid update version.")?;
    if next <= current {
        return Err("Only newer app versions can be installed.".into());
    }
    let expected =
        format!("{RELEASES}/download/v{version}/ConanForge-v{version}-windows-x64-setup.exe");
    if url != expected {
        return Err("This update does not point to an official ConanForge installer.".into());
    }
    Ok(())
}

pub fn registered_location_matches(exe: &Path, location: &str) -> bool {
    let Some(parent) = exe.parent() else {
        return false;
    };
    let Ok(parent) = parent.canonicalize() else {
        return false;
    };
    let Ok(registered) = Path::new(location.trim_matches('"')).canonicalize() else {
        return false;
    };
    exe.file_name()
        .is_some_and(|n| n == "conan-mod-manager.exe")
        && parent == registered
}

fn is_installed() -> bool {
    #[cfg(windows)]
    {
        use winreg::{
            RegKey,
            enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_64KEY},
        };
        let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ConanForge Mod Manager",
            KEY_READ | KEY_WOW64_64KEY,
        ) else {
            return false;
        };
        let Ok(location) = key.get_value::<String, _>("InstallLocation") else {
            return false;
        };
        std::env::current_exe().is_ok_and(|exe| registered_location_matches(&exe, &location))
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn backup_before_install(engine: &Engine, confirmed: bool) -> Result<PathBuf> {
    if !confirmed {
        return Err("Confirm Install and restart first.".into());
    }
    engine.online.ensure_idle()?;
    let root = engine.data_dir.join("backups").join("app-updates");
    paths::writable_path(&root)?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let folder = tempfile::Builder::new()
        .prefix("before-update-")
        .tempdir_in(&root)
        .map_err(|e| e.to_string())?;
    engine
        .store
        .connection
        .backup("main", folder.path().join("manager.sqlite3"), None)
        .map_err(|e| format!("Could not back up app settings; update was not started: {e}"))?;
    Ok(folder.keep())
}

fn view(update: &Update, downloaded: bool) -> Value {
    json!({"version":update.version,"notes":update.body,"downloaded":downloaded})
}

#[tauri::command]
pub async fn app_update(
    action: UpdateAction,
    progress: Channel<Progress>,
    window: tauri::WebviewWindow,
    app: AppHandle,
    updates: State<'_, AppUpdates>,
    engine: State<'_, Arc<Mutex<Engine>>>,
) -> Result<Value> {
    if window.label() != "main" {
        return Err("App updates are available only in the main window.".into());
    }
    match action {
        UpdateAction::Status => {
            let engine = engine.lock().map_err(|_| "Workspace unavailable.")?;
            Ok(
                json!({"currentVersion":app.package_info().version.to_string(),"checkAtStartup":startup_preference(&engine.store)?,"installed":is_installed()}),
            )
        }
        UpdateAction::Preference { enabled } => {
            engine
                .lock()
                .map_err(|_| "Workspace unavailable.")?
                .store
                .put("app_update_startup", &enabled)?;
            Ok(json!(enabled))
        }
        UpdateAction::OpenReleases => {
            crate::integrations::open_site(RELEASES)?;
            Ok(Value::Null)
        }
        action => {
            let mut session = updates
                .0
                .try_lock()
                .map_err(|_| "An app update operation is already running.")?;
            match action {
                UpdateAction::Check => {
                    // A failed or empty recheck must not leave an old candidate installable.
                    session.candidate = None;
                    session.downloaded = None;
                    let candidate = app.updater_builder().timeout(Duration::from_secs(20)).build()
                        .map_err(|e| e.to_string())?.check().await
                        .map_err(|_| "Could not check GitHub for app updates. You can keep using ConanForge and try again later.")?;
                    if let Some(mut update) = candidate {
                        validate_release(
                            &app.package_info().version.to_string(),
                            &update.version,
                            update.download_url.as_str(),
                        )?;
                        update.timeout = Some(Duration::from_secs(180));
                        let result = view(&update, false);
                        session.candidate = Some(update);
                        Ok(result)
                    } else {
                        Ok(Value::Null)
                    }
                }
                UpdateAction::Download { version } => {
                    if !is_installed() {
                        return Err("Install ConanForge with the Windows setup first to use in-app updates.".into());
                    }
                    session.downloaded = None;
                    let update = session
                        .candidate
                        .as_ref()
                        .filter(|u| u.version == version)
                        .ok_or("Check for this update again.")?;
                    let too_large = tokio::sync::Notify::new();
                    let mut bytes = 0_u64;
                    let download = update.download(
                        |chunk, total| {
                            bytes = bytes.saturating_add(chunk as u64);
                            if bytes > MAX_DOWNLOAD || total.is_some_and(|size| size > MAX_DOWNLOAD)
                            {
                                too_large.notify_one();
                            }
                            let _ = progress.send(Progress { bytes, total });
                        },
                        || {},
                    );
                    let payload = tokio::select! {
                        biased;
                        _ = too_large.notified() => return Err("The app update exceeds the 128 MiB download limit.".into()),
                        result = tokio::time::timeout(Duration::from_secs(180), download) => result
                            .map_err(|_| "The update download timed out. Try again later.")?
                            .map_err(|_| "The update could not be downloaded or its signature/version could not be verified. Nothing was installed.")?,
                    };
                    if payload.len() as u64 > MAX_DOWNLOAD {
                        return Err("The app update is too large.".into());
                    }
                    let result = view(update, true);
                    session.downloaded = Some(payload);
                    Ok(result)
                }
                UpdateAction::Install { version, confirmed } => {
                    if !confirmed {
                        return Err("Confirm Install and restart first.".into());
                    }
                    if !is_installed() {
                        return Err(
                            "Use the Windows installer before installing updates from the app."
                                .into(),
                        );
                    }
                    let update = session
                        .candidate
                        .as_ref()
                        .filter(|u| u.version == version)
                        .ok_or("Check for this update again.")?;
                    let payload = session
                        .downloaded
                        .as_ref()
                        .ok_or("Download and verify the update first.")?;
                    update
                        .verify(payload)
                        .map_err(|_| "Update verification failed. Nothing was installed.")?;
                    // Serialize with profile changes, Apply and library operations, and reject
                    // an active provider job before the Windows installer exits this process.
                    let engine = engine.lock().map_err(|_| "Workspace unavailable.")?;
                    let _backup = backup_before_install(&engine, confirmed)?;
                    update
                        .install(payload)
                        .map_err(|e| format!("The installer could not start: {e}"))?;
                    Ok(Value::Null)
                }
                _ => unreachable!(),
            }
        }
    }
}
