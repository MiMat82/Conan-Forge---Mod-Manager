use crate::{
    model::{Entry, Result},
    paths,
};
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

pub const MAX_LIST_BYTES: usize = 1024 * 1024;
pub const MAX_ENTRIES: usize = 4096;

pub fn decode(bytes: &[u8]) -> Result<String> {
    if bytes.len() > MAX_LIST_BYTES {
        return Err("modlist.txt exceeds the 1 MiB limit.".into());
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        if !bytes.len().is_multiple_of(2) {
            return Err("Invalid UTF-16 modlist.".into());
        }
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        String::from_utf16(&units).map_err(|e| e.to_string())
    } else {
        String::from_utf8(bytes.to_vec())
            .map(|s| s.trim_start_matches('\u{feff}').to_string())
            .map_err(|_| "Use UTF-8 or UTF-16 LE with BOM for modlist.txt.".into())
    }
}

fn normalize(path: PathBuf) -> PathBuf {
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => (),
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(c.as_os_str()),
        }
    }
    result
}

pub fn parse(bytes: &[u8], base: &Path) -> Result<Vec<Entry>> {
    paths::absolute(base)?;
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for (index, line) in decode(bytes)?.lines().enumerate() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || line.starts_with("//")
        {
            continue;
        }
        // Conan's optional leading * is a load-list marker, not a disabled flag.
        let raw = line.strip_prefix('*').unwrap_or(line).trim();
        let raw = raw
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(raw);
        let path = Path::new(raw);
        let resolved = normalize(if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        });
        let value = paths::display(&resolved);
        paths::validate_pak(&value).map_err(|e| format!("Line {}: {e}", index + 1))?;
        if !seen.insert(paths::key(&resolved)) {
            return Err(format!("Line {} repeats a mod path.", index + 1));
        }
        entries.push(Entry {
            path: value,
            enabled: true,
        });
        if entries.len() > MAX_ENTRIES {
            return Err("Too many mod entries (maximum 4096).".into());
        }
    }
    Ok(entries)
}

pub fn validate(entries: &[Entry]) -> Result<()> {
    if entries.len() > MAX_ENTRIES {
        return Err("Too many mod entries (maximum 4096).".into());
    }
    let mut seen = HashSet::new();
    for entry in entries {
        paths::validate_pak(&entry.path)?;
        if !seen.insert(paths::key(Path::new(&entry.path))) {
            return Err("A profile cannot contain the same mod path twice.".into());
        }
    }
    Ok(())
}

pub fn render(entries: &[Entry]) -> Result<String> {
    validate(entries)?;
    let result: String = entries
        .iter()
        .filter(|e| e.enabled)
        .map(|e| format!("*{}\r\n", e.path))
        .collect();
    if result.len() > MAX_LIST_BYTES {
        return Err("Generated modlist exceeds 1 MiB.".into());
    }
    Ok(result)
}
