# Phase 2 validation — 18 November 2026

## Version 0.2.4 — ConanForge branding and startup

The manager uses the supplied logo in its sidebar, transparent animated startup window and Windows icon. The intro plays three edited smithing strikes before revealing the main window. Skip/Enter/Escape stop the intro, reduced-motion mode removes the animation, and frontend/native timeouts prevent a failed intro from indefinitely hiding the manager. Main-window commands remain restricted to the main window. The app identifier and database schema are unchanged.

The former **Preview changes** button is now **Activate Mods**, including when a profile has unsaved edits. It still saves edits and opens the installation review; actual deployment requires the existing explicit confirmation and **Back up & apply** action.

Checks passed: **64 Rust tests** (11 unit, 40 core, 13 integration), **8 frontend tests**, TypeScript/Vite build for both pages, clippy with warnings denied and Rust formatting. The two native startup tests check concurrent completion/skip/fallback idempotence and retry after a failed reveal. Five additional frontend tests exercise the actual splash module with fake host/media objects: audio completion, skipping before artwork loads, missing artwork/blocked audio fallback, Escape/Enter, and no late intro after native fallback. They do not simulate physical audio output. No test exclusions or game-running guard bypasses were used.

The logo's alpha channel was checked for genuine transparency and the artwork/icon visually reviewed. Tauri generated the Windows icon from the emblem master. Audio analysis found three events at 0.254, 1.150 and 2.158 seconds, all within 14 ms of the designed animation accents; true peak -5.6 dBTP, -24.2 LUFS, zero clipped samples. The app plays at 65% volume. This analysis does not establish physical speaker loudness or the user's preferred sound. Sound attribution, source and CC BY 3.0 license are included; see [branding notes](BRANDING.md).

Native v0.2.4 verification observed the actual transparent splash over the desktop, followed by the main window with its logo and emblem title-bar icon. The saved game path and Default profile with all three selected mods were present. The Activate Mods label appeared in the accessibility tree. The user then pressed physical Escape to stop Computer Use; no further native inputs were sent. Physical speaker playback and native Skip interaction were not verified. Packaged v0.2.4 SHA-256: `AFA5ACDC173D4054DEC848F811E94D557A852EAFFFECCB1B174D39B8B8572A96`.

## Version 0.2.2 — game Mods folder deployment

The reported problem was absence from Conan's Mods menu. Read-only inspection found both enabled packages in the managed library and absolute references in the game's modlist, but no packages in its Mods folder. The game's own 21:24–21:25 UTC log showed both `BuildCastlesLikeYouHaveNoLife` and `IMM_WAR` being extracted, mounted, and contributing asset-registry packages. This establishes that external paths loaded in this installation; it does not establish visibility in the in-game menu or gameplay correctness.

Apply now stages and copies enabled PAK/UCAS/UTOC files into `ConanSandbox/Mods`, retaining original filenames and writing a relative-filename modlist. The preview lists source, destination, copy/reuse status and byte size. Existing packages with identical SHA-256 content are reused; conflicting content or duplicate destination filenames block deployment. Original library/Workshop files and unrelated game files are preserved. No database migration or new dependencies were needed.

Checks passed: **62 Rust tests** (9 unit, 40 core, 13 integration), **3 frontend tests**, TypeScript/Vite production build, clippy across all targets with warnings denied, and Rust formatting. The complete Rust suite ran with the game closed; no exclusions or guard bypasses were needed. Ten added tests cover deployment with companions, unchanged Workshop originals, reuse/no-op, missing copies despite an unchanged list, filename/companion conflicts, source content changes with preserved metadata, changed destinations, in-place packages, and deterministic publication/list-commit rollback failures.

Tests use isolated temporary installations and synthetic payloads. Source hashes are checked during preview and apply, staged copies are verified, and only newly created unchanged files are removed on rollback. Crash/power-loss and physical full-disk faults are not qualified. Replacing/removing existing package versions is outside this fix and conflicts fail visibly.

