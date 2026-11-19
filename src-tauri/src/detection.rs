use crate::{
    model::{Detection, Result},
    paths,
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub const GAME_EXE: &str = "ConanSandbox/Binaries/Win64/ConanSandbox-Win64-Shipping.exe";
pub const CLIENT_EXECUTABLES: [&str; 3] = [
    "ConanSandbox-Win64-Shipping.exe",
    "ConanSandbox_BE.exe",
    "ConanSandbox.exe",
];

pub fn is_game_root(root: &Path) -> bool {
    CLIENT_EXECUTABLES.iter().any(|name| {
        root.join("ConanSandbox/Binaries/Win64")
            .join(name)
            .is_file()
    })
}

pub fn resolve_game(selection: &Path) -> Result<PathBuf> {
    paths::absolute(selection)?;
    if selection.is_file()
        && !selection.file_name().is_some_and(|n| {
            CLIENT_EXECUTABLES
                .iter()
                .any(|name| n.eq_ignore_ascii_case(name))
        })
    {
        return Err("Select ConanSandbox.exe, ConanSandbox_BE.exe, ConanSandbox-Win64-Shipping.exe, or the game folder.".into());
    }
    for candidate in selection.ancestors().take(6) {
        if is_game_root(candidate) {
            return Ok(candidate.to_path_buf());
        }
    }
    Err("No supported Conan client executable was found in ConanSandbox/Binaries/Win64 beneath the selected folder.".into())
}

pub fn add_common_installations(found: &mut Detection, drives: &[PathBuf]) {
    for drive in drives {
        for prefix in [
            "Games",
            "SteamLibrary/steamapps/common",
            "Program Files (x86)/Steam/steamapps/common",
            "Program Files/Steam/steamapps/common",
            "",
        ] {
            for name in ["Conan Exiles Enhanced", "Conan Exiles", "ConanExiles"] {
                let root = drive.join(prefix).join(name);
                if is_game_root(&root)
                    && !found
                        .game_roots
                        .iter()
                        .any(|p| paths::key(Path::new(p)) == paths::key(&root))
                {
                    found.game_roots.push(paths::display(&root));
                }
            }
        }
    }
}

pub fn startup_settings(
    mut settings: crate::model::Settings,
    found: &Detection,
) -> crate::model::Settings {
    if settings.game_root.is_empty() && found.game_roots.len() == 1 {
        settings.game_root = found.game_roots[0].clone();
        if settings.scan_roots.is_empty() {
            settings.scan_roots = found.workshop_roots.clone();
        }
    }
    settings
}

#[derive(Debug)]
enum Value {
    Text(String),
    Object(Vec<(String, Value)>),
}

fn tokens(text: &str) -> Result<Vec<String>> {
    let mut input = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut tokens = Vec::new();
    while let Some(c) = input.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '/' && input.peek() == Some(&'/') {
            for ch in input.by_ref() {
                if ch == '\n' {
                    break;
                }
            }
        } else if c == '{' || c == '}' {
            tokens.push(c.to_string());
        } else if c == '"' {
            let mut value = String::new();
            let mut closed = false;
            while let Some(ch) = input.next() {
                if ch == '"' {
                    closed = true;
                    break;
                }
                if ch == '\\' && matches!(input.peek(), Some('\\' | '"')) {
                    value.push(input.next().ok_or("Incomplete VDF escape")?);
                } else {
                    value.push(ch);
                }
            }
            if !closed {
                return Err("Unclosed VDF string".into());
            }
            tokens.push(format!("s{value}"));
        } else {
            return Err("Unsupported unquoted VDF token".into());
        }
    }
    Ok(tokens)
}

fn object(
    tokens: &[String],
    cursor: &mut usize,
    nested: bool,
    depth: usize,
) -> Result<Vec<(String, Value)>> {
    if depth > 16 {
        return Err("VDF nesting limit exceeded".into());
    }
    let mut pairs = Vec::new();
    while *cursor < tokens.len() {
        let key = &tokens[*cursor];
        *cursor += 1;
        if key == "}" && nested {
            return Ok(pairs);
        }
        let key = key.strip_prefix('s').ok_or("Expected VDF key")?.to_string();
        let value = tokens.get(*cursor).ok_or("Missing VDF value")?;
        *cursor += 1;
        let value = if value == "{" {
            Value::Object(object(tokens, cursor, true, depth + 1)?)
        } else {
            Value::Text(
                value
                    .strip_prefix('s')
                    .ok_or("Expected VDF value")?
                    .to_string(),
            )
        };
        pairs.push((key, value));
    }
    if nested {
        return Err("Unclosed VDF object".into());
    }
    Ok(pairs)
}

