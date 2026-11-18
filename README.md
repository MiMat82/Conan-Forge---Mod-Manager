<p align="center">
  <img src="docs/images/banner.webp" alt="ConanForge Mod Manager. Forge your next adventure. Find and import, choose and order, review and activate." width="100%">
</p>

<h1 align="center">ConanForge Mod Manager</h1>
<p align="center"><strong>A Windows workspace for Conan Exiles Enhanced mods.</strong><br>Bring in your mods, build a load order, and review every change before it reaches your game.</p>
<p align="center">
  <a href="docs/USER-GUIDE.md"><strong>Illustrated user guide</strong></a> ·
  <a href="#availability">Availability</a> ·
  <a href="docs/ROADMAP.md">Development roadmap</a> ·
  <a href="https://github.com/MiMat82/Conan-Forge---Mod-Manager/issues">Feedback</a>
</p>

---

## Your collection, under your control

![ConanForge's load-order screen showing selected and unselected mods, the three-step workflow, and the Activate Mods button.](docs/images/app-overview.webp)

*Actual v0.3.0 interface, rendered with fictional example packages and neutral folders for documentation. These are not mod recommendations or evidence of a provider download.*

| Find your mods | Shape your load order | Review and activate |
| --- | --- | --- |
| Look up Steam Workshop items by URL or ID. Import authorized Nexus downloads, ZIPs, PAKs or extracted folders. | Keep ordered profiles, tick the mods you want, and move them with the arrow buttons. | See the file-copy plan and load-list changes. Confirm explicitly before applying. |
| SteamCMD is found or set up automatically when needed. | Saving a profile does **not** activate it. | The previous load list is backed up; Workshop originals stay untouched. |

## From download to your next adventure

**1 · Find & import**  
Open **Discover & import**. Use a Workshop URL or item number, the Nexus workflow, or files already on your PC. Downloads enter the manager's library.

**2 · Choose & order**  
Open **Mod library**, click **+ Add to profile**, then open **Load order**. Tick the packages to activate and arrange them top to bottom.

**3 · Review & activate**  
Click **Activate Mods**. Review the proposed changes, close the game, tick the confirmation and choose **Back up & apply**. Restart Conan to load the selection.

[Follow the illustrated guide, with numbered click targets →](docs/USER-GUIDE.md)

## Look inside

| Connect your game | Find a Workshop mod |
| --- | --- |
| [![Game detection and manual folder selection, numbered 1 to 3.](docs/images/01-game-location.webp)](docs/USER-GUIDE.md#1-connect-your-game) | [![Workshop lookup and download controls, numbered 1 to 4.](docs/images/02-workshop.webp)](docs/USER-GUIDE.md#2-get-mods-into-your-library) |
| **Detected automatically, selectable manually.** Choose a game folder or supported Conan executable. | **A URL or a number gets you started.** SteamCMD setup happens when the download needs it. |

| Choose what to load | See exactly what changes |
| --- | --- |
| [![Checkboxes, ordering arrows and Activate Mods highlighted.](docs/images/04-choose-order.webp)](docs/USER-GUIDE.md#4-choose-and-order-your-mods) | [![Review dialog with before-and-after lists and explicit confirmation.](docs/images/05-review-activate.webp)](docs/USER-GUIDE.md#5-review-and-activate) |
| **Selected means selected.** A profile is a plan until you apply it. | **Confirm the final step.** Package copies and the load list are reviewed together. |

## Built around your local workspace

- **Game detection:** checks Steam records and common installation folders; accepts a manually selected folder or executable.
- **Local library:** scans `.pak` packages and matching `.ucas` / `.utoc` companions.
- **Profiles:** save, duplicate, rename, reorder and switch between mod selections.
- **Load lists:** import and export `modlist.txt`.
- **Controlled installation:** copies selected packages into `ConanSandbox/Mods`, reuses identical files, and blocks conflicting files instead of overwriting them.
- **ConanForge identity:** transparent logo intro, three brief smithing strikes, a Skip control and a matching Windows icon.
- **Under the hood:** Tauri 2, Rust, TypeScript/React and local SQLite storage.

## Availability

**Development preview: v0.3.0 for Windows x64.** This repository currently contains the product showcase and illustrated guide. A public application download has **not** been published yet; there is no installer to download from this page today.

The local preview has been built and exercised during development. Before distributing binaries more widely, the project still needs clean-machine testing, the Nexus integration's public-distribution preparation and a source-license decision. The [roadmap](docs/ROADMAP.md) separates implemented features from future work.

## Know what is protected

The manager preserves subscribed Workshop originals and backs up the previous **load list** when applying changes. It does **not** back up world saves or determine whether a mod is compatible with Enhanced. Back up `ConanSandbox/Saved` separately and check each author's compatibility and load-order instructions.

Changing or removing mods can affect an existing world. Automatic mod updates, dependency resolution and server management are not included in this preview.

---

An independent community project. Not affiliated with or endorsed by Funcom, Valve or Nexus Mods.  
[Artwork and sound credits](docs/CREDITS.md) · [Usage and troubleshooting](docs/USER-GUIDE.md)
