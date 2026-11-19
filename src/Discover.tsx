import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { formatSize } from "./domain";

interface RemoteMod {
  provider: string;
  id: string;
  title: string;
  summary: string;
  updated: number;
  size: number;
  files: { id: string; name: string; version: string; size: number }[];
}
interface OnlineStatus {
  nexusApiEnabled: boolean;
  connected: boolean;
  premium: boolean;
  job: {
    running: boolean;
    stage: string;
    bytes: number;
    error: string | null;
    imported: string | null;
  };
  history: {
    digest: string;
    path: string;
    provider: string;
    modId: string;
    fileId: string;
    name: string;
    created: number;
  }[];
}
const online = <T,>(kind: string, fields = {}) =>
  invoke<T>("request", {
    action: { type: "online", action: { kind, ...fields } },
  });

export default function Discover({
  onImported,
}: {
  onImported: () => Promise<void>;
}) {
  const [provider, setProvider] = useState("steam");
  const [input, setInput] = useState("");
  const [key, setKey] = useState("");
  const [nxm, setNxm] = useState("");
  const [status, setStatus] = useState<OnlineStatus | null>(null);
  const [result, setResult] = useState<RemoteMod | null>(null);
  const [latest, setLatest] = useState<RemoteMod[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const afterImport = useRef(onImported);
  afterImport.current = onImported;
  const locked = busy || !status || status.job.running;
  async function run(work: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await work();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    void online<OnlineStatus>("status")
      .then(setStatus)
      .catch((e) => setError(String(e)));
  }, []);
  useEffect(() => {
    if (status?.job.imported && !status.job.running)
      void afterImport.current().catch((e) => setError(String(e)));
  }, [status?.job.imported, status?.job.running]);
  useEffect(() => {
    if (!status?.job.running) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const next = await online<OnlineStatus>("status");
        if (!alive) return;
        setStatus(next);
        if (next.job.running) timer = setTimeout(() => void poll(), 1000);
      } catch (e) {
        if (alive) {
          setError(String(e));
          timer = setTimeout(() => void poll(), 3000);
        }
      }
    }
    timer = setTimeout(() => void poll(), 500);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [status?.job.running]);
  async function lookup(id = input, source = provider) {
    setNxm("");
    setResult(
      await online<RemoteMod>("lookup", { provider: source, input: id }),
    );
  }
  async function importFile(directory = false) {
    const path = await open(
      directory
        ? { directory: true, title: "Choose an extracted mod folder" }
        : {
            title: "Import a mod package",
            filters: [
              {
                name: "ZIP archive or PAK package",
                extensions: ["zip", "pak"],
              },
            ],
          },
    );
    if (typeof path === "string")
      setStatus(await online<OnlineStatus>("import", { path }));
  }
  function browse(site: string, id?: string) {
    void run(async () => {
      await online("open_site", { site, id: id ?? null });
    });
  }
  return (
    <div className="discover">
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h2>Find your next mod</h2>
            <p>
              Look up an official mod page. Downloads enter your library; you
              choose when to add and apply them.
            </p>
          </div>
          <span className="pill">Online on request</span>
        </div>
        <div className="settings-body">
          <fieldset disabled={locked} className="workspace">
            <div className="discover-tabs" aria-label="Mod provider">
              <button
                className={provider === "steam" ? "primary" : "secondary"}
                aria-pressed={provider === "steam"}
                onClick={() => {
                  setProvider("steam");
                  setResult(null);
                  setInput("");
                  setNxm("");
                }}
              >
                Steam Workshop
              </button>
              <button
                className={provider === "nexus" ? "primary" : "secondary"}
                aria-pressed={provider === "nexus"}
                onClick={() => {
                  setProvider("nexus");
                  setResult(null);
                  setInput("");
                }}
              >
                Nexus Mods
              </button>
              <button onClick={() => browse(provider)}>Browse website ↗</button>
            </div>
            {provider === "nexus" && !status?.nexusApiEnabled && (
              <div className="provider-account">
                <h3>Download on Nexus, then import here</h3>
                <p>Click Browse website, choose a compatible mod and download it. Use Import ZIP / PAK or Import extracted folder below to add it to your library.</p>
                <p>Direct Nexus downloads and update checks are not available in this public preview.</p>
              </div>
            )}
            {provider === "nexus" && status?.nexusApiEnabled && (
              <div className="provider-account">
                {status?.connected ? (
                  <div className="input-row">
                    <p>
                      Connected for this session ·{" "}
                      {status.premium
                        ? "Premium downloads available"
                        : "Website authorization required for downloads"}
                    </p>
                    <button
                      onClick={() =>
                        void run(async () =>
                          setStatus(await online("disconnect")),
                        )
                      }
                    >
                      Disconnect
                    </button>
                  </div>
                ) : (
                  <>
                    <p>
                      For this personal testing build, enter your personal Nexus
                      API key. It stays in memory for this app session.
                    </p>
                    <label>
                      Nexus API key
                      <div className="input-row">
                        <input
                          type="password"
                          autoComplete="off"
                          spellCheck={false}
                          value={key}
                          onChange={(e) => setKey(e.target.value)}
                          placeholder="Enter your personal API key"
                        />
                        <button
                          disabled={!key.trim()}
                          onClick={() =>
                            void run(async () => {
                              const secret = key;
                              setKey("");
                              setStatus(
                                await online("connect", { key: secret }),
                              );
                            })
                          }
                        >
                          Connect
                        </button>
                        <button onClick={() => browse("nexus_key")}>
                          Get a key ↗
                        </button>
                      </div>
                    </label>
                  </>
                )}
              </div>
            )}
            {(provider === "steam" || status?.nexusApiEnabled) && <form
              onSubmit={(e) => {
                e.preventDefault();
                if (!locked && input.trim()) void run(() => lookup());
              }}
            >
              <label>
                {provider === "steam"
                  ? "Workshop item URL or ID"
                  : "Conan Exiles Enhanced Nexus mod URL or ID"}
                <div className="input-row">
                  <input
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    placeholder={
                      provider === "steam"
                        ? "https://steamcommunity.com/sharedfiles/filedetails/?id=…"
                        : "https://www.nexusmods.com/conanexilesenhanced/mods/…"
                    }
                  />
                  <button
                    className="primary"
                    disabled={
                      !input.trim() ||
                      (provider === "nexus" && !status?.connected)
                    }
                  >
                    Look up mod
                  </button>
                </div>
              </label>
            </form>}
            {provider === "nexus" && status?.nexusApiEnabled && (
              <button
                disabled={!status?.connected}
                onClick={() =>
                  void run(async () => setLatest(await online("latest")))
                }
              >
                Show recently added mods
              </button>
            )}
            <p className="muted">
              An item listed for Conan Exiles is not proof of Enhanced
              compatibility. Check the author’s game-version notes before
              downloading.
            </p>
          </fieldset>
          {busy && <p role="status">Working…</p>}
          {error && (
            <p role="alert" className="alert error">
              {error}
            </p>
          )}
        </div>
      </section>
      {result && (
        <section className="panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">
                {result.provider === "steam" ? "STEAM WORKSHOP" : "NEXUS MODS"}{" "}
                · {result.id}
              </p>
              <h2>{result.title}</h2>
            </div>
            <button
              disabled={locked}
              onClick={() => browse(result.provider, result.id)}
            >
              Author’s page ↗
            </button>
          </div>
          <div className="settings-body">
            <p className="remote-summary">{result.summary}</p>
            <p className="muted">
              {result.updated
                ? `Updated ${new Date(result.updated * 1000).toLocaleDateString()}`
                : "Update date unavailable"}
              {result.size > 0 ? ` · ${formatSize(result.size)}` : ""}
            </p>
            {result.provider === "steam" ? (
              <>
                <p>
                  SteamCMD is found automatically or downloaded from Valve when
                  needed. The app verifies it and runs a private copy in a
                  separate folder. Items requiring account ownership must be
                  downloaded through Steam, then scanned locally.
                </p>
                <div className="input-row">
                  <button
                    className="primary"
                    disabled={locked}
                    onClick={() =>
                      void run(async () => {
                        setStatus(
                          await online("download_steam", { id: result.id }),
                        );
                      })
                    }
                  >
                    Download to library
                  </button>
                </div>
              </>
            ) : (
              <>
                {!status?.premium && (
                  <label>
                    Website-issued nxm:// link for the file below (optional if
                    importing manually)
                    <input
                      type="password"
                      autoComplete="off"
                      value={nxm}
                      onChange={(e) => setNxm(e.target.value)}
                      placeholder="Paste the authorized download link for your account"
                    />
                    <small>
                      Or download on Nexus and use Import ZIP / PAK below. Your
                      existing mod-manager link handler stays unchanged.
                    </small>
                  </label>
                )}
                <div className="remote-files">
                  {result.files.map((file) => (
                    <div className="remote-file" key={file.id}>
                      <div>
                        <strong>{file.name}</strong>
                        <small>
                          Version {file.version || "unspecified"} ·{" "}
                          {file.size
                            ? formatSize(file.size)
                            : "size unavailable"}
                        </small>
                      </div>
                      <button
                        disabled={
                          locked ||
                          (!status?.premium && !nxm.trim()) ||
                          !/\.(zip|pak)$/i.test(file.name)
                        }
                        onClick={() =>
                          void run(async () => {
                            const link = nxm;
                            setNxm("");
                            setStatus(
                              await online("download_nexus", {
                                mod_id: result.id,
                                file_id: file.id,
                                nxm: link,
                              }),
                            );
                          })
                        }
                      >
                        {/\.(zip|pak)$/i.test(file.name)
                          ? "Download to library"
                          : "Extract manually"}
                      </button>
                    </div>
                  ))}
                </div>
                {!result.files.length && (
                  <p>
                    No downloadable files were returned. Check the author’s
                    page.
                  </p>
                )}
              </>
            )}
          </div>
        </section>
      )}
      {provider === "nexus" && latest.length > 0 && (
        <section className="panel">
          <div className="panel-heading">
            <h2>Recently added on Nexus</h2>
          </div>
          <div className="settings-body recent-mods">
            {latest.map((mod) => (
              <button
                key={mod.id}
                disabled={locked}
                onClick={() => void run(() => lookup(mod.id, "nexus"))}
              >
                <strong>{mod.title}</strong>
                <small>#{mod.id} · View files →</small>
              </button>
            ))}
          </div>
        </section>
      )}
      <section className="panel">
        <div className="panel-heading">
          <div>
            <h2>Bring downloads into your library</h2>
            <p>
              ZIP and PAK packages are copied into managed storage. Matching
              UCAS / UTOC companions stay together.
            </p>
          </div>
        </div>
        <div className="settings-body">
          <div className="input-row">
            <button
              disabled={locked}
              onClick={() => void run(() => importFile())}
            >
              Import ZIP / PAK…
            </button>
            <button
              disabled={locked}
              onClick={() => void run(() => importFile(true))}
            >
              Import extracted folder…
            </button>
          </div>
          <p className="muted">
            Extract 7z / RAR archives first. Imports up to 16 GiB are supported.
            New versions get separate paths; existing profiles keep their
            current files.
          </p>
          {status?.job.stage && (
            <div className="transfer-status" role="status">
              <div>
                <strong>{status.job.stage}</strong>
                {status.job.bytes > 0 && (
                  <small>{formatSize(status.job.bytes)} processed</small>
                )}
                {status.job.imported && (
                  <small>
                    Ready in Mod library. Add it to a profile, then preview
                    changes.
                  </small>
                )}
              </div>
              {status.job.running && (
                <button
                  onClick={() =>
                    void run(async () => setStatus(await online("cancel")))
                  }
                >
                  Cancel transfer
                </button>
              )}
            </div>
          )}
          {status?.job.error && (
            <p className="alert error" role="alert">
              {status.job.error}
            </p>
          )}
          <details className="scan-notes">
            <summary>Import history ({status?.history.length ?? 0})</summary>
            {status?.history.map((item) => (
              <div className="history-item" key={item.digest}>
                <strong>{item.name}</strong>
                <small>
                  {item.provider}
                  {item.modId ? ` #${item.modId}` : ""} ·{" "}
                  {new Date(item.created * 1000).toLocaleString()}
                </small>
                <span className="file-path">{item.path}</span>
              </div>
            ))}
          </details>
        </div>
      </section>
    </div>
  );
}
