use conan_mod_manager_lib::{
    Action, Engine, deployment, detection,
    model::{Entry, Profile, Settings},
    modlist, paths, scanner,
    storage::Store,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    game: PathBuf,
    pak: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        let game = root.join("game");
        let data = root.join("data");
        let pak = root.join("steamapps/workshop/content/440900/123/Desert.pak");
        fs::create_dir_all(game.join("ConanSandbox/Mods")).unwrap();
        fs::create_dir_all(game.join("ConanSandbox/Binaries/Win64")).unwrap();
        fs::write(
            game.join(detection::GAME_EXE),
            b"synthetic executable fixture",
        )
        .unwrap();
        fs::create_dir_all(pak.parent().unwrap()).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(&pak, b"synthetic package, not a real game mod").unwrap();
        Self {
            _temp: temp,
            root,
            game,
            pak,
            data,
        }
    }
    fn profile(&self) -> Profile {
        Profile {
            id: 1,
            name: "Test".into(),
            revision: 0,
            entries: vec![Entry {
                path: paths::display(&self.pak),
                enabled: true,
            }],
        }
    }
    fn target(&self) -> PathBuf {
        self.game.join("ConanSandbox/Mods/modlist.txt")
    }
    fn plan(&self) -> deployment::Prepared {
        deployment::prepare(self.profile(), &self.game).unwrap()
    }
}

