# Development roadmap

[Product page](../README.md) · [Illustrated guide](USER-GUIDE.md)

## Implemented in the local preview

| Phase | Scope | Status |
| --- | --- | --- |
| 1 | Windows Tauri/Rust app, game discovery, local package scanning, SQLite profiles, modlist import/export, review and explicit apply with load-list backups | Implemented |
| 2 | Workshop lookup/download, automatic verified SteamCMD setup, Nexus API workflow, managed local imports, deployment into the game's Mods folder | Implemented; live Nexus account qualification remains |
| Branding | Supplied ConanForge logo, Windows icon, transparent skippable startup with smithing sounds | Implemented |
| 3 | Forge-inspired styling, visible 1–2–3 workflow, selected-versus-applied language, persistent activation action and completion/restart guidance | Implemented in v0.3.0 |

The project owner has reported successful in-game use of local imports and a Workshop download. Development checks include 64 Rust tests and 8 frontend tests. UI checks with documentation fixtures exercise selection, preview and explicit confirmation; fixtures are not proof of live provider behavior.

## Before a public application release

- Finish clean-machine Windows and WebView2 qualification.
- Complete [public-distribution preparation for Nexus](https://help.nexusmods.com/article/114-api-acceptable-use-policy) and test authorized free/Premium account routes.
- Choose a source license and confirm redistribution arrangements for branding.
- Prepare release packaging and decide on executable signing.

No public binary release or release date is promised here.

## Possible later work

Mod update checking and version management, server profiles, sharing portable profile definitions, and deeper compatibility/dependency assistance are future candidates. They are not presented as available features.

Updates must preserve the current principle: the user can review and explicitly approve changes before game files are changed.
