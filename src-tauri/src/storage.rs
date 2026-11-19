use crate::{
    model::{Profile, Result, Scan, Settings},
    modlist, paths,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub struct Store {
    pub connection: Connection,
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn name(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        return Err("Profile names must contain 1–80 printable characters.".into());
    }
    Ok(value)
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        paths::writable_path(path)?;
        let mut connection = Connection::open(path).map_err(error)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(error)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(error)?;
        if version > 2 {
            return Err("This database was created by a newer manager. Use that version.".into());
        }
        if version == 0 {
            let tx = connection.transaction().map_err(error)?;
            tx.execute_batch("CREATE TABLE profiles (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, revision INTEGER NOT NULL DEFAULT 0, entries TEXT NOT NULL DEFAULT '[]'); CREATE TABLE state (key TEXT PRIMARY KEY, value TEXT NOT NULL); INSERT INTO profiles(name) VALUES ('Default'); PRAGMA user_version = 1;").map_err(error)?;
            tx.commit().map_err(error)?;
        }
        if version < 2 {
            connection.execute_batch("BEGIN IMMEDIATE; CREATE TABLE acquisitions (digest TEXT PRIMARY KEY, record TEXT NOT NULL); PRAGMA user_version = 2; COMMIT;").map_err(error)?;
        }
        Ok(Self { connection })
    }
    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id,name,revision,entries FROM profiles ORDER BY id")
            .map_err(error)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(error)?;
        rows.map(|r| {
            let (id, name, revision, json) = r.map_err(error)?;
            Ok(Profile {
                id,
                name,
                revision,
                entries: serde_json::from_str(&json).map_err(error)?,
            })
        })
        .collect()
    }
    pub fn profile(&self, id: i64) -> Result<Profile> {
        self.profiles()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or("Profile no longer exists.".into())
    }
    pub fn create(&self, value: &str) -> Result<Profile> {
        self.connection
            .execute("INSERT INTO profiles(name) VALUES (?1)", [name(value)?])
            .map_err(error)?;
        self.profile(self.connection.last_insert_rowid())
    }
    pub fn save(&self, profile: &Profile) -> Result<Profile> {
        modlist::validate(&profile.entries)?;
        let changed = self.connection.execute("UPDATE profiles SET name=?1, entries=?2, revision=revision+1 WHERE id=?3 AND revision=?4", params![name(&profile.name)?, serde_json::to_string(&profile.entries).map_err(error)?, profile.id, profile.revision]).map_err(error)?;
        if changed != 1 {
            return Err("Profile changed in another window. Reload before saving.".into());
        }
        self.profile(profile.id)
    }
    pub fn delete(&mut self, id: i64, revision: i64) -> Result<()> {
        let tx = self.connection.transaction().map_err(error)?;
        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM profiles", [], |r| r.get(0))
            .map_err(error)?;
        if count <= 1 {
            return Err("Keep at least one profile.".into());
        }
        if tx
            .execute(
                "DELETE FROM profiles WHERE id=?1 AND revision=?2",
                params![id, revision],
            )
            .map_err(error)?
            != 1
        {
            return Err("Profile changed. Reload before deleting.".into());
        }
        tx.commit().map_err(error)
    }
    pub fn settings(&self) -> Result<Settings> {
        let value = self
            .connection
            .query_row("SELECT value FROM state WHERE key='settings'", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(error)?;
        value
            .map(|v| serde_json::from_str(&v).map_err(error))
            .unwrap_or(Ok(Settings::default()))
    }
    pub fn set_settings(&self, settings: &Settings) -> Result<()> {
        if !settings.game_root.is_empty() {
            paths::absolute(Path::new(&settings.game_root))?;
        }
        if settings.scan_roots.len() > 32 {
            return Err("Choose at most 32 scan folders.".into());
        }
        for root in &settings.scan_roots {
            paths::absolute(Path::new(root))?;
        }
        self.put("settings", settings)
    }
    pub fn put(&self, key: &str, value: &impl serde::Serialize) -> Result<()> {
        self.connection.execute("INSERT INTO state(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,serde_json::to_string(value).map_err(error)?]).map_err(error)?;
        Ok(())
    }
    pub fn catalog(&self) -> Result<serde_json::Value> {
        let value = self
            .connection
            .query_row("SELECT value FROM state WHERE key='catalog'", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(error)?;
        value
            .map(|v| serde_json::from_str(&v).map_err(error))
            .unwrap_or_else(|| serde_json::to_value(Scan::default()).map_err(error))
    }
    pub fn acquisitions(&self) -> Result<Vec<crate::integrations::library::Acquisition>> {
        let mut stmt = self
            .connection
            .prepare("SELECT record FROM acquisitions ORDER BY rowid DESC")
            .map_err(error)?;
        stmt.query_map([], |r| r.get::<_, String>(0))
            .map_err(error)?
            .map(|r| serde_json::from_str(&r.map_err(error)?).map_err(error))
            .collect()
    }
    pub fn add_acquisition(
        &self,
        record: &crate::integrations::library::Acquisition,
    ) -> Result<()> {
        self.connection
            .execute(
                "INSERT INTO acquisitions(digest,record) VALUES (?1,?2)",
                params![record.digest, serde_json::to_string(record).map_err(error)?],
            )
            .map_err(error)?;
        Ok(())
    }
    pub fn refresh_acquisition(
        &self,
        record: &crate::integrations::library::Acquisition,
    ) -> Result<()> {
        let changed = self
            .connection
            .execute(
                "UPDATE acquisitions SET record=?1 WHERE digest=?2",
                params![serde_json::to_string(record).map_err(error)?, record.digest],
            )
            .map_err(error)?;
        if changed != 1 {
            return Err("The managed import changed. Try again.".into());
        }
        Ok(())
    }
}
