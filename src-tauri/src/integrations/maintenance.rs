use super::{Progress, library};
use crate::{model::Result, paths, scanner, storage::Store};
use rusqlite::{Transaction, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::SystemTime};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePreview {
    pub token: String,
    pub digest: String,
    pub folder: String,
    pub packages: Vec<String>,
    pub bytes: u64,
    pub file_count: usize,
    pub profiles: Vec<String>,
}

pub fn acquisition(store: &Store, digest: &str) -> Result<library::Acquisition> {
    store
        .acquisitions()?
        .into_iter()
        .find(|a| a.digest == digest)
        .ok_or("This managed import no longer exists. Scan the library again.".into())
}

pub fn contains(folder: &str, path: &str) -> bool {
    let prefix = paths::key(Path::new(folder))
        .trim_end_matches('/')
        .to_owned()
        + "/";
    paths::key(Path::new(path)).starts_with(&prefix)
        || fs::canonicalize(path).is_ok_and(|resolved| paths::key(&resolved).starts_with(&prefix))
}

fn owned_folder(data: &Path, record: &library::Acquisition) -> Result<()> {
    let folder = Path::new(&record.path);
    let root = data.join("library");
    paths::writable_path(&root)?;
    paths::writable_path(folder)?;
    if !folder.is_dir()
        || paths::key(folder.parent().ok_or("Missing library parent")?) != paths::key(&root)
        || !folder
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("transfer-"))
        || fs::canonicalize(folder)
            .map_err(|e| e.to_string())?
            .parent()
            != Some(
                fs::canonicalize(&root)
                    .map_err(|e| e.to_string())?
                    .as_path(),
            )
    {
        return Err("Only an app-owned imported bundle can be deleted here.".into());
    }
    validate_contents(folder)
}

fn validate_contents(folder: &Path) -> Result<()> {
    let mut pending = vec![(folder.to_path_buf(), 0)];
    let mut count = 0;
    while let Some((dir, depth)) = pending.pop() {
        if depth > 24 {
            return Err("Bundle exceeds the folder depth limit.".into());
        }
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            count += 1;
            if count > 100_000 {
                return Err("Bundle contains too many entries.".into());
            }
            let path = entry.map_err(|e| e.to_string())?.path();
            let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            paths::writable_path(&path)?;
            if paths::linked(&meta) {
                return Err("Linked files cannot be deleted by the manager.".into());
            }
            if meta.is_dir() {
                pending.push((path, depth + 1));
            } else if !meta.is_file() || !library::mod_file(&path) {
                return Err("This bundle contains unexpected files. Nothing was deleted.".into());
            }
        }
    }
    Ok(())
}

pub fn preview(data: &Path, store: &Store, digest: &str) -> Result<DeletePreview> {
    let record = acquisition(store, digest)?;
    owned_folder(data, &record)?;
    if library::fingerprint(Path::new(&record.path), &Progress::default())? != record.digest {
        return Err("The managed files changed since import. Nothing was deleted.".into());
    }
    let scan = scanner::scan(std::slice::from_ref(&record.path))?;
    let mut profiles: Vec<String> = store
        .profiles()?
        .into_iter()
        .filter(|p| p.entries.iter().any(|e| contains(&record.path, &e.path)))
        .map(|p| p.name)
        .collect();
    let game = store.settings()?.game_root;
    if !game.is_empty() {
        let mods = Path::new(&game).join("ConanSandbox/Mods");
        let list = mods.join("modlist.txt");
        if list.exists()
            && crate::modlist::parse(&crate::deployment::read_list(&list)?, &mods)?
                .iter()
                .any(|e| contains(&record.path, &e.path))
        {
            profiles.push("Current game load list (direct library reference)".into());
        }
    }
    Ok(DeletePreview {
        token: format!(
            "{:x}",
            Sha256::digest(format!("{}:{:?}", record.digest, SystemTime::now()))
        ),
        digest: record.digest,
        folder: record.path,
        bytes: scan
            .mods
            .iter()
            .flat_map(|m| &m.files)
            .map(|f| f.size)
            .sum(),
        file_count: scan.mods.iter().map(|m| m.files.len()).sum(),
        packages: scan.mods.into_iter().map(|m| m.name).collect(),
        profiles,
    })
}

pub fn delete(
    data: &Path,
    store: &Store,
    prepared: DeletePreview,
    confirmed: bool,
) -> Result<String> {
    if !confirmed {
        return Err("Confirm deletion after reviewing the imported bundle.".into());
    }
    let tx = Transaction::new_unchecked(&store.connection, TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let current = preview(data, store, &prepared.digest)?;
    if current.folder != prepared.folder {
        return Err("The library folder changed. Review again.".into());
    }
    if !current.profiles.is_empty() {
        return Err(format!(
            "Remove this bundle from these profiles and save first: {}. Disabled entries also count.",
            current.profiles.join(", ")
        ));
    }
    // Keep the files recoverable until the database transaction commits.
    let staging = library::staging(data)?;
    let parked = staging.path().join("deleted-bundle");
    fs::rename(&current.folder, &parked)
        .map_err(|e| format!("Could not remove the library copy: {e}"))?;
    let committed = (|| {
        validate_contents(&parked)?;
        if library::fingerprint(&parked, &Progress::default())? != prepared.digest {
            return Err("Files changed during deletion; the bundle was restored.".into());
        }
        if tx
            .execute(
                "DELETE FROM acquisitions WHERE digest=?1",
                [&prepared.digest],
            )
            .map_err(|e| e.to_string())?
            != 1
        {
            return Err("Library record changed. Review again.".into());
        }
        store.put("catalog", &crate::model::Scan::default())?;
        tx.commit().map_err(|e| e.to_string())
    })();
    if let Err(error) = committed {
        if fs::rename(&parked, &current.folder).is_err() {
            let retained = staging.keep();
            return Err(format!(
                "Deletion failed; files retained for recovery at {}: {error}",
                paths::display(&retained)
            ));
        }
        return Err(error);
    }
    let retained = paths::display(staging.path());
    match staging.close() {
        Ok(()) => Ok("Library copy deleted. Copies already installed in the game remain; use Load order and Apply to deactivate them.".into()),
        Err(_) => Ok(format!("Removed from the library, but Windows could not clear retained files at {retained}. Game files remain unchanged.")),
    }
}