The Windows release built and the packaged v0.2.2 executable launched successfully. Native UI inspection confirmed the saved game location, Default profile and both enabled mods survived the update. Preview showed two copies totaling 769,963,808 bytes (734.3 MiB) into `D:\Games\Conan Exiles Enhanced\ConanSandbox\Mods`, and the proposed list `*BuildCastlesLikeYouHaveNoLife.pak` then `*IMM_WAR.pak`. Apply stayed disabled until explicit confirmation; the agent left the preview open for the user. Preview itself left the live game list unchanged.

After that handoff, Apply completed in the running application. Read-only verification confirmed both game copies match their library originals by SHA-256: `4064A9DE524245A9A2E5BC6BFBFA1C213FE48B09295560149DBBE20D09E49617` for BuildCastlesLikeYouHaveNoLife and `5386EA2DBA934836646AF7C9C5868EC9AC113C92B5474BAD3B1E90217FD5ECB9` for IMM_WAR. The live list matches the proposed relative names in the same order, with SHA-256 `6462CD234B000E3EC71B472DB4DB8E4155BB4D9D3E9E97F8EC5597AFABE70511`. Backup `backups/apply-E9MoJa` has the exact prior list (`6F1D71E8495007627B34535E4920AA3E941C501B6DB3ED8059BFA7DB1067B260`), the package manifest, and `applied.json` reporting verified completion. No apply lock remained. The app was then minimized; no further UI inputs were sent. The new deployment has not yet been verified in Conan's Mods menu after a restart.

Packaged executable SHA-256: `4DF7CE9FD548E4C428B217F1EB50DF114B6CBEE7847F3FADF09A021E37C10167`. No extra third-party code or dependencies were added, and existing license notices were retained.

## Version 0.2.1 — automatic SteamCMD setup

The Steam download action now takes only the Workshop ID. The native file picker was removed. Setup checks the managed launcher cache, PATH, app-adjacent and common local SteamCMD folders, then downloads the official Valve ZIP if no trusted copy is available. It verifies the Windows signature, certificate trust/revocation and Valve publisher on staged/cached/execution copies. Existing installations are read and copied, never run in place or modified.

Six new setup tests cover finding a valid local candidate, unchanged original files, download fallback, cache reuse without downloading, failed/cancelled setup preserving prior cache, untrusted download rejection, malformed/traversing/oversized/multi-file/symlink installer archives, and native rejection of unsigned/non-Valve executables. The complete suite contains 52 Rust tests plus 3 frontend tests.

Final-code checks: 47 Rust tests passed (6 setup, 13 integration, 28 core), 3 frontend tests passed, TypeScript/Vite build passed, and clippy/format checks passed. A full-suite attempt encountered 5 expected environmental blocks: the user had started `ConanSandbox-Win64-Shipping.exe`, so the real game-running guard rejected fixture deployment calls and the test asserting an idle process list. Those five were explicitly excluded from the final rerun; the guard was not bypassed and the user's game was not stopped. All 52 had passed earlier in this change before the game started; that earlier run preceded the final bootstrap-retry adjustment.

A live smoke check started with a fresh temporary app-data directory and no configured SteamCMD path. Automatic acquisition, actual Windows Valve signature verification and cache reuse passed, followed by an anonymous Workshop download/import of Enhanced item 3759410117. One acquisition was recorded in the temporary database. No load order was applied. The example is reproducible with `cargo run --manifest-path src-tauri/Cargo.toml --example provider_smoke -- 3759410117 --download`.

The fresh-bootstrap test exposed exit code 7 after Valve's old launcher replaces itself. The downloader now retries once only if that launcher's bytes changed and its replacement passes the same Valve verification. The retry must exit successfully and produce valid packages before import. No provider error is otherwise ignored. Windows trust validation required closing the staging write handle first; verification now uses the closed private file.

Only the launcher is cached; Valve's runtime still initializes inside a fresh private working directory per transfer. Automatic discovery is bounded and may miss unusual directories, in which case the official-download fallback is used. Certificate checks may need network access and can delay cancellation. Clean-machine/network-failure behavior remains covered by the limits below.

