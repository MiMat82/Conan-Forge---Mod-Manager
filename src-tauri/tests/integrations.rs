use conan_mod_manager_lib::{
    Action, Engine,
    integrations::{self, OnlineAction, Progress, library, nexus, steamcmd},
    paths,
    storage::Store,
};
use serde_json::json;
use std::{
    fs::{self, File},
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

fn archive(root: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join("test.zip");
    let mut out = zip::ZipWriter::new(File::create(&path).unwrap());
    for (name, bytes) in entries {
        out.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        out.write_all(bytes).unwrap();
    }
    out.finish().unwrap();
    path
}
fn data(root: &Path) -> PathBuf {
    let p = root.join("data");
    fs::create_dir(&p).unwrap();
    Store::open(&p.join("manager.sqlite3")).unwrap();
    p
}

#[test]
fn zip_import_publishes_companions_deduplicates_and_keeps_originals_and_profiles() {
    let root = tempfile::tempdir().unwrap();
    let data = data(root.path());
    let zip = archive(
        root.path(),
        &[
            ("bundle/Mod.pak", b"pak fixture"),
            ("bundle/Mod.ucas", b"ucas"),
            ("bundle/Mod.utoc", b"utoc"),
            ("setup.exe", b"never execute or extract"),
            ("README.txt", b"skip"),
        ],
    );
    let original = fs::read(&zip).unwrap();
    let progress = Progress::default();
    let first = library::import(&data, &zip, "nexus", "10", "20", &progress).unwrap();
    assert_eq!(
        fs::read(Path::new(&first).join("bundle/Mod.utoc")).unwrap(),
        b"utoc"
    );
    assert!(!Path::new(&first).join("setup.exe").exists());
    assert_eq!(
        library::import(&data, &zip, "nexus", "10", "20", &progress).unwrap(),
        first
    );
    assert_eq!(fs::read(&zip).unwrap(), original);
    let store = Store::open(&data.join("manager.sqlite3")).unwrap();
    assert_eq!(store.acquisitions().unwrap().len(), 1);
    assert!(store.profiles().unwrap()[0].entries.is_empty());
    assert_eq!(store.acquisitions().unwrap()[0].mod_id, "10");
    assert!(fs::read_dir(data.join("staging")).unwrap().next().is_none());
}
#[test]
fn new_versions_are_separate_and_changed_managed_duplicates_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let data = data(root.path());
    let p = root.path().join("Mod.pak");
    fs::write(&p, b"version one").unwrap();
    let progress = Progress::default();
    let first = library::import(&data, &p, "local", "", "", &progress).unwrap();
    fs::write(&p, b"version two").unwrap();
    let second = library::import(&data, &p, "local", "", "", &progress).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        fs::read(Path::new(&first).join("Mod.pak")).unwrap(),
        b"version one"
    );
    fs::write(Path::new(&second).join("Mod.pak"), b"tampered").unwrap();
    assert!(
        library::import(&data, &p, "local", "", "", &progress)
            .unwrap_err()
            .contains("changed")
    );
}
#[test]
fn archive_rejects_traversal_streams_collisions_devices_and_missing_companions() {
    for name in [
        "../escape.pak",
        "a/../../escape.pak",
        "C:/escape.pak",
        "/absolute.pak",
        "a\\..\\escape.pak",
        "a.pak:stream",
        "a./m.pak",
        "CON.pak",
    ] {
        let root = tempfile::tempdir().unwrap();
        let data = data(root.path());
        let zip = archive(root.path(), &[(name, b"malicious")]);
        assert!(
            library::import(&data, &zip, "local", "", "", &Progress::default()).is_err(),
            "{name}"
        );
        assert!(!data.join("library").exists());
    }
    for entries in [
        vec![("A.pak", b"pak".as_slice()), ("a.pak", b"pak".as_slice())],
        vec![("A.pak", b"pak".as_slice()), ("A.ucas", b"ucas".as_slice())],
        vec![("orphan.utoc", b"utoc".as_slice())],
    ] {
        let root = tempfile::tempdir().unwrap();
        let data = data(root.path());
        let zip = archive(root.path(), &entries);
        assert!(library::import(&data, &zip, "local", "", "", &Progress::default()).is_err());
    }
}
#[test]
fn cancelled_and_oversized_copies_do_not_publish() {
    let root = tempfile::tempdir().unwrap();
    let data = data(root.path());
    let p = root.path().join("Mod.pak");
    fs::write(&p, b"pak").unwrap();
    let progress = Progress::default();
    progress.cancel.store(true, Ordering::Relaxed);
    assert!(
        library::import(&data, &p, "local", "", "", &progress)
            .unwrap_err()
            .contains("Cancelled")
    );
    assert!(!data.join("library").exists());
    let mut bytes = b"too big".as_slice();
    let mut dest = vec![];
    let mut total = library::MAX_BYTES;
    assert!(
        library::copy_bounded(&mut bytes, &mut dest, &Progress::default(), &mut total).is_err()
    );
    assert!(dest.is_empty());
}
#[test]
fn imports_workshop_by_copy_without_writing_workshop() {
    let root = tempfile::tempdir().unwrap();
    let data = data(root.path());
    let source = root.path().join("steamapps/workshop/content/440900/123");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("Mod.pak"), b"workshop original").unwrap();
    let imported =
        library::import(&data, &source, "steam", "123", "1", &Progress::default()).unwrap();
    assert!(Path::new(&imported).starts_with(&data));
    assert_eq!(
        fs::read(source.join("Mod.pak")).unwrap(),
        b"workshop original"
    );
    assert_eq!(fs::read_dir(source).unwrap().count(), 1);
}
#[test]
fn provider_identifiers_cannot_inject_commands_or_change_game() {
    assert!(
        integrations::provider_id("nexus", "https://www.nexusmods.com/conanexiles/mods/155")
            .is_err()
    );
    assert_eq!(
        integrations::provider_id(
            "steam",
            "https://steamcommunity.com/sharedfiles/filedetails/?id=123&searchtext=x"
        )
        .unwrap(),
        "123"
    );
    assert_eq!(
        integrations::provider_id(
            "nexus",
            "https://www.nexusmods.com/conanexilesenhanced/mods/155?tab=files"
        )
        .unwrap(),
        "155"
    );
    for value in [
        "0",
        "-1",
        "1 +quit",
        "1\n2",
        "18446744073709551616",
        "https://steamcommunity.com.evil.test/sharedfiles/filedetails/?id=1",
    ] {
        assert!(integrations::provider_id("steam", value).is_err());
    }
    assert!(integrations::provider_id("nexus", "https://www.nexusmods.com/skyrim/mods/1").is_err());
    assert!(integrations::site_url("file", None).is_err());
    assert!(integrations::site_url("steam", Some("12&x=y")).is_err());
}

