# Public preview validation — v0.4.1

19 November 2026. Windows x64, Rust 1.99.1 MSVC and Node.js 26.13.1. This report covers the original portable release. The same executable is now also distributed through the [additional Windows installer](INSTALLER.md); its validation is recorded separately on the release page.

## Automated checks

| Check | Result |
| --- | --- |
| `npm test` | 9 passed |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 77 passed: 11 unit, 40 core, 13 integration, 12 maintenance, 1 public-release test |
| `npm run build` | TypeScript and production frontend passed |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Passed |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `npm run test:ui` | 11 browser checks passed; fictional native responses, no provider or real collection access |
| `npm run desktop:build` | Optimized Windows executable built with default features; file/product version 0.4.1 |

The public-release test verifies that Nexus connect, lookup, latest and download requests are rejected before network access. Status reports the disabled API capability, while browser source links remain available. Nexus library update checks also report that manual source review is needed.

The browser checks cover menu order, disabled external-file deletion, update results, no automatic Apply, profile deletion blockers, cancel, confirmed deletion with refresh, public Nexus instructions, hidden API-key/lookup controls, website/import controls and no provider connection on navigation. `scripts/verify-ui.mjs` and `tests/fixtures/docs-fixture.js` reproduce them. Documentation screenshots are captured from that production frontend with fictional packages.

## Exact executable and packaging

The release executable was launched on the development Windows machine. Its main window appeared with the title **ConanForge Mod Manager**, the process was responsive, and it closed normally with exit code 0. Every SQLite table row and the selected game's `modlist.txt` bytes were compared before and after launch and remained unchanged. This is a startup check, not an end-to-end native UI or gameplay test.

The Windows ZIP contains that executable, setup instructions, MIT license, guide/images and third-party notices/licenses. The source ZIP is exported from the release commit. Archives are checked for CRC errors; the extracted executable's SHA-256 must match the launched binary. `SHA256SUMS.txt` records hashes of both ZIPs and the executable. No game/mod packages, provider credentials, app database, dependency caches or compiled development folders are included in the source archive.

## Boundaries

- No new real Workshop download or gameplay test was performed for this exact public build. Previous local previews had owner-reported gameplay and download success.
- Clean Windows installation, unsigned-download prompts and missing-WebView2 onboarding remain unqualified.
- No live Nexus API/account test was performed; public API access remains disabled pending registration and qualification.
- Library deletion tests use temporary fixtures. Real user mods and subscribed Workshop files were not deleted or modified.
- The original package was portable only. The later installer addition does not add application signing or an automatic updater.

See [release notes](RELEASE-v0.4.1.md), [Phase 4 tests](PHASE4-VALIDATION.md), and [safety and recovery](SAFETY.md) for the remaining limitations.
