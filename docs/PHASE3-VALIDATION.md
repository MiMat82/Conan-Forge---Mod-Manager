# Version 0.3.0 validation

Phase 3 focuses on visual polish and clearer activation. It adds a copper/charcoal visual theme based on the supplied logo, a numbered Find / Choose / Review workflow, selected/not-selected labels, a selected-entry count, a sticky Activate Mods action and completion/restart guidance. Both review entry points use the same save-and-preview function. The existing explicit Apply gate and Rust deployment behavior are retained. No new dependency or database migration was introduced.

## Checks completed

| Check | Result |
| --- | --- |
| Rust tests on v0.3.0 | 64 passed: 11 unit, 40 core, 13 integration; no exclusions |
| Frontend tests | 8 passed, including startup completion/skip/failure cases |
| TypeScript and Vite production build | Passed for main and splash pages |
| Clippy, all targets, warnings denied | Passed |
| Rust formatting | Passed |
| Windows release build | Passed; executable product/version inspected as ConanForge Mod Manager 0.3.0 |
| Browser UI at 900 × 640 | No horizontal overflow; sticky activation control remained visible |
| Browser UI at 1440 × 1200 | Reviewed load order, library, location, provider and review screens |
| Production frontend with isolated fake host | Editing selection led to save then preview; no apply before confirmation; confirmed apply called once; completion and restart guidance appeared; editing afterward cleared old success |
| Public GitHub page | README, hero, six numbered guide screenshots and overview uploaded; all README and guide images loaded in an unauthenticated browser |

The screenshot/interaction fixtures render the actual frontend with fictional mod names, neutral folders and simulated provider responses. They are not live Workshop/Nexus downloads and never touch game files. The guide explicitly identifies the examples. Browser tests cover the interface, not Windows native dialogs, physical audio or clean-machine startup.

The v0.2.4 native splash/main-window check is recorded in [Phase 2 validation](PHASE2-VALIDATION.md). Native interaction stopped when the user pressed physical Escape. The subsequent v0.3.0 changes were verified in the browser; v0.3.0 was built but not launched again through desktop automation. Existing v0.2.4 processes were not terminated to replace their executable. The new package uses `ConanForge-Windows/ConanForge.exe`.

The live game's modlist SHA-256 remained `6FE521D03512833134DF45472AE14244801476F734929D102BC18C7B18D47060` throughout this visual/documentation work. No real-game Apply was performed in this phase.

v0.3.0 executable SHA-256: `D289C4552744E9FAB88172351F269DCDEAAD380BE8AEEB574C5AED3EC75415B8`.

## Publication boundary

Only the product README, user guide, roadmap, credits, images and Git attributes were published to the user's public repository. Application source, executables, local verification reports, app data, API keys and mod payloads were not uploaded. The complete source and Windows preview remain in the local deliverables. Public application distribution still needs the checks listed in the roadmap, including Nexus preparation, live account qualification and a license decision.
