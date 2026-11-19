pub mod deployment;
pub mod detection;
pub mod integrations;
pub mod model;
pub mod modlist;
pub mod paths;
pub mod scanner;
mod startup;
pub mod storage;

use model::{Profile, Result, Settings};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::Manager;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Online {
        action: integrations::OnlineAction,
    },
    Bootstrap,
    Detect,
    ResolveGame {
        path: String,
    },
    SaveSettings {
        settings: Settings,
    },
    Scan,
    PreviewLibraryDelete {
        digest: String,
    },
    DeleteLibrary {
        token: String,
        confirmed: bool,
    },
    CreateProfile {
        name: String,
    },
    SaveProfile {
        profile: Profile,
    },
    DeleteProfile {
        id: i64,
        revision: i64,
    },
    ImportModlist {
        path: String,
        base: String,
        id: i64,
        revision: i64,
    },
    ExportModlist {
        id: i64,
        path: String,
    },
    Preview {
        id: i64,
    },
    Apply {
        token: String,
        confirmed: bool,
    },
}
pub struct Engine {
    online: integrations::Service,
    pub store: storage::Store,
    pub data_dir: PathBuf,
    pending: Option<deployment::Prepared>,
    pending_delete: Option<integrations::maintenance::DeletePreview>,
    startup_detection: Option<model::Detection>,
}
impl Engine {
    pub fn open(data_dir: &Path) -> Result<Self> {
        paths::writable_path(data_dir)?;
        fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let store = storage::Store::open(&data_dir.join("manager.sqlite3"))?;
        Ok(Self {
            online: integrations::Service::new(data_dir),
            store,
            data_dir: data_dir.to_path_buf(),
            pending: None,
            pending_delete: None,
            startup_detection: None,
        })
    }
    pub fn dispatch(&mut self, action: Action) -> Result<Value> {
        match action {
            Action::Online { action } => self.online.dispatch(action),
            Action::Bootstrap => {
                if self.startup_detection.is_none() {
                    let found = detection::detect();
                    let settings = detection::startup_settings(self.store.settings()?, &found);
                    self.store.set_settings(&settings)?;
                    self.startup_detection = Some(found);
                }
                Ok(
                    json!({ "profiles":self.store.profiles()?, "settings":self.store.settings()?, "catalog":self.store.catalog()?, "acquisitions":self.store.acquisitions()?, "dataDir":paths::display(&self.data_dir), "integrations":integrations::status(), "detection":self.startup_detection }),
                )
            }
            Action::Detect => Ok(json!(detection::detect())),
            Action::PreviewLibraryDelete { digest } => {
                self.pending_delete = None;
                self.online.ensure_idle()?;
                let prepared =
                    integrations::maintenance::preview(&self.data_dir, &self.store, &digest)?;
                let value = json!(prepared);
                self.pending_delete = Some(prepared);
                Ok(value)
            }
            Action::DeleteLibrary { token, confirmed } => {
                self.online.ensure_idle()?;
                let prepared = self
                    .pending_delete
                    .take()
                    .ok_or("Review the library deletion first.")?;
                if prepared.token != token {
                    return Err("This deletion preview expired. Review again.".into());
                }
                self.pending = None;
                Ok(json!(integrations::maintenance::delete(
                    &self.data_dir,
                    &self.store,
                    prepared,
                    confirmed
                )?))
            }
            Action::ResolveGame { path } => Ok(json!(paths::display(&detection::resolve_game(
                Path::new(&path)
            )?))),
            Action::SaveSettings { mut settings } => {
                if !settings.game_root.is_empty() {
                    settings.game_root =
                        paths::display(&detection::resolve_game(Path::new(&settings.game_root))?);
                }
                self.pending = None;
                self.store.set_settings(&settings)?;
                Ok(json!(settings))
            }
            Action::Scan => {
                self.pending = None;
                let settings = self.store.settings()?;
                let mut roots = settings.scan_roots;
                if self.data_dir.join("library").is_dir() {
                    roots.push(paths::display(&self.data_dir.join("library")));
                }
                if !settings.game_root.is_empty() {
                    let mods = Path::new(&settings.game_root).join("ConanSandbox/Mods");
                    if mods.is_dir() {
                        roots.push(paths::display(&mods));
                    }
                }
                let scan = scanner::scan(&roots)?;
                self.store.put("catalog", &scan)?;
                Ok(json!(scan))
            }
            Action::CreateProfile { name } => Ok(json!(self.store.create(&name)?)),
            Action::SaveProfile { profile } => {
                self.pending = None;
                Ok(json!(self.store.save(&profile)?))
            }
            Action::DeleteProfile { id, revision } => {
                self.pending = None;
                self.store.delete(id, revision)?;
                Ok(Value::Null)
            }
            Action::ImportModlist {
                path,
                base,
                id,
                revision,
            } => {
                let mut profile = self.store.profile(id)?;
                if profile.revision != revision {
                    return Err("Profile changed. Reload before importing.".into());
                }
                profile.entries =
                    modlist::parse(&deployment::read_list(Path::new(&path))?, Path::new(&base))?;
                self.pending = None;
                Ok(json!(self.store.save(&profile)?))
            }
            Action::ExportModlist { id, path } => {
                deployment::export(&self.store.profile(id)?, Path::new(&path))?;
                Ok(Value::Null)
            }
            Action::Preview { id } => {
                self.pending = None;
                let prepared = deployment::prepare(
                    self.store.profile(id)?,
                    Path::new(&self.store.settings()?.game_root),
                )?;
                let value = json!(prepared.view);
                self.pending = Some(prepared);
                Ok(value)
            }
            Action::Apply { token, confirmed } => {
                let pending = self
                    .pending
                    .take()
                    .ok_or("Preview the changes before applying.")?;
                if pending.view.token != token {
                    return Err("This preview has expired. Preview again.".into());
                }
                let current = self.store.profile(pending.profile.id)?;
                if self.store.settings()?.game_root != paths::display(&pending.game_root) {
                    return Err("Game folder changed. Preview again.".into());
                }
                Ok(json!(deployment::apply(
                    pending,
                    &current,
                    &self.data_dir,
                    confirmed
                )?))
            }
        }
    }
}
#[tauri::command]
async fn request(
    action: Action,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<Mutex<Engine>>>,
) -> Result<Value> {
    if window.label() != "main" {
        return Err("Manager actions are available only in the main window.".into());
    }
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        state
            .lock()
            .map_err(|_| "Manager state is unavailable. Restart the app.".to_string())?
            .dispatch(action)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(startup::Startup::default());
            let data = app.path().app_local_data_dir()?;
            let engine = Engine::open(&data).map_err(std::io::Error::other)?;
            app.manage(Arc::new(Mutex::new(engine)));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                // A broken image, silent audio device or failed splash page must never hide the manager forever.
                let _ = startup::finish(&handle);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "splash" && matches!(event, tauri::WindowEvent::Destroyed) {
                let _ = startup::finish(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            request,
            startup::show_intro,
            startup::finish_intro
        ])
        .run(tauri::generate_context!())
        .expect("Could not start ConanForge Mod Manager");
}
