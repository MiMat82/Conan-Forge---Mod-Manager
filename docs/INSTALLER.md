# Windows setup — v0.4.1

## Install

1. Download **ConanForge-v0.4.1-windows-x64-setup.exe** from the [official release](https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/tag/v0.4.1).
2. Run the setup wizard, review the license and choose an installation folder.
3. Finish setup and open **ConanForge Mod Manager** from the Windows Start menu.

The installer is for Windows x64 and installs for your current Windows user. If Microsoft Edge WebView2 is missing, setup downloads Microsoft's bootstrapper and installs the runtime; this requires internet access. The installer and app are currently unsigned, so Windows may display an unknown-publisher warning.

Existing profiles and the imported library stay under `%LOCALAPPDATA%\com.local.conanmodmanager`. Setup installs the manager and documentation; it does not activate mods or alter the game or subscribed Workshop files. Use **Activate Mods** inside the app when you want to review and apply a load order.

Use **Windows Settings → Apps → Installed apps → ConanForge Mod Manager → Uninstall** to remove the installed program. Keep any option to remove application data unchecked if you want to retain your profiles, imports and backups. Game-installed mod copies are separate from the manager installation.

The portable Windows ZIP remains available for users who prefer extraction to installation. Close an existing portable or installed ConanForge before running another copy.

## Build this installer

The installer is an additional package for the existing v0.4.1 release. The original release tag, portable ZIP and source ZIP remain unchanged. Its application executable is byte-for-byte the same as the published portable executable; only the installer configuration and accompanying documentation are new.

1. Run `npm ci` in this checkout.
2. Extract `ConanForge.exe` from the official v0.4.1 Windows ZIP and place a copy at `src-tauri/target/release/conan-mod-manager.exe`. Preserve any local development executable separately first.
3. Run `npm run installer:bundle` on Windows. The script checks the application SHA-256 before and after bundling. Tauri downloads its NSIS build tools if needed.
4. The setup executable is written under `src-tauri/target/release/bundle/nsis/`.

This packaging command does not compile the application. It uses `--no-binary-patching` so the executable's release hash remains intact. The expected v0.4.1 SHA-256 is `c68d6813646c9041210cea60efc9976d6f38e8750b2769dd7aff4ca541e7279c`. A rebuilt or modified binary must be qualified as a separate application build before distribution.

Configuration: `src-tauri/tauri.installer.conf.json`. The packaging checkout includes the app's MIT license, third-party notices, sound/dependency licenses and illustrated guide as installed resources.

The installer uses [Tauri's NSIS packaging](https://v2.tauri.app/distribute/windows-installer/) and its download-bootstrapper WebView2 installation mode. A clean Windows machine without WebView2 is a separate verification boundary from an installation test on a development PC.
