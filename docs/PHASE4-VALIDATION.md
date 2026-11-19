# Phase 4: library maintenance validation

Date: 19 November 2026. Local preview: v0.4.0.

## Implemented

- Per-import update checks, timestamps and source-page links in Mod library.
- Workshop comparison uses the recorded download timestamp. Metadata changes can also move that timestamp; this is a prompt to review the item, not a compatibility guarantee.
- Nexus follows the author's old-file/new-file relationships for the downloaded file. Cycles, malformed history, branches and unavailable replacements do not produce a false up-to-date result. Unlinked uploads need manual review.
- Imported bundles can be deleted after a read-only preview and explicit confirmation. Saved references, including disabled entries and filesystem aliases, block deletion. Direct references in the selected game's modlist also block it.
- Deletion is restricted to the recorded direct child of the app-owned library, checks the complete bundle fingerprint, rejects junctions/unexpected files, revalidates inside a database write transaction, and restores the parked files if database changes fail. A cleanup failure reports its retained staging location.
- Existing game copies, Workshop originals and local source downloads are not deleted. Application identity and SQLite schema version remain unchanged.
- Re-downloading identical content from the same provider refreshes its version metadata; a verified provider download can add provenance to a previous local import. Unrelated providers do not overwrite existing identity.

## Automated verification

- `cargo test --manifest-path src-tauri/Cargo.toml`: 76 passed (11 unit, 40 core, 13 integration, 12 library-maintenance tests).
- `npm test`: 9 passed, including Windows path ownership and prefix-collision coverage.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: passed.
- Rust formatting and TypeScript/production frontend builds: passed.
- `npm run desktop:build`: optimized Windows release built successfully.
- Headless Chrome with the production frontend and fictional native-response fixtures: seven flow checks passed (navigation, external-file controls, update result, no automatic Apply, used-profile deletion blocker, cancel, confirmed delete with refreshed library).
- Windows junction test confirms redirected bundles are refused and profile references through an alias are recognized.
- A forced SQLite delete failure restores the package files and retains the original acquisition record.
- Read-only live Steam metadata lookup for item 3723560519 succeeded using the existing provider client. No live download was performed.

The UI checks live in the workspace's `work/check-library-ui.mjs`; their JSON result and screenshot are retained there. The deletion tests use temporary fixtures, not the user's collection.

## Provider references

The metadata behavior was checked against [Valve's GetPublishedFileDetails API](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage) and [Nexus's official file/update type definitions](https://github.com/Nexus-Mods/node-nexus-api/blob/master/src/types.ts). No third-party implementation was copied or added as a dependency.

## Limits and next phase

- Live Nexus account/file history, new Workshop downloads and gameplay have not been requalified in this phase.
- Update-check results are session information; recheck after restarting. Provider IDs and downloaded versions remain stored in SQLite.
- No bulk checking, automatic version replacement, package rollback UI or game-file uninstall is included.
- Different-content game filename conflicts still block Apply. The next phase is reviewed replacement with backups of affected packages and the load list, followed by explicit Apply and restore support.
- A process/power failure between parking a bundle and database commit can leave recoverable files in app staging; startup recovery for interrupted maintenance is not yet automated.
