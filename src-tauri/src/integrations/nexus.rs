use super::{
    Progress, RemoteFile, RemoteMod, client, json_response, library, number, numeric_id, text,
};
use crate::model::Result;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Session {
    key: String,
    pub premium: bool,
    pub user_id: u64,
}
pub const API_ENABLED: bool = cfg!(feature = "nexus-testing");
pub const PUBLIC_NOTICE: &str = "Nexus API access is not enabled in this public preview. Use View source or Browse website, download there, then import the ZIP / PAK.";
fn api(key: &str, route: &str, query: &[(String, String)]) -> Result<Value> {
    if !API_ENABLED {
        return Err(PUBLIC_NOTICE.into());
    }
    let mut header =
        reqwest::header::HeaderValue::from_str(key).map_err(|_| "Invalid Nexus key")?;
    header.set_sensitive(true);
    json_response(
        client(30)?
            .get(format!("https://api.nexusmods.com/v1/{route}"))
            .header("apikey", header)
            .header("Application-Name", "Conan Exiles Enhanced Mod Manager")
            .header("Application-Version", env!("CARGO_PKG_VERSION"))
            .query(query)
            .send()
            .map_err(|_| "Could not reach Nexus Mods. Check your connection.")?,
    )
}
pub fn connect(key: String) -> Result<Session> {
    let key = key.trim().to_string();
    if key.len() < 10 || key.len() > 4096 || key.chars().any(char::is_control) {
        return Err("Enter your personal Nexus API key.".into());
    }
    let user = api(&key, "users/validate.json", &[])?;
    let user_id = number(&user, "user_id");
    if user_id == 0 {
        return Err("Nexus did not validate this account.".into());
    }
    Ok(Session {
        key,
        user_id,
        premium: user["is_premium"].as_bool().unwrap_or(false),
    })
}
pub fn map_mod(v: &Value) -> Result<RemoteMod> {
    Ok(RemoteMod {
        provider: "nexus".into(),
        id: numeric_id(&number(v, "mod_id").to_string())?,
        title: text(v, "name", 200),
        summary: text(v, "summary", 2000),
        updated: number(v, "updated_timestamp"),
        size: 0,
        files: vec![],
    })
}
pub fn lookup(session: &Session, id: &str) -> Result<RemoteMod> {
    let id = numeric_id(id)?;
    let mut result = map_mod(&api(
        &session.key,
        &format!("games/conanexilesenhanced/mods/{id}.json"),
        &[],
    )?)?;
    let files = files(session, &id)?;
    for file in files["files"]
        .as_array()
        .ok_or("Nexus returned no file list")?
        .iter()
        .take(500)
    {
        if number(file, "category_id") == 6 || number(file, "category_id") == 7 {
            continue;
        }
        result.files.push(RemoteFile {
            id: numeric_id(&number(file, "file_id").to_string())?,
            name: text(file, "file_name", 240),
            version: text(file, "version", 100),
            size: number(file, "size_in_bytes"),
        });
    }
    Ok(result)
}
pub fn files(session: &Session, id: &str) -> Result<Value> {
    let id = numeric_id(id)?;
    api(
        &session.key,
        &format!("games/conanexilesenhanced/mods/{id}/files.json"),
        &[],
    )
}
pub fn latest(session: &Session) -> Result<Vec<RemoteMod>> {
    api(
        &session.key,
        "games/conanexilesenhanced/mods/latest_added.json",
        &[],
    )?
    .as_array()
    .ok_or("Nexus returned no mod list")?
    .iter()
    .take(30)
    .map(map_mod)
    .collect()
}
pub fn nxm_query(
    input: &str,
    mod_id: &str,
    file_id: &str,
    user_id: u64,
    now: u64,
) -> Result<Vec<(String, String)>> {
    if input.len() > 8192 {
        return Err("Authorization link is too long.".into());
    }
    let u=reqwest::Url::parse(input).map_err(|_| "Paste the nxm:// download link issued by Nexus, or download in your browser and import the archive.")?;
    if u.scheme() != "nxm"
        || u.host_str() != Some("conanexilesenhanced")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
        || u.path() != format!("/mods/{mod_id}/files/{file_id}")
    {
        return Err("The authorization link is for a different game or file.".into());
    }
    let mut fields = HashMap::new();
    for (k, v) in u.query_pairs() {
        if fields.insert(k.into_owned(), v.into_owned()).is_some() {
            return Err("Duplicate authorization fields.".into());
        }
    }
    if fields.get("user_id").and_then(|x| x.parse::<u64>().ok()) != Some(user_id)
        || fields
            .get("expires")
            .and_then(|x| x.parse::<u64>().ok())
            .unwrap_or(0)
            <= now
        || fields
            .get("key")
            .is_none_or(|k| k.is_empty() || k.len() > 4096)
    {
        return Err("Authorization is expired or belongs to another Nexus account. Get a new website download link.".into());
    }
    Ok(["key", "expires"]
        .iter()
        .map(|k| (k.to_string(), fields[*k].clone()))
        .collect())
}
pub fn cdn_url(value: &str) -> Result<reqwest::Url> {
    let u = reqwest::Url::parse(value).map_err(|_| "Invalid Nexus download URL")?;
    let host = u.host_str().unwrap_or("");
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
        || !(host.ends_with(".nexusmods.com") || host.ends_with(".nexus-cdn.com"))
    {
        return Err("Nexus returned an unrecognized download host. Download through the website and import the file instead.".into());
    }
    Ok(u)
}
pub fn transfer(url: &str, dest: &Path, progress: &Progress) -> Result<()> {
    let mut url = cdn_url(url)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "Could not start transfer")?;
    runtime.block_on(async {
        let client=reqwest::Client::builder().user_agent(concat!("ConanModManager/",env!("CARGO_PKG_VERSION"))).connect_timeout(Duration::from_secs(15)).read_timeout(Duration::from_secs(30)).timeout(Duration::from_secs(1800)).redirect(reqwest::redirect::Policy::none()).build().map_err(|_| "Could not initialize HTTPS")?;
        let mut response=None;
        for _ in 0..6 {
            progress.check()?;
            let r=client.get(url.clone()).send().await.map_err(|_| "Download connection failed or timed out")?;
            if r.status().is_redirection() {
                url=cdn_url(url.join(r.headers().get("location").and_then(|v|v.to_str().ok()).ok_or("Missing download redirect")?).map_err(|_| "Invalid download redirect")?.as_str())?;
            } else { response=Some(r); break; }
        }
        let mut r=response.ok_or("Too many download redirects")?;
        if !r.status().is_success() { return Err("Download was denied or expired. Obtain a new authorized link.".into()); }
        let expected=r.content_length();
        if expected.is_some_and(|n| n>library::MAX_BYTES) { return Err("Download exceeds 16 GiB. Use a local source folder instead.".into()); }
        let mut out=File::create_new(dest).map_err(|_| "Could not create download file")?;
        let mut total=0;
        loop {
            progress.check()?;
            let next=r.chunk();
            tokio::pin!(next);
            let bytes=loop { tokio::select! {
                result=&mut next => break result.map_err(|_| "Download interrupted or timed out")?,
                _=tokio::time::sleep(Duration::from_millis(200)) => progress.check()?,
            }};
            let Some(bytes)=bytes else {break};
            total+=bytes.len() as u64;
            if total>library::MAX_BYTES { return Err("Download exceeds 16 GiB.".into()); }
            out.write_all(&bytes).map_err(|_| "Could not save download. Check free disk space.")?;
            progress.report("Downloading from Nexus…",total);
        }
        if expected.is_some_and(|n| n!=total) { return Err("Download size mismatch.".into()); }
        out.sync_all().map_err(|e|e.to_string())
    })
}
pub fn download(
    data: &Path,
    session: &Session,
    mod_id: &str,
    file_id: &str,
    nxm: &str,
    progress: &Progress,
) -> Result<String> {
    let detail = api(
        &session.key,
        &format!("games/conanexilesenhanced/mods/{mod_id}/files/{file_id}.json"),
        &[],
    )?;
    let name = text(&detail, "file_name", 240);
    let relative = library::archive_path(&name)?;
    if relative.components().count() != 1
        || !relative
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip") || e.eq_ignore_ascii_case("pak"))
    {
        return Err("This download format needs manual extraction. Download in your browser, extract 7z/RAR, then import the .pak.".into());
    }
    let query = if nxm.trim().is_empty() && session.premium {
        vec![]
    } else {
        nxm_query(
            nxm.trim(),
            mod_id,
            file_id,
            session.user_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs(),
        )?
    };
    progress.check()?;
    let links = api(
        &session.key,
        &format!("games/conanexilesenhanced/mods/{mod_id}/files/{file_id}/download_link.json"),
        &query,
    )?;
    let url = links
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v["URI"].as_str())
        .ok_or("Nexus returned no authorized download")?;
    let temp = library::staging(data)?;
    let source = temp.path().join(relative);
    transfer(url, &source, progress)?;
    library::import(data, &source, "nexus", mod_id, file_id, progress)
}
