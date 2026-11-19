# Provider integration boundaries

## Steam Workshop / SteamCMD

The app uses Valve's documented public [`ISteamRemoteStorage/GetPublishedFileDetails`](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage#GetPublishedFileDetails) endpoint. It requests one item at a time and checks the returned item ID and consumer app ID 440900. Steam's full search is opened in the system browser; no publisher API key or subscription mutation is attempted. Legacy and Enhanced items may coexist under app ID 440900, so authors' compatibility notes still matter.

On a Workshop download request, the app checks its launcher cache, PATH (up to 128 absolute entries), beside the app, standard Program Files/local-app-data SteamCMD directories, and `steamcmd`, `Tools/SteamCMD`, and `Games/SteamCMD` on fixed drives. This bounded lookup does not search entire drives. If no trusted copy is found, it downloads the fixed HTTPS Windows ZIP from Valve, as linked by the [official SteamCMD page](https://developer.valvesoftware.com/wiki/SteamCMD). The ZIP is limited to 16 MiB, must contain only a plain root `steamcmd.exe`, and is extracted with a 64 MiB launcher limit. HTTP redirects are not followed.

Windows Authenticode trust, revocation checks and the Valve publisher name are verified on the private staged file before it replaces the cached launcher. An invalid existing installation is skipped without changing it; failed/cancelled setup preserves the prior cache and never runs unverified code. Windows certificate validation may need internet access and can delay cancellation. The cached launcher is checked again before reuse and its private execution copy is checked before launch. Setup is automatic on request, never an unsolicited startup download. A copy runs in an app-owned temporary directory with numeric-only item arguments and anonymous login. The Windows process is created suspended, assigned to a kill-on-close job, then resumed. Updater children inherit the job. Cancellation/exit closes the job; normal completion waits for all children, then validates actual downloaded packages. No Steam password or Steam Guard code is requested. The downloader never points SteamCMD at the installed game, an existing SteamCMD workspace, or subscribed Workshop storage.

Only the verified launcher is cached at `tools/steamcmd/steamcmd.exe` under app data. SteamCMD still bootstraps its runtime in each private transfer directory. This trades startup time/bandwidth for a small, isolated implementation. SteamCMD and third-party mod files are not bundled or redistributed with the manager. The 16 GiB limit applies to managed imports, not Valve's temporary runtime/download cache.

## Nexus Mods

**Public v0.4.1 builds:** direct API access is disabled by default in the Rust request layer. The UI offers website browsing and manual ZIP/PAK imports without requesting an API key. The `nexus-testing` Cargo feature exists only for private development/registration testing; it must not be enabled in public release artifacts. The protocol implementation below describes that private testing path.

The current [Enhanced catalog](https://www.nexusmods.com/conanexilesenhanced/mods/) is separate from `/conanexiles/`. All API routes and signed-link validation use `conanexilesenhanced` to avoid ambiguous numeric IDs.

The factual API descriptions in [Nexus's official API documentation](https://github.com/Nexus-Mods/node-nexus-api/blob/master/docs/classes/_nexus_.nexus.md) informed this independent Rust implementation. No GPL client code was copied or linked. Routes: `users/validate.json`; Enhanced mods `latest_added.json`; `mods/{id}.json`; `mods/{id}/files.json`; individual file details and `download_link.json`.

Each API request identifies this application and its semantic version. The key is attached only to the fixed `api.nexusmods.com` origin, with redirects disabled. Response bodies and exception URLs are not exposed in errors. Metadata responses are capped at 2 MiB. User actions initiate requests; there is no background polling of Nexus or automatic retry on HTTP 429.

Free-account `nxm://` authorization is checked against the selected game, mod, file, logged-in account and expiry. Premium API access uses the provider's account permission. CDN transfer requests contain no API-key header; HTTPS redirects are limited and each host must belong to `*.nexusmods.com` or `*.nexus-cdn.com`. An unrecognized legitimate CDN requires a future allowlist update or manual browser download/import.

API keys and signed links are held in memory for the active session/job and are never persisted intentionally. Disconnect affects future operations; an already-running transfer holds its own session until it ends. Normal OS process-memory/pagefile behavior is outside this guarantee.

The [Nexus API Acceptable Use Policy](https://help.nexusmods.com/article/114-api-acceptable-use-policy) permits personal use and testing with personal API keys, and requires app registration before a public-facing integration leaves testing. Registration and live authenticated-account qualification remain outstanding, so public builds disable this API path. The app does not impersonate Vortex or another registered client.
