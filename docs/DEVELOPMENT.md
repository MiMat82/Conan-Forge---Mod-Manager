# ConanForge Mod Manager

Public preview, version 0.5.1. Windows desktop application for Conan Exiles Enhanced using Tauri 2, Rust, React + TypeScript, Vite and bundled SQLite. This is an independent manager; it is not affiliated with Funcom, Valve or Nexus Mods.

The supplied ConanForge logo now appears in the manager and Windows application icon. Startup shows a transparent floating logo with three brief forge strikes, then opens the manager. Click **Skip intro**, press **Escape** or **Enter** to open immediately. Reduced-motion preferences are respected, and audio/page failure has a fallback. Existing profiles, settings and app-data location are retained. See [branding and sound credits](BRANDING.md).

## Discover and import

Open **Discover & import**. No mod provider is contacted automatically at startup. A separate app-update check contacts GitHub by default and can be disabled in App updates.

- **Steam Workshop:** paste a Workshop item URL or ID to see its title, description, size and update date. **Browse website** opens Steam's full catalog. **Download to library** automatically finds SteamCMD or downloads Valve's official launcher if missing. Windows verifies its signature and Valve publisher before it is cached or run. The manager copies the verified launcher to a new private working folder, runs it anonymously, waits for updater children, then imports the downloaded packages. The original SteamCMD installation and subscribed Workshop folders are not modified. Some items require account ownership; use Steam itself for those, then add its Workshop folder as a read-only scan source.
- **Nexus Mods:** open the Enhanced catalog in the browser, download a compatible mod there, then import its ZIP/PAK or extracted folder. The public build does not request API keys. Direct Nexus API features remain available only behind the private `nexus-testing` Cargo feature pending registration and account qualification.
- **Local imports:** choose a ZIP, a PAK with its adjacent companion files, or an extracted mod folder. Extract 7z/RAR archives manually first. Only `.pak`, `.ucas` and `.utoc` files are copied; archive executables are never extracted or run. ZIP paths, symlinks, collisions, companion completeness, sizes and CRCs are checked. Limits: 10,000 ZIP entries and 16 GiB of package data per import.
- **Managed library:** imports are staged, hashed and then published into a new app-owned directory. Duplicate content reuses an intact existing copy. Different versions get separate paths; existing profiles do not switch versions automatically. Successful imports and source IDs are recorded in SQLite and appear in **Mod library** after a scan. Add the desired version to your profile, save, preview, and explicitly apply.
- One transfer runs at a time per app instance, with status and cancellation. The game and profiles remain unchanged by downloads/imports. SteamCMD has a 30-minute limit; its own updater/download disk usage is not capped by the subsequent 16 GiB import limit. Nexus transfers have a 30-minute overall limit and 30-second stalled-read limit. Cancellation during provider metadata/connection setup can take up to 30 seconds.

Public releases must be built with default features. Do not enable `nexus-testing` in redistributed packages; that feature is for private development and the Nexus registration review only. See [provider details](PROVIDERS.md).

## Run the app

Download the EXE installer from the [v0.5.1 release](https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/tag/v0.5.1), run setup, then use the Start menu shortcut. The portable ZIP is also available. Windows Authenticode signing is not yet included. In-app updates verify a separate cryptographic signature tied to the installer version. See [installer instructions](INSTALLER.md) and the [publisher workflow](PUBLISHING.md).

To develop from source, install Node.js 26, Rust via rustup, and the Visual Studio C++ Build Tools/Windows SDK. The project pins Rust 1.99.1; it does not change your global toolchain.

```powershell
npm ci
npm run tauri dev
```

Build a standalone Windows executable:

```powershell
npm run desktop:build
```

The binary is `src-tauri/target/release/conan-mod-manager.exe`. No separate frontend server is required for the built app. `npm run dev` alone opens a browser-only preview; file operations require Tauri.

