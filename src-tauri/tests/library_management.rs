use conan_mod_manager_lib::{
    Action, Engine,
    integrations::{OnlineAction, Progress, library, updates},
    model::{Entry, Settings},
    paths,
};
use serde_json::json;
use std::{fs, path::Path};

fn imported() -> (tempfile::TempDir, Engine, library::Acquisition) {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("originals");
    fs::create_dir(&source).unwrap();
    for (name, bytes) in [
        ("A.pak", "alpha"),
        ("A.ucas", "content"),
        ("A.utoc", "table"),
        ("B.pak", "beta"),
    ] {
        fs::write(source.join(name), bytes).unwrap();
    }
    let engine = Engine::open(&root.path().join("manager")).unwrap();
    library::import(
        &engine.data_dir,
        &source,
        "local",
        "",
        "",
        &Progress::default(),
    )
    .unwrap();
    let record = engine.store.acquisitions().unwrap().remove(0);
    (root, engine, record)
}
fn preview(engine: &mut Engine, record: &library::Acquisition) -> String {
    engine
        .dispatch(Action::PreviewLibraryDelete {
            digest: record.digest.clone(),
        })
        .unwrap()["token"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn confirmed_deletion_removes_entire_bundle_and_history_but_preserves_originals_and_game_copies() {
    let (root, mut engine, record) = imported();
    let game = root.path().join("game");
    let mods = game.join("ConanSandbox/Mods");
    fs::create_dir_all(&mods).unwrap();
    fs::write(mods.join("A.pak"), "installed copy").unwrap();
    fs::write(mods.join("modlist.txt"), "*A.pak\n").unwrap();
    engine
        .store
        .set_settings(&Settings {
            game_root: paths::display(&game),
            scan_roots: vec![],
        })
        .unwrap();
    let view = engine
        .dispatch(Action::PreviewLibraryDelete {
            digest: record.digest.clone(),
        })
        .unwrap();
    assert_eq!(view["packages"].as_array().unwrap().len(), 2);
    assert_eq!(view["fileCount"], 4);
    assert!(view["profiles"].as_array().unwrap().is_empty());
    assert!(Path::new(&record.path).exists());
    engine
        .dispatch(Action::DeleteLibrary {
            token: view["token"].as_str().unwrap().into(),
            confirmed: true,
        })
        .unwrap();
    assert!(!Path::new(&record.path).exists());
    assert!(engine.store.acquisitions().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(mods.join("A.pak")).unwrap(),
        "installed copy"
    );
    assert_eq!(
        fs::read_to_string(mods.join("modlist.txt")).unwrap(),
        "*A.pak\n"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("originals/A.pak")).unwrap(),
        "alpha"
    );
    assert!(engine.store.profiles().unwrap()[0].entries.is_empty());
    assert_eq!(
        fs::read_dir(engine.data_dir.join("staging"))
            .unwrap()
            .count(),
        0
    );
    // Deleted content can be imported again; stale deduplication records must not block it.
    library::import(
        &engine.data_dir,
        &root.path().join("originals"),
        "local",
        "",
        "",
        &Progress::default(),
    )
    .unwrap();
}

#[test]
fn deletion_requires_a_fresh_issued_preview_and_explicit_confirmation() {
    let (_root, mut engine, record) = imported();
    assert!(
        engine
            .dispatch(Action::DeleteLibrary {
                token: "forged".into(),
                confirmed: true
            })
            .is_err()
    );
    let old = preview(&mut engine, &record);
    let _new = preview(&mut engine, &record);
    assert!(
        engine
            .dispatch(Action::DeleteLibrary {
                token: old,
                confirmed: true
            })
            .is_err()
    );
    let token = preview(&mut engine, &record);
    assert!(
        engine
            .dispatch(Action::DeleteLibrary {
                token,
                confirmed: false
            })
            .is_err()
    );
    assert!(Path::new(&record.path).join("A.pak").exists());
    assert_eq!(engine.store.acquisitions().unwrap().len(), 1);
}

#[test]
fn disabled_references_in_any_profile_added_after_review_block_deletion() {
    let (_root, mut engine, record) = imported();
    let token = preview(&mut engine, &record);
    let mut other = engine.store.create("Other saved profile").unwrap();
    other.entries.push(Entry {
        path: paths::display(&Path::new(&record.path).join("B.pak")).to_uppercase(),
        enabled: false,
    });
    engine.store.save(&other).unwrap();
    let error = engine
        .dispatch(Action::DeleteLibrary {
            token,
            confirmed: true,
        })
        .unwrap_err();
    assert!(error.contains("Other saved profile"));
    assert!(Path::new(&record.path).join("B.pak").exists());
    let view = engine
        .dispatch(Action::PreviewLibraryDelete {
            digest: record.digest.clone(),
        })
        .unwrap();
    assert_eq!(view["profiles"][0], "Other saved profile");
}

#[test]
fn direct_game_load_list_references_block_deletion() {
    let (root, mut engine, record) = imported();
    let game = root.path().join("game");
    let mods = game.join("ConanSandbox/Mods");
    fs::create_dir_all(&mods).unwrap();
    fs::write(
        mods.join("modlist.txt"),
        format!(
            "*{}\n",
            paths::display(&Path::new(&record.path).join("A.pak"))
        ),
    )
    .unwrap();
    engine
        .store
        .set_settings(&Settings {
            game_root: paths::display(&game),
            scan_roots: vec![],
        })
        .unwrap();
    let token = preview(&mut engine, &record);
    assert!(
        engine
            .dispatch(Action::DeleteLibrary {
                token,
                confirmed: true
            })
            .unwrap_err()
            .contains("Current game load list")
    );
    assert!(Path::new(&record.path).exists());
}

#[test]
fn changed_or_unexpected_files_after_review_prevent_deletion() {
    for extra in [false, true] {
        let (_root, mut engine, record) = imported();
        let token = preview(&mut engine, &record);
        let path = Path::new(&record.path).join(if extra { "personal.txt" } else { "A.pak" });
        fs::write(&path, "edits").unwrap();
        assert!(
            engine
                .dispatch(Action::DeleteLibrary {
                    token,
                    confirmed: true
                })
                .is_err()
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "edits");
        assert_eq!(engine.store.acquisitions().unwrap().len(), 1);
    }
}

#[test]
fn database_failure_restores_bundle_and_keeps_history() {
    let (_root, mut engine, record) = imported();
    let token = preview(&mut engine, &record);
    engine.store.connection.execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON acquisitions BEGIN SELECT RAISE(ABORT, 'fixture database failure'); END;").unwrap();
    assert!(
        engine
            .dispatch(Action::DeleteLibrary {
                token,
                confirmed: true
            })
            .unwrap_err()
            .contains("fixture database failure")
    );
    assert_eq!(
        fs::read_to_string(Path::new(&record.path).join("A.pak")).unwrap(),
        "alpha"
    );
    assert_eq!(engine.store.acquisitions().unwrap().len(), 1);
    assert_eq!(
        fs::read_dir(engine.data_dir.join("staging"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn forged_history_cannot_target_external_or_workshop_folders() {
    let (root, mut engine, mut record) = imported();
    for path in [
        root.path().join("originals"),
        engine.data_dir.join("library"),
        engine.data_dir.join("library/../originals"),
        root.path().join("steamapps/workshop/content/440900/123"),
    ] {
        record.path = paths::display(&path);
        engine.store.refresh_acquisition(&record).unwrap();
        assert!(
            engine
                .dispatch(Action::PreviewLibraryDelete {
                    digest: record.digest.clone()
                })
                .is_err()
        );
    }
    assert_eq!(
        fs::read_to_string(root.path().join("originals/A.pak")).unwrap(),
        "alpha"
    );
}

#[cfg(windows)]
#[test]
fn deletion_rejects_junctions_and_detects_profiles_using_an_alias() {
    let (root, mut engine, record) = imported();
    let link = Path::new(&record.path).join("redirected");
    let original = root.path().join("originals");
    let make_link = |link: &Path, target: &Path| {
        let result = std::process::Command::new("cmd.exe")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    make_link(&link, &original);
    assert!(
        engine
            .dispatch(Action::PreviewLibraryDelete {
                digest: record.digest.clone()
            })
            .is_err()
    );
    fs::remove_dir(&link).unwrap();
    let alias = root.path().join("alias");
    make_link(&alias, Path::new(&record.path));
    let mut profile = engine.store.profiles().unwrap().remove(0);
    profile.entries.push(Entry {
        path: paths::display(&alias.join("A.pak")),
        enabled: true,
    });
    engine.store.save(&profile).unwrap();
    let token = preview(&mut engine, &record);
    assert!(
        engine
            .dispatch(Action::DeleteLibrary {
                token,
                confirmed: true
            })
            .unwrap_err()
            .contains("Default")
    );
    fs::remove_dir(alias).unwrap();
    assert_eq!(fs::read_to_string(original.join("A.pak")).unwrap(), "alpha");
}

#[test]
fn provider_deduplication_keeps_source_identity_and_refreshes_verified_version() {
    let (root, engine, record) = imported();
    let source = root.path().join("originals");
    let first = library::import(
        &engine.data_dir,
        &source,
        "steam",
        "123",
        "1000",
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(first, record.path);
    library::import(
        &engine.data_dir,
        &source,
        "steam",
        "123",
        "2000",
        &Progress::default(),
    )
    .unwrap();
    let updated = engine.store.acquisitions().unwrap().remove(0);
    assert_eq!(
        (&*updated.provider, &*updated.mod_id, &*updated.file_id),
        ("steam", "123", "2000")
    );
    library::import(
        &engine.data_dir,
        &source,
        "local",
        "",
        "",
        &Progress::default(),
    )
    .unwrap();
    library::import(
        &engine.data_dir,
        &source,
        "nexus",
        "99",
        "88",
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(engine.store.acquisitions().unwrap()[0].provider, "steam");
}

#[test]
fn workshop_checks_require_a_valid_baseline_and_do_not_guess_from_local_dates() {
    assert_eq!(updates::steam_status("100", 200).0, "available");
    assert_eq!(updates::steam_status("100", 100).0, "current");
    for (saved, remote) in [("", 200), ("0", 200), ("100", 0), ("100", 99)] {
        assert_eq!(updates::steam_status(saved, remote).0, "unknown");
    }
}

#[test]
fn nexus_checks_follow_file_lineage_not_unrelated_optional_files() {
    let unrelated = json!({ "files": [{"file_id": 10, "category_id": 1}, {"file_id": 90, "category_id": 3}], "file_updates": [] });
    assert_eq!(
        updates::nexus_status("10", &unrelated).unwrap().0,
        "current"
    );
    let chain = json!({ "files": [{"file_id": 30, "category_id": 1}], "file_updates": [{"old_file_id": 10, "new_file_id": 20}, {"old_file_id": 20, "new_file_id": 30}] });
    assert_eq!(updates::nexus_status("10", &chain).unwrap().0, "available");
    for category in [4, 6, 7] {
        let mut hidden = chain.clone();
        hidden["files"][0]["category_id"] = json!(category);
        assert_eq!(updates::nexus_status("10", &hidden).unwrap().0, "unknown");
    }
    assert!(updates::nexus_status("10", &json!({"files": []})).is_err());
    let looped = json!({"files": [], "file_updates": [{"old_file_id": 10, "new_file_id": 20}, {"old_file_id": 20, "new_file_id": 10}]});
    assert!(updates::nexus_status("10", &looped).is_err());
    let branches = json!({"files": [], "file_updates": [{"old_file_id": 10, "new_file_id": 20}, {"old_file_id": 10, "new_file_id": 30}]});
    assert_eq!(updates::nexus_status("10", &branches).unwrap().0, "unknown");
}

#[test]
fn local_and_disconnected_checks_report_limits_without_altering_files_or_profiles() {
    let (_root, mut engine, mut record) = imported();
    let before = engine.store.profiles().unwrap();
    let local = engine
        .dispatch(Action::Online {
            action: OnlineAction::CheckUpdate {
                digest: record.digest.clone(),
            },
        })
        .unwrap();
    assert_eq!(local["state"], "untracked");
    record.provider = "nexus".into();
    record.mod_id = "10".into();
    record.file_id = "20".into();
    engine.store.refresh_acquisition(&record).unwrap();
    let nexus = engine
        .dispatch(Action::Online {
            action: OnlineAction::CheckUpdate {
                digest: record.digest,
            },
        })
        .unwrap();
    if conan_mod_manager_lib::integrations::nexus::API_ENABLED {
        assert_eq!(nexus["state"], "error");
        assert!(nexus["message"].as_str().unwrap().contains("Connect Nexus"));
    } else {
        assert_eq!(nexus["state"], "untracked");
        assert!(
            nexus["message"]
                .as_str()
                .unwrap()
                .contains("public preview")
        );
    }
    assert_eq!(engine.store.profiles().unwrap(), before);
    assert_eq!(
        fs::read_to_string(Path::new(&record.path).join("A.pak")).unwrap(),
        "alpha"
    );
}
