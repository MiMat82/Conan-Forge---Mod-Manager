use crate::model::Result;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub fn display(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Some(tail) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{tail}")
    } else {
        s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
    }
}

pub fn key(path: &Path) -> String {
    display(path).replace('\\', "/").to_lowercase()
}

pub fn linked(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

pub fn workshop(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("workshop")
    })
}

pub fn absolute(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Choose an absolute path without parent-directory (..) segments.".into());
    }
    Ok(())
}

// Reject redirected write paths instead of trusting their displayed location.
pub fn writable_path(path: &Path) -> Result<()> {
    absolute(path)?;
    #[cfg(windows)]
    {
        use std::path::Prefix;
        if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err("Write destinations must be on a local Windows drive.".into());
        }
        for component in path.components() {
            if let Component::Normal(name) = component {
                let name = name.to_string_lossy();
                let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
                if name.contains(':')
                    || name.ends_with([' ', '.'])
                    || name.chars().any(char::is_control)
                    || matches!(
                        stem.as_str(),
                        "CON"
                            | "PRN"
                            | "AUX"
                            | "NUL"
                            | "COM1"
                            | "COM2"
                            | "COM3"
                            | "COM4"
                            | "COM5"
                            | "COM6"
                            | "COM7"
                            | "COM8"
                            | "COM9"
                            | "LPT1"
                            | "LPT2"
                            | "LPT3"
                            | "LPT4"
                            | "LPT5"
                            | "LPT6"
                            | "LPT7"
                            | "LPT8"
                            | "LPT9"
                    )
                {
                    return Err("Write paths cannot contain device names, alternate streams or trailing spaces/dots.".into());
                }
            }
        }
    }
    if workshop(path) {
        return Err("Steam Workshop is read-only. Choose a different destination.".into());
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if linked(&meta) => {
                return Err(format!(
                    "Refusing a linked or redirected write path: {}",
                    display(ancestor)
                ));
            }
            Ok(_) => {
                let resolved = fs::canonicalize(ancestor).map_err(|e| e.to_string())?;
                if workshop(&resolved) {
                    return Err("The destination resolves into Steam Workshop.".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(format!("Cannot inspect {}: {e}", display(ancestor))),
        }
    }
    Ok(())
}

pub fn game_target(root: &Path) -> Result<PathBuf> {
    writable_path(root)?;
    if !root.join("ConanSandbox").is_dir() {
        return Err("Select the game installation folder containing ConanSandbox.".into());
    }
    let target = root.join("ConanSandbox").join("Mods").join("modlist.txt");
    writable_path(&target)?;
    if target.exists() && !target.is_file() {
        return Err("modlist.txt is not a regular file.".into());
    }
    Ok(target)
}

pub fn validate_pak(path: &str) -> Result<()> {
    if path.is_empty() || path.chars().any(|c| c.is_control() || c == '*' || c == '"') {
        return Err("Mod paths must not contain control characters, quotes or asterisks.".into());
    }
    absolute(Path::new(path))?;
    if !Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pak"))
    {
        return Err(format!("Only .pak entries belong in modlist.txt: {path}"));
    }
    Ok(())
}
