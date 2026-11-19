use super::{Progress, client};
use crate::{model::Result, paths};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const LAUNCHER_URL: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
const ZIP_LIMIT: u64 = 16 * 1024 * 1024;
const EXE_LIMIT: u64 = 64 * 1024 * 1024;

fn candidates() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        roots.extend(
            std::env::split_paths(&path)
                .filter(|p| p.is_absolute())
                .take(128),
        );
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        roots.extend([parent.to_path_buf(), parent.join("steamcmd")]);
    }
    for variable in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = std::env::var_os(variable) {
            let base = PathBuf::from(base);
            roots.extend([
                base.join("SteamCMD"),
                base.join("Programs/SteamCMD"),
                base.join("Steam/steamcmd"),
            ]);
        }
    }
    #[cfg(windows)]
    for letter in 'A'..='Z' {
        let root = format!("{letter}:\\");
        let wide: Vec<u16> = root.encode_utf16().chain(Some(0)).collect();
        if unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide.as_ptr()) } == 3 {
            for suffix in ["steamcmd", "Tools/SteamCMD", "Games/SteamCMD"] {
                roots.push(Path::new(&root).join(suffix));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    roots
        .into_iter()
        .map(|p| p.join("steamcmd.exe"))
        .filter(|p| seen.insert(paths::key(p)))
        .collect()
}

fn fetch_launcher(progress: &Progress) -> Result<Vec<u8>> {
    progress.check()?;
    progress.report("Downloading SteamCMD from Valve…", 0);
    let mut response = client(30)?.get(LAUNCHER_URL).send().map_err(
        |_| "Could not download SteamCMD from Valve. Check your connection and try again.",
    )?;
    if !response.status().is_success() || response.content_length().is_some_and(|n| n > ZIP_LIMIT) {
        return Err("Valve's SteamCMD download was unavailable or exceeded the safety limit. Try again later.".into());
    }
    let expected = response.content_length();
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        progress.check()?;
        let n = response
            .read(&mut buffer)
            .map_err(|_| "SteamCMD download was interrupted")?;
        if n == 0 {
            break;
        }
        if bytes.len() as u64 + n as u64 > ZIP_LIMIT {
            return Err("SteamCMD download exceeded the safety limit.".into());
        }
        bytes.extend_from_slice(&buffer[..n]);
        progress.report("Downloading SteamCMD from Valve…", bytes.len() as u64);
    }
    if expected.is_some_and(|n| n != bytes.len() as u64) {
        return Err("SteamCMD download was incomplete.".into());
    }
    Ok(bytes)
}

