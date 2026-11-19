use conan_mod_manager_lib::{
    integrations::{Progress, steamcmd, steamcmd_setup},
    storage::Store,
};

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let id = args
        .get(1)
        .ok_or("Usage: provider_smoke WORKSHOP_ID [--download]")?;
    let item = steamcmd::lookup(id)?;
    println!(
        "Live Steam metadata: {} / {} / {} bytes",
        item.id, item.title, item.size
    );
    if args.get(2).is_some_and(|s| s == "--download") {
        let data = tempfile::tempdir().map_err(|e| e.to_string())?;
        Store::open(&data.path().join("manager.sqlite3"))?;
        let progress = Progress::default();
        let launcher = steamcmd_setup::ensure_launcher(data.path(), &progress)?;
        println!("Automatic SteamCMD setup and Valve signature check succeeded.");
        assert_eq!(
            steamcmd_setup::ensure_launcher(data.path(), &progress)?,
            launcher
        );
        println!("Verified launcher cache was reused.");
        let path = match steamcmd::download(data.path(), id, &progress) {
            Ok(path) => path,
            Err(e) => {
                println!("Download rejected safely: {e}");
                return Err(e);
            }
        };
        println!("Isolated download/import succeeded: {path}");
        println!(
            "Completed acquisitions: {}",
            Store::open(&data.path().join("manager.sqlite3"))?
                .acquisitions()?
                .len()
        );
    }
    Ok(())
}
