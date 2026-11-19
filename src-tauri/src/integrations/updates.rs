use super::{library::Acquisition, nexus, numeric_id, steamcmd};
use crate::model::Result;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub digest: String,
    pub state: String,
    pub message: String,
    pub checked_at: u64,
}

pub fn steam_status(saved: &str, remote: u64) -> (&'static str, &'static str) {
    match saved.parse::<u64>().ok().filter(|n| *n > 0) {
        Some(local) if remote > local => (
            "available",
            "The Workshop item has changed since this copy was downloaded. Review its page before downloading again.",
        ),
        Some(local) if remote == local => ("current", "No newer Workshop timestamp was reported."),
        _ => (
            "unknown",
            "The downloaded version cannot be compared reliably with Steam. Review the Workshop page.",
        ),
    }
}

pub fn nexus_status(saved: &str, value: &Value) -> Result<(&'static str, &'static str)> {
    let id = numeric_id(saved)?
        .parse::<u64>()
        .map_err(|_| "Invalid saved Nexus file ID")?;
    let files = value["files"]
        .as_array()
        .ok_or("Nexus returned no file list")?;
    let links = value["file_updates"]
        .as_array()
        .ok_or("Nexus did not provide file-update history")?;
    if links.iter().any(|link| {
        ["old_file_id", "new_file_id"]
            .iter()
            .any(|key| link[*key].as_u64().is_none_or(|id| id == 0))
    }) {
        return Err("Nexus returned invalid file-update history.".into());
    }
    let mut current = id;
    let mut seen = HashSet::new();
    loop {
        if !seen.insert(current) {
            return Err("Nexus returned a cyclic update history. Review the mod page.".into());
        }
        let successors: HashSet<_> = links
            .iter()
            .filter(|link| link["old_file_id"].as_u64() == Some(current))
            .map(|link| {
                link["new_file_id"]
                    .as_u64()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| "Invalid Nexus update history".to_string())
            })
            .collect::<Result<_>>()?;
        if successors.len() > 1 {
            return Ok((
                "unknown",
                "This Nexus file has several replacement branches. Choose the correct file on its page.",
            ));
        }
        match successors.into_iter().next() {
            Some(next) => current = next,
            None => break,
        }
    }
    let active = files.iter().any(|file| {
        file["file_id"].as_u64() == Some(current)
            && matches!(file["category_id"].as_u64(), Some(1..=3))
    });
    if current != id && active {
        Ok((
            "available",
            "The author linked a newer file to your downloaded version. Review the Nexus file list before downloading.",
        ))
    } else if current == id && active {
        Ok((
            "current",
            "No author-linked replacement was found for this file. Unlinked uploads may require a manual check.",
        ))
    } else {
        Ok((
            "unknown",
            "The saved file or its replacement is unavailable or archived. Review the Nexus page.",
        ))
    }
}

pub fn check(record: &Acquisition, session: Option<&nexus::Session>) -> UpdateCheck {
    let result = (|| -> Result<(&str, &str)> {
        match record.provider.as_str() {
            "steam" => Ok(steam_status(
                &record.file_id,
                steamcmd::lookup(&record.mod_id)?.updated,
            )),
            "nexus" if !nexus::API_ENABLED => Ok(("untracked", nexus::PUBLIC_NOTICE)),
            "nexus" => nexus_status(
                &record.file_id,
                &nexus::files(
                    session.ok_or("Connect Nexus in Discover & import, then check again.")?,
                    &record.mod_id,
                )?,
            ),
            _ => Ok((
                "untracked",
                "This local import has no recorded provider version. Check its original download page manually.",
            )),
        }
    })();
    let (state, message) = match result {
        Ok((state, message)) => (state.to_owned(), message.to_owned()),
        Err(message) => ("error".into(), message),
    };
    UpdateCheck {
        digest: record.digest.clone(),
        state,
        message,
        checked_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    }
}