fn extract_launcher(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() as u64 > ZIP_LIMIT {
        return Err("SteamCMD archive is too large.".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| "Valve returned an invalid SteamCMD archive")?;
    if zip.len() != 1 {
        return Err("Unexpected SteamCMD archive contents.".into());
    }
    let mut entry = zip
        .by_index(0)
        .map_err(|_| "Could not read SteamCMD archive")?;
    if !entry.name().eq_ignore_ascii_case("steamcmd.exe")
        || entry.is_dir()
        || entry.size() > EXE_LIMIT
        || entry
            .unix_mode()
            .is_some_and(|m| !matches!(m & 0o170000, 0 | 0o100000))
    {
        return Err("Unexpected SteamCMD launcher entry.".into());
    }
    let size = entry.size();
    let mut exe = Vec::new();
    entry
        .by_ref()
        .take(EXE_LIMIT + 1)
        .read_to_end(&mut exe)
        .map_err(|_| "SteamCMD archive failed its integrity check")?;
    if exe.is_empty() || exe.len() as u64 != size || exe.len() as u64 > EXE_LIMIT {
        return Err("SteamCMD launcher size mismatch.".into());
    }
    Ok(exe)
}

// Copy first, then verify the exact private file that will be cached or executed.
fn prepare_launcher(
    data: &Path,
    candidates: &[PathBuf],
    progress: &Progress,
    fetch: impl FnOnce(&Progress) -> Result<Vec<u8>>,
    verify: impl Fn(&Path) -> Result<()>,
) -> Result<PathBuf> {
    let directory = data.join("tools/steamcmd");
    let cached = directory.join("steamcmd.exe");
    paths::writable_path(&cached)?;
    progress.check()?;
    progress.report("Looking for SteamCMD…", 0);
    if cached.is_file()
        && fs::metadata(&cached).is_ok_and(|m| m.len() <= EXE_LIMIT)
        && verify(&cached).is_ok()
    {
        progress.check()?;
        return Ok(cached);
    }
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let staged = tempfile::NamedTempFile::new_in(&directory)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    let mut found = false;
    for candidate in candidates {
        progress.check()?;
        if paths::writable_path(candidate).is_err()
            || !candidate.is_file()
            || !fs::metadata(candidate).is_ok_and(|m| m.len() <= EXE_LIMIT)
        {
            continue;
        }
        if fs::copy(candidate, &staged).is_ok() && verify(&staged).is_ok() {
            found = true;
            progress.report("Found and verified SteamCMD", 0);
            break;
        }
    }
    if !found {
        let bytes = extract_launcher(&fetch(progress)?)?;
        progress.check()?;
        fs::write(&staged, &bytes).map_err(|e| e.to_string())?;
        progress.report("Checking Valve's SteamCMD signature…", 0);
        verify(&staged)?;
    }
    progress.check()?;
    paths::writable_path(&cached)?;
    fs::OpenOptions::new()
        .write(true)
        .open(&staged)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    staged.persist(&cached).map_err(|e| e.to_string())?;
    Ok(cached)
}

pub fn ensure_launcher(data: &Path, progress: &Progress) -> Result<PathBuf> {
    prepare_launcher(data, &candidates(), progress, fetch_launcher, verify_valve)
}

#[cfg(windows)]
pub fn verify_valve(path: &Path) -> Result<()> {
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        Security::{
            Cryptography::{CERT_NAME_SIMPLE_DISPLAY_TYPE, CertGetNameStringW},
            WinTrust::*,
        },
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide.as_ptr(),
        ..Default::default()
    };
    let mut trust = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_FILE,
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
        ..Default::default()
    };
    trust.Anonymous.pFile = &mut file;
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let mut publisher = String::new();
    let status;
    unsafe {
        status = WinVerifyTrust(
            INVALID_HANDLE_VALUE,
            &mut action,
            &mut trust as *mut _ as *mut _,
        );
        if status == 0 {
            let provider = WTHelperProvDataFromStateData(trust.hWVTStateData);
            if !provider.is_null() {
                let signer = WTHelperGetProvSignerFromChain(provider, 0, 0, 0);
                if !signer.is_null()
                    && (*signer).csCertChain > 0
                    && !(*signer).pasCertChain.is_null()
                {
                    let cert = (*(*signer).pasCertChain).pCert;
                    if !cert.is_null() {
                        let mut name = [0u16; 256];
                        let n = CertGetNameStringW(
                            cert,
                            CERT_NAME_SIMPLE_DISPLAY_TYPE,
                            0,
                            std::ptr::null(),
                            name.as_mut_ptr(),
                            name.len() as u32,
                        );
                        if n > 1 && n as usize <= name.len() {
                            publisher = String::from_utf16_lossy(&name[..n as usize - 1]);
                        }
                    }
                }
            }
        }
        trust.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(
            INVALID_HANDLE_VALUE,
            &mut action,
            &mut trust as *mut _ as *mut _,
        );
    }
    if !matches!(
        publisher.as_str(),
        "Valve" | "Valve Corp." | "Valve Corporation"
    ) {
        return Err(format!(
            "SteamCMD could not be verified as a trusted Valve executable (Windows status 0x{:08X}). Nothing was run. Check your connection and Windows certificate trust, then try again.",
            status as u32
        ));
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn verify_valve(_: &Path) -> Result<()> {
    Err("SteamCMD setup requires Windows signature verification.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Write},
        sync::atomic::Ordering,
    };
    fn archive(names: &[&str]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in names {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"approved").unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    fn verify_fixture(path: &Path) -> Result<()> {
        if fs::read(path).unwrap() == b"approved" {
            Ok(())
        } else {
            Err("Rejected fixture signature".into())
        }
    }
    #[test]
    fn finds_valid_candidate_without_modifying_sources_or_downloading() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let rejected = root.path().join("bad.exe");
        let accepted = root.path().join("good.exe");
        fs::write(&rejected, b"bad").unwrap();
        fs::write(&accepted, b"approved").unwrap();
        let path = prepare_launcher(
            &data,
            &[rejected.clone(), accepted.clone()],
            &Progress::default(),
            |_| panic!("Must not download"),
            verify_fixture,
        )
        .unwrap();
        assert_eq!(path, data.join("tools/steamcmd/steamcmd.exe"));
        assert_eq!(fs::read(path).unwrap(), b"approved");
        assert_eq!(fs::read(rejected).unwrap(), b"bad");
        assert_eq!(fs::read(accepted).unwrap(), b"approved");
    }
    #[test]
    fn downloads_when_missing_and_reuses_verified_cache() {
        let root = tempfile::tempdir().unwrap();
        let progress = Progress::default();
        let path = prepare_launcher(
            root.path(),
            &[],
            &progress,
            |_| Ok(archive(&["steamcmd.exe"])),
            verify_fixture,
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"approved");
        assert_eq!(
            prepare_launcher(
                root.path(),
                &[],
                &progress,
                |_| panic!("Cache must avoid download"),
                verify_fixture
            )
            .unwrap(),
            path
        );
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }
    #[test]
    fn failure_or_cancel_never_replaces_previous_cache() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("tools/steamcmd/steamcmd.exe");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"old invalid cache").unwrap();
        assert!(
            prepare_launcher(
                root.path(),
                &[],
                &Progress::default(),
                |_| Err("Network failed".into()),
                verify_fixture
            )
            .is_err()
        );
        let progress = Progress::default();
        assert!(
            prepare_launcher(
                root.path(),
                &[],
                &progress,
                |p| {
                    p.cancel.store(true, Ordering::Relaxed);
                    Ok(archive(&["steamcmd.exe"]))
                },
                verify_fixture
            )
            .unwrap_err()
            .contains("Cancelled")
        );
        assert_eq!(fs::read(&target).unwrap(), b"old invalid cache");
        assert_eq!(fs::read_dir(target.parent().unwrap()).unwrap().count(), 1);
    }
    #[test]
    fn untrusted_download_is_never_cached() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            prepare_launcher(
                root.path(),
                &[],
                &Progress::default(),
                |_| Ok(archive(&["steamcmd.exe"])),
                |_| Err("Bad signature".into())
            )
            .is_err()
        );
        assert!(!root.path().join("tools/steamcmd/steamcmd.exe").exists());
    }
    #[test]
    fn installer_archive_accepts_only_one_bounded_plain_launcher() {
        assert_eq!(
            extract_launcher(&archive(&["steamcmd.exe"])).unwrap(),
            b"approved"
        );
        for names in [
            vec!["../steamcmd.exe"],
            vec!["folder/steamcmd.exe"],
            vec!["other.exe"],
            vec!["steamcmd.exe", "extra.dll"],
        ] {
            assert!(extract_launcher(&archive(&names)).is_err());
        }
        assert!(extract_launcher(b"truncated").is_err());
        assert!(extract_launcher(&vec![0; ZIP_LIMIT as usize + 1]).is_err());
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.add_symlink(
            "steamcmd.exe",
            "../outside.exe",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        assert!(extract_launcher(&zip.finish().unwrap().into_inner()).is_err());
    }
    #[test]
    #[cfg(windows)]
    fn native_trust_check_rejects_unsigned_or_non_valve_files() {
        let root = tempfile::tempdir().unwrap();
        let exe = root.path().join("steamcmd.exe");
        fs::write(&exe, b"MZ not an executable").unwrap();
        assert!(verify_valve(&exe).is_err());
        assert!(verify_valve(&std::env::current_exe().unwrap()).is_err());
        let cmd = PathBuf::from(std::env::var_os("WINDIR").unwrap()).join("System32/cmd.exe");
        assert!(verify_valve(&cmd).is_err());
    }
}
