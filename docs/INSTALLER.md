# Windows setup and app updates

## Install

1. Download **ConanForge-v0.5.1-windows-x64-setup.exe** from the [official release](https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/tag/v0.5.1).
2. Close any open ConanForge copy, run setup and choose an installation folder.
3. Open **ConanForge Mod Manager** from the Windows Start menu.

Setup installs for your current Windows user on Windows x64. If Microsoft Edge WebView2 is missing, setup downloads Microsoft's bootstrapper; this requires internet access. The app and setup do not yet have a Windows Authenticode certificate, so Windows may show an unknown-publisher warning. The separate updater signature is checked by ConanForge and does not remove that Windows warning.

Existing profiles and imported packages stay under `%LOCALAPPDATA%\com.local.conanmodmanager`. Setup installs the manager and documentation. Mod activation remains an explicit action inside the app.

**Upgrading from v0.4.1:** install the current version manually once. Older copies have no updater. **Upgrading from installed v0.5.0:** use **App updates → Download app update → Install and restart**. Portable copies can check for updates and open the releases page; use setup once to enable in-app installation.

## Update from the app

Startup checks are enabled by default. Checks contact the project's public GitHub update feed and notify you when a newer version is available. They do not download or install it. Turn off **Check for app updates at startup** to check manually instead.

1. Open **App updates**, or follow the notification's **Review app update** button.
2. Read the release notes and choose **Download app update**. ConanForge checks the installer's signature and its signed version.
3. Save unfinished profile/location changes and wait for any mod transfer to finish.
4. Select **Install and restart…**, tick the confirmation and choose **Install and restart**.

The app backs up its SQLite database under `backups/app-updates/before-update-*/manager.sqlite3`, closes, runs setup and reopens. This backup covers app settings, profiles and acquisition records; it is not a backup of mod packages or world saves. A failed backup or signature check prevents installation. Offline checks leave the app usable; try again later.

## Uninstall

Use **Windows Settings → Apps → Installed apps → ConanForge Mod Manager → Uninstall**. Leave any option to remove application data unchecked to keep profiles, imports and backups. Game-installed mod copies are separate from the manager installation.

## Publisher workflow

See [Publishing signed updates](PUBLISHING.md). The earlier v0.4.1 installer wrapped the existing portable executable without patching it. From v0.5.0 onward, build the current source and let Tauri mark the executable as an NSIS installation before signing the setup. Qualify the final packaged executable, not a pre-bundle copy.

The installer uses [Tauri NSIS packaging](https://v2.tauri.app/distribute/windows-installer/); updates use the [Tauri updater](https://v2.tauri.app/plugin/updater/). A clean machine without WebView2 remains a separate qualification boundary.
