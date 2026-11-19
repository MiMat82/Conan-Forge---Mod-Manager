# Public preview validation — v0.5.0

19 November 2026, Windows x64, Rust 1.99.1 MSVC, Node.js 26.13.1. Public build uses default features; Nexus API integration stays disabled.

| Check | Result |
| --- | --- |
| Frontend unit tests | 9 passed |
| Rust tests | 84 passed: 12 unit, 6 app-update, 40 core, 13 import/provider, 12 library maintenance, 1 public-release |
| Browser flow checks | 19 passed, including startup check, no automatic download/install, offline recovery, unsaved edits, confirmation/cancel and active-transfer refusal |
| TypeScript/Vite production build | Passed |
| Rust Clippy / formatting | Passed |
| Optimized Windows build and signed NSIS package | Completed |
| Actual updater signature verification | Valid installer/version accepted; changed bytes, relabeled version and invalid signature rejected |
| Existing per-user installation | v0.4.1 upgraded to v0.5.0; previous installation preserved separately and 38 backup files hash-verified |
| Installed program and resources | NSIS marker accounted for; installed app matched the expected binary and 39 packaged resources matched their source hashes |
| Native installed application | Rendered App updates, reported version 0.5.0 and installed eligibility, startup preference enabled, main window responsive, normal exit code 0 |
| Existing user state | All database rows unchanged; 36 library/backup/game-Mods files retained their SHA-256, size and modification time; 46 game-Saved entries retained size and modification time |

The browser uses fictional native responses and an example future version. Native inspection uses the real installed WebView2 application and real Rust commands. Game saves were checked by metadata, not by content hashes. No mod Apply, provider download or deletion was performed during update qualification.

Database backup integrity, retained prior backups, missing confirmation, backup failure and active-transfer blocking are tested in isolated real SQLite/file-system fixtures. The upgrade above used the setup executable: v0.4.1 has no updater. An actual installation/restart between two released updater-enabled versions has **not yet been exercised**. The signed release feed and anonymous asset readback are verified after publication and recorded with the release downloads.

The verification example initially lacked the Windows Common Controls manifest required by its native imports. Its example-only manifest was corrected, then all four signature checks passed using the actual updater library with a mock window runtime. No manager database or installer is opened by that example.

Windows Authenticode signing, a clean PC without WebView2, an interrupted installer/power loss and every provider/mod combination remain unqualified. This is a public preview, not a stable-release claim. Full artifact hashes are in the release's `SHA256SUMS.txt` and accompanying validation text.