#[test]
fn archive_links_and_corrupt_zip_files_never_publish() {
    let root = tempfile::tempdir().unwrap();
    let data = data(root.path());
    let path = root.path().join("links.zip");
    let mut out = zip::ZipWriter::new(File::create(&path).unwrap());
    out.add_symlink(
        "link.pak",
        "../outside.pak",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    out.finish().unwrap();
    assert!(
        library::import(&data, &path, "local", "", "", &Progress::default())
            .unwrap_err()
            .contains("links")
    );
    fs::write(&path, b"not a ZIP").unwrap();
    assert!(library::import(&data, &path, "local", "", "", &Progress::default()).is_err());
    assert!(!data.join("library").exists());
}

#[test]
fn cancellation_during_a_copy_stops_before_the_next_chunk() {
    struct Reader {
        progress: Progress,
        calls: usize,
    }
    impl Read for Reader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.calls += 1;
            buffer[0] = 42;
            self.progress.cancel.store(true, Ordering::Relaxed);
            Ok(1)
        }
    }
    let progress = Progress::default();
    let mut reader = Reader {
        progress: progress.clone(),
        calls: 0,
    };
    let mut out = vec![];
    let mut total = 0;
    assert!(
        library::copy_bounded(&mut reader, &mut out, &progress, &mut total)
            .unwrap_err()
            .contains("Cancelled")
    );
    assert_eq!(reader.calls, 1);
    assert_eq!(out, vec![42]);
}
#[test]
fn steam_metadata_rejects_foreign_games_and_unavailable_items() {
    let mut v = json!({"response":{"publishedfiledetails":[{"result":1,"consumer_app_id":440900,"publishedfileid":"123","title":"Test mod","file_size":"42","time_updated":1234}]}});
    let item = steamcmd::map_item(&v, "123").unwrap();
    assert_eq!(item.size, 42);
    assert_eq!(item.updated, 1234);
    assert!(steamcmd::map_item(&v, "456").is_err());
    v["response"]["publishedfiledetails"][0]["consumer_app_id"] = json!(123);
    assert!(steamcmd::map_item(&v, "123").is_err());
}
#[test]
fn nxm_authorization_is_bound_to_file_account_expiry_and_cdn() {
    let link = "nxm://conanexilesenhanced/mods/12/files/34?key=secret&expires=200&user_id=56";
    assert_eq!(
        nexus::nxm_query(link, "12", "34", 56, 100).unwrap().len(),
        2
    );
    for (mod_id, file_id, user, now) in [
        ("13", "34", 56, 100),
        ("12", "35", 56, 100),
        ("12", "34", 57, 100),
        ("12", "34", 56, 200),
    ] {
        assert!(nexus::nxm_query(link, mod_id, file_id, user, now).is_err());
    }
    assert!(nexus::nxm_query(&format!("{link}&key=second"), "12", "34", 56, 100).is_err());
    assert!(nexus::cdn_url("https://premium-files.nexusmods.com/path?token=secret").is_ok());
    for u in [
        "http://premium-files.nexusmods.com/file",
        "https://127.0.0.1/file",
        "https://nexusmods.com.evil.test/file",
        "https://user:pass@premium-files.nexusmods.com/file",
        "file:///C:/secret",
    ] {
        assert!(nexus::cdn_url(u).is_err());
    }
    assert_eq!(nexus::map_mod(&json!({"mod_id":1,"name":"A","summary":"<script>plain text</script>","updated_timestamp":123})).unwrap().summary,"<script>plain text</script>");
}
#[test]
fn provider_http_errors_and_response_limits_do_not_expose_secrets() {
    for (status, body) in [
        ("403 Forbidden", "secret signed URL and key".to_string()),
        ("200 OK", "x".repeat(2_097_153)),
        ("200 OK", "not json".to_string()),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = s.read(&mut request);
            let _ = write!(
                s,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        });
        let response = integrations::client(5)
            .unwrap()
            .get(format!("http://{address}"))
            .send()
            .unwrap();
        let error = integrations::json_response(response).unwrap_err();
        assert!(!error.contains("secret"));
        server.join().unwrap();
    }
}
#[test]
fn migration_preserves_phase_one_profiles_and_settings() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("manager.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE profiles(id INTEGER PRIMARY KEY,name TEXT NOT NULL UNIQUE COLLATE NOCASE,revision INTEGER NOT NULL DEFAULT 0,entries TEXT NOT NULL DEFAULT '[]');CREATE TABLE state(key TEXT PRIMARY KEY,value TEXT NOT NULL);INSERT INTO profiles VALUES(7,'My original profile',3,'[]');INSERT INTO state VALUES('settings','{\"gameRoot\":\"\",\"scanRoots\":[]}');PRAGMA user_version=1;").unwrap();
    drop(db);
    let store = Store::open(&path).unwrap();
    assert_eq!(store.profile(7).unwrap().revision, 3);
    assert_eq!(store.profile(7).unwrap().name, "My original profile");
    assert!(store.settings().unwrap().scan_roots.is_empty());
    assert!(store.acquisitions().unwrap().is_empty());
    assert_eq!(
        store
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}
#[test]
fn background_import_completes_and_engine_scan_includes_managed_library() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let source = root.path().join("Mod.pak");
    fs::write(&source, b"fixture").unwrap();
    let mut engine = Engine::open(&data).unwrap();
    engine
        .dispatch(Action::Online {
            action: OnlineAction::Import {
                path: paths::display(&source),
            },
        })
        .unwrap();
    let start = Instant::now();
    loop {
        let status = engine
            .dispatch(Action::Online {
                action: OnlineAction::Status,
            })
            .unwrap();
        if status["job"]["running"] == false {
            assert!(status["job"]["error"].is_null(), "{status}");
            assert_eq!(status["history"].as_array().unwrap().len(), 1);
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        engine.dispatch(Action::Scan).unwrap()["mods"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(engine.store.profiles().unwrap()[0].entries.is_empty());
}
