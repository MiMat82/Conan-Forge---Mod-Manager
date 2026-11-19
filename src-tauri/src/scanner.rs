use crate::{
    model::{FileStamp, ModPackage, Result, Scan},
    paths,
};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub fn stamp(path: &Path) -> Result<FileStamp> {
    let resolved = fs::canonicalize(path).map_err(|e| format!("{}: {e}", paths::display(path)))?;
    let meta = fs::metadata(&resolved).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err(format!("Not a regular file: {}", paths::display(path)));
    }
    Ok(FileStamp {
        path: paths::display(&resolved),
        size: meta.len(),
        modified: meta
            .modified()
            .map_err(|e| e.to_string())?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos(),
    })
}

pub fn package(path: &Path) -> Result<ModPackage> {
    let primary = stamp(path)?;
    let canonical = Path::new(&primary.path);
    let stem = canonical
        .file_stem()
        .ok_or("Missing mod filename")?
        .to_string_lossy();
    let mut files = vec![primary.clone()];
    let mut kinds = HashSet::new();
    for item in
        fs::read_dir(canonical.parent().ok_or("Missing mod folder")?).map_err(|e| e.to_string())?
    {
        let item = item.map_err(|e| e.to_string())?;
        let p = item.path();
        if p.file_stem()
            .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(&stem))
        {
            let ext = p
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if ext == "ucas" || ext == "utoc" {
                kinds.insert(ext);
                files.push(stamp(&p)?);
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let parts: Vec<_> = canonical
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let workshop_id = parts
        .windows(4)
        .find(|w| {
            w[0].eq_ignore_ascii_case("workshop")
                && w[1].eq_ignore_ascii_case("content")
                && w[2] == "440900"
                && w[3].chars().all(|c| c.is_ascii_digit())
        })
        .map(|w| w[3].clone());
    let mut warnings = Vec::new();
    if kinds.len() == 1 {
        warnings.push("Incomplete companion pair: both .ucas and .utoc must be present.".into());
    }
    if files.iter().any(|f| f.size == 0) {
        warnings.push("This package contains an empty file.".into());
    }
    Ok(ModPackage {
        path: primary.path.clone(),
        name: stem.into_owned(),
        files,
        workshop_id,
        warnings,
    })
}

pub fn scan(roots: &[String]) -> Result<Scan> {
    if roots.len() > 34 {
        return Err(
            "Choose no more than 32 scan folders, plus the game and managed library.".into(),
        );
    }
    let mut result = Scan::default();
    let mut seen = HashSet::new();
    let mut files = BTreeMap::new();
    let mut visited = 0;
    for root in roots {
        paths::absolute(Path::new(root))?;
        let mut stack = vec![(PathBuf::from(root), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            if depth > 24 {
                result
                    .warnings
                    .push(format!("Depth limit reached at {}", paths::display(&dir)));
                continue;
            }
            let meta = match fs::symlink_metadata(&dir) {
                Ok(m) => m,
                Err(e) => {
                    result
                        .warnings
                        .push(format!("{}: {e}", paths::display(&dir)));
                    continue;
                }
            };
            if paths::linked(&meta) {
                result
                    .warnings
                    .push(format!("Skipped linked folder: {}", paths::display(&dir)));
                continue;
            }
            let resolved = fs::canonicalize(&dir).map_err(|e| e.to_string())?;
            if !seen.insert(paths::key(&resolved)) {
                continue;
            }
            let children = match fs::read_dir(&resolved) {
                Ok(v) => v,
                Err(e) => {
                    result
                        .warnings
                        .push(format!("{}: {e}", paths::display(&dir)));
                    continue;
                }
            };
            for item in children {
                visited += 1;
                if visited > 100_000 {
                    return Err(
                        "Scan exceeds 100,000 entries. Select a more specific mod folder.".into(),
                    );
                }
                let item = item.map_err(|e| e.to_string())?;
                let p = item.path();
                let meta = fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
                if paths::linked(&meta) {
                    result
                        .warnings
                        .push(format!("Skipped linked item: {}", paths::display(&p)));
                    continue;
                }
                if meta.is_dir() {
                    stack.push((p, depth + 1));
                } else if meta.is_file() {
                    let ext = p
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    if ["pak", "ucas", "utoc"].contains(&ext.as_str()) {
                        files.insert(paths::key(&p), p);
                    }
                }
            }
        }
    }
    let mut paired = HashSet::new();
    for p in files
        .values()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pak")))
    {
        match package(p) {
            Ok(m) => {
                paired.extend(m.files.iter().map(|f| paths::key(Path::new(&f.path))));
                result.mods.push(m);
            }
            Err(e) => result.warnings.push(e),
        }
    }
    for (key, p) in &files {
        if !paired.contains(key) && !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pak")) {
            result.orphan_files.push(paths::display(p));
        }
    }
    result
        .mods
        .sort_by_key(|m| (m.name.to_lowercase(), m.path.clone()));
    Ok(result)
}
