# Development roadmap

[Product page](../README.md) · [Illustrated guide](USER-GUIDE.md)

## Implemented in the preview

| Phase | Scope | Status |
| --- | --- | --- |
| 1 | Windows Tauri/Rust app, game discovery, local package scanning, SQLite profiles, modlist import/export, review and explicit apply with load-list backups | Implemented |
| 2 | Workshop lookup/download, automatic verified SteamCMD setup, managed local imports, deployment into the game's Mods folder | Public build uses Nexus browser downloads/manual import; direct API remains testing-only |
| Branding | Supplied ConanForge logo, Windows icon, transparent skippable startup with smithing sounds | Implemented |
| 3 | Forge-inspired styling, visible 1–2–3 workflow, selected-versus-applied language, persistent activation action and completion/restart guidance | Implemented in v0.3.0 |
| 4 | Reviewed deletion of unused imported bundles; per-bundle update checks; clear managed/external copy labels | Workshop checks included; Nexus checks disabled in public builds |
| 5 | App-update startup notification, signed downloads, explicit installation/restart and database backup | v0.5.0 public preview |
| First release | Windows EXE installer, portable ZIP, source, MIT license, checksums, public Nexus boundary and illustrated documentation | v0.4.1 public preview |

The project owner has reported successful in-game use of local imports and Workshop downloads. See [phase 4 validation](PHASE4-VALIDATION.md) for the latest test scope. Earlier UI checks with documentation fixtures exercise selection, preview and explicit confirmation; fixtures are not proof of live provider behavior.

## Before a stable release

- Finish clean-machine Windows and WebView2 qualification.
- Complete [public-distribution preparation for Nexus](https://help.nexusmods.com/article/114-api-acceptable-use-policy) and test authorized free/Premium account routes.
- Continue dependency and asset/license reviews as the release changes.
- Add Windows Authenticode signing and continue qualifying upgrades across future releases. Signed in-app updates are implemented in v0.5.0.

The first public release is a preview. A stable-release date is not promised.

## Next planned phase: reviewed version replacement

Download the chosen update as a separate library version, show which profiles and installed files it would replace, back up affected packages and the load list, then require explicit Apply. Add restore/rollback before allowing replacement of different-content game files. A library deletion must remain separate from uninstalling a mod from the game.

## Possible later work

Server profiles, sharing portable profile definitions, bulk update checks, game-file uninstall previews and deeper compatibility/dependency assistance are future candidates. They are not presented as available features.

Updates must preserve the current principle: the user can review and explicitly approve changes before game files are changed.
