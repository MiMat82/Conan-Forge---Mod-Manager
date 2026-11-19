# Publishing signed app updates

The updater uses a public static feed at `updates/windows-x64.json` on this repository's `main` branch. It can announce preview releases, which GitHub's `/latest` endpoint excludes. Only publish a feed after its corresponding release and installer are publicly downloadable. Previously published installers and tags remain intact.

## Publisher key

The application embeds only `src-tauri/updater-public.key`. On the publisher's Windows account, `scripts/publisher.ps1` keeps the private key in `%LOCALAPPDATA%\ConanForgePublishing\updater-private-key.dpapi`, encrypted with Windows DPAPI CurrentUser. The folder is restricted to that account and SYSTEM. The bundle command decrypts it in memory and passes it to Tauri only for signing, then clears the environment variables. Never commit or upload the private key, decrypted key, credentials, app data, or private mod files.

Preserve this key and the Windows account's recovery material. A DPAPI file copied to a different Windows account is not a portable key backup. Losing the signing key prevents updates to clients that trust it; changing the public key alone does not migrate installed clients. A new fork must create its own signing identity and change the app identifier and official URLs deliberately. `-Action Init` is for first-time publisher setup, not routine releases.

## Build and verify

1. Update the version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json` and the sidebar. Update release notes and documentation. Keep the public build's default Cargo features; do not enable `nexus-testing`.
2. Run `npm ci`, `npm run check`, `npm run test:ui`, Cargo formatting and Clippy checks. Refresh dependency notices with `node scripts/collect-notices.mjs` when dependencies change.
3. Run `npm run installer:build`. This builds the application and bundles/signs NSIS using the protected key. The installer and `.exe.sig` appear under `src-tauri/target/release/bundle/nsis/`. The CLI and updater versions in the lockfiles support signatures bound to the application version; do not replace them with legacy signatures.
4. Copy both files into a new release staging folder. Name the setup `ConanForge-vVERSION-windows-x64-setup.exe` and its signature the same name plus `.sig`. Tauri sets the executable's bundle marker to NSIS inside setup, then restores the build-output copy. Verify the installed executable separately: its only expected difference from the unbundled binary is `__TAURI_BUNDLE_TYPE_VAR_UNK` becoming `__TAURI_BUNDLE_TYPE_VAR_NSS`. Retain both hashes and test the installed copy. Preserve any existing installation and live user data.
5. Write concise update notes to a text file, then prepare a candidate feed (replace VERSION):

```powershell
node scripts/prepare-update.mjs release/vVERSION/ConanForge-vVERSION-windows-x64-setup.exe release/vVERSION/ConanForge-vVERSION-windows-x64-setup.exe.sig release/vVERSION/update-notes.txt release/vVERSION/windows-x64.json
cargo run --manifest-path src-tauri/Cargo.toml --example verify_update -- release/vVERSION/windows-x64.json release/vVERSION/ConanForge-vVERSION-windows-x64-setup.exe
```

The Rust example uses the actual updater to verify the signature/version and reject altered bytes, a relabeled version and an invalid signature. It opens neither the manager's database nor an installer. The feed-preparation script alone does not cryptographically verify anything.

For a read-only check of the version currently served to the updater, run `cargo run --manifest-path src-tauri/Cargo.toml --example verify_update -- --check-online`. This diagnostic reports the feed version regardless of the installed version; it never downloads or installs an update. An immediate post-publication check may still see the previous feed; wait and use the app's normal check again before treating publication as complete.

6. Qualify installation, startup, an actual update from an older updater-enabled test build, and retained profiles/library/game files. Record the precise artifact hashes and any untested boundaries. Prepare source/portable packages and SHA-256 checksums; exclude private data, signing material and development caches.

## Publish in this order

1. Commit reviewed source/docs, tag the release commit, and push. Create the GitHub release with the matching installer, signature, checksums, source and verification report. Public previews remain prereleases.
2. Download the public installer and signature anonymously. Compare hashes and run `verify_update` against those downloaded bytes.
3. Copy the validated candidate into `updates/windows-x64.json`, commit and push the feed. Never announce an unavailable asset. Verify the public raw feed, download URL and signature again.
4. Check that an older updater-enabled build offers the release and the current build reports no newer version. Complete the installation/restart check against the exact published installer.

The app requires a strictly newer semantic version and the exact ConanForge release URL. It will not silently downgrade. If a published update is faulty, withdraw the feed or point it back to the previous known-good release to stop further offers, then publish a newly numbered fix. Already-updated clients still need a newer version. The updater signature authenticates the installer and version; release-note text is delivered by the HTTPS feed and is rendered as plain text.

There is no unattended publishing or installation. Windows Authenticode signing and clean-machine/WebView2 qualification remain separate work.
