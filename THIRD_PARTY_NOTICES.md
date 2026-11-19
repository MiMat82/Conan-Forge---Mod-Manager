# Third-party notices

This is independently implemented software. The ConanForge branding uses the logo supplied by the project owner, with generated transparent and icon adaptations. The supplied artwork's provenance and trademark permissions have not been independently established. No game files, commercial mod payloads or Nexus screenshots are included.

## Startup sound and branding

- **"Smithing Sound" by Hansjörg Malthaner (Varkalandar)**: [original recording](https://opengameart.org/content/smithing-sound), [creator](https://opengameart.org/users/varkalandar). Used under [Creative Commons Attribution 3.0 Unported](https://creativecommons.org/licenses/by/3.0/); full license in `licenses/smithing-CC-BY-3.0.txt`. Changes: three strikes excerpted and retimed, fades, short echoes, and level adjustment. The result is `public/brand/forge-intro.wav`. Attribution is also embedded in the app's Locations & storage credits.
- Logo preparation and exact image prompts are recorded in [branding notes](docs/BRANDING.md). The logo request concerns this manager; Conan Exiles executables and their icons are not altered.

## Behavioral references

- [Vercadi/conan-exiles-enhanced-manager](https://github.com/Vercadi/conan-exiles-enhanced-manager): its README and [MIT license](https://github.com/Vercadi/conan-exiles-enhanced-manager/blob/main/LICENSE) were inspected. No implementation or artwork was copied.
- [MadBuffoon/Conan-Exiles-Tools](https://github.com/MadBuffoon/Conan-Exiles-Tools): referenced by the original brief. A root LICENSE could not be retrieved; reuse permission was not established, so no code or assets were copied.
- The [Nexus page supplied in the brief](https://www.nexusmods.com/conanexiles/mods/155) returned HTTP 403 during this session. No Nexus assets were downloaded or redistributed.
- [Tauri 2 documentation](https://v2.tauri.app/start/) informed the platform setup. The starter scaffold was generated with create-tauri-app 4.7.5; its Tauri-derived code is covered by the included Tauri MIT notice. Template branding has been replaced.
- [Funcom's Enhanced hotfix notes](https://forums.funcom.com/t/conan-exiles-enhanced-may-hotfixes/297230/1) explicitly discuss ignoring Legacy mods. The app therefore never treats file extensions or executable names as proof of Enhanced compatibility.

## Dependencies

Phase 2 uses independently written protocol clients based on Valve/Nexus documentation. Nexus's GPL Node client was not copied or linked. SteamCMD is discovered locally or fetched from Valve on request and is not bundled. A real Workshop mod was downloaded only into temporary verification storage; no mod payload is included in these deliverables. See [provider sources and terms](docs/PROVIDERS.md).

`THIRD_PARTY_LICENSES.txt` includes notices and license text collected from the locked Windows Cargo dependency graph and production npm packages. It also records exact unmodified source archive URLs. In particular, recipients can obtain MPL-covered dependency source from these URLs; no vendored dependency source was modified. Preserve this file when distributing the executable.

Some registry packages omit root license files. Their unmodified upstream license texts are supplied in `licenses/` and added by `scripts/collect-notices.mjs`:

- alloc-stdlib: [upstream BSD license](https://github.com/dropbox/rust-alloc-no-stdlib/blob/master/LICENSE).
- webview2-com, webview2-com-sys, webview2-com-macros: [upstream MIT license](https://github.com/wravery/webview2-rs/blob/main/LICENSE).
- tauri-plugin: MIT license from the installed Tauri crate, matching the [upstream MIT license](https://github.com/tauri-apps/tauri/blob/dev/LICENSE-MIT).
- selectors: Mozilla Public License 2.0, supplied from the installed cssparser crate's copy of that license. Exact selectors source is linked in the dependency notices.

Microsoft WebView2 runtime and SDK/loader components are subject to Microsoft's terms. The application uses the system WebView2 runtime. The EXE installer downloads Microsoft's runtime bootstrapper only when needed; no runtime installer is embedded. See [Microsoft WebView2 distribution documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution). SQLite's engine is public domain; the Rust bindings have their own included licenses.

## Installer components

The additional Windows setup package uses unmodified NSIS 3.13, including its LZMA decompressor, and Tauri's `nsis_tauri_utils` 0.5.3 plugin. License texts are included in `licenses/installer-nsis.txt` and `licenses/installer-tauri-utils-MIT.txt`. NSIS source is available from the [official NSIS repository, v313 tag](https://github.com/kichik/nsis/tree/v313); the plugin's exact source is available at [nsis_tauri_utils-v0.5.3](https://github.com/tauri-apps/nsis-tauri-utils/tree/nsis_tauri_utils-v0.5.3). These installer components retain their upstream licenses; they are not relicensed under the application's MIT license.

To regenerate the notices after changing dependencies:

```powershell
node scripts/collect-notices.mjs
```

The collector fails if a selected package lacks license text, rather than silently omitting it. The original application source and documentation use the MIT license in `LICENSE`. Third-party assets/dependencies retain their own terms; the source license does not grant trademark rights or relicense the startup sound or branding.
