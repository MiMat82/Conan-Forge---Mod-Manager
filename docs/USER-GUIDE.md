# ConanForge: illustrated user guide

**For the v0.3.0 Windows preview.** Follow the numbered circles in each picture. Images show the actual interface with fictional example mods and neutral paths; your filenames, folders and sizes will differ. The pictured Workshop number is an example, not a recommended download.

[Back to the product page](../README.md) · [Roadmap](ROADMAP.md)

## Before you start

The application requires Windows x64, the Microsoft Edge WebView2 runtime and a Conan Exiles client installation. Online downloads also need internet access; scanning, profiles and local imports work locally.

A public download is still being prepared. When the Windows package is available:

1. Download `ConanForge-Windows.zip` from an official release in this repository.
2. Right-click the ZIP and choose **Extract All**. Keep the accompanying documentation and license files.
3. Run `ConanForge.exe` from the extracted folder. Do not run it from inside the ZIP.
4. The transparent logo intro plays three short smithing strikes, then opens the manager. **Skip intro**, **Escape** or **Enter** opens it immediately.

If Windows reports that WebView2 is missing, get the Evergreen Runtime from [Microsoft's WebView2 page](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). The current local preview is unsigned; a public signed build is not yet available. Do not disable Windows protection to run an untrusted copy.

Before changing an established mod setup, make a separate copy of `ConanSandbox/Saved`. ConanForge's automatic backups cover the load list, not your world or character.

## 1. Connect your game

![1 Open Locations and storage. 2 Choose the game folder. 3 Save locations and scan.](images/01-game-location.webp)

ConanForge looks for supported Conan client installations at startup. A saved manual choice takes priority. If exactly one installation is found on first use, it is selected automatically.

1. Open **Locations & storage**.
2. Check the game path. If it is missing or wrong, click **Choose folder…** or **Choose .exe…**.
3. Click **Save locations & scan**.

A typical game root is `D:\Games\Conan Exiles Enhanced`. You may also select a supported executable inside `ConanSandbox\Binaries\Win64`:

- `ConanSandbox.exe`
- `ConanSandbox_BE.exe`
- `ConanSandbox-Win64-Shipping.exe`

The manager resolves those paths to the game installation. If several installations are found, choose the one you actually use. **Detect again** repeats discovery.

The game's `ConanSandbox\Mods` folder is scanned automatically if present. To include existing local or subscribed Workshop downloads, click **+ Browse source folder**, choose the folder, then **Save locations & scan**.

A Steam Workshop source normally ends in:

```text
steamapps\workshop\content\440900
```

These source folders are read without changing the Workshop originals.

## 2. Get mods into your library

### Steam Workshop

![1 Steam Workshop. 2 Item URL or ID. 3 Look up mod. 4 Download to library.](images/02-workshop.webp)

Open **Discover & import**, then:

1. Select **Steam Workshop**.
2. Paste the author's Workshop item URL or its numeric ID. Replace the example number in the picture with your chosen item.
3. Click **Look up mod**. Review the title and author's page; confirm the mod supports your game version.
4. Click **Download to library** and wait for the transfer to finish.

**You do not need to find SteamCMD yourself.** The manager looks for a suitable launcher, or downloads the official launcher from Valve and verifies it when needed. Downloads run through a private working copy. It does not modify an existing SteamCMD installation or your subscribed Workshop folders.

Some Workshop items require account ownership and cannot be downloaded anonymously. For those, subscribe/download through Steam, then add the Workshop `440900` folder as a local scan source.

A completed download is in your **library**. It is not yet activated.

### Nexus Mods or a file you already downloaded

![1 Nexus Mods tab. 2 Session API key area. 3 Local ZIP or PAK import.](images/06-nexus-local.webp)

There are two routes:

**Import a browser download**

1. Download a compatible mod from its author's page.
2. In **Discover & import**, click **Import ZIP / PAK…** or **Import extracted folder…**.
3. Select the downloaded file or extracted mod folder and wait for import to complete.

Keep matching `.ucas` and `.utoc` files beside the `.pak` when importing a package. Extract RAR and 7z archives first; this preview directly imports ZIP and PAK files.

**Use the preview's Nexus connection**

1. Select **Nexus Mods**.
2. Use **Get a key ↗** and enter your own personal API key, then **Connect**. The key remains in memory for this app session and is not saved to the database.
3. Paste a URL or mod ID from the **Conan Exiles Enhanced** Nexus catalog and click **Look up mod**, or choose **Show recently added mods**.
4. Select an available ZIP/PAK file and follow the displayed download requirements.

The implemented direct-download route supports Premium authorization. Other accounts need a matching, unexpired website-issued `nxm://` link or a manual browser download followed by import. The app does not replace another manager's link handler or bypass authorization. Live free/Premium account download qualification remains outstanding.

The Legacy Conan catalog and Enhanced catalog are separate. Do not substitute a Legacy numeric ID into the Enhanced catalog.

Never include an API key or a private download link in a public issue.

## 3. Add mods to a profile

![1 Add a library package to the current profile. 2 Open Load order.](images/03-mod-library.webp)

1. Open **Mod library** and click **+ Add to profile** on each package you want.
2. Open **Load order** to review the selection.

**Added ✓** means that source package is already in the selected profile. It does not mean it has been installed into the game.

Use the search field for filenames or Workshop IDs. **Scan folders** refreshes the library if you added files outside the app.

For another setup, use **+ New profile** or **Duplicate**. Choose a useful name such as “Solo building” or “Co-op world”. Save or discard unsaved edits before switching profiles.

## 4. Choose and order your mods

![1 Tick a mod for activation. 2 Use arrows to set order. 3 Click Activate Mods.](images/04-choose-order.webp)

1. Tick the mods you want to load. Unticked entries remain in the profile as **Not selected**.
2. Use **↑** and **↓** to arrange them. Follow the authors' dependency and load-order instructions; the manager does not infer the correct order.
3. Click **Activate Mods** when the selection is ready.

The selection count shows how many entries will be included. **Save profile** saves your plan locally; it does not change the game. **Activate Mods** saves outstanding profile edits and opens the review.

The **×** beside a row removes it from the profile. It does not delete the downloaded package or a previously installed game copy.

## 5. Review and activate

![1 Compare current and proposed lists. 2 Confirm the game is closed and backups are ready. 3 Back up and apply.](images/05-review-activate.webp)

1. Compare **Current file** with **After apply**, and review the file-copy list and warnings.
2. Close Conan and any Conan server. Back up important world saves separately, then tick the confirmation.
3. Click **Back up & apply**. If the selected files and load list are already identical, the action may say **Verify installed mods** instead.

The app checks the source files and destination again. If a file changed after preview, a different file uses the same target name, or the game is still running, resolve the reported problem and create a fresh preview.

On success, ConanForge reports completion and the backup location. **Restart Conan Exiles** to load the selection and refresh its in-game Mods menu.

Do not expect imported mods to appear in the game before this step.

## What happens to my files?

```mermaid
flowchart LR
    A["Workshop / Nexus / local files"] --> B["ConanForge library"]
    B --> C["Your profile: selection + order"]
    C --> D["Activate Mods: review"]
    D --> E["Explicit confirmation"]
    E --> F["Copies in ConanSandbox/Mods"]
    E --> G["Backup previous modlist.txt"]
    F --> H["Write ordered modlist.txt"]
    G --> H
    H --> I["Restart Conan"]
```

| Place | Purpose |
| --- | --- |
| `ConanSandbox/Mods` in the selected game | Applied PAK packages, matching companions and the active `modlist.txt`. |
| `ConanSandbox/Saved` | World/character saves. Keep a separate backup. |
| `%LOCALAPPDATA%\com.local.conanmodmanager\library` | Imported managed package versions. Profiles may refer to them. |
| `%LOCALAPPDATA%\com.local.conanmodmanager\manager.sqlite3` | Saved profiles, settings, scans and acquisition history. |
| `%LOCALAPPDATA%\com.local.conanmodmanager\backups` | Previous load lists and details of each apply. |

Apply uses local filenames such as `*ExampleMod.pak` in profile order. Identical game files are reused. Conflicting files are blocked. Disabled or unrelated packages already in the Mods folder are left in place, so they may remain visible in the game's available-mods list even when absent from the active load list.

A later change to a source file does not silently replace the installed game copy. Automatic replacement/update installation is not implemented.

## Import, export and recovery

**Import an existing load list:** in **Load order**, click **Import list**, choose `modlist.txt`, and check the base folder used to resolve relative filenames. Missing packages must be resolved before activation.

**Export a profile:** click **Export list ↗** and choose a new `.txt` file outside the game. This exports the selected entries; it does not bundle or distribute the mod files.

**Recover an older load order:** locate the relevant backup's `original.txt`, import it into a profile, and set the base folder to the original game's `ConanSandbox/Mods` for relative filenames. Review and apply with the game closed. Packages referenced by that list must still be available. This restores the load order, not an earlier world save or deleted package version.

## Troubleshooting

| What you see | What to do |
| --- | --- |
| No game detected | Select the game folder or one of the supported client executables manually. Detection is bounded and does not search every folder on every drive. |
| Library is empty | Import your downloaded packages, or add the folder that actually contains them and run a scan. |
| Downloaded mod is missing in Conan | Add it to a profile, tick it, use Activate Mods, confirm Apply, then restart Conan. Check that the game path is the installation you launch. |
| Apply is disabled | Save changed folder settings; in the review, tick the confirmation after completing its checks. |
| Same-name file conflict | Inspect the conflicting versions and retain backups. This preview intentionally does not overwrite an existing different package. |
| Missing companion files | Put the matching UCAS/UTOC pair beside its PAK, then rescan or re-import the complete package. |
| Workshop requires a Steam account | Download through Steam and scan its Workshop folder. Authenticated SteamCMD login is not in this preview. |
| Mod is listed but causes problems | Check Enhanced support, dependencies and the author's load-order guidance. File scanning does not establish compatibility. |

For a report, include the app version, Windows version, exact error text and steps to reproduce. Remove personal paths, API keys and private links from screenshots/logs. [Open an issue](https://github.com/MiMat82/Conan-Forge---Mod-Manager/issues).

[Back to the product page](../README.md)
