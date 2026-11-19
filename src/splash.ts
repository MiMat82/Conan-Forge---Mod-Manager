import { invoke, isTauri } from "@tauri-apps/api/core";

const logo = document.querySelector<HTMLImageElement>("#forge-logo")!;
const skip = document.querySelector<HTMLButtonElement>("#skip")!;
const audio = new Audio("/brand/forge-intro.wav");
audio.volume = 0.65;
audio.preload = "auto";
let finishing = false;
let fallback: ReturnType<typeof setTimeout> | undefined;

async function finish() {
  if (finishing) return;
  finishing = true;
  clearTimeout(fallback);
  audio.pause();
  try {
    if (isTauri()) await invoke("finish_intro");
    else window.location.replace("/");
  } catch {
    finishing = false;
    skip.textContent = "Open ConanForge";
  }
}

skip.addEventListener("click", () => void finish());
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" || event.key === "Enter") void finish();
});
window.addEventListener("pagehide", () => audio.pause());
audio.addEventListener("ended", () => void finish());

async function start() {
  await logo.decode().catch(() => undefined);
  if (finishing) return;
  if (isTauri() && !(await invoke<boolean>("show_intro"))) return;
  document.body.classList.add("playing");
  // A missing or blocked audio track must still open the manager promptly.
  fallback = setTimeout(() => void finish(), 4600);
  await audio.play().catch(() => undefined);
}

void start().catch(() => void finish());
