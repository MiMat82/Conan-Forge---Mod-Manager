# Safety boundaries

## Protected data

The manager reads existing `.pak`, `.ucas` and `.utoc` source files without modifying them. Imports/downloads create separate app-owned copies. Explicit apply copies enabled packages into the selected installation's `ConanSandbox/Mods` and writes its `modlist.txt`. It never overwrites existing package content or touches saves. Identical installed files are reused; conflicting files block apply. The Mods folder is created only during apply if needed.

All write destinations are checked for Workshop path components and canonical Workshop resolution. All existing ancestor directories/files must be free of reparse points, including junctions and symlinks. Windows device paths, alternate data streams, reserved device names, parent traversal, network write targets and trailing dot/space paths are rejected. Redirected installations must be selected using their real local destination path. The scanner skips redirected entries with visible warnings.

The frontend cannot supply arbitrary proposed bytes to apply. Rust keeps the reviewed profile, original bytes, source metadata, destination state and package hashes with a one-use preview token. Apply requires that token and explicit confirmation. It rechecks the current stored profile, game folder, original bytes, enabled paths, companion sets and SHA-256 content before commit. Changes to a profile, settings, or scan invalidate the pending preview.

Missing enabled files, duplicate resolved paths or destination filenames, empty package files, incomplete or unexpected companion pairs, conflicting destination content and known running Conan game/server processes block apply. Unavailable process inspection also blocks apply. Disabled entries may remain missing. No game or server is started or terminated.

## Write sequence

1. Validate the preview, current profile and file state; check running processes.
2. Acquire a create-only `.cemm-apply.lock` in the target Mods folder. Other manager instances must fail while it exists.
3. Persist original bytes, proposed bytes and a hashed manifest to a new app-data backup directory. Never overwrite an earlier backup.
4. Stage every missing package and the proposed list in the destination directory, flush, and verify package hashes. Complete all staging before publishing any package.
5. Recheck paths, current target, source and destination content, companion sets and running processes. Publish packages with create-only atomic persistence; an appearing destination is never overwritten.
6. Verify installed hashes and recheck the game/list, then use Windows `ReplaceFileW` for an existing list or non-replacing `MoveFileExW` for a new list. No delete-then-write fallback is used. If publication or list commit fails, remove only files created by this apply whose content still matches. Report any rollback failures with the retained backup location.
7. Read back the list, compare bytes, and write the completion receipt. If only the receipt fails, report that the load list was applied and verified while the receipt could not be saved.

Backup/staging failures stop before publication. The backup and proposed file remain available for inspection. No-op applies create no backup or game files. Export remains a separate create-only operation with absolute source paths and is blocked anywhere under ConanSandbox or Workshop. Disabled/unrelated installed packages are preserved.

## Limits

- This protects ordinary single-user workflows, not an adversarial process racing filesystem/path changes. The app's lock is advisory to other manager instances; the game and other editors do not honor it.
- A game could start immediately after the final process check. Keep it closed while applying.
- Apply checks canonical paths, metadata, companion sets and SHA-256 content. Hashing and staging large mods can take time and require space for the full copied payload. No extra copy is made for identical installed content.
- Package updates with a conflicting filename are blocked rather than overwritten. Automatic package replacement/removal is outside this fix.
- Absolute mod paths are deliberately machine-specific. A profile export is not a transferable mod installation or a frozen version snapshot.
- Scan grouping uses matching stems. It does not inspect embedded IoStore containers or infer archive layouts. A standalone `.pak` may itself contain an Enhanced payload; its lack of external companions is not treated as proof of incompatibility.
- Only load lists are backed up. Removing a mod from a world can lose game content even when the list write is correct. Make separate save backups.
- The default VDF reader supports quoted Valve key/value files, braces, escaped quotes/backslashes and line comments; unsupported syntax produces a diagnostic and manual selection remains available.
- Copy/list-commit failure rollback is tested. A crash/power loss between package publication and list replacement can leave new files without an applied list; inspect the manifest and live list before cleanup. Full-disk fault injection, clean-machine installation and the new copy flow's actual game-menu visibility have not been qualified.

## Acquisition safety

ZIP entries are validated before extraction, including non-mod entries. Absolute/parent/device/stream paths, links, duplicate case-insensitive names, oversized data and incomplete companion groups fail the import. Executables and scripts inside archives are skipped. Completed files move from staging into new managed directories; no previous package version or profile entry is overwritten. Content fingerprints are not author signatures, malware scans, or proof of game compatibility. Only import mods from authors you trust.

Provider secrets are never written to settings/history. Downloads, imports and authentication do not apply a load order. Disk-full/cancel/validation failures leave profiles and game files unchanged. Crash-left staging is excluded from scans. A rare history-write failure after publication retains the completed files and reports their location; it does not delete package data. See [provider boundaries](PROVIDERS.md).
