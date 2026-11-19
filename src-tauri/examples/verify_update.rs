//! Verify a prepared release with the same updater implementation used by the app.
//! No app engine, webview, installer, or user database is opened.
use std::{env, fs};
use tauri_plugin_updater::UpdaterExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    let online = args.len() == 2 && args[1] == "--check-online";
    if args.len() != 3 && !online {
        return Err("Usage: verify_update <manifest.json> <setup.exe> OR --check-online".into());
    }
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    *context.config_mut() = serde_json::from_str(include_str!("../tauri.conf.json"))?;
    context.config_mut().app.windows.clear();
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)?;
    let updater = app
        .handle()
        .updater_builder()
        .timeout(std::time::Duration::from_secs(20))
        // Release verification must accept the build's own version too.
        .version_comparator(|_, _| true)
        .build()?;
    if online {
        let update = tauri::async_runtime::block_on(updater.check())?
            .ok_or("Endpoint returned no release content")?;
        println!(
            "Online feed version: {}. Asset: {}",
            update.version, update.download_url
        );
        return Ok(());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let mut payload = fs::read(&args[2])?;
    let update = updater
        .restore_update(metadata.clone())?
        .ok_or("Missing update")?;
    conan_mod_manager_lib::app_updates::validate_release(
        "0.0.0",
        &update.version,
        update.download_url.as_str(),
    )?;
    update.verify(&payload)?;
    println!(
        "PASS: official release URL, installer signature and signed version {}",
        update.version
    );
    let last = payload.last_mut().ok_or("Empty installer")?;
    *last ^= 1;
    assert!(
        update.verify(&payload).is_err(),
        "Tampered installer accepted"
    );
    *payload.last_mut().unwrap() ^= 1;
    let mut relabeled = metadata.clone();
    relabeled["version"] = serde_json::json!("999.0.0");
    let wrong_version = updater.restore_update(relabeled)?.ok_or("Missing update")?;
    assert!(
        wrong_version.verify(&payload).is_err(),
        "Relabeled version accepted"
    );
    let mut bad_signature = metadata;
    bad_signature["platforms"]["windows-x86_64"]["signature"] = serde_json::json!("invalid");
    let bad_update = updater
        .restore_update(bad_signature)?
        .ok_or("Missing update")?;
    assert!(
        bad_update.verify(&payload).is_err(),
        "Invalid signature accepted"
    );
    println!("PASS: changed installer, relabeled version and invalid signature rejected");
    Ok(())
}
