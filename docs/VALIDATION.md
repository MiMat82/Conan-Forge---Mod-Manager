# Phase 1 validation — historical baseline, 18 November 2026

Current version 0.3.0 results are in [Phase 3 validation](PHASE3-VALIDATION.md); earlier results are in [Phase 2 validation](PHASE2-VALIDATION.md). The report below records the earlier 0.1.0 build and its scope at that time.

Environment: Windows x64, Node 26.13.1, Rust 1.99.1 MSVC. Dependencies are pinned in `Cargo.lock` and `package-lock.json`.

| Check | Result |
| --- | --- |
| Rust test suite | 33 passed, 0 failed, 0 ignored |
| Frontend domain tests | 3 passed, 0 failed |
| TypeScript checking and production Vite build | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo fmt -- --check` | Passed |
| Windows release executable | Built with Tauri 2 |
| Native startup | Observed the actual Windows WebView2 application running |
| Actual machine discovery | Found `D:\Games\Conan Exiles Enhanced`, Steam app ID 440900; no Workshop source folder found |
| Startup selection | Native UI selected that installation automatically and persisted it |
| Read-only scan | Native UI reported 0 packages for the configured folders |
| Native executable picker | Opened successfully and cancelled; no executable was launched |
| Native preview | Displayed the real target path, empty-profile warning and disabled Apply button until confirmation; cancelled without writes |
| Dependency notices | 268 packages enumerated; no missing selected license text |

## Test coverage

The 33 Rust tests cover UTF-8/BOM/UTF-16 modlists, Unicode and relative paths, marker/comment handling, missing entries, disabled flags, ordering, duplicate paths, malformed input, complete/incomplete companion sets, orphan files, overlapping scan roots, missing folders, modern/legacy Steam VDF, manifest detection, common installation folders, each of the three client executable names, server-only rejection, manual file/folder selection, single/multiple startup candidates, saved selection precedence, SQLite persistence and schema guard, optimistic profile revisions, profile validation/deletion, read-only preview, explicit confirmation, stale files/profile/token rejection, actual Windows atomic apply, exact original-byte backup, no-op applies, first-write manifests, blocked backups/locks, Workshop write rejection, Windows junctions, alternate data streams/device paths, and the real command flow through scanning/import/preview/apply/reopen.

The frontend tests check immutable reordering with disabled flags, Windows case/separator duplicate handling, and path labels/import folders. TypeScript verifies the frontend build; Rust tests exercise the real backend actions rather than a mocked persistence layer.

## What this establishes

The application builds and runs on this Windows machine, and the safety/persistence operations pass against isolated temporary directories. The native UI has been observed using the real Rust backend. The supplied game executable path and `steam_appid.txt` were inspected read-only.

No apply was performed against the user's installed game. Its `ConanSandbox/Mods` folder was absent before development and remained absent at the final read-only check. Test `.pak`/companion files are synthetic bytes, not Unreal mods. Tests do not establish real game loading, Enhanced mod compatibility, world-save safety, game/server process races, physical power-loss recovery, arbitrary disk failures, a clean Windows install, or third-party provider functionality.

Online Nexus/Workshop browsing, SteamCMD downloads and Nexus API integration are not implemented; their module boundaries are present for Phase 2. The app can scan already downloaded/extracted mods from either source.

Packaged executable SHA-256: `7BAA096E11B9A35CB1E1A5A790F85970221132B841DCFCFE3EE9F8DF8207F171`.