#[test]
fn imports_markers_comments_bom_unicode_and_relative_paths_in_order() {
    let f = Fixture::new();
    let bytes = "\u{feff}# note\r\n*Desert.pak\r\n\"Ørken space.pak\"\n; comment\n\n".as_bytes();
    let entries = modlist::parse(bytes, &f.game).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, paths::display(&f.game.join("Desert.pak")));
    assert!(entries.iter().all(|e| e.enabled));
    assert_eq!(
        modlist::parse(modlist::render(&entries).unwrap().as_bytes(), &f.root).unwrap(),
        entries
    );
}
#[test]
fn utf16_le_import_and_bad_encoding() {
    let f = Fixture::new();
    let bytes: Vec<_> = [0xff, 0xfe]
        .into_iter()
        .chain("*Desert.pak\r\n".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    assert_eq!(modlist::parse(&bytes, &f.game).unwrap().len(), 1);
    assert!(modlist::decode(&[0xff, 0xfe, 0]).is_err());
    assert!(modlist::decode(&[0xfe, 0xff]).is_err());
}
#[test]
fn imports_missing_paths_but_rejects_duplicates_companions_and_controls() {
    let f = Fixture::new();
    assert_eq!(modlist::parse(b"missing.pak", &f.game).unwrap().len(), 1);
    for bytes in [
        b"a.pak\nA.PAK".as_slice(),
        b"a.utoc",
        b"**a.pak",
        b"a\0.pak",
    ] {
        assert!(modlist::parse(bytes, &f.game).is_err());
    }
    assert!(modlist::decode(&vec![b'x'; modlist::MAX_LIST_BYTES + 1]).is_err());
}
#[test]
fn relative_parent_paths_resolve_against_original_game_mods_folder() {
    let f = Fixture::new();
    let entries = modlist::parse(b"*../Mods/Foo.pak", &f.game.join("ConanSandbox/Mods")).unwrap();
    assert_eq!(
        paths::key(Path::new(&entries[0].path)),
        paths::key(&f.game.join("ConanSandbox/Mods/Foo.pak"))
    );
}
#[test]
fn export_skips_disabled_without_losing_order() {
    let f = Fixture::new();
    let mut p = f.profile();
    p.entries.push(Entry {
        path: paths::display(&f.root.join("missing.pak")),
        enabled: false,
    });
    assert_eq!(
        modlist::render(&p.entries).unwrap(),
        format!("*{}\r\n", paths::display(&f.pak))
    );
}
#[test]
fn scans_companion_bundles_orphans_and_duplicate_roots_without_writes() {
    let f = Fixture::new();
    fs::write(f.pak.with_extension("ucas"), b"data").unwrap();
    fs::write(f.pak.with_extension("UTOC"), b"index").unwrap();
    fs::write(f.pak.parent().unwrap().join("Orphan.ucas"), b"orphan").unwrap();
    let result = scanner::scan(&[
        paths::display(f.pak.parent().unwrap()),
        paths::display(&f.root),
    ])
    .unwrap();
    assert_eq!(result.mods.len(), 1);
    assert_eq!(result.mods[0].files.len(), 3);
    assert_eq!(result.mods[0].workshop_id.as_deref(), Some("123"));
    assert!(result.mods[0].warnings.is_empty());
    assert_eq!(result.orphan_files.len(), 1);
    assert!(!f.target().exists());
}
#[test]
fn incomplete_companions_and_empty_files_are_visible_but_block_apply() {
    let f = Fixture::new();
    fs::write(f.pak.with_extension("ucas"), b"data").unwrap();
    assert!(!scanner::package(&f.pak).unwrap().warnings.is_empty());
    assert!(deployment::prepare(f.profile(), &f.game).is_err());
    fs::write(f.pak.with_extension("utoc"), b"").unwrap();
    assert!(deployment::prepare(f.profile(), &f.game).is_err());
}
#[test]
fn modern_and_legacy_vdf_libraries_with_escaped_paths() {
    let modern = r#""libraryfolders" { "0" { "path" "C:\\Steam" "apps" { "440900" "100" } } "1" { "path" "D:\\Steam Library" } }"#;
    assert_eq!(
        detection::library_paths(modern).unwrap(),
        vec![
            PathBuf::from(r"C:\Steam"),
            PathBuf::from(r"D:\Steam Library")
        ]
    );
    assert_eq!(
        detection::library_paths("// hi\n\"libraryfolders\" { \"1\" \"E:/Steam\" }").unwrap(),
        vec![PathBuf::from("E:/Steam")]
    );
    assert!(detection::library_paths("\"libraryfolders\" { \"1\"").is_err());
}
#[test]
fn detects_secondary_library_manifest_and_workshop() {
    let f = Fixture::new();
    let steam = f.root.join("steam");
    let library = f.root.join("library");
    fs::create_dir_all(steam.join("steamapps")).unwrap();
    fs::create_dir_all(library.join("steamapps/common/Custom Conan/ConanSandbox")).unwrap();
    let exe = library
        .join("steamapps/common/Custom Conan")
        .join(detection::GAME_EXE);
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, b"fixture").unwrap();
    fs::create_dir_all(library.join("steamapps/workshop/content/440900")).unwrap();
    let escaped = paths::display(&library).replace('\\', "\\\\");
    fs::write(
        steam.join("steamapps/libraryfolders.vdf"),
        format!("\"libraryfolders\" {{ \"1\" {{ \"path\" \"{escaped}\" }} }}"),
    )
    .unwrap();
    fs::write(
        library.join("steamapps/appmanifest_440900.acf"),
        "\"AppState\" { \"appid\" \"440900\" \"installdir\" \"Custom Conan\" }",
    )
    .unwrap();
    let result = detection::detect_from(vec![steam]);
    assert_eq!(result.game_roots.len(), 1);
    assert_eq!(result.workshop_roots.len(), 1);
    assert!(result.warnings.is_empty());
}
#[test]
fn database_persists_order_toggles_settings_catalog_and_rejects_stale_save() {
    let f = Fixture::new();
    let db = f.data.join("test.sqlite3");
    let store = Store::open(&db).unwrap();
    let mut p = store.create("Server A").unwrap();
    p.entries = f.profile().entries;
    p.entries[0].enabled = false;
    let saved = store.save(&p).unwrap();
    assert!(store.save(&p).is_err());
    assert!(store.create("server a").is_err());
    store
        .set_settings(&Settings {
            game_root: paths::display(&f.game),
            scan_roots: vec![paths::display(&f.root)],
        })
        .unwrap();
    store
        .put(
            "catalog",
            &scanner::scan(&[paths::display(&f.root)]).unwrap(),
        )
        .unwrap();
    drop(store);
    let store = Store::open(&db).unwrap();
    assert_eq!(store.profile(saved.id).unwrap(), saved);
    assert_eq!(store.settings().unwrap().game_root, paths::display(&f.game));
    assert_eq!(
        store.catalog().unwrap()["mods"].as_array().unwrap().len(),
        1
    );
}
#[test]
fn invalid_profile_does_not_replace_saved_profile_and_last_profile_cannot_be_deleted() {
    let f = Fixture::new();
    let mut store = Store::open(&f.data.join("db")).unwrap();
    let before = store.profile(1).unwrap();
    let mut invalid = before.clone();
    invalid.name = " ".into();
    assert!(store.save(&invalid).is_err());
    assert_eq!(store.profile(1).unwrap(), before);
    assert!(store.delete(1, 0).is_err());
    let next = store.create("extra").unwrap();
    store.delete(next.id, next.revision).unwrap();
    assert_eq!(store.profiles().unwrap().len(), 1);
}
#[test]
fn preview_is_read_only_even_when_mods_folder_is_absent() {
    let f = Fixture::new();
    fs::remove_dir(f.game.join("ConanSandbox/Mods")).unwrap();
    let p = f.plan();
    assert!(p.view.changed);
    assert!(!f.game.join("ConanSandbox/Mods").exists());
    assert!(!f.data.join("backups").exists());
}
#[test]
fn explicit_apply_backs_up_exact_original_and_never_changes_workshop() {
    let f = Fixture::new();
    let original = b"\xef\xbb\xbf*old.pak\r\n";
    fs::write(f.target(), original).unwrap();
    let pak_before = fs::read(&f.pak).unwrap();
    let stamp_before = scanner::stamp(&f.pak).unwrap();
    let plan = f.plan();
    let expected = plan.view.after.clone();
    assert_eq!(expected, "*Desert.pak\r\n");
    assert!(plan.view.files[0].copy);
    let receipt = deployment::apply(plan, &f.profile(), &f.data, true).unwrap();
    assert_eq!(fs::read(f.target()).unwrap(), expected.as_bytes());
    assert_eq!(
        fs::read(f.target().with_file_name("Desert.pak")).unwrap(),
        pak_before
    );
    let backup = PathBuf::from(receipt.backup_dir.unwrap());
    assert_eq!(fs::read(backup.join("original.txt")).unwrap(), original);
    assert!(backup.join("applied.json").is_file());
    assert_eq!(fs::read(&f.pak).unwrap(), pak_before);
    assert_eq!(scanner::stamp(&f.pak).unwrap(), stamp_before);
    assert!(!f.game.join("ConanSandbox/Mods/.cemm-apply.lock").exists());
}
#[test]
fn first_apply_records_that_no_previous_modlist_existed() {
    let f = Fixture::new();
    let receipt = deployment::apply(f.plan(), &f.profile(), &f.data, true).unwrap();
    let backup = PathBuf::from(receipt.backup_dir.unwrap());
    assert!(!backup.join("original.txt").exists());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(backup.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["previouslyExisted"], false);
}
#[test]
fn unconfirmed_and_stale_modlist_changes_do_not_write() {
    let f = Fixture::new();
    assert!(deployment::apply(f.plan(), &f.profile(), &f.data, false).is_err());
    assert!(!f.target().exists());
    let plan = f.plan();
    fs::write(f.target(), b"external change").unwrap();
    assert!(deployment::apply(plan, &f.profile(), &f.data, true).is_err());
    assert_eq!(fs::read(f.target()).unwrap(), b"external change");
    assert!(!f.data.join("backups").exists());
}
#[test]
fn changed_profile_and_changed_mods_expire_preview() {
    let f = Fixture::new();
    let mut current = f.profile();
    current.revision += 1;
    assert!(deployment::apply(f.plan(), &current, &f.data, true).is_err());
    let plan = f.plan();
    fs::write(&f.pak, b"updated by Steam").unwrap();
    assert!(deployment::apply(plan, &f.profile(), &f.data, true).is_err());
    assert!(!f.target().exists());
}
#[test]
fn missing_enabled_files_block_preview_but_disabled_files_do_not() {
    let f = Fixture::new();
    let mut p = f.profile();
    p.entries.push(Entry {
        path: paths::display(&f.root.join("absent.pak")),
        enabled: true,
    });
    assert!(deployment::prepare(p.clone(), &f.game).is_err());
    p.entries[1].enabled = false;
    assert!(deployment::prepare(p, &f.game).is_ok());
}
#[test]
fn no_op_apply_has_no_new_backup() {
    let f = Fixture::new();
    fs::write(f.target(), b"*Desert.pak\r\n").unwrap();
    fs::copy(&f.pak, f.target().with_file_name("Desert.pak")).unwrap();
    let receipt = deployment::apply(f.plan(), &f.profile(), &f.data, true).unwrap();
    assert!(!receipt.changed);
    assert!(receipt.backup_dir.is_none());
    assert!(!f.data.join("backups").exists());
}
#[test]
fn apply_copies_companions_and_preserves_unrelated_and_disabled_mods() {
    let f = Fixture::new();
    fs::write(f.pak.with_extension("ucas"), b"payload").unwrap();
    fs::write(f.pak.with_extension("utoc"), b"index").unwrap();
    let unrelated = f.target().with_file_name("Other.pak");
    fs::write(&unrelated, b"keep me").unwrap();
    let mut profile = f.profile();
    profile.entries.push(Entry {
        path: paths::display(&unrelated),
        enabled: false,
    });
    let plan = deployment::prepare(profile.clone(), &f.game).unwrap();
    assert_eq!(plan.view.files.len(), 3);
    deployment::apply(plan, &profile, &f.data, true).unwrap();
    for extension in ["pak", "ucas", "utoc"] {
        assert_eq!(
            fs::read(f.pak.with_extension(extension)).unwrap(),
            fs::read(f.target().with_file_name(format!("Desert.{extension}"))).unwrap()
        );
    }
    assert_eq!(fs::read(unrelated).unwrap(), b"keep me");
    assert_eq!(fs::read(f.target()).unwrap(), b"*Desert.pak\r\n");
}
#[test]
fn matching_load_list_still_installs_missing_packages_and_reuses_identical_files() {
    let f = Fixture::new();
    fs::write(f.target(), b"*Desert.pak\r\n").unwrap();
    let plan = f.plan();
    assert!(plan.view.changed);
    deployment::apply(plan, &f.profile(), &f.data, true).unwrap();
    let plan = f.plan();
    assert!(!plan.view.changed);
    assert!(!plan.view.files[0].copy);
}
#[test]
fn filename_conflicts_and_unexpected_companions_block_preview_without_overwrites() {
    let f = Fixture::new();
    let target = f.target().with_file_name("Desert.pak");
    fs::write(&target, b"previous version").unwrap();
    assert!(
        deployment::prepare(f.profile(), &f.game)
            .unwrap_err()
            .contains("different content")
    );
    assert_eq!(fs::read(&target).unwrap(), b"previous version");
    fs::copy(&f.pak, &target).unwrap();
    fs::write(target.with_extension("utoc"), b"stale index").unwrap();
    assert!(
        deployment::prepare(f.profile(), &f.game)
            .unwrap_err()
            .contains("Unexpected companion")
    );
    assert!(!f.target().exists());
    assert!(!f.data.join("backups").exists());
}
#[test]
fn two_enabled_versions_with_same_filename_block_preview() {
    let f = Fixture::new();
    let another = f.root.join("Desert.pak");
    fs::copy(&f.pak, &another).unwrap();
    let mut profile = f.profile();
    profile.entries.push(Entry {
        path: paths::display(&another),
        enabled: true,
    });
    assert!(
        deployment::prepare(profile, &f.game)
            .unwrap_err()
            .contains("same filename")
    );
}
#[test]
fn source_content_change_with_same_timestamp_expires_preview() {
    let f = Fixture::new();
    let plan = f.plan();
    let modified = fs::metadata(&f.pak).unwrap().modified().unwrap();
    let bytes = fs::read(&f.pak).unwrap();
    fs::write(&f.pak, vec![b'x'; bytes.len()]).unwrap();
    fs::File::options()
        .write(true)
        .open(&f.pak)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    assert_eq!(scanner::stamp(&f.pak).unwrap(), plan.sources[0]);
    assert!(deployment::apply(plan, &f.profile(), &f.data, true).is_err());
    assert!(!f.target().exists());
    assert!(!f.target().with_file_name("Desert.pak").exists());
}
#[test]
fn destination_appearance_or_change_after_preview_blocks_apply() {
    let f = Fixture::new();
    let target = f.target().with_file_name("Desert.pak");
    let plan = f.plan();
    fs::copy(&f.pak, &target).unwrap();
    assert!(deployment::apply(plan, &f.profile(), &f.data, true).is_err());
    let plan = f.plan();
    fs::write(&target, b"external change").unwrap();
    assert!(deployment::apply(plan, &f.profile(), &f.data, true).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"external change");
    assert!(!f.target().exists());
    assert!(!f.data.join("backups").exists());
}
#[test]
fn profile_can_reference_existing_game_packages_without_copying_them() {
    let f = Fixture::new();
    let target = f.target().with_file_name("Desert.pak");
    fs::copy(&f.pak, &target).unwrap();
    let mut profile = f.profile();
    profile.entries[0].path = paths::display(&target);
    let plan = deployment::prepare(profile.clone(), &f.game).unwrap();
    assert!(!plan.view.files[0].copy);
    let stamp = scanner::stamp(&target).unwrap();
    deployment::apply(plan, &profile, &f.data, true).unwrap();
    assert_eq!(scanner::stamp(&target).unwrap(), stamp);
}
#[test]
fn empty_profile_warns_before_disabling_all_mods() {
    let f = Fixture::new();
    let mut p = f.profile();
    p.entries.clear();
    let plan = deployment::prepare(p, &f.game).unwrap();
    assert!(plan.view.after.is_empty());
    assert!(plan.view.warnings.iter().any(|w| w.contains("ALL mods")));
}
#[test]
fn export_is_create_only_and_cannot_bypass_apply_or_write_workshop() {
    let f = Fixture::new();
    let dest = f.root.join("export.txt");
    deployment::export(&f.profile(), &dest).unwrap();
    let old = fs::read(&dest).unwrap();
    assert!(deployment::export(&f.profile(), &dest).is_err());
    assert_eq!(fs::read(&dest).unwrap(), old);
    assert!(deployment::export(&f.profile(), &f.target()).is_err());
    assert!(deployment::export(&f.profile(), &f.pak.parent().unwrap().join("list.txt")).is_err());
    assert!(deployment::export(&f.profile(), &f.pak).is_err());
}
#[test]
fn workshop_destinations_and_parent_traversal_are_rejected() {
    let f = Fixture::new();
    let fake = f.pak.parent().unwrap().join("game");
    fs::create_dir_all(fake.join("ConanSandbox")).unwrap();
    assert!(deployment::prepare(f.profile(), &fake).is_err());
    assert!(paths::writable_path(&f.root.join("..\\outside.txt")).is_err());
}
#[test]
fn backup_failure_and_existing_apply_lock_leave_original_intact() {
    let f = Fixture::new();
    fs::write(f.target(), b"before").unwrap();
    fs::write(f.data.join("backups"), b"blocked").unwrap();
    assert!(deployment::apply(f.plan(), &f.profile(), &f.data, true).is_err());
    assert_eq!(fs::read(f.target()).unwrap(), b"before");
    fs::write(f.game.join("ConanSandbox/Mods/.cemm-apply.lock"), b"held").unwrap();
    assert!(deployment::apply(f.plan(), &f.profile(), &f.data, true).is_err());
    assert_eq!(fs::read(f.target()).unwrap(), b"before");
}
#[test]
fn backend_requires_issued_preview_and_invalidates_it_on_save() {
    let f = Fixture::new();
    let mut engine = Engine::open(&f.data).unwrap();
    assert!(
        engine
            .dispatch(Action::Apply {
                token: "forged".into(),
                confirmed: true
            })
            .is_err()
    );
    engine
        .dispatch(Action::SaveSettings {
            settings: Settings {
                game_root: paths::display(&f.game),
                scan_roots: vec![],
            },
        })
        .unwrap();
    let mut p = engine.store.profile(1).unwrap();
    p.entries = f.profile().entries;
    let p: Profile =
        serde_json::from_value(engine.dispatch(Action::SaveProfile { profile: p }).unwrap())
            .unwrap();
    let preview = engine.dispatch(Action::Preview { id: 1 }).unwrap();
    engine.dispatch(Action::SaveProfile { profile: p }).unwrap();
    assert!(
        engine
            .dispatch(Action::Apply {
                token: preview["token"].as_str().unwrap().into(),
                confirmed: true
            })
            .is_err()
    );
    assert!(!f.target().exists());
}
#[test]
fn real_command_flow_import_scan_preview_apply_and_reopen_sqlite() {
    let f = Fixture::new();
    let import = f.root.join("import.txt");
    fs::write(&import, modlist::render(&f.profile().entries).unwrap()).unwrap();
    let mut engine = Engine::open(&f.data).unwrap();
    engine
        .dispatch(Action::SaveSettings {
            settings: Settings {
                game_root: paths::display(&f.game),
                scan_roots: vec![paths::display(f.pak.parent().unwrap())],
            },
        })
        .unwrap();
    let scan = engine.dispatch(Action::Scan).unwrap();
    assert_eq!(scan["mods"].as_array().unwrap().len(), 1);
    engine
        .dispatch(Action::ImportModlist {
            path: paths::display(&import),
            base: paths::display(&f.game),
            id: 1,
            revision: 0,
        })
        .unwrap();
    let preview = engine.dispatch(Action::Preview { id: 1 }).unwrap();
    assert!(!f.target().exists());
    engine
        .dispatch(Action::Apply {
            token: preview["token"].as_str().unwrap().into(),
            confirmed: true,
        })
        .unwrap();
    drop(engine);
    let mut engine = Engine::open(&f.data).unwrap();
    assert_eq!(
        engine.dispatch(Action::Bootstrap).unwrap()["profiles"][0]["entries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn process_name_filter_covers_clients_and_servers_without_matching_manager() {
    assert!(deployment::is_game_process(
        "ConanSandbox-Win64-Shipping.exe"
    ));
    assert!(deployment::is_game_process("ConanSandboxServer.exe"));
    assert!(!deployment::is_game_process("conan-mod-manager.exe"));
    deployment::ensure_game_closed().unwrap();
}
#[cfg(windows)]
#[test]
fn junction_destinations_are_blocked_and_scans_skip_junctions() {
    let f = Fixture::new();
    let link = f.root.join("redirected");
    let output = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(paths::display(f.pak.parent().unwrap()).replace('/', "\\"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(paths::writable_path(&link.join("list.txt")).is_err());
    let scan = scanner::scan(&[paths::display(&link)]).unwrap();
    assert!(scan.mods.is_empty());
    assert!(!scan.warnings.is_empty());
    fs::remove_dir(&link).unwrap();
}
#[test]
fn scan_missing_folder_reports_the_problem() {
    let f = Fixture::new();
    let scan = scanner::scan(&[paths::display(&f.root.join("missing"))]).unwrap();
    assert!(scan.mods.is_empty());
    assert_eq!(scan.warnings.len(), 1);
}
#[test]
fn database_newer_schema_is_not_modified() {
    let f = Fixture::new();
    let file = f.data.join("future.db");
    let db = rusqlite::Connection::open(&file).unwrap();
    db.pragma_update(None, "user_version", 99).unwrap();
    drop(db);
    assert!(Store::open(&file).is_err());
}

#[cfg(windows)]
#[test]
fn alternate_streams_and_device_write_paths_are_rejected() {
    let f = Fixture::new();
    for name in ["pak.bin:export.txt", "NUL.txt", "CON", "trailing. "] {
        assert!(paths::writable_path(&f.root.join(name)).is_err());
    }
    assert!(paths::writable_path(Path::new(r"\\.\C:\file.txt")).is_err());
    assert!(paths::writable_path(Path::new(r"\\server\share\file.txt")).is_err());
}

#[test]
fn manual_selection_accepts_root_binary_directory_or_exact_executable() {
    let f = Fixture::new();
    for selection in [
        &f.game,
        &f.game.join("ConanSandbox/Binaries/Win64"),
        &f.game.join(detection::GAME_EXE),
    ] {
        assert_eq!(
            paths::key(&detection::resolve_game(selection).unwrap()),
            paths::key(&f.game)
        );
    }
    assert!(detection::resolve_game(&f.pak).is_err());
    assert!(detection::resolve_game(&f.root.join("missing")).is_err());
}

#[test]
fn common_games_folder_detection_requires_the_specific_executable() {
    let f = Fixture::new();
    let game = f.root.join("Games/Conan Exiles Enhanced");
    fs::create_dir_all(game.join("ConanSandbox/Binaries/Win64")).unwrap();
    let mut found = conan_mod_manager_lib::model::Detection::default();
    detection::add_common_installations(&mut found, std::slice::from_ref(&f.root));
    assert!(found.game_roots.is_empty());
    fs::write(game.join(detection::GAME_EXE), b"fixture").unwrap();
    detection::add_common_installations(&mut found, std::slice::from_ref(&f.root));
    assert_eq!(found.game_roots.len(), 1);
    detection::add_common_installations(&mut found, std::slice::from_ref(&f.root));
    assert_eq!(found.game_roots.len(), 1);
}

#[test]
fn startup_selects_one_game_preserves_manual_selection_and_never_guesses_between_two() {
    use conan_mod_manager_lib::model::Detection;
    let found = Detection {
        game_roots: vec![r"D:\Games\Conan Exiles Enhanced".into()],
        workshop_roots: vec![r"D:\SteamLibrary\steamapps\workshop\content\440900".into()],
        ..Default::default()
    };
    let selected = detection::startup_settings(Settings::default(), &found);
    assert_eq!(selected.game_root, found.game_roots[0]);
    assert_eq!(selected.scan_roots, found.workshop_roots);
    let manual = Settings {
        game_root: r"E:\Custom Conan".into(),
        scan_roots: vec![],
    };
    assert_eq!(
        detection::startup_settings(manual.clone(), &found).game_root,
        manual.game_root
    );
    let mut multiple = found;
    multiple.game_roots.push(r"E:\Another Conan".into());
    assert!(
        detection::startup_settings(Settings::default(), &multiple)
            .game_root
            .is_empty()
    );
}

#[test]
fn each_client_executable_is_detected_but_server_only_install_is_not() {
    let f = Fixture::new();
    let base = f.root.join("Games/Conan Exiles Enhanced");
    let bin = base.join("ConanSandbox/Binaries/Win64");
    fs::create_dir_all(&bin).unwrap();
    for name in detection::CLIENT_EXECUTABLES {
        let exe = bin.join(name);
        fs::write(&exe, b"fixture").unwrap();
        assert_eq!(
            paths::key(&detection::resolve_game(&exe).unwrap()),
            paths::key(&base)
        );
        let mut found = conan_mod_manager_lib::model::Detection::default();
        detection::add_common_installations(&mut found, std::slice::from_ref(&f.root));
        assert_eq!(found.game_roots.len(), 1);
        fs::remove_file(exe).unwrap();
    }
    fs::write(bin.join("ConanSandboxServer.exe"), b"fixture").unwrap();
    assert!(!detection::is_game_root(&base));
    assert!(deployment::is_game_process("ConanSandbox_BE.exe"));
    assert!(deployment::is_game_process("ConanSandbox.exe"));
}
