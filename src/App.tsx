import { useEffect, useRef, useState, type ReactNode } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  Bootstrap,
  LibraryDeletePreview,
  UpdateCheck,
  Detection,
  Preview,
  Profile,
  Receipt,
  Scan,
  Settings,
} from "./types";
import {
  addEntry,
  acquisitionForPath,
  fileName,
  formatSize,
  moveEntry,
  parentPath,
  pathKey,
} from "./domain";
import "./App.css";
import Discover from "./Discover";
import AppUpdates from "./AppUpdates";

const request = <T,>(type: string, fields = {}) =>
  invoke<T>("request", { action: { type, ...fields } });
const emptyScan: Scan = { mods: [], warnings: [], orphanFiles: [] };

function Modal({
  title,
  children,
  close,
}: {
  title: string;
  children: ReactNode;
  close: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog
      ref={ref}
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      aria-labelledby="dialog-title"
    >
      <div className="dialog-heading">
        <h2 id="dialog-title">{title}</h2>
        <button aria-label="Close dialog" onClick={close}>
          ×
        </button>
      </div>
      {children}
    </dialog>
  );
}

export default function App() {
  const [page, setPage] = useState<
    "order" | "library" | "locations" | "discover" | "updates"
  >("order");
  const [data, setData] = useState<Bootstrap | null>(null);
  const [draft, setDraft] = useState<Profile | null>(null);
  const [locations, setLocations] = useState<Settings>({
    gameRoot: "",
    scanRoots: [],
  });
  const [catalog, setCatalog] = useState<Scan>(emptyScan);
  const [detected, setDetected] = useState<Detection | null>(null);
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState("");
  const [appInstalling, setAppInstalling] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [preview, setPreview] = useState<Preview | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const [newName, setNewName] = useState("");
  const [newProfile, setNewProfile] = useState<"empty" | "copy" | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [libraryDelete, setLibraryDelete] = useState<LibraryDeletePreview | null>(null);
  const [updateChecks, setUpdateChecks] = useState<Record<string, UpdateCheck>>({});
  const [importing, setImporting] = useState<{
    path: string;
    base: string;
  } | null>(null);
  const saved = data?.profiles.find((p) => p.id === draft?.id);
  const dirty = !!draft && JSON.stringify(saved) !== JSON.stringify(draft);
  const locationsDirty =
    !!data && JSON.stringify(data.settings) !== JSON.stringify(locations);
  const enabled = draft?.entries.filter((e) => e.enabled).length ?? 0;

  async function run(label: string, action: () => Promise<void>) {
    setBusy(label);
    setError("");
    setNotice("");
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy("");
    }
  }
  async function load(id?: number) {
    const next = await request<Bootstrap>("bootstrap");
    setData(next);
    setCatalog(next.catalog);
    setLocations(next.settings);
    setDetected(next.detection);
    setDraft(next.profiles.find((p) => p.id === id) ?? next.profiles[0]);
  }
  useEffect(() => {
    if (!isTauri()) {
      setError(
        "This is the browser preview. Open the Windows app to access your files and profiles.",
      );
      return;
    }
    void run("Opening your workspace…", () => load());
  }, []);

  function update(profile: Profile) {
    setDraft(profile);
    setPreview(null);
    setReceipt(null);
  }
  function accept(profile: Profile) {
    setDraft(profile);
    setData(
      (previous) =>
        previous && {
          ...previous,
          profiles: previous.profiles.map((p) =>
            p.id === profile.id ? profile : p,
          ),
        },
    );
  }
  async function saveProfile() {
    if (!draft) throw new Error("Select a profile first.");
    const next = dirty
      ? await request<Profile>("save_profile", { profile: draft })
      : draft;
    accept(next);
    return next;
  }
  function reviewActivation() {
    void run("Checking your selected mods…", async () => {
      const profile = await saveProfile();
      const next = await request<Preview>("preview", { id: profile.id });
      setConfirmed(false);
      setPreview(next);
    });
  }
  async function scan() {
    setPreview(null);
    const result = await request<Scan>("scan");
    setCatalog(result);
    const latest = await request<Bootstrap>("bootstrap");
    setData((previous) => previous && { ...previous, acquisitions: latest.acquisitions });
    setNotice(
      `Scan complete: ${result.mods.length} packages found. Compatibility remains unverified.`,
    );
  }
  async function chooseGame(executable = false) {
    const path = await open(
      executable
        ? {
            title: "Select a ConanSandbox client executable",
            filters: [{ name: "Game executable", extensions: ["exe"] }],
          }
        : { directory: true, title: "Choose the Conan Exiles game folder" },
    );
    if (typeof path === "string")
      setLocations({
        ...locations,
        gameRoot: await request<string>("resolve_game", { path }),
      });
  }
  async function chooseSource() {
    const path = await open({
      directory: true,
      title: "Choose a local mods or Workshop content folder",
    });
    if (
      typeof path === "string" &&
      !locations.scanRoots.some((p) => pathKey(p) === pathKey(path))
    )
      setLocations({ ...locations, scanRoots: [...locations.scanRoots, path] });
  }
  const closeModal = () => {
    if (!busy) {
      setPreview(null);
      setNewProfile(null);
      setDeleting(false);
      setLibraryDelete(null);
      setImporting(null);
    }
  };
  const dialogError = error && (
    <p className="alert error" role="alert">
      {error}
    </p>
  );

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <img
            className="brand-logo"
            src="/brand/conanforge-logo.png"
            alt="ConanForge Mod Manager"
          />
        </div>
        <div className="edition">FOR CONAN EXILES ENHANCED</div>
        <nav aria-label="Main navigation">
          <button
            className={page === "discover" ? "active" : ""}
            onClick={() => setPage("discover")}
          >
            <span>↗</span> Discover & import
          </button>
          <button
            className={page === "library" ? "active" : ""}
            onClick={() => setPage("library")}
          >
            <span>▦</span> Mod library <b>{catalog.mods.length}</b>
          </button>
          <button
            className={page === "order" ? "active" : ""}
            onClick={() => setPage("order")}
          >
            <span>≡</span> Load order <b>{draft?.entries.length ?? 0}</b>
          </button>
          <button
            className={page === "locations" ? "active" : ""}
            onClick={() => setPage("locations")}
          >
            <span>⌖</span> Locations & storage
          </button>
          <button className={page === "updates" ? "active" : ""} onClick={() => setPage("updates")}>
            <span>↓</span> App updates
          </button>
        </nav>
        <div className="sidebar-bottom">
          <span className="status-dot" /> Local workspace
          <p>Workshop originals stay untouched.</p>
          <small>v0.5.1 · Public preview</small>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>
            YOUR COLLECTION /{" "}
            <strong>
              {page === "discover"
                ? "DISCOVER & IMPORT"
                : page === "order"
                  ? "LOAD ORDER"
                  : page === "library"
                    ? "MOD LIBRARY"
                    : page === "updates" ? "APP UPDATES" : "LOCATIONS"}
            </strong>
          </span>
          <span className="pill">● Your local workspace</span>
        </header>
        <div className="content">
          <div className="page-heading" hidden={page === "updates"}>
            <div>
              <p className="eyebrow">CONAN EXILES ENHANCED</p>
              <h1>
                {page === "discover"
                  ? "Expand your collection."
                  : page === "order"
                    ? "Forge your next adventure."
                    : page === "library"
                      ? "A home for every mod."
                      : "Connect your collection."}
              </h1>
              <p className="muted">
                {page === "discover"
                  ? "Find mods, bring in authorized downloads, and keep every change under your control."
                  : page === "order"
                    ? "Choose your mods and their order. Review once, then activate when you’re ready."
                    : page === "library"
                      ? "Read local packages and Workshop downloads without changing the originals."
                      : "Choose your game and mod folders. Detection checks Steam records and common installation folders."}
              </p>
            </div>
          </div>
          {error && (
            <div className="alert error" role="alert">
              {error}
              <button aria-label="Dismiss error" onClick={() => setError("")}>
                ×
              </button>
            </div>
          )}
          <div aria-live="polite">
            {busy && <div className="alert">{busy}</div>}
            {notice && <div className="alert success">{notice}</div>}
          </div>
          <AppUpdates ready={!!data} shown={page === "updates"} unsaved={dirty || locationsDirty}
            blocked={!!busy || !!preview || !!libraryDelete || !!newProfile || deleting || !!importing}
            onOpen={() => setPage("updates")} onInstalling={setAppInstalling} />
          <fieldset
            disabled={!!busy || !data || appInstalling}
            hidden={page === "updates"}
            className="workspace"
            aria-busy={!!busy}
          >
            {page !== "locations" && (
              <div className="workflow" aria-label="Mod activation steps">
                <button
                  className={page === "discover" ? "current" : ""}
                  onClick={() => setPage("discover")}
                >
                  <span className="step-number">1</span>
                  <span><strong>Find & import</strong><small>Workshop, Nexus or local files</small></span>
                </button>
                <button
                  className={page === "library" || page === "order" ? "current" : ""}
                  onClick={() => setPage("library")}
                >
                  <span className="step-number">2</span>
                  <span><strong>Choose & order</strong><small>Add mods to your profile</small></span>
                </button>
                <button
                  disabled={!locations.gameRoot || locationsDirty || !draft}
                  onClick={reviewActivation}
                >
                  <span className="step-number">3</span>
                  <span><strong>Review & activate</strong><small>Confirm before changing the game</small></span>
                </button>
              </div>
            )}
            {page !== "locations" && page !== "discover" && (
              <div className="connection-strip">
                <span className="file-path" title={data?.settings.gameRoot}>
                  {data?.settings.gameRoot
                    ? `GAME SELECTED · ${data.settings.gameRoot}`
                    : "GAME NOT SELECTED · Choose an installation to get started"}
                </span>
                <button onClick={() => setPage("locations")}>
                  Change location
                </button>
              </div>
            )}
            {page !== "locations" && page !== "discover" && (
              <section className="profile-bar" aria-label="Current profile">
                <label>
                  PROFILE
                  <select
                    value={draft?.id ?? ""}
                    disabled={dirty}
                    onChange={(e) => {
                      const p = data?.profiles.find(
                        (p) => p.id === Number(e.target.value),
                      );
                      if (p) update(p);
                    }}
                  >
                    {data?.profiles.map((p) => (
                      <option value={p.id} key={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </label>
                <div className="profile-actions">
                  <span className={dirty ? "unsaved" : "muted"}>
                    {dirty
                      ? "Unsaved profile changes"
                      : "Profile saved locally"}
                  </span>
                  {dirty && (
                    <>
                      <button onClick={() => saved && update(saved)}>
                        Discard edits
                      </button>
                      <button
                        className="secondary"
                        onClick={() =>
                          void run("Saving profile…", async () => {
                            await saveProfile();
                            setNotice(
                              "Profile saved. The game load list has not changed.",
                            );
                          })
                        }
                      >
                        Save profile
                      </button>
                    </>
                  )}
                  <button
                    onClick={() => {
                      setNewName("");
                      setNewProfile("empty");
                    }}
                  >
                    + New profile
                  </button>
                </div>
              </section>
            )}
            {page === "order" && (
              <>
                <div className="selection-summary">
                  <strong>{enabled} selected <span>/ {draft?.entries.length ?? 0} in this profile</span></strong>
                  <span>Saving a profile does not activate it.</span>
                </div>
                <section className="panel">
                  <div className="panel-heading">
                    <div>
                      <h2>Choose the mods to activate</h2>
                      <p>Tick a mod to include it. Use the arrows to set the load order.</p>
                    </div>
                    <div className="actions">
                      <button
                        onClick={() =>
                          void run("Choosing a load list…", async () => {
                            const path = await open({
                              title: "Import modlist.txt",
                              filters: [
                                { name: "Load list", extensions: ["txt"] },
                              ],
                            });
                            if (typeof path === "string")
                              setImporting({ path, base: parentPath(path) });
                          })
                        }
                      >
                        Import list
                      </button>
                      <button
                        onClick={() =>
                          void run("Exporting load list…", async () => {
                            const profile = await saveProfile();
                            const path = await save({
                              title: "Export to a NEW file outside the game",
                              defaultPath: "modlist.txt",
                              filters: [
                                { name: "Load list", extensions: ["txt"] },
                              ],
                            });
                            if (path) {
                              await request("export_modlist", {
                                id: profile.id,
                                path,
                              });
                              setNotice(`Exported enabled entries to ${path}`);
                            }
                          })
                        }
                      >
                        Export list ↗
                      </button>
                    </div>
                  </div>
                  <div className="profile-name">
                    <label>
                      Profile name
                      <input
                        value={draft?.name ?? ""}
                        maxLength={80}
                        onChange={(e) =>
                          draft && update({ ...draft, name: e.target.value })
                        }
                      />
                    </label>
                    <button
                      onClick={() => {
                        setNewName(`${draft?.name ?? "Profile"} copy`);
                        setNewProfile("copy");
                      }}
                    >
                      Duplicate
                    </button>
                    <button
                      className="danger-link"
                      disabled={dirty || (data?.profiles.length ?? 0) < 2}
                      onClick={() => setDeleting(true)}
                    >
                      Delete profile
                    </button>
                  </div>
                  {draft?.entries.length ? (
                    <ol className="mod-order">
                      {draft.entries.map((entry, index) => {
                        const mod = catalog.mods.find(
                          (m) => pathKey(m.path) === pathKey(entry.path),
                        );
                        return (
                          <li
                            className={!entry.enabled ? "disabled-entry" : ""}
                            key={entry.path}
                          >
                            <span className="order-number">
                              {String(index + 1).padStart(2, "0")}
                            </span>
                            <input
                              type="checkbox"
                              checked={entry.enabled}
                              aria-label={`Select ${fileName(entry.path)} for activation`}
                              onChange={(e) =>
                                update({
                                  ...draft,
                                  entries: draft.entries.map((v, i) =>
                                    i === index
                                      ? { ...v, enabled: e.target.checked }
                                      : v,
                                  ),
                                })
                              }
                            />
                            <div className="mod-info">
                              <strong>{fileName(entry.path)} <span className="selection-tag">{entry.enabled ? "Selected" : "Not selected"}</span></strong>
                              <span className="file-path" title={entry.path}>
                                {entry.path}
                              </span>
                              <small
                                className={
                                  mod?.warnings.length
                                    ? "warning-text"
                                    : "muted"
                                }
                              >
                                {mod
                                  ? mod.warnings.join(" ") ||
                                    `${mod.workshopId ? "Workshop" : "Local"} · ${mod.files.length} file${mod.files.length === 1 ? "" : "s"} · Compatibility unverified`
                                  : "Not in last scan · checked again before apply"}
                              </small>
                            </div>
                            <div className="row-actions">
                              <button
                                aria-label={`Move ${fileName(entry.path)} up`}
                                disabled={index === 0}
                                onClick={() =>
                                  update({
                                    ...draft,
                                    entries: moveEntry(
                                      draft.entries,
                                      index,
                                      index - 1,
                                    ),
                                  })
                                }
                              >
                                ↑
                              </button>
                              <button
                                aria-label={`Move ${fileName(entry.path)} down`}
                                disabled={index === draft.entries.length - 1}
                                onClick={() =>
                                  update({
                                    ...draft,
                                    entries: moveEntry(
                                      draft.entries,
                                      index,
                                      index + 1,
                                    ),
                                  })
                                }
                              >
                                ↓
                              </button>
                              <button
                                aria-label={`Remove ${fileName(entry.path)} from profile`}
                                onClick={() =>
                                  update({
                                    ...draft,
                                    entries: draft.entries.filter(
                                      (_, i) => i !== index,
                                    ),
                                  })
                                }
                              >
                                ×
                              </button>
                            </div>
                          </li>
                        );
                      })}
                    </ol>
                  ) : (
                    <div className="empty">
                      <span className="empty-icon">≡</span>
                      <h3>Your next adventure starts here</h3>
                      <p>
                        Add mods from your library or import an existing
                        modlist.txt.
                        <br />
                        Your game stays as it is until you apply a preview.
                      </p>
                      <button
                        className="secondary"
                        onClick={() => setPage("library")}
                      >
                        Browse mod library →
                      </button>
                    </div>
                  )}
                  <div className="panel-footer">
                    <span>
                      Apply copies enabled mods and their companions into your
                      game's Mods folder.
                    </span>
                    <button onClick={() => setPage("library")}>
                      + Add mods
                    </button>
                  </div>
                </section>
                <div className="apply-bar">
                  <div>
                    <strong>{enabled} {enabled === 1 ? "mod" : "mods"} selected for activation</strong>
                    <p>
                      Next: review the files and confirm. Your game changes only after Apply.
                    </p>
                  </div>
                  <button
                    className="primary"
                    disabled={!locations.gameRoot || locationsDirty}
                    onClick={reviewActivation}
                  >
                    Activate Mods →
                  </button>
                </div>
                {!locations.gameRoot && (
                  <p className="warning-text">
                    Choose your game folder in Locations & storage to enable
                    preview.
                  </p>
                )}
                {receipt && (
                  <section className="alert success">
                    <div>
                      <strong>{receipt.changed ? "Activation complete" : "Installed mods verified"}</strong>
                      <p>{receipt.message}</p>
                      <p>Restart Conan Exiles to load this selection and refresh its Mods menu.</p>
                      {receipt.backupDir && (
                        <p className="wrap">Backup: {receipt.backupDir}</p>
                      )}
                    </div>
                  </section>
                )}
              </>
            )}
            {page === "library" && (
              <section className="panel">
                <div className="panel-heading">
                  <div>
                    <h2>Local mod library</h2>
                    <p>
                      Last saved scan · {catalog.mods.length} packages ·
                      compatibility unverified
                    </p>
                    <p>Check tracked downloads for updates. Delete removes an imported library bundle after review.</p>
                  </div>
                  <button
                    className="secondary"
                    disabled={locationsDirty}
                    onClick={() => void run("Scanning your mod folders…", scan)}
                  >
                    ↻ Scan folders
                  </button>
                </div>
                <div className="search">
                  <label>
                    Find a mod
                    <input
                      type="search"
                      placeholder="Search by filename or Workshop ID…"
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                    />
                  </label>
                </div>
                {catalog.mods.length === 0 && (
                  <div className="empty">
                    <span className="empty-icon">▦</span>
                    <h3>Let’s find your mods</h3>
                    <p>
                      Add Workshop or local mod folders, then run a scan.
                      <br />
                      No downloads or changes to your files are made.
                    </p>
                    <button
                      className="secondary"
                      onClick={() => setPage("locations")}
                    >
                      Choose folders →
                    </button>
                  </div>
                )}
                <div className="library-grid">
                  {catalog.mods
                    .filter((m) =>
                      `${m.name} ${m.workshopId ?? ""}`
                        .toLowerCase()
                        .includes(search.toLowerCase()),
                    )
                    .map((mod) => {
                      const acquisition = acquisitionForPath(data?.acquisitions ?? [], mod.path);
                      const check = acquisition && updateChecks[acquisition.digest];
                      const added = draft?.entries.some(
                        (e) => pathKey(e.path) === pathKey(mod.path),
                      );
                      return (
                        <article className="mod-card" key={mod.path}>
                          <div className="card-top">
                            <span className="package-icon">◇</span>
                            <span className="tag">
                              {acquisition
                                ? `${acquisition.provider === "steam" ? "WORKSHOP" : acquisition.provider === "nexus" ? "NEXUS" : "LOCAL"} · LIBRARY COPY`
                                : mod.workshopId
                                ? `WORKSHOP · ${mod.workshopId}`
                                : "EXTERNAL / GAME COPY"}
                            </span>
                          </div>
                          <h3>{mod.name}</h3>
                          <p className="file-path" title={mod.path}>
                            {mod.path}
                          </p>
                          <p className="muted">
                            {formatSize(
                              mod.files.reduce((a, f) => a + f.size, 0),
                            )}{" "}
                            ·{" "}
                            {mod.files
                              .map((f) => fileName(f.path).split(".").pop())
                              .join(" + ")}
                          </p>
                          {mod.warnings.map((w) => (
                            <p className="warning-text" key={w}>
                              {w}
                            </p>
                          ))}
                          <div className="card-footer">
                            <small>Compatibility unverified</small>
                            <button
                              disabled={added || !!mod.warnings.length}
                              onClick={() =>
                                draft &&
                                update({
                                  ...draft,
                                  entries: addEntry(draft.entries, mod.path),
                                })
                              }
                            >
                              {added ? "Added ✓" : "+ Add to profile"}
                            </button>
                          </div>
                          <div className="library-actions">
                            <button disabled={!acquisition} onClick={() => acquisition && void run("Checking the source for updates…", async () => {
                              const result = await request<UpdateCheck>("online", { action: { kind: "check_update", digest: acquisition.digest } });
                              setUpdateChecks((previous) => ({ ...previous, [acquisition.digest]: result }));
                            })}>Check for updates</button>
                            {acquisition && ["steam", "nexus"].includes(acquisition.provider) && (
                              <button onClick={() => void run("Opening the mod page…", async () => {
                                await request("online", { action: { kind: "open_site", site: acquisition.provider, id: acquisition.modId } });
                              })}>View source ↗</button>
                            )}
                            <button className="danger" disabled={!acquisition || dirty} title={dirty ? "Save or discard profile changes first" : "Review this entire imported bundle before deleting"}
                              onClick={() => acquisition && void run("Reviewing library files…", async () => {
                                setLibraryDelete(await request<LibraryDeletePreview>("preview_library_delete", { digest: acquisition.digest }));
                              })}>Delete library copy…</button>
                          </div>
                          {check && <div className={`update-status ${check.state}`} role="status">
                            <strong>{({ available: "Update available", current: "No linked update found", unknown: "Manual review needed", untracked: "Local import", error: "Check failed" })[check.state]}</strong>
                            <p>{check.message}</p>
                            <small>Checked {new Date(check.checkedAt * 1000).toLocaleString()}</small>
                            {check.state === "available" && <p>Use Discover & import to download the version you want. Your profile and installed files are not switched automatically.</p>}
                          </div>}
                          {!acquisition && <p className="muted">This file is outside the managed library. Manage it in its source folder or Steam. Use Load order and Apply to deactivate game copies.</p>}
                        </article>
                      );
                    })}
                </div>
                {(catalog.warnings.length > 0 ||
                  catalog.orphanFiles.length > 0) && (
                  <details className="scan-notes">
                    <summary>
                      Scan notes (
                      {catalog.warnings.length + catalog.orphanFiles.length})
                    </summary>
                    {catalog.warnings.map((w, i) => (
                      <p key={i}>{w}</p>
                    ))}
                    {catalog.orphanFiles.map((p) => (
                      <p key={p}>Companion without a matching .pak: {p}</p>
                    ))}
                  </details>
                )}
              </section>
            )}
            {page === "discover" && <Discover onImported={scan} />}
            {page === "locations" && (
              <>
                <section className="panel">
                  <div className="panel-heading">
                    <div>
                      <h2>Game installation</h2>
                      <p>
                        Checked automatically at startup for Conan client
                        executables. Your saved selection takes priority.
                      </p>
                    </div>
                    <button
                      className="secondary"
                      onClick={() =>
                        void run(
                          "Looking for Conan Exiles installations…",
                          async () => {
                            const result = await request<Detection>("detect");
                            setDetected(result);
                            setLocations({
                              gameRoot:
                                locations.gameRoot ||
                                (result.gameRoots.length === 1
                                  ? result.gameRoots[0]
                                  : ""),
                              scanRoots: [
                                ...new Set([
                                  ...locations.scanRoots,
                                  ...result.workshopRoots,
                                ]),
                              ],
                            });
                          },
                        )
                      }
                    >
                      Detect again
                    </button>
                  </div>
                  <div className="settings-body">
                    <label>
                      Game folder or executable
                      <div className="input-row">
                        <input
                          placeholder="D:\Games\Conan Exiles Enhanced"
                          value={locations.gameRoot}
                          onChange={(e) =>
                            setLocations({
                              ...locations,
                              gameRoot: e.target.value,
                            })
                          }
                        />
                        <button
                          onClick={() =>
                            void run("Choosing game folder…", () =>
                              chooseGame(),
                            )
                          }
                        >
                          Choose folder…
                        </button>
                        <button
                          onClick={() =>
                            void run("Choosing game executable…", () =>
                              chooseGame(true),
                            )
                          }
                        >
                          Choose .exe…
                        </button>
                      </div>
                    </label>
                    <p className="muted">
                      Accepts ConanSandbox.exe, ConanSandbox_BE.exe, or
                      ConanSandbox-Win64-Shipping.exe inside
                      ConanSandbox/Binaries/Win64.
                    </p>
                    {detected && (
                      <div className="detection">
                        {detected.gameRoots.length > 1 && (
                          <p>
                            More than one installation was found. Select the one
                            you use.
                          </p>
                        )}
                        {detected.gameRoots.map((p) => (
                          <button
                            key={p}
                            onClick={() =>
                              setLocations({ ...locations, gameRoot: p })
                            }
                          >
                            {p}
                          </button>
                        ))}
                        {detected.warnings.map((w) => (
                          <p key={w}>{w}</p>
                        ))}
                      </div>
                    )}
                    <h3>Mod source folders</h3>
                    <p className="muted">
                      The game’s Mods folder is included automatically. Add
                      Workshop content/440900 or your local mod folders.
                    </p>
                    {locations.scanRoots.map((path, index) => (
                      <div className="input-row source-row" key={index}>
                        <input
                          aria-label={`Mod source folder ${index + 1}`}
                          value={path}
                          onChange={(e) =>
                            setLocations({
                              ...locations,
                              scanRoots: locations.scanRoots.map((p, i) =>
                                i === index ? e.target.value : p,
                              ),
                            })
                          }
                        />
                        <button
                          aria-label={`Remove source ${path}`}
                          onClick={() =>
                            setLocations({
                              ...locations,
                              scanRoots: locations.scanRoots.filter(
                                (_, i) => i !== index,
                              ),
                            })
                          }
                        >
                          Remove
                        </button>
                      </div>
                    ))}
                    <div className="actions">
                      <button
                        onClick={() =>
                          void run("Choosing mod source…", chooseSource)
                        }
                      >
                        + Browse source folder
                      </button>
                      <button
                        onClick={() =>
                          setLocations({
                            ...locations,
                            scanRoots: [...locations.scanRoots, ""],
                          })
                        }
                      >
                        + Enter path
                      </button>
                    </div>
                  </div>
                  <div className="panel-footer">
                    <span>
                      {locationsDirty
                        ? "Unsaved folder changes"
                        : "Folder settings saved"}
                    </span>
                    <button
                      className="primary"
                      onClick={() =>
                        void run("Saving locations and scanning…", async () => {
                          const settings = await request<Settings>(
                            "save_settings",
                            { settings: locations },
                          );
                          setData((d) => d && { ...d, settings });
                          setLocations(settings);
                          setPreview(null);
                          await scan();
                        })
                      }
                    >
                      Save locations & scan
                    </button>
                  </div>
                </section>
                <section className="panel storage-panel">
                  <h2>Storage & recovery</h2>
                  <p>
                    Profiles, settings and your last scan are stored locally in
                    SQLite.
                  </p>
                  <code>{data?.dataDir ?? "Available in the desktop app"}</code>
                  <p>
                    Each apply saves the old list as{" "}
                    <code>backups/apply-…/original.txt</code>, the proposed list
                    and a manifest. To recover an older load order, import its
                    original.txt and set the relative-path folder to the game’s
                    ConanSandbox/Mods folder, then preview and apply.
                  </p>
                  <p className="warning-text">
                    Backups cover modlist.txt only. Keep separate backups of
                    world saves. A load order change may affect existing worlds.
                  </p>
                  <details>
                    <summary>ConanForge credits</summary>
                    <p>
                      Startup sound: “Smithing Sound” by Hansjörg Malthaner
                      (Varkalandar), CC BY 3.0. Edited into three strikes with
                      fades, echoes and adjusted volume.
                    </p>
                    <p className="wrap">
                      Source: https://opengameart.org/content/smithing-sound
                      <br />
                      Creator: https://opengameart.org/users/varkalandar
                      <br />
                      License: https://creativecommons.org/licenses/by/3.0/
                    </p>
                    <p>
                      Logo supplied by the project owner; transparent and icon
                      adaptations generated for ConanForge.
                    </p>
                  </details>
                </section>
                <section className="integrations">
                  {data?.integrations.map((i) => (
                    <div className="panel" key={i.name}>
                      <span className="tag">{i.available ? "AVAILABLE" : "BROWSER IMPORT"}</span>
                      <h3>{i.name}</h3>
                      <p>{i.detail}</p>
                    </div>
                  ))}
                </section>
              </>
            )}
          </fieldset>
        </div>
      </main>
      {preview && (
        <Modal title="Review & activate your mods" close={closeModal}>
          {dialogError}
          <p className="review-intro">Check the list below, close Conan, then confirm. Apply copies the selected files into the game’s Mods folder and backs up the previous load list. World saves need a separate backup.</p>
          <p className="wrap">
            <strong>{preview.profileName}</strong> · {preview.enabledCount}{" "}
            enabled mods
            <br />
            {preview.target}
          </p>
          <div className="diff">
            <div>
              <h3>Current file</h3>
              <pre>{preview.before || "(empty or absent)"}</pre>
            </div>
            <div>
              <h3>After apply</h3>
              <pre>{preview.after || "(all mods disabled)"}</pre>
            </div>
          </div>
          <h3>
            Game mod files ·{" "}
            {formatSize(
              preview.files
                .filter((f) => f.copy)
                .reduce((total, f) => total + f.size, 0),
            )}{" "}
            to copy
          </h3>
          <ul className="preview-warnings">
            {preview.files.map((file) => (
              <li key={file.target} className="wrap">
                <strong>
                  {file.copy ? "Copy" : "Reuse identical file"}:{" "}
                  {fileName(file.target)}
                </strong>
                {" · "}
                {formatSize(file.size)}
                <br />
                From: {file.source}
                <br />
                To: {file.target}
              </li>
            ))}
          </ul>
          <ul className="preview-warnings">
            {preview.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
          <label className="confirm-row">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
              disabled={!!busy}
            />{" "}
            I have closed the game and server, reviewed these files and the load
            order, and backed up any important world saves.
          </label>
          <div className="dialog-actions">
            <button disabled={!!busy} onClick={closeModal}>
              Cancel
            </button>
            <button
              className="primary"
              disabled={!confirmed || !!busy}
              onClick={() =>
                void run(
                  "Backing up, copying mods and applying load order…",
                  async () => {
                    try {
                      const result = await request<Receipt>("apply", {
                        token: preview.token,
                        confirmed,
                      });
                      setReceipt(result);
                      setPreview(null);
                      setNotice(result.message);
                    } catch (e) {
                      setPreview(null);
                      throw e;
                    }
                  },
                )
              }
            >
              {preview.changed ? "Back up & apply" : "Verify installed mods"}
            </button>
          </div>
        </Modal>
      )}
      {newProfile && (
        <Modal
          title={
            newProfile === "copy" ? "Duplicate profile" : "Create a profile"
          }
          close={closeModal}
        >
          {dialogError}
          <label>
            Profile name
            <input
              autoFocus
              value={newName}
              maxLength={80}
              onChange={(e) => setNewName(e.target.value)}
              disabled={!!busy}
            />
          </label>
          <div className="dialog-actions">
            <button disabled={!!busy} onClick={closeModal}>
              Cancel
            </button>
            <button
              className="primary"
              disabled={
                !newName.trim() || !!busy || (dirty && newProfile === "empty")
              }
              onClick={() =>
                void run("Creating profile…", async () => {
                  const source =
                    newProfile === "copy" ? await saveProfile() : null;
                  let profile = await request<Profile>("create_profile", {
                    name: newName,
                  });
                  if (source)
                    profile = await request<Profile>("save_profile", {
                      profile: { ...profile, entries: source.entries },
                    });
                  await load(profile.id);
                  setNewProfile(null);
                })
              }
            >
              Create profile
            </button>
          </div>
          {dirty && newProfile === "empty" && (
            <p>
              Save or discard your current edits before creating an empty
              profile.
            </p>
          )}
        </Modal>
      )}
      {libraryDelete && (
        <Modal title="Delete this library bundle?" close={closeModal}>
          {dialogError}
          <p>This permanently deletes the manager’s imported copy: {libraryDelete.fileCount} files ({formatSize(libraryDelete.bytes)}), including companions.</p>
          <ul>{libraryDelete.packages.map((name, index) => <li key={`${index}-${name}`}>{name}</li>)}</ul>
          <p className="file-path">{libraryDelete.folder}</p>
          <p>Copies in the game and original downloads stay in place. To deactivate mods in the game, change Load order and Apply.</p>
          {libraryDelete.profiles.length > 0 && <p className="alert warning">Used by: {libraryDelete.profiles.join(", ")}. Remove these mods from those profiles and save first. Disabled entries also count.</p>}
          <div className="dialog-actions">
            <button disabled={!!busy} onClick={closeModal}>Cancel</button>
            <button className="danger" disabled={!!busy || dirty || libraryDelete.profiles.length > 0}
              onClick={() => void run("Deleting library copy…", async () => {
                const message = await request<string>("delete_library", { token: libraryDelete.token, confirmed: true });
                setLibraryDelete(null);
                await scan();
                setNotice(message);
              })}>Delete library copy permanently</button>
          </div>
        </Modal>
      )}
      {deleting && draft && (
        <Modal title="Delete this profile?" close={closeModal}>
          {dialogError}
          <p>
            Delete “{draft.name}” from this manager? The game load list and mod
            files will stay unchanged.
          </p>
          <div className="dialog-actions">
            <button disabled={!!busy} onClick={closeModal}>
              Cancel
            </button>
            <button
              className="danger"
              disabled={!!busy}
              onClick={() =>
                void run("Deleting profile…", async () => {
                  await request("delete_profile", {
                    id: draft.id,
                    revision: draft.revision,
                  });
                  await load();
                  setDeleting(false);
                })
              }
            >
              Delete profile
            </button>
          </div>
        </Modal>
      )}
      {importing && draft && (
        <Modal title="Import a load list" close={closeModal}>
          {dialogError}
          <p className="wrap">{importing.path}</p>
          <p>
            This replaces the entries in “{draft.name}”. All imported entries
            are enabled. Missing files remain listed and block apply.
          </p>
          <label>
            Resolve relative paths from this folder
            <input
              value={importing.base}
              onChange={(e) =>
                setImporting({ ...importing, base: e.target.value })
              }
              disabled={!!busy}
            />
          </label>
          <p className="muted">
            For a backup or a list moved from your game, use its original
            ConanSandbox/Mods folder.
          </p>
          <div className="dialog-actions">
            <button disabled={!!busy} onClick={closeModal}>
              Cancel
            </button>
            <button
              className="primary"
              disabled={!!busy || !importing.base}
              onClick={() =>
                void run("Importing load list…", async () => {
                  const profile = await saveProfile();
                  accept(
                    await request<Profile>("import_modlist", {
                      ...importing,
                      id: profile.id,
                      revision: profile.revision,
                    }),
                  );
                  setPreview(null);
                  setImporting(null);
                  setNotice(
                    "Load list imported into the profile. Nothing applied to the game.",
                  );
                })
              }
            >
              Replace profile entries
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
