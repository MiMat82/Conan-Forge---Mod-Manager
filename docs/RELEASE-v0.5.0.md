# ConanForge v0.5.0 — app updates

Windows x64 public preview. [Download the installer](https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/download/v0.5.0/ConanForge-v0.5.0-windows-x64-setup.exe).

## New

- **App updates** in the sidebar, with checking enabled at startup by default. A notification appears when a newer version is available. The setting can be turned off.
- Release notes, a separate download action, progress, and explicit **Install and restart** confirmation.
- Verification of the installer's signature and signed version. Only newer releases from the official ConanForge release location are accepted.
- A SQLite database backup before installation. Unsaved profile/location edits and active mod transfers prevent installation.
- Repeatable signed-release tooling and a documented publisher workflow.

**Install v0.5.0 manually once if you currently use v0.4.1.** That version has no updater. Installed v0.5.0 copies can update to future releases from inside the app. Portable copies can check and open GitHub, but must use setup to enable in-app installation.

Your existing profiles, imported library and game files are retained. App updates do not update or activate mods. Workshop originals remain read-only. Nexus continues to use browser downloads and manual imports in public builds.

## Downloads

- `ConanForge-v0.5.0-windows-x64-setup.exe`: recommended per-user Windows installer.
- The matching `.exe.sig`: updater signature; normal users do not need to open it.
- `ConanForge-v0.5.0-windows-x64.zip`: portable app and documentation.
- `ConanForge-v0.5.0-source.zip`: source snapshot with licenses.
- `SHA256SUMS.txt`: SHA-256 checksums for release downloads.

The updater signature is separate from Windows Authenticode signing. Windows can still display an unknown-publisher warning. A clean Windows machine without WebView2, all provider/account combinations and mod compatibility remain outside the verification evidence for this preview. See the accompanying release validation report and [installation guide](INSTALLER.md).
