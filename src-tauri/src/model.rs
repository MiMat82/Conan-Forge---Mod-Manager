use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: i64,
    pub name: String,
    pub revision: i64,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub game_root: String,
    pub scan_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileStamp {
    pub path: String,
    pub size: u64,
    pub modified: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModPackage {
    pub path: String,
    pub name: String,
    pub files: Vec<FileStamp>,
    pub workshop_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub mods: Vec<ModPackage>,
    pub warnings: Vec<String>,
    pub orphan_files: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub steam_roots: Vec<String>,
    pub game_roots: Vec<String>,
    pub workshop_roots: Vec<String>,
    pub warnings: Vec<String>,
}