fn parse(text: &str) -> Result<Vec<(String, Value)>> {
    if text.len() > 4 * 1024 * 1024 {
        return Err("Steam manifest exceeds 4 MiB".into());
    }
    object(&tokens(text)?, &mut 0, false, 0)
}
fn text_field<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    pairs.iter().find_map(|(k, v)| match v {
        Value::Text(s) if k.eq_ignore_ascii_case(key) => Some(s.as_str()),
        _ => None,
    })
}
fn section<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a [(String, Value)]> {
    pairs.iter().find_map(|(k, v)| match v {
        Value::Object(s) if k.eq_ignore_ascii_case(key) => Some(s.as_slice()),
        _ => None,
    })
}

pub fn library_paths(text: &str) -> Result<Vec<PathBuf>> {
    let parsed = parse(text)?;
    let library = section(&parsed, "libraryfolders").ok_or("Missing libraryfolders section")?;
    Ok(library
        .iter()
        .filter_map(|(k, v)| {
            if k.parse::<u32>().is_err() {
                return None;
            }
            match v {
                Value::Text(s) => Some(PathBuf::from(s)),
                Value::Object(o) => text_field(o, "path").map(PathBuf::from),
            }
        })
        .collect())
}

pub fn detect_from(steam_roots: Vec<PathBuf>) -> Detection {
    let mut found = Detection::default();
    let mut libraries = Vec::new();
    for root in steam_roots.into_iter().filter(|p| p.is_dir()) {
        found.steam_roots.push(paths::display(&root));
        libraries.push(root.clone());
        let file = root.join("steamapps/libraryfolders.vdf");
        match fs::read_to_string(&file)
            .map_err(|e| e.to_string())
            .and_then(|s| library_paths(&s))
        {
            Ok(paths) => libraries.extend(paths),
            Err(e) => found
                .warnings
                .push(format!("{}: {e}", paths::display(&file))),
        }
    }
    let mut seen = HashSet::new();
    for library in libraries {
        if !seen.insert(paths::key(&library)) {
            continue;
        }
        let workshop = library.join("steamapps/workshop/content/440900");
        if workshop.is_dir() {
            found.workshop_roots.push(paths::display(&workshop));
        }
        let manifest = library.join("steamapps/appmanifest_440900.acf");
        if manifest.is_file() {
            let candidate = (|| -> Result<PathBuf> {
                let parsed = parse(&fs::read_to_string(&manifest).map_err(|e| e.to_string())?)?;
                let state = section(&parsed, "AppState").ok_or("Missing AppState")?;
                if text_field(state, "appid") != Some("440900") {
                    return Err("Manifest app ID is not 440900".into());
                }
                let dir = text_field(state, "installdir").ok_or("Missing installdir")?;
                if dir.is_empty() || dir == "." || dir == ".." || dir.contains(['/', '\\', ':']) {
                    return Err("Invalid Steam installation directory".into());
                }
                Ok(library.join("steamapps/common").join(dir))
            })();
            match candidate {
                Ok(root) if is_game_root(&root) => found.game_roots.push(paths::display(&root)),
                Ok(_) => found.warnings.push(format!(
                    "Installation in {} is missing a supported Conan client executable.",
                    paths::display(&manifest)
                )),
                Err(e) => found
                    .warnings
                    .push(format!("{}: {e}", paths::display(&manifest))),
            }
        }
        for name in ["Conan Exiles", "Conan Exiles Enhanced"] {
            let root = library.join("steamapps/common").join(name);
            if is_game_root(&root)
                && !found
                    .game_roots
                    .iter()
                    .any(|p| paths::key(Path::new(p)) == paths::key(&root))
            {
                found.game_roots.push(paths::display(&root));
            }
        }
    }
    found
}

pub fn detect() -> Detection {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        use winreg::{
            RegKey,
            enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY},
        };
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam")
            && let Ok(path) = key.get_value::<String, _>("SteamPath")
        {
            roots.push(PathBuf::from(path));
        }
        if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(r"SOFTWARE\Valve\Steam", KEY_READ | KEY_WOW64_32KEY)
            && let Ok(path) = key.get_value::<String, _>("InstallPath")
        {
            roots.push(PathBuf::from(path));
        }
        if let Some(base) = std::env::var_os("ProgramFiles(x86)") {
            roots.push(PathBuf::from(base).join("Steam"));
        }
    }
    let mut seen = HashSet::new();
    roots.retain(|p| seen.insert(paths::key(p)));
    let mut found = detect_from(roots);
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
        let drives: Vec<_> = ('A'..='Z')
            .filter_map(|letter| {
                let path = format!("{letter}:\\");
                let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
                // DRIVE_FIXED = 3; avoid waking removable or network drives at startup.
                (unsafe { GetDriveTypeW(wide.as_ptr()) } == 3).then(|| PathBuf::from(path))
            })
            .collect();
        add_common_installations(&mut found, &drives);
    }
    if found.game_roots.is_empty() {
        found.warnings.push("No installation found in Steam libraries or common game folders. Choose the game folder or executable manually.".into());
    }
    found
}
