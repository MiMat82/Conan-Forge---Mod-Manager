use conan_mod_manager_lib::{
    Engine,
    app_updates::{
        backup_before_install, registered_location_matches, startup_preference, validate_release,
    },
};
use std::fs;

fn url(v: &str) -> String {
    format!(
        "https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/download/v{v}/ConanForge-v{v}-windows-x64-setup.exe"
    )
}

#[test]
fn permits_only_newer_versions_from_the_exact_release_location() {
    assert!(validate_release("0.5.0", "0.6.0", &url("0.6.0")).is_ok());
    for version in ["0.4.1", "0.5.0", "0.5.0-beta.1", "not-a-version"] {
        assert!(validate_release("0.5.0", version, &url(version)).is_err());
    }
    for location in [
        url("0.6.0").replace("https:", "http:"),
        url("0.6.0").replace("github.com", "github.com.evil.test"),
        url("0.6.0") + "?redirect=x",
        url("0.7.0"),
        url("0.6.0").replace("MiMat82", "other"),
    ] {
        assert!(validate_release("0.5.0", "0.6.0", &location).is_err());
    }
}

#[test]
fn startup_checks_default_on_and_preference_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    assert!(startup_preference(&engine.store).unwrap());
    engine.store.put("app_update_startup", &false).unwrap();
    drop(engine);
    let reopened = Engine::open(dir.path()).unwrap();
    assert!(!startup_preference(&reopened.store).unwrap());
}

#[test]
fn only_the_registered_install_directory_is_eligible() {
    let dir = tempfile::tempdir().unwrap();
    let installed = dir.path().join("Installed app");
    let portable = dir.path().join("Installed app portable");
    fs::create_dir(&installed).unwrap();
    fs::create_dir(&portable).unwrap();
    let exe = installed.join("conan-mod-manager.exe");
    assert!(registered_location_matches(
        &exe,
        &format!("\"{}\"", installed.display())
    ));
    assert!(!registered_location_matches(
        &portable.join("conan-mod-manager.exe"),
        installed.to_str().unwrap()
    ));
    assert!(!registered_location_matches(
        &installed.join("ConanForge.exe"),
        installed.to_str().unwrap()
    ));
}

#[test]
fn install_confirmation_is_required_before_any_backup_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    assert!(backup_before_install(&engine, false).is_err());
    assert!(!dir.path().join("backups").exists());
}

#[test]
fn database_backup_is_readable_complete_and_keeps_prior_backups() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    let profile = engine.store.create("My saved order").unwrap();
    engine.store.put("app_update_startup", &false).unwrap();
    let first = backup_before_install(&engine, true).unwrap();
    let before = fs::read(first.join("manager.sqlite3")).unwrap();
    let second = backup_before_install(&engine, true).unwrap();
    assert_ne!(first, second);
    assert_eq!(before, fs::read(first.join("manager.sqlite3")).unwrap());
    let backup =
        conan_mod_manager_lib::storage::Store::open(&second.join("manager.sqlite3")).unwrap();
    assert_eq!(backup.profile(profile.id).unwrap(), profile);
    assert!(!startup_preference(&backup).unwrap());
    assert_eq!(engine.store.profile(profile.id).unwrap(), profile);
}

#[test]
fn backup_failure_prevents_the_install_preparation() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path()).unwrap();
    fs::write(dir.path().join("backups"), b"not a directory").unwrap();
    assert!(backup_before_install(&engine, true).is_err());
    assert_eq!(
        fs::read(dir.path().join("backups")).unwrap(),
        b"not a directory"
    );
}