Windows release 0.2.1 built successfully. SHA-256: `AB50F74D7533EA323553536334D3DC2FEA75F9A36948077747C22C9D0246B2C2`. The updated native button was not clicked again while the user was playing; the same backend path was exercised through the live smoke example and the frontend production build passed. No game files were changed.

## Version 0.2.0 — previous build

Version 0.2.0, Windows x64, Rust 1.99.1 MSVC, Node 26.13.1. Locked dependencies; personal testing build.

| Check | Observed result |
| --- | --- |
| Rust tests | 46 passed: 33 existing core tests plus 13 acquisition/provider tests |
| Frontend domain tests | 3 passed |
| TypeScript / Vite production build | Passed |
| Rust clippy, all targets, warnings denied | Passed |
| Rust formatting check | Passed |
| Final Windows release build | Built and launched the packaged v0.2.0 executable |
| Native startup / migration | Existing Default profile and saved `D:\Games\Conan Exiles Enhanced` location appeared after upgrade |
| Native live lookup | Workshop ID 3759410117 rendered its real title/description and download controls |
| Native Nexus screen | Correct Enhanced URL shown; API-key input masked and lookup disabled while disconnected |
| Native import picker | ZIP/PAK filter shown; picker cancelled without importing or applying |
| Dependency notices | 318 packages; zero missing selected license texts |
| Live Steam API | Public lookup of Workshop item 3759410117 returned VeloXs Tattoos - Enhanced, app 440900, 23,063,206 bytes |
| Real isolated SteamCMD download | Passed through the application's Rust downloader and importer, using a temporary database/library |
| Game apply | Not performed |
| Live authenticated Nexus downloads | Not tested; no personal API key was requested in chat or supplied to the agent |

The Steam smoke check used Valve's official Windows SteamCMD ZIP. The downloaded launcher had a valid Valve Authenticode signature. The app copied the launcher into its own staging folder, ran anonymous download, waited for the updater's child processes, found the downloaded PAK, imported it and recorded one acquisition. The smoke example's temporary data was removed when it exited. A separate diagnostic SteamCMD workspace and downloaded test item remain under this chat's `work/steamcmd-check`; neither is included in deliverables. Nothing was applied to the installed game.

Packaged executable SHA-256: `53CE289B088D7B76AE666DC9BEAED6F7BB529F76D8677A6EB4237E561FC1D643`.

An initial attempt exposed a real defect: SteamCMD's bootstrap process exited before its updated child finished. The runner now owns the suspended process in a Windows job before resuming it and waits for the job's active children. The repeated end-to-end download then passed. The primary process exit code is checked, and package presence/validation is required independently.

New tests exercise actual ZIP extraction, full companion preservation, payload fingerprints, duplicate reuse, separate version paths, tampered duplicate rejection, original Workshop preservation, cancellation before/during copying, size limits, directory traversal, alternate streams, reserved devices, case collisions, symlink entries, malformed archives, missing companions, strict provider IDs, foreign-game metadata, wrong/expired/wrong-account NXM links, CDN host restrictions, local HTTP error/body-limit handling without secret disclosure, schema-1 migration preserving profiles/settings, and the real background import/status/scan command flow. They use synthetic payloads and isolated temporary directories.

## Verification limits

- The new Nexus workflow follows official protocol documentation but needs live free/Premium account qualification. It may need adjustment for provider API/CDN changes or account-specific responses.
- Steam account login/Steam Guard, automatic updates, in-app full-text Workshop search, automatic 7z/RAR extraction, public Nexus app registration and code signing are not implemented.
- Cancellation of file copies is tested. Native process-tree ownership and updater completion were exercised by the successful real download; cancellation during an actual stalled Steam or Nexus network transfer was not fault-injected.
- The 0.2.0/0.2.1 agent-run smoke checks did not launch Conan. Later inspection for 0.2.2 found successful game mount records after the user applied and launched it. Compatibility for other mods, dependencies, mod safety, menu visibility and gameplay correctness remain separate checks.
- No clean Windows machine, physical power loss or disk-full fault injection was tested. Current game files were only read during verification.
- SQLite schema 2 preserves existing data, but version 0.1 cannot reopen the upgraded database. Use 0.2 or newer after upgrade.
