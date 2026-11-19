# ConanForge v0.4.1 — first public preview

19 November 2026 · Windows x64 · MIT source license

ConanForge brings mod discovery, a local library, ordered profiles and explicit activation into one Windows desktop app for Conan Exiles Enhanced.

## Download and start

1. Download **ConanForge-v0.4.1-windows-x64-setup.exe** from [this release](https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/tag/v0.4.1).
2. Run setup and open **ConanForge Mod Manager** from the Start menu. Setup downloads Microsoft WebView2 if missing (internet required). This preview is unsigned. The original portable ZIP remains available.
3. Check the detected game under **Locations & storage**, or choose its folder/executable manually.
4. Follow **Discover & import → Mod library → Load order → Activate Mods**. Review the file-copy plan, then confirm **Back up & apply** with the game closed.

[Illustrated instructions and troubleshooting](USER-GUIDE.md)

The source ZIP contains the same reviewed application source as the release tag. The installer wraps that exact executable with additional setup configuration and documentation; see [installer details](INSTALLER.md). The original tag and ZIPs remain unchanged. `SHA256SUMS.txt` includes installer and original package hashes. Use `Get-FileHash .\ConanForge-v0.4.1-windows-x64-setup.exe -Algorithm SHA256` and compare its result with that file.

## Included

- Startup game discovery and manual folder/executable selection.
- Read-only scans of local PAK packages, UCAS/UTOC companions and Steam Workshop folders.
- Steam Workshop item lookup and isolated SteamCMD downloads, with automatic discovery/setup of Valve's verified launcher when needed.
- Nexus website browsing and manual ZIP/PAK/folder import. Direct Nexus API access is disabled in this public preview pending application registration and account testing.
- SQLite-backed ordered profiles, load-list import/export, previewed package copies, original load-list backups and explicit Apply.
- Workshop update checks and reviewed deletion of unused imported library bundles. Saved profile references block deletion; original downloads, Workshop files and game-installed copies remain separate.
- ConanForge logo, Windows icon and skippable forge-sound startup.
- Illustrated guide, third-party notices and MIT-licensed original source/documentation.

## What has been checked

- 77 Rust tests and 9 frontend tests passed.
- Rust formatting, Clippy with warnings treated as errors, TypeScript and production frontend build passed.
- 11 browser interface checks passed against the production frontend using fictional native responses, including public Nexus controls and the deletion confirmation flow.
- Windows release build and package integrity checks are recorded in [release validation](RELEASE-VALIDATION-v0.4.1.md).

Screenshots use fictional example packages. Browser fixtures test the interface, not live provider downloads or gameplay. Earlier local previews were tested in the owner's game; this exact public package has not been qualified on a clean Windows machine or across mod combinations.

## Known limits

- Backups cover the previous `modlist.txt`, **not world saves**. Back up `ConanSandbox/Saved` separately before changing mods.
- Enhanced compatibility and load order depend on the mod author's instructions; the app does not resolve dependencies or certify compatibility.
- Different-content files with the same destination filename block Apply. Automatic replacement, package rollback and game-file uninstall are future work.
- Update checks do not download, replace or activate anything automatically. Steam metadata changes can also trigger an update notice. Nexus updates require visiting the source page in this public build.
- A process/power failure during library deletion may leave recoverable files in app staging; automatic maintenance recovery is not included yet.
- Windows x64 only. The installer is unsigned. No automatic app updater or bundled game/mod/SteamCMD files.

Original source and documentation use [MIT](../LICENSE). Sound, dependencies and trademarks retain their separate terms; keep the included notices with the package. ConanForge is an independent project, unaffiliated with Funcom, Valve or Nexus Mods.
