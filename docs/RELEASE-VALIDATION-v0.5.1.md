# Live in-app upgrade validation — v0.5.0 to v0.5.1

19 November 2026. Tested on the project owner's Windows x64 PC using the publicly released installer and the existing installed v0.5.0 application. This was the actual desktop update flow, not a browser fixture or a manual run of the new setup executable.

## Observed result

| Step | Result |
| --- | --- |
| Public installer and feed | Anonymous downloads matched local SHA-256; valid installer/version accepted by the updater; changed bytes, wrong version and invalid signature rejected |
| Discover update | Installed v0.5.0 offered v0.5.1 through the public GitHub feed |
| Download | The app downloaded and verified the update; installation did not start automatically |
| Cancel | **Not now** closed the confirmation and kept v0.5.0 open; all preserved state remained unchanged |
| Install | The app's **Install and restart** checkbox and button were used; an automatic SQLite backup was created before setup |
| Automatic restart | Original process 13544 exited; v0.5.1 process 9724 appeared automatically at 19:20:54 CET with installer parent process 18348, before any manual launch of the new app |
| Installed build | Product version and Installed apps registration reported 0.5.1; the executable matched the expected NSIS-marked build hash |
| Reopened interface | Main window responsive; real native status returned 0.5.1, installed eligibility and startup checking enabled; a real GitHub check reported up to date |
| Resources and shortcuts | All 41 packaged documentation/license resources and both Windows shortcut targets verified |
| User data | All SQLite table rows and schema unchanged; one profile with eight entries and eight acquisition records retained |
| Mod/save preservation | All 82 library, game-Mods, game-Saved and existing-backup files retained their SHA-256, size and modification time |
| Automatic backup | Exactly one new app-update database snapshot; SQLite integrity check passed and all rows matched the pre-update database |

The preservation baseline included verified copies of approximately 4.3 GB of user files and the previous installation. These backups remain local and are not distributed. No mod Apply, import, deletion, or game launch was performed during the test.

## Test instrumentation and limits

Immediately after publication the installed app still reported no newer version. A later normal check offered v0.5.1; this delay occurred before any download or installation. A publisher diagnostic now reports the version served to the actual updater client.

The monitor reattached to the existing process after that delay. Its exit-code assertion stopped because an attached PowerShell `Process` did not expose an exit code. A controlled helper reproduced a null attached exit code alongside a known exit code 0. The old app's numeric exit code therefore remains unobserved; it is not claimed as 0. Independent readback established the automatic new process, installed version/hash, responsive interface and preserved data. The app was not manually launched to make the automatic-restart test pass.

The automated suite also passed: 84 Rust tests, 9 frontend tests, 19 browser flow checks, production build, Rust formatting and Clippy. Browser checks use fictional native responses; the upgrade evidence above comes from the real installed app.

This qualifies this Windows machine and this published version pair. A clean PC without WebView2, interrupted installation/power loss, Windows Authenticode signing and all provider/mod combinations remain separate work. Full download checksums and screenshots are attached to the [v0.5.1 release](https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/tag/v0.5.1).
