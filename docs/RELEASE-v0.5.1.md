# ConanForge v0.5.1 — update verification release

This small Windows x64 preview adds the current version number to the **You're up to date** message. It provides the first follow-up release for exercising the complete in-app update from v0.5.0.

There are no changes to mod deployment, profile formats, storage locations, or the database schema.

## Upgrade from v0.5.0

1. Open **App updates** and choose **Check for app updates** if the startup notification has not appeared.
2. Choose **Download app update** and wait for **Verified and ready**.
3. Save any edits and let mod transfers finish. Click **Install and restart…**, tick the confirmation and confirm installation.
4. ConanForge backs up its database, installs the signed release and reopens. **App updates** then shows version **0.5.1**.

For first-time installation or v0.4.1, use [ConanForge-v0.5.1-windows-x64-setup.exe](https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/download/v0.5.1/ConanForge-v0.5.1-windows-x64-setup.exe). Portable copies also need setup once before using in-app installation. The `.exe.sig` is for the updater; it does not need to be opened manually.

The release includes the installer, its updater signature, a portable ZIP, source, checksums and a validation report. The live upgrade result is added to that report after the published update has been tested. Windows Authenticode signing and a clean-PC/WebView2 installation remain separate qualification work. No mods or game files are distributed.
