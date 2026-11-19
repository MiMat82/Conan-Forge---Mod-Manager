use super::Progress;
use crate::{model::Result, paths, scanner, storage::Store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_BYTES: u64 = 16 * 1024 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Acquisition {
    pub digest: String,
    pub path: String,
    pub provider: String,
    pub mod_id: String,
    pub file_id: String,
    pub name: String,
    pub created: u64,
}
pub fn staging(data: &Path) -> Result<tempfile::TempDir> {
    let root = data.join("staging");
    paths::writable_path(&root)?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    tempfile::Builder::new()
        .prefix("transfer-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())
}
pub fn mod_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| {
        ["pak", "ucas", "utoc"]
            .iter()
            .any(|ext| e.eq_ignore_ascii_case(ext))
    })
}
pub fn archive_path(name: &str) -> Result<PathBuf> {
    let normalized = name.replace('\\', "/");
    if normalized.len() > 240 || normalized.starts_with('/') {
        return Err("Archive contains an unsafe path.".into());
    }
    let parts: Vec<_> = normalized.trim_end_matches('/').split('/').collect();
    if parts.len() > 24
        || parts.iter().any(|p| {
            p.is_empty()
                || matches!(*p, "." | "..")
                || p.contains([':', '*', '?', '"', '<', '>', '|'])
                || p.ends_with([' ', '.'])
                || p.chars().any(char::is_control)
        })
    {
        return Err("Archive contains an unsafe path.".into());
    }
    Ok(parts.iter().collect())
}
pub fn copy_bounded(
    reader: &mut impl Read,
    writer: &mut impl Write,
    progress: &Progress,
    total: &mut u64,
) -> Result<()> {
    let mut buffer = [0u8; 128 * 1024];
    loop {
        progress.check()?;
        let n = reader
            .read(&mut buffer)
            .map_err(|_| "Could not read package data")?;
        if n == 0 {
            break;
        }
        *total = total.checked_add(n as u64).ok_or("Transfer too large")?;
        if *total > MAX_BYTES {
            return Err("Transfer exceeds the 16 GiB safety limit. Use a local source folder for larger mods.".into());
        }
        writer
            .write_all(&buffer[..n])
            .map_err(|_| "Could not write package data. Check free disk space.")?;
        progress.report("Copying package data…", *total);
    }
    Ok(())
}
pub fn extract_zip(source: &Path, destination: &Path, progress: &Progress) -> Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(source).map_err(|e| e.to_string())?).map_err(|_| "Not a supported ZIP archive. Extract 7z/RAR archives manually and import their .pak files.")?;
    if zip.len() > 10_000 {
        return Err("Archive has more than 10,000 entries.".into());
    }
    let mut seen = HashSet::new();
    let mut total = 0;
    let mut declared = 0u64;
    for i in 0..zip.len() {
        progress.check()?;
        let mut item = zip
            .by_index(i)
            .map_err(|_| "Encrypted or damaged ZIP entry")?;
        let relative = archive_path(item.name())?;
        if item
            .unix_mode()
            .is_some_and(|m| !matches!(m & 0o170000, 0 | 0o100000 | 0o040000))
        {
            return Err("Archive links and special files are not allowed.".into());
        }
        let dest = destination.join(&relative);
        paths::writable_path(&dest)?;
        if !seen.insert(paths::key(&relative)) {
            return Err("Archive contains duplicate or case-colliding paths.".into());
        }
        if item.is_dir() || !mod_file(&relative) {
            continue;
        }
        declared = declared
            .checked_add(item.size())
            .ok_or("Archive too large")?;
        if declared > MAX_BYTES {
            return Err("Expanded package data exceeds 16 GiB.".into());
        }
        fs::create_dir_all(dest.parent().ok_or("Missing package parent")?)
            .map_err(|e| e.to_string())?;
        let mut out = File::create_new(&dest).map_err(|e| e.to_string())?;
        let before = total;
        copy_bounded(&mut item, &mut out, progress, &mut total)?;
        if total - before != item.size() {
            return Err("ZIP entry size did not match its header.".into());
        }
        out.sync_all().map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn copy_file(source: &Path, dest: &Path, progress: &Progress, total: &mut u64) -> Result<()> {
    if paths::linked(&fs::symlink_metadata(source).map_err(|e| e.to_string())?) {
        return Err("Linked mod files cannot be imported.".into());
    }
    paths::writable_path(dest)?;
    fs::create_dir_all(dest.parent().ok_or("Missing package folder")?)
        .map_err(|e| e.to_string())?;
    let mut out = File::create_new(dest).map_err(|e| e.to_string())?;
    copy_bounded(
        &mut File::open(source).map_err(|e| e.to_string())?,
        &mut out,
        progress,
        total,
    )?;
    out.sync_all().map_err(|e| e.to_string())
}
pub fn fingerprint(root: &Path, progress: &Progress) -> Result<String> {
    let scan = scanner::scan(&[paths::display(root)])?;
    if scan.mods.is_empty()
        || !scan.warnings.is_empty()
        || !scan.orphan_files.is_empty()
        || scan.mods.iter().any(|m| !m.warnings.is_empty())
    {
        return Err("Package is incomplete, empty, linked, or contains no .pak files. Check its .ucas/.utoc companions.".into());
    }
    let mut files: Vec<_> = scan.mods.iter().flat_map(|m| m.files.iter()).collect();
    files.sort_by_key(|f| paths::key(Path::new(&f.path)));
    let canonical = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    for file in files {
        let resolved = fs::canonicalize(&file.path).map_err(|e| e.to_string())?;
        let relative = resolved
            .strip_prefix(&canonical)
            .map_err(|_| "Package escaped its folder")?;
        digest.update(paths::key(relative).as_bytes());
        digest.update([0]);
        digest.update(file.size.to_le_bytes());
        let mut f = File::open(resolved).map_err(|e| e.to_string())?;
        loop {
            progress.check()?;
            let n = f.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            digest.update(&buffer[..n]);
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}
pub fn import(
    data: &Path,
    source: &Path,
    provider: &str,
    mod_id: &str,
    file_id: &str,
    progress: &Progress,
) -> Result<String> {
    paths::absolute(source)?;
    progress.check()?;
    let temp = staging(data)?;
    let content = temp.path().join("content");
    fs::create_dir(&content).map_err(|e| e.to_string())?;
    let mut total = 0;
    if source.is_dir() {
        let scan = scanner::scan(&[paths::display(source)])?;
        if !scan.warnings.is_empty()
            || !scan.orphan_files.is_empty()
            || scan.mods.iter().any(|m| !m.warnings.is_empty())
        {
            return Err("Source contains incomplete packages or unreadable/linked files.".into());
        }
        let root = fs::canonicalize(source).map_err(|e| e.to_string())?;
        for file in scan.mods.iter().flat_map(|m| &m.files) {
            let p = fs::canonicalize(&file.path).map_err(|e| e.to_string())?;
            copy_file(
                &p,
                &content.join(
                    p.strip_prefix(&root)
                        .map_err(|_| "Source escaped its folder")?,
                ),
                progress,
                &mut total,
            )?;
        }
    } else if source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        extract_zip(source, &content, progress)?;
    } else if source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pak"))
    {
        let package = scanner::package(source)?;
        if !package.warnings.is_empty() {
            return Err(package.warnings.join(" "));
        }
        for f in package.files {
            let p = Path::new(&f.path);
            copy_file(
                p,
                &content.join(p.file_name().ok_or("Missing filename")?),
                progress,
                &mut total,
            )?;
        }
    } else {
        return Err(
            "Choose a ZIP archive or .pak file. Extract 7z/RAR files manually first.".into(),
        );
    }
    progress.report("Verifying package fingerprints…", total);
    let digest = fingerprint(&content, progress)?;
    let store = Store::open(&data.join("manager.sqlite3"))?;
    if let Some(mut previous) = store
        .acquisitions()?
        .into_iter()
        .find(|a| a.digest == digest)
    {
        if fingerprint(Path::new(&previous.path), progress)? != digest {
            return Err("The existing managed copy was changed. Restore it before importing this duplicate.".into());
        }
        if matches!(provider, "steam" | "nexus")
            && (previous.provider == "local"
                || (previous.provider == provider && previous.mod_id == mod_id))
        {
            previous.provider = provider.into();
            previous.mod_id = mod_id.into();
            previous.file_id = file_id.into();
            store.refresh_acquisition(&previous)?;
        }
        return Ok(previous.path);
    }
    progress.check()?;
    let library = data.join("library");
    paths::writable_path(&library)?;
    fs::create_dir_all(&library).map_err(|e| e.to_string())?;
    let dest = library.join(temp.path().file_name().ok_or("Missing staging name")?);
    paths::writable_path(&dest)?;
    fs::rename(&content, &dest).map_err(|e| e.to_string())?;
    let record = Acquisition {
        digest,
        path: paths::display(&dest),
        provider: provider.into(),
        mod_id: mod_id.into(),
        file_id: file_id.into(),
        name: source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        created: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs(),
    };
    store.add_acquisition(&record).map_err(|e| {
        format!(
            "Files were retained at {} but history could not be saved: {e}",
            record.path
        )
    })?;
    Ok(record.path)
}
