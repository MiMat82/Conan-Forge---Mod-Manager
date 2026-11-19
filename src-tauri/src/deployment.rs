use crate::{
    model::{FileStamp, Profile, Result},
    modlist, paths, scanner,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: String,
    pub target: String,
    pub profile_name: String,
    pub before: String,
    pub after: String,
    pub changed: bool,
    pub warnings: Vec<String>,
    pub enabled_count: usize,
    pub files: Vec<PackageCopy>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageCopy {
    pub source: String,
    pub target: String,
    pub size: u64,
    pub sha256: String,
    pub copy: bool,
}
#[derive(Debug)]
pub struct Prepared {
    pub view: Preview,
    pub profile: Profile,
    pub game_root: PathBuf,
    pub before: Option<Vec<u8>>,
    pub sources: Vec<FileStamp>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub target: String,
    pub backup_dir: Option<String>,
    pub changed: bool,
    pub message: String,
}

pub fn read_list(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", paths::display(path)))?;
    let mut bytes = Vec::new();
    file.take((modlist::MAX_LIST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > modlist::MAX_LIST_BYTES {
        return Err("modlist.txt exceeds 1 MiB.".into());
    }
    Ok(bytes)
}
fn existing(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if paths::linked(&meta) || !meta.is_file() {
                return Err("modlist.txt must be an ordinary, unlinked file.".into());
            }
            read_list(path).map(Some)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn sources(profile: &Profile) -> Result<Vec<FileStamp>> {
    let mut seen = HashSet::new();
    let mut stamps = Vec::new();
    for entry in profile.entries.iter().filter(|e| e.enabled) {
        let package = scanner::package(Path::new(&entry.path))?;
        if !seen.insert(paths::key(Path::new(&package.path))) {
            return Err("Two enabled paths resolve to the same mod. Remove the duplicate.".into());
        }
        if !package.warnings.is_empty() {
            return Err(format!("{}: {}", package.name, package.warnings.join(" ")));
        }
        stamps.extend(package.files);
    }
    Ok(stamps)
}
fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", paths::display(path)))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn destination_hash(path: &Path) -> Result<Option<String>> {
    paths::writable_path(path)?;
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !paths::linked(&meta) => hash_file(path).map(Some),
        Ok(_) => Err(format!(
            "Not an ordinary mod file: {}",
            paths::display(path)
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn package_copies(sources: &[FileStamp], folder: &Path) -> Result<Vec<PackageCopy>> {
    let mut names = HashSet::new();
    let mut files = Vec::new();
    for source in sources {
        let path = Path::new(&source.path);
        let target = folder.join(path.file_name().ok_or("Missing mod filename")?);
        if !names.insert(paths::key(&target)) {
            return Err(format!(
                "Two enabled mods use the same filename: {}. Keep only one version enabled.",
                paths::display(&target)
            ));
        }
        let sha256 = hash_file(path)?;
        let before = destination_hash(&target)?;
        if before.as_ref().is_some_and(|hash| hash != &sha256) {
            return Err(format!(
                "{} already contains different content. Existing mod files are never overwritten. Resolve this file conflict before applying.",
                paths::display(&target)
            ));
        }
        files.push(PackageCopy {
            source: source.path.clone(),
            target: paths::display(&target),
            size: source.size,
            sha256,
            copy: before.is_none(),
        });
    }
    check_extra_companions(&files)?;
    Ok(files)
}
fn check_extra_companions(files: &[PackageCopy]) -> Result<()> {
    let names: HashSet<_> = files
        .iter()
        .map(|f| paths::key(Path::new(&f.target)))
        .collect();
    for file in files.iter().filter(|f| {
        Path::new(&f.target)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pak"))
    }) {
        for extension in ["ucas", "utoc"] {
            let companion = Path::new(&file.target).with_extension(extension);
            paths::writable_path(&companion)?;
            if companion.try_exists().map_err(|e| e.to_string())?
                && !names.contains(&paths::key(&companion))
            {
                return Err(format!(
                    "Unexpected companion already in the game Mods folder: {}. Resolve this file conflict before applying.",
                    paths::display(&companion)
                ));
            }
        }
    }
    Ok(())
}
fn check_files(plan: &Prepared, installed: bool) -> Result<()> {
    if sources(&plan.profile)? != plan.sources {
        return Err("Mod files changed since preview. Scan and preview again.".into());
    }
    check_extra_companions(&plan.view.files)?;
    for file in &plan.view.files {
        let expected = if file.copy && !installed {
            None
        } else {
            Some(file.sha256.clone())
        };
        if hash_file(Path::new(&file.source))? != file.sha256
            || destination_hash(Path::new(&file.target))? != expected
        {
            return Err(format!(
                "Mod file changed since preview: {}. Preview again.",
                file.target
            ));
        }
    }
    Ok(())
}
pub fn prepare(profile: Profile, game_root: &Path) -> Result<Prepared> {
    let target = paths::game_target(game_root)?;
    modlist::validate(&profile.entries)?;
    let sources = sources(&profile)?;
    let files = package_copies(&sources, target.parent().ok_or("Missing Mods folder")?)?;
    let mut after = String::new();
    for entry in profile.entries.iter().filter(|e| e.enabled) {
        let resolved = fs::canonicalize(&entry.path).map_err(|e| e.to_string())?;
        let name = resolved
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid mod filename")?;
        after.push_str(&format!("*{name}\r\n"));
    }
    if after.len() > modlist::MAX_LIST_BYTES {
        return Err("Generated modlist exceeds 1 MiB.".into());
    }
    let before = existing(&target)?;
    let mut warnings = vec![
        "Package format does not prove Enhanced compatibility. Check the mod author's supported game version.".into(),
        "Enabled packages are copied into the game's Mods folder. Identical files are reused; conflicting files block apply. Source files stay untouched.".into(),
        "Disabled and unrelated packages are not removed. Restart the game after applying to refresh its Mods menu.".into(),
        "This backs up the load list only. Back up your world saves before changing mods.".into(),
    ];
    if after.is_empty() {
        warnings.push("This profile disables ALL mods. Loading an existing modded world may remove its content.".into());
    }
    let token = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}:{}:{}:{:?}",
            paths::display(&target),
            profile.id,
            profile.revision,
            SystemTime::now()
        ))
    );
    let view = Preview {
        token,
        target: paths::display(&target),
        profile_name: profile.name.clone(),
        before: before
            .as_ref()
            .map(|b| modlist::decode(b).unwrap_or_else(|_| String::from_utf8_lossy(b).into_owned()))
            .unwrap_or_default(),
        changed: before.as_deref() != Some(after.as_bytes()) || files.iter().any(|f| f.copy),
        after,
        warnings,
        enabled_count: profile.entries.iter().filter(|e| e.enabled).count(),
        files,
    };
    Ok(Prepared {
        view,
        profile,
        game_root: game_root.to_path_buf(),
        before,
        sources,
    })
}
pub fn is_game_process(name: &str) -> bool {
    let name = name.to_lowercase();
    name.starts_with("conansandbox")
        || matches!(
            name.as_str(),
            "conanexiles.exe" | "conanexilesenhanced.exe" | "conanexilesdedicatedserver.exe"
        )
}
pub fn ensure_game_closed() -> Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, ERROR_NO_MORE_FILES, GetLastError, INVALID_HANDLE_VALUE},
            System::Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
        };
        // Toolhelp only inspects process names; it never terminates the game.
        unsafe {
            let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if handle == INVALID_HANDLE_VALUE {
                return Err("Cannot inspect running games. Apply is blocked.".into());
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(handle, &mut entry);
            while ok != 0 {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                if is_game_process(&name) {
                    CloseHandle(handle);
                    return Err(format!("Close {name} before applying mods."));
                }
                ok = Process32NextW(handle, &mut entry);
            }
            let last = GetLastError();
            CloseHandle(handle);
            if last != ERROR_NO_MORE_FILES {
                return Err("Process inspection failed. Apply is blocked.".into());
            }
        }
    }
    Ok(())
}
fn new_file(path: &Path, bytes: &[u8]) -> Result<()> {
    paths::writable_path(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", paths::display(path)))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
struct ApplyLock {
    path: PathBuf,
    _file: File,
}
impl Drop for ApplyLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
fn replace_atomic(stage: &Path, target: &Path, had_original: bool) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_WRITE_THROUGH, MoveFileExW, ReplaceFileW,
        };
        let from: Vec<u16> = stage.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = unsafe {
            if had_original {
                ReplaceFileW(
                    to.as_ptr(),
                    from.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                )
            } else {
                MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH)
            }
        };
        if ok == 0 {
            return Err(format!(
                "Atomic write failed: {}. The backup is retained.",
                std::io::Error::last_os_error()
            ));
        }
    }
    #[cfg(not(windows))]
    {
        if !had_original && target.exists() {
            return Err("Target appeared after preview".into());
        }
        fs::rename(stage, target).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn publish_copies(
    stages: Vec<(PackageCopy, tempfile::TempPath)>,
    commit: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let mut installed: Vec<PackageCopy> = Vec::new();
    let result = (|| {
        for (file, stage) in stages {
            paths::writable_path(Path::new(&file.target))?;
            stage
                .persist_noclobber(&file.target)
                .map_err(|e| format!("Cannot install {}: {e}", file.target))?;
            installed.push(file);
        }
        commit()
    })();
    if let Err(error) = result {
        let mut retained = Vec::new();
        for file in installed.iter().rev() {
            // Remove only files created by this apply, and only if their content still matches.
            if destination_hash(Path::new(&file.target))
                .ok()
                .flatten()
                .as_ref()
                != Some(&file.sha256)
                || fs::remove_file(&file.target).is_err()
            {
                retained.push(file.target.clone());
            }
        }
        return Err(if retained.is_empty() {
            error
        } else {
            format!(
                "{error} Some new files could not be rolled back: {}. Inspect the backup manifest before retrying.",
                retained.join(", ")
            )
        });
    }
    Ok(())
}
pub fn apply(
    plan: Prepared,
    current: &Profile,
    data_dir: &Path,
    confirmed: bool,
) -> Result<Receipt> {
    if !confirmed {
        return Err("Explicit confirmation is required after reviewing the preview.".into());
    }
    if &plan.profile != current {
        return Err("Profile changed since preview. Preview again.".into());
    }
    ensure_game_closed()?;
    let target = paths::game_target(&plan.game_root)?;
    if paths::display(&target) != plan.view.target {
        return Err("Destination changed since preview.".into());
    }
    if existing(&target)? != plan.before {
        return Err("modlist.txt changed since preview. Preview again.".into());
    }
    check_files(&plan, false)?;
    if !plan.view.changed {
        return Ok(Receipt {
            target: plan.view.target,
            backup_dir: None,
            changed: false,
            message: "The installed packages and load list already match this profile. No files were written.".into(),
        });
    }
    paths::writable_path(data_dir)?;
    let parent = target.parent().ok_or("Missing destination folder")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    paths::writable_path(parent)?;
    let lock_path = parent.join(".cemm-apply.lock");
    paths::writable_path(&lock_path)?;
    let lock_file = OpenOptions::new().write(true).create_new(true).open(&lock_path).map_err(|_| "Another apply is running, or .cemm-apply.lock remains after an interrupted apply. See recovery instructions.".to_string())?;
    let _lock = ApplyLock {
        path: lock_path,
        _file: lock_file,
    };
    let backups = data_dir.join("backups");
    paths::writable_path(&backups)?;
    fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    let backup = tempfile::Builder::new()
        .prefix("apply-")
        .tempdir_in(&backups)
        .map_err(|e| e.to_string())?
        .keep();
    if let Some(bytes) = &plan.before {
        new_file(&backup.join("original.txt"), bytes)?;
    }
    new_file(&backup.join("proposed.txt"), plan.view.after.as_bytes())?;
    let manifest = serde_json::json!({
        "target":plan.view.target, "profile":plan.profile, "previouslyExisted":plan.before.is_some(),
        "createdUnix":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs(),
        "originalSha256":plan.before.as_ref().map(|b|format!("{:x}",Sha256::digest(b))),
        "proposedSha256":format!("{:x}",Sha256::digest(plan.view.after.as_bytes())),
        "files":plan.view.files
    });
    new_file(
        &backup.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )?;
    let mut copies = Vec::new();
    for file in plan.view.files.iter().filter(|f| f.copy) {
        let mut stage = tempfile::Builder::new()
            .prefix(".cemm-package-")
            .tempfile_in(parent)
            .map_err(|e| e.to_string())?;
        let mut source = File::open(&file.source).map_err(|e| e.to_string())?;
        let size = std::io::copy(&mut source, &mut stage).map_err(|e| e.to_string())?;
        stage.as_file().sync_all().map_err(|e| e.to_string())?;
        let stage = stage.into_temp_path();
        if size != file.size || hash_file(&stage)? != file.sha256 {
            return Err(
                "Mod content changed while copying. Nothing applied; preview again.".into(),
            );
        }
        copies.push((file.clone(), stage));
    }
    let mut stage = tempfile::Builder::new()
        .prefix(".cemm-")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?;
    stage
        .write_all(plan.view.after.as_bytes())
        .and_then(|_| stage.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    let staged = stage.into_temp_path();
    paths::game_target(&plan.game_root)?;
    ensure_game_closed()?;
    if existing(&target)? != plan.before {
        return Err(
            "Files changed while preparing the backup. Nothing applied; preview again.".into(),
        );
    }
    check_files(&plan, false)?;
    publish_copies(copies, || {
        paths::game_target(&plan.game_root)?;
        ensure_game_closed()?;
        check_files(&plan, true)?;
        if existing(&target)? != plan.before {
            return Err("modlist.txt changed while copying. Preview again.".into());
        }
        replace_atomic(&staged, &target, plan.before.is_some())
    })
    .map_err(|e| format!("{e} Backup: {}", paths::display(&backup)))?;
    if existing(&target)?.as_deref() != Some(plan.view.after.as_bytes()) {
        return Err(format!(
            "Write verification failed. Inspect the load list and backup at {} before retrying.",
            paths::display(&backup)
        ));
    }
    let recorded = new_file(&backup.join("applied.json"), b"{\"verified\":true}").is_ok();
    Ok(Receipt { target: plan.view.target, backup_dir: Some(paths::display(&backup)), changed: true,
        message: if recorded { "Mods installed in the game's Mods folder and load order verified. Restart Conan to refresh its Mods menu." } else { "Mods and load order applied and verified; backup retained, but the completion receipt could not be saved." }.into() })
}
pub fn export(profile: &Profile, path: &Path) -> Result<()> {
    paths::writable_path(path)?;
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
    {
        return Err("Export to a new .txt file.".into());
    }
    if path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("ConanSandbox")
    }) {
        return Err(
            "Export outside the game installation. Use Preview and Apply for the live load list."
                .into(),
        );
    }
    // Export is create-only, so it cannot overwrite a game file or an existing backup.
    new_file(path, modlist::render(&profile.entries)?.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staged(folder: &Path, name: &str) -> (PackageCopy, tempfile::TempPath) {
        let mut stage = tempfile::NamedTempFile::new_in(folder).unwrap();
        stage.write_all(b"new package").unwrap();
        let stage = stage.into_temp_path();
        (
            PackageCopy {
                source: "fixture".into(),
                target: paths::display(&folder.join(name)),
                size: 11,
                sha256: hash_file(&stage).unwrap(),
                copy: true,
            },
            stage,
        )
    }

    #[test]
    fn failed_list_commit_rolls_back_new_packages_and_preserves_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("existing.pak"), b"keep me").unwrap();
        let stages = vec![staged(dir.path(), "one.pak"), staged(dir.path(), "two.pak")];
        let result = publish_copies(stages, || Err("injected list failure".into()));
        assert!(result.unwrap_err().contains("injected list failure"));
        assert!(!dir.path().join("one.pak").exists());
        assert!(!dir.path().join("two.pak").exists());
        assert_eq!(
            fs::read(dir.path().join("existing.pak")).unwrap(),
            b"keep me"
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_second_copy_does_not_clobber_newly_appeared_destination() {
        let dir = tempfile::tempdir().unwrap();
        let stages = vec![staged(dir.path(), "one.pak"), staged(dir.path(), "two.pak")];
        fs::write(dir.path().join("two.pak"), b"external file").unwrap();
        assert!(publish_copies(stages, || panic!("must not commit the list")).is_err());
        assert!(!dir.path().join("one.pak").exists());
        assert_eq!(
            fs::read(dir.path().join("two.pak")).unwrap(),
            b"external file"
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn rollback_retains_files_changed_by_another_process() {
        let dir = tempfile::tempdir().unwrap();
        let result = publish_copies(vec![staged(dir.path(), "one.pak")], || {
            fs::write(dir.path().join("one.pak"), b"external edit").unwrap();
            Err("injected failure".into())
        });
        assert!(result.unwrap_err().contains("could not be rolled back"));
        assert_eq!(
            fs::read(dir.path().join("one.pak")).unwrap(),
            b"external edit"
        );
    }
}
