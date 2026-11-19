pub mod library;
pub mod maintenance;
pub mod nexus;
pub mod steamcmd;
pub mod steamcmd_setup;
pub mod updates;

use crate::{model::Result, storage::Store};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMod {
    pub provider: String,
    pub id: String,
    pub title: String,
    pub summary: String,
    pub updated: u64,
    pub size: u64,
    pub files: Vec<RemoteFile>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFile {
    pub id: String,
    pub name: String,
    pub version: String,
    pub size: u64,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OnlineAction {
    Status,
    CheckUpdate {
        digest: String,
    },
    Connect {
        key: String,
    },
    Disconnect,
    Lookup {
        provider: String,
        input: String,
    },
    Latest,
    OpenSite {
        site: String,
        id: Option<String>,
    },
    Import {
        path: String,
    },
    DownloadNexus {
        mod_id: String,
        file_id: String,
        nxm: String,
    },
    DownloadSteam {
        id: String,
    },
    Cancel,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub running: bool,
    pub stage: String,
    pub bytes: u64,
    pub error: Option<String>,
    pub imported: Option<String>,
}
#[derive(Clone)]
pub struct Progress {
    pub job: Arc<Mutex<Job>>,
    pub cancel: Arc<AtomicBool>,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            job: Arc::new(Mutex::new(Job::default())),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl Progress {
    pub fn check(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("Cancelled. No profile or game files were changed.".into())
        } else {
            Ok(())
        }
    }
    pub fn report(&self, stage: &str, bytes: u64) {
        if let Ok(mut j) = self.job.lock() {
            j.stage = stage.into();
            j.bytes = bytes;
        }
    }
}
pub struct Service {
    data: PathBuf,
    nexus: Option<nexus::Session>,
    progress: Progress,
}
impl Service {
    pub fn new(data: &Path) -> Self {
        Self {
            data: data.into(),
            nexus: None,
            progress: Progress::default(),
        }
    }
    pub fn dispatch(&mut self, action: OnlineAction) -> Result<Value> {
        match action {
            OnlineAction::Status => self.snapshot(),
            OnlineAction::CheckUpdate { digest } => {
                let store = Store::open(&self.data.join("manager.sqlite3"))?;
                let record = maintenance::acquisition(&store, &digest)?;
                Ok(json!(updates::check(&record, self.nexus.as_ref())))
            }
            OnlineAction::Connect { key } => {
                self.nexus = None;
                self.nexus = Some(nexus::connect(key)?);
                self.snapshot()
            }
            OnlineAction::Disconnect => {
                self.nexus = None;
                self.snapshot()
            }
            OnlineAction::Lookup { provider, input } => match provider.as_str() {
                "steam" => Ok(json!(steamcmd::lookup(&provider_id("steam", &input)?)?)),
                "nexus" => Ok(json!(nexus::lookup(
                    self.session()?,
                    &provider_id("nexus", &input)?
                )?)),
                _ => Err("Choose Steam Workshop or Nexus Mods.".into()),
            },
            OnlineAction::Latest => Ok(json!(nexus::latest(self.session()?)?)),
            OnlineAction::OpenSite { site, id } => {
                open_site(&site_url(&site, id.as_deref())?)?;
                Ok(Value::Null)
            }
            OnlineAction::Import { path } => self.start(move |data, progress| {
                library::import(data, Path::new(&path), "local", "", "", progress)
            }),
            OnlineAction::DownloadNexus {
                mod_id,
                file_id,
                nxm,
            } => {
                let session = self.session()?.clone();
                let mod_id = numeric_id(&mod_id)?;
                let file_id = numeric_id(&file_id)?;
                self.start(move |data, progress| {
                    nexus::download(data, &session, &mod_id, &file_id, &nxm, progress)
                })
            }
            OnlineAction::DownloadSteam { id } => {
                let id = numeric_id(&id)?;
                self.start(move |data, progress| steamcmd::download(data, &id, progress))
            }
            OnlineAction::Cancel => {
                self.progress.cancel.store(true, Ordering::Relaxed);
                self.snapshot()
            }
        }
    }
    fn session(&self) -> Result<&nexus::Session> {
        if !nexus::API_ENABLED {
            return Err(nexus::PUBLIC_NOTICE.into());
        }
        self.nexus
            .as_ref()
            .ok_or("Connect a personal Nexus API key for this session first.".into())
    }
    pub fn ensure_idle(&self) -> Result<()> {
        if self
            .progress
            .job
            .lock()
            .map_err(|_| "Download status unavailable")?
            .running
        {
            return Err(
                "Wait for the current import or download to finish before deleting library files."
                    .into(),
            );
        }
        Ok(())
    }
    fn snapshot(&self) -> Result<Value> {
        Ok(
            json!({"nexusApiEnabled":nexus::API_ENABLED, "connected":self.nexus.is_some(), "premium":self.nexus.as_ref().is_some_and(|s| s.premium), "job":self.progress.job.lock().map_err(|_| "Download status unavailable")?.clone(), "history":Store::open(&self.data.join("manager.sqlite3"))?.acquisitions()?}),
        )
    }
    fn start(
        &mut self,
        work: impl FnOnce(&Path, &Progress) -> Result<String> + Send + 'static,
    ) -> Result<Value> {
        {
            let mut job = self
                .progress
                .job
                .lock()
                .map_err(|_| "Download status unavailable")?;
            if job.running {
                return Err("Wait for the current transfer to finish or cancel it.".into());
            }
            *job = Job {
                running: true,
                stage: "Preparing…".into(),
                ..Default::default()
            };
            self.progress.cancel.store(false, Ordering::Relaxed);
        }
        let progress = self.progress.clone();
        let data = self.data.clone();
        // Keep long transfers outside Engine's database mutex so cancel/status remain responsive.
        std::thread::spawn(move || {
            let result = work(&data, &progress);
            if let Ok(mut job) = progress.job.lock() {
                job.running = false;
                match result {
                    Ok(path) => {
                        job.stage = "Added to managed library".into();
                        job.imported = Some(path);
                    }
                    Err(e) => {
                        job.stage = "Transfer stopped".into();
                        job.error = Some(e);
                    }
                }
            }
        });
        self.snapshot()
    }
}
pub fn numeric_id(input: &str) -> Result<String> {
    if input.is_empty()
        || input.len() > 20
        || !input.bytes().all(|c| c.is_ascii_digit())
        || input.parse::<u64>().unwrap_or(0) == 0
    {
        return Err("Enter a valid positive mod ID.".into());
    }
    Ok(input.parse::<u64>().map_err(|_| "Invalid ID")?.to_string())
}
pub fn provider_id(provider: &str, input: &str) -> Result<String> {
    let input = input.trim();
    if let Ok(id) = numeric_id(input) {
        return Ok(id);
    }
    let u = reqwest::Url::parse(input).map_err(|_| "Enter a mod ID or its official page URL.")?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
    {
        return Err("Use an official HTTPS mod page URL.".into());
    }
    match (provider, u.host_str()) {
        ("steam", Some("steamcommunity.com"))
            if matches!(
                u.path(),
                "/sharedfiles/filedetails/" | "/sharedfiles/filedetails"
            ) =>
        {
            numeric_id(
                &u.query_pairs()
                    .find(|(k, _)| k == "id")
                    .ok_or("Missing Workshop ID")?
                    .1,
            )
        }
        ("nexus", Some("www.nexusmods.com" | "nexusmods.com")) => {
            let parts: Vec<_> = u.path().trim_matches('/').split('/').collect();
            if parts.len() == 3 && parts[0] == "conanexilesenhanced" && parts[1] == "mods" {
                numeric_id(parts[2])
            } else {
                Err("Choose a Conan Exiles Enhanced Nexus mod page. The legacy catalog uses different mod IDs.".into())
            }
        }
        _ => Err("This URL does not belong to the selected provider.".into()),
    }
}
pub fn client(seconds: u64) -> Result<Client> {
    Client::builder()
        .user_agent(concat!("ConanModManager/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize HTTPS.".into())
}
pub fn json_response(mut response: Response) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => "Provider denied access. Check your key, account permissions, and website download authorization.".into(),
            429 => "Provider rate limit reached. Wait before trying again.".into(),
            _ => format!("Provider returned HTTP {}. Try again later.", status.as_u16()),
        });
    }
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(2_097_153)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read provider response")?;
    if bytes.len() > 2_097_152 {
        return Err("Provider response exceeded the safety limit.".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Provider returned invalid JSON.".into())
}
pub fn text(value: &Value, field: &str, max: usize) -> String {
    value[field]
        .as_str()
        .unwrap_or("")
        .chars()
        .take(max)
        .collect()
}
pub fn number(value: &Value, field: &str) -> u64 {
    value[field]
        .as_u64()
        .or_else(|| value[field].as_str()?.parse().ok())
        .unwrap_or(0)
}
pub fn site_url(site: &str, id: Option<&str>) -> Result<String> {
    let id = id.map(numeric_id).transpose()?;
    Ok(match (site, id) {
        ("steam", Some(id)) => {
            format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}")
        }
        ("steam", None) => "https://steamcommunity.com/app/440900/workshop/".into(),
        ("nexus", Some(id)) => {
            format!("https://www.nexusmods.com/conanexilesenhanced/mods/{id}?tab=files")
        }
        ("nexus", None) => "https://www.nexusmods.com/conanexilesenhanced/mods/".into(),
        ("nexus_key", None) => "https://www.nexusmods.com/users/myaccount?tab=api%20access".into(),
        ("steamcmd", None) => "https://developer.valvesoftware.com/wiki/SteamCMD".into(),
        _ => return Err("Unknown provider page.".into()),
    })
}
fn open_site(url: &str) -> Result<()> {
    #[cfg(windows)]
    {
        let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let result = unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            return Err("Could not open your default browser.".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err("Opening provider pages is supported by the Windows build.".into())
    }
}
pub fn status() -> Value {
    json!([
        {"name":"Steam Workshop", "available":true, "detail":"Public item lookup; optional isolated SteamCMD anonymous downloads. Some items require Steam account ownership."},
        {"name":"Nexus Mods", "available":nexus::API_ENABLED, "detail":if nexus::API_ENABLED {"Personal API testing build; not for public distribution."} else {"Browse Nexus in your browser and import downloaded ZIP/PAK files. Direct API features await registration."}}
    ])
}