Run `npm run check` for frontend tests, the production web build and Rust tests.
Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
and `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` for Rust checks.
After a frontend build, `npm run test:ui` runs 19 headless Chrome checks with fictional
native responses and saves results/screenshots under ignored `release/qa/`. Set
`CHROME_PATH` if Chrome is not in its usual Windows Program Files folder. This does
not call providers or modify a real collection, and is not a native-desktop test.

## What works

- Automatic startup discovery through Steam registry entries, modern/legacy `libraryfolders.vdf`, app manifest 440900, and bounded common game folders on fixed Windows drives. It does not crawl entire drives.
- Recognizes `ConanSandbox.exe`, `ConanSandbox_BE.exe`, and `ConanSandbox-Win64-Shipping.exe` under `ConanSandbox/Binaries/Win64`. Server-only executables are not client detection candidates.
- One discovered installation is selected automatically on first use. Several installations require a choice. A saved manual selection takes priority. Use **Locations & storage → Choose folder… / Choose .exe…** to change it; a typed binary-folder/executable path also resolves to the game root when saved.
- Recursive read-only scans of local folders and Workshop `content/440900`. Groups same-stem `.pak`, `.ucas`, `.utoc` files, reports orphan companions, empty files, incomplete pairs, inaccessible folders and skipped redirects.
- SQLite-backed profiles with names, order, enabled flags, duplication and deletion; saved settings and the last library scan persist across restarts. Use up/down buttons for keyboard-accessible ordering.
- Import UTF-8 (with or without BOM) and BOM-marked UTF-16 LE modlists. Optional `*` prefixes and comments are accepted. Missing entries remain visible; duplicates and malformed entries are rejected. Relative paths resolve against an explicitly shown base folder.
- Export enabled entries, in order, to a **new** `.txt` file outside the game. Export never overwrites an existing file.
- Preview exact current/proposed load lists and every package copy. Explicit apply copies enabled packages into the game's Mods folder, backs up the original list, replaces it atomically, and verifies the result. Source packages, including Steam Workshop originals, are never modified.

## Workflow

1. Open the app. Review the detected game location, or select a folder/executable manually.
2. Add local or Workshop source folders in **Locations & storage** and select **Save locations & scan**. The game's `ConanSandbox/Mods` folder is scanned automatically if it already exists.
3. Add packages from **Mod library**, or import an existing `modlist.txt` into a profile. Save your profile changes.
4. Adjust order and enabled flags. **Activate Mods** saves any profile edits and opens the review. It validates all enabled paths and companion pairs, hashes their content, and lists copies/reused files and space needed for copies. Opening the review does not create a Mods folder or alter the game.
5. Close the game/server, back up important world saves, review the preview and confirmation checkbox, then choose **Back up & apply**. The app independently checks known running client/server process names.

Apply copies enabled `.pak` files and their same-stem `.ucas`/`.utoc` companions into `ConanSandbox/Mods`, then writes `modlist.txt` using `*ModName.pak` entries in profile order with Windows line endings. Restart Conan afterward to refresh its Mods menu. Identical destination files are reused; different content with the same filename, duplicate enabled filenames, and unexpected destination companions block apply. Package replacement/removal is deliberately not automatic. Disabled and unrelated files remain in the Mods folder but are not added to the active list.

Profiles and exports keep their original absolute source paths. A future source update does not change the deployed copy; applying a different version with the same filename currently requires resolving the destination conflict first. An empty profile disables every mod and requires the same explicit preview/confirmation.

Game version and compatibility remain **unverified**. Package extensions and executable names do not establish Enhanced compatibility, dependencies, asset conflicts or load-order correctness. Live gameplay must be checked separately.

## Storage and recovery

