import { useEffect, useRef, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { formatSize } from "./domain";

interface Info { currentVersion: string; checkAtStartup: boolean; installed: boolean }
interface Candidate { version: string; notes: string | null; downloaded: boolean }
interface Progress { bytes: number; total: number | null }
const call = <T,>(kind: string, fields = {}, progress?: Channel<Progress>) =>
  invoke<T>("app_update", { action: { kind, ...fields }, progress: progress ?? new Channel<Progress>() });

export default function AppUpdates({ ready, shown, unsaved, blocked, onOpen, onInstalling }: {
  ready: boolean; shown: boolean; unsaved: boolean; blocked: boolean;
  onOpen: () => void; onInstalling: (value: boolean) => void;
}) {
  const [info, setInfo] = useState<Info | null>(null);
  const [candidate, setCandidate] = useState<Candidate | null>(null);
  const [phase, setPhase] = useState<"idle" | "checking" | "current" | "downloading" | "installing">("idle");
  const [error, setError] = useState("");
  const [progress, setProgress] = useState<Progress>({ bytes: 0, total: null });
  const [confirming, setConfirming] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const [savingPreference, setSavingPreference] = useState(false);
  const started = useRef(false);
  const modal = useRef<HTMLDialogElement>(null);
  const inFlight = useRef(false);
  const working = ["checking", "downloading", "installing"].includes(phase);
  const installBlocked = unsaved || blocked || working;

  async function check() {
    if (inFlight.current) return;
    inFlight.current = true;
    setPhase("checking"); setError(""); setCandidate(null);
    try {
      const update = await call<Candidate | null>("check");
      setCandidate(update); setPhase(update ? "idle" : "current");
    } catch (e) { setError(String(e)); setPhase("idle"); }
    finally { inFlight.current = false; }
  }
  useEffect(() => {
    if (!ready || started.current) return;
    started.current = true;
    void call<Info>("status").then((value) => {
      setInfo(value);
      if (value.checkAtStartup) void check();
    }).catch((e) => setError(String(e)));
  }, [ready]);
  useEffect(() => { if (confirming) modal.current?.showModal(); }, [confirming]);

  async function download() {
    if (!candidate || inFlight.current) return;
    inFlight.current = true;
    setPhase("downloading"); setError(""); setProgress({ bytes: 0, total: null });
    const channel = new Channel<Progress>();
    channel.onmessage = setProgress;
    try { setCandidate(await call<Candidate>("download", { version: candidate.version }, channel)); }
    catch (e) { setError(String(e)); }
    finally { setPhase("idle"); inFlight.current = false; }
  }
  async function install() {
    if (!candidate?.downloaded || !confirmed || installBlocked || inFlight.current) return;
    inFlight.current = true;
    setPhase("installing"); setError(""); onInstalling(true);
    try {
      await call("install", { version: candidate.version, confirmed: true });
    } catch (e) {
      setError(String(e)); setPhase("idle"); setConfirming(false);
      inFlight.current = false; onInstalling(false);
    }
  }
  async function preference(enabled: boolean) {
    setSavingPreference(true); setError("");
    try { await call("preference", { enabled }); setInfo((previous) => previous && { ...previous, checkAtStartup: enabled }); }
    catch (e) { setError(String(e)); }
    finally { setSavingPreference(false); }
  }
  const openReleases = () => void call("open_releases").catch((e) => setError(String(e)));
  if (!shown) return candidate ? (
    <div className="app-update-banner" role="status">
      <span>ConanForge {candidate.version} {candidate.downloaded ? "is ready to install" : "is available"}.</span>
      <button onClick={onOpen}>Review app update →</button>
    </div>
  ) : null;
  return <section className="panel app-updates" aria-labelledby="updates-title">
    <div className="panel-heading">
      <div><p className="eyebrow">CONANFORGE UPDATES</p><h2 id="updates-title">Keep your forge up to date.</h2>
        <p>New app versions come from our GitHub releases. You decide when to install.</p></div>
      <span className="pill">{info ? `Installed version ${info.currentVersion}` : "Loading version…"}</span>
    </div>
    <div className="settings-body">
      <label className="update-preference"><input type="checkbox" checked={info?.checkAtStartup ?? true}
        disabled={!info || savingPreference || working} onChange={(e) => void preference(e.target.checked)} />
        Check for app updates at startup</label>
      <p className="muted">Checks contact GitHub. Nothing is downloaded or installed automatically.</p>
      <div className="actions"><button className="primary" onClick={() => void check()} disabled={!info || working}>
        {phase === "checking" ? "Checking GitHub…" : "Check for app updates"}</button>
        <button onClick={openReleases} disabled={phase === "installing"}>View GitHub releases ↗</button></div>
      {phase === "current" && <p className="alert success" role="status">You’re up to date. No newer app version is available.</p>}
      {error && <p className="alert error" role="alert">{error}</p>}
      {info && !info.installed && <p className="alert">This is a portable or development copy. Use the Windows installer once to enable installation of updates from the app. Your existing profiles and library will remain available.</p>}
      {candidate && <div className="app-update-card">
        <span className="eyebrow">{candidate.downloaded ? "VERIFIED AND READY" : "NEW APP VERSION"}</span>
        <h3>ConanForge {candidate.version}</h3>
        <p className="update-release-notes">{candidate.notes || "See GitHub for the release notes."}</p>
        {phase === "downloading" && <div role="status"><p>Downloading and verifying… {formatSize(progress.bytes)}{progress.total ? ` of ${formatSize(progress.total)}` : ""}</p>
          <progress aria-label="App update download" max={progress.total || undefined} value={progress.total ? Math.min(progress.bytes, progress.total) : undefined} /></div>}
        {!candidate.downloaded ? <button className="primary" onClick={() => void download()} disabled={!info?.installed || working}>Download app update</button>
          : <button className="primary" onClick={() => { setConfirmed(false); setConfirming(true); }} disabled={installBlocked}>Install and restart…</button>}
        {unsaved && <p className="warning-text">Save or discard your profile and location changes before installing.</p>}
        {blocked && <p className="muted">Finish the current action before installing an app update.</p>}
        <p className="muted">The app verifies the installer’s signature and version before installation. Your app database is backed up before setup starts. Mods and game files stay separate.</p>
      </div>}
    </div>
    {confirming && <dialog ref={modal} onCancel={(e) => { e.preventDefault(); if (phase !== "installing") setConfirming(false); }} aria-labelledby="update-confirm-title">
      <div className="dialog-heading"><h2 id="update-confirm-title">Install ConanForge {candidate?.version}?</h2></div>
      <div className="dialog-body"><p>ConanForge will close, install the verified update and reopen. Your saved profiles, imported library and game files are kept.</p>
        <p>Any active mod download or import must finish first.</p>
        <label><input type="checkbox" checked={confirmed} onChange={(e) => setConfirmed(e.target.checked)} disabled={phase === "installing"} /> My changes are saved. I’m ready to close and update ConanForge.</label>
        {unsaved && <p className="warning-text">Save your changes before continuing.</p>}
      </div>
      <div className="dialog-actions"><button disabled={phase === "installing"} onClick={() => setConfirming(false)}>Not now</button>
        <button className="primary" disabled={!confirmed || installBlocked} onClick={() => void install()}>{phase === "installing" ? "Starting installer…" : "Install and restart"}</button></div>
    </dialog>}
  </section>;
}
