use super::{Progress, RemoteMod, client, json_response, library, number, numeric_id, text};
use crate::{model::Result, paths};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

pub fn map_item(value: &Value, expected: &str) -> Result<RemoteMod> {
    let item = value["response"]["publishedfiledetails"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("Steam returned no item")?;
    if number(item, "result") != 1
        || number(item, "consumer_app_id") != 440900
        || text(item, "publishedfileid", 20) != expected
    {
        return Err("Workshop item is unavailable or is not for Conan Exiles (440900).".into());
    }
    if number(item, "file_type") == 2 {
        return Err(
            "Collections are not individual mods. Open the collection and choose a mod.".into(),
        );
    }
    Ok(RemoteMod {
        provider: "steam".into(),
        id: expected.into(),
        title: text(item, "title", 200),
        summary: text(item, "description", 2000),
        updated: number(item, "time_updated"),
        size: number(item, "file_size"),
        files: vec![],
    })
}
pub fn lookup(id: &str) -> Result<RemoteMod> {
    let id = numeric_id(id)?;
    let result = json_response(
        client(30)?
            .post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
            .form(&[("itemcount", "1"), ("publishedfileids[0]", &id)])
            .send()
            .map_err(|_| "Could not reach Steam Workshop.")?,
    )?;
    map_item(&result, &id)
}
pub fn download(data: &Path, id: &str, progress: &Progress) -> Result<String> {
    let id = numeric_id(id)?;
    let remote = lookup(&id)?;
    if remote.size > library::MAX_BYTES {
        return Err("This item exceeds the 16 GiB managed-import limit.".into());
    }
    progress.check()?;
    let executable = super::steamcmd_setup::ensure_launcher(data, progress)?;
    let temp = library::staging(data)?;
    let launcher = temp.path().join("steamcmd.exe");
    fs::copy(executable, &launcher).map_err(|e| e.to_string())?;
    super::steamcmd_setup::verify_valve(&launcher)?;
    progress.check()?;
    progress.report("SteamCMD: initializing / downloading anonymously…", 0);
    let original_launcher = fs::read(&launcher).map_err(|e| e.to_string())?;
    let mut exit_code = run(&launcher, temp.path(), &id, progress)?;
    // Valve's original bootstrap returns 7 after replacing itself; retry only a changed, verified launcher.
    if exit_code == 7 && fs::read(&launcher).map_err(|e| e.to_string())? != original_launcher {
        super::steamcmd_setup::verify_valve(&launcher)?;
        progress.check()?;
        progress.report("SteamCMD updated; starting Workshop download…", 0);
        exit_code = run(&launcher, temp.path(), &id, progress)?;
    }
    if exit_code != 0 {
        return Err(format!(
            "SteamCMD exited with code {exit_code}. Check your connection and item access; account-required items must be downloaded through Steam."
        ));
    }
    progress.check()?;
    let relative = format!("steamapps/workshop/content/440900/{id}");
    let source=[temp.path().join(&relative),temp.path().join("payload").join(&relative)].into_iter().find(|p|p.is_dir()).ok_or("SteamCMD did not produce a package. The item may require Steam account ownership. Subscribe/download in Steam, then scan your existing Workshop folder.")?;
    library::import(
        data,
        &source,
        "steam",
        &id,
        &remote.updated.to_string(),
        progress,
    )
}
// A Windows job owns the suspended process before it starts, including updater children.
#[cfg(windows)]
pub fn run(executable: &Path, root: &Path, id: &str, progress: &Progress) -> Result<u32> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
                QueryInformationJobObject, SetInformationJobObject,
            },
            Threading::{
                CREATE_NO_WINDOW, CREATE_SUSPENDED, CreateProcessW, GetExitCodeProcess,
                PROCESS_INFORMATION, ResumeThread, STARTUPINFOW, WaitForSingleObject,
            },
        },
    };
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    let id = numeric_id(id)?;
    paths::writable_path(root)?;
    let exe = paths::display(executable);
    let cwd = paths::display(root);
    if exe.contains('"') || cwd.contains('"') {
        return Err("Invalid SteamCMD path".into());
    }
    let app = wide(&exe);
    let dir = wide(&cwd);
    let mut command = wide(&format!(
        "\"{exe}\" +force_install_dir \"{cwd}\\payload\" +login anonymous +workshop_download_item 440900 {id} validate +quit"
    ));
    unsafe {
        let job = Handle(CreateJobObjectW(std::ptr::null(), std::ptr::null()));
        if job.0.is_null() {
            return Err("Could not create isolated SteamCMD job.".into());
        }
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of_val(&limits) as u32,
        ) == 0
        {
            return Err("Could not constrain SteamCMD process lifetime.".into());
        }
        let mut startup: STARTUPINFOW = std::mem::zeroed();
        startup.cb = std::mem::size_of_val(&startup) as u32;
        let mut info: PROCESS_INFORMATION = std::mem::zeroed();
        if CreateProcessW(
            app.as_ptr(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_NO_WINDOW | CREATE_SUSPENDED,
            std::ptr::null(),
            dir.as_ptr(),
            &startup,
            &mut info,
        ) == 0
        {
            return Err("Could not run the selected SteamCMD executable.".into());
        }
        let process = Handle(info.hProcess);
        let thread = Handle(info.hThread);
        if AssignProcessToJobObject(job.0, process.0) == 0 {
            windows_sys::Win32::System::Threading::TerminateProcess(process.0, 1);
            return Err("Could not isolate SteamCMD; launch was stopped.".into());
        }
        if ResumeThread(thread.0) == u32::MAX {
            return Err("Could not start SteamCMD.".into());
        }
        let start = Instant::now();
        loop {
            progress.check()?;
            if start.elapsed() > Duration::from_secs(1800) {
                return Err(
                    "SteamCMD exceeded 30 minutes. Try downloading in Steam instead.".into(),
                );
            }
            if WaitForSingleObject(process.0, 200) == WAIT_OBJECT_0 {
                let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
                if QueryInformationJobObject(
                    job.0,
                    JobObjectBasicAccountingInformation,
                    &mut accounting as *mut _ as *mut _,
                    std::mem::size_of_val(&accounting) as u32,
                    std::ptr::null_mut(),
                ) == 0
                {
                    return Err("Could not inspect SteamCMD child processes.".into());
                }
                if accounting.ActiveProcesses == 0 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        let mut code = 1;
        if GetExitCodeProcess(process.0, &mut code) == 0 {
            return Err("Could not inspect SteamCMD's exit status.".into());
        }
        Ok(code)
    }
}
#[cfg(not(windows))]
pub fn run(_: &Path, _: &Path, _: &str, _: &Progress) -> Result<u32> {
    Err("SteamCMD downloads are supported by the Windows build.".into())
}