Application data: `%LOCALAPPDATA%\com.local.conanmodmanager\`.

- `manager.sqlite3`: profiles, settings, last scan and completed acquisition history. Schema 2 upgrades schema 1 without replacing profiles/settings; older 0.1 builds cannot reopen the upgraded database.
- `tools/steamcmd/steamcmd.exe`: verified launcher cache. Setup checks this first, then PATH, beside the app and common SteamCMD folders on fixed drives. It does not crawl whole disks or download anything at startup.
- `library/transfer-*`: completed immutable-by-convention managed package versions. Keep these folders while profiles reference them.
- `staging/transfer-*`: private in-progress download/extraction folders. Normal failures/cancellation remove them. A crash can leave staging folders; they are never automatically scanned. Close all app instances before manually removing abandoned staging data.
- `backups/app-updates/before-update-*/manager.sqlite3`: complete SQLite snapshot taken before explicit app-update installation.
- `backups/apply-*/original.txt`: exact previous bytes, if a previous list existed.
- `proposed.txt`, `manifest.json`: proposal, target, profile, timestamp, source/destination copy plan and SHA-256 hashes.
- `applied.json`: written only after the final load list was verified. A missing receipt means completion is not established; inspect the live file against the manifest before taking action.

To restore a prior load order, import its `original.txt` into a profile, setting the base folder to the original game's `ConanSandbox/Mods` for relative entries. Preview and apply. This creates another backup. Byte-for-byte restoration of unusual encodings/comments can instead be done manually while the game is closed. When the original list did not exist, the manifest records `previouslyExisted: false`; no fake empty original is created.

If an interrupted apply leaves `.cemm-apply.lock` in the game's Mods folder, close all manager instances, inspect the backup and live modlist, then remove only that stale lock before retrying. Do not remove a lock while an apply is running.

Ordinary copy/list-commit failures roll back files newly installed by that apply, provided their content is still unchanged. Existing packages are never removed. A crash can leave completed copies or `.cemm-package-*` temporary files before the list is committed. Compare the live list and hashes with the retained manifest before manually cleaning up; do not delete files simply because the completion receipt is missing.

World saves, mod packages and their versions are **not** backed up. No retention cleanup deletes old backups automatically. See [Safety boundaries](SAFETY.md).

## Tests

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

The Rust tests create isolated synthetic game/Workshop directories. They never deploy to an installed game. They exercise actual SQLite persistence and actual Windows file replacement. `cargo run --manifest-path src-tauri/Cargo.toml --example detect` prints read-only discovery results on the current machine.

## Scope and architecture

The initially empty workspace is now a standalone Git repository. `src/App.tsx` and `src/domain.ts` own the interface and profile editing. `src-tauri/src/lib.rs` exposes one typed action boundary, using blocking workers so file operations do not block the webview event loop. Rust modules separate detection, scanning, parsing, storage, path validation and deployment. Tauri grants only the native dialogs needed; there is no frontend filesystem, shell or HTTP permission, and the release webview has a restrictive CSP.

`src-tauri/src/integrations/` contains provider APIs, the managed importer and background transfer state. `src/Discover.tsx` owns the provider interface. Transfers run outside the profile engine mutex; status/cancel remain available. Provider metadata is rendered as text, remote HTML/images are not loaded into the webview, and only fixed official browser destinations can be opened. App updates use `src-tauri/src/app_updates.rs` and `src/AppUpdates.tsx`: startup/manual checks, exact official asset URL validation, signed-version verification, installed-copy eligibility, explicit installation and a pre-install SQLite backup. The native command rejects installation during mod transfers. Frontend edits must be saved or discarded first. Server management, unattended installation, authenticated SteamCMD login, dependency/conflict analysis and in-game compatibility checking remain outside this phase.

Implementation reaches frontend callers, Rust command dispatch, SQLite schema version 2, Tauri/version configuration, dependency lockfiles, fixtures, and documentation. Existing game data remains outside the project. Public repository material was used as a behavioral reference, not copied implementation. See [Third-party notices](../THIRD_PARTY_NOTICES.md) and [validation results](PHASE2-VALIDATION.md).
