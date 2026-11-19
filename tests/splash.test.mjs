import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { Script } from "node:vm";
import ts from "typescript";

const code = ts.transpileModule(
  readFileSync(new URL("../src/splash.ts", import.meta.url), "utf8"),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } },
).outputText;
const settle = () => new Promise(setImmediate);

function splash({ decode = Promise.resolve(), blockedAudio = false, alreadyOpen = false } = {}) {
  const calls = [];
  const timers = new Map();
  const skip = new EventTarget();
  const document = new EventTarget();
  const classes = new Set();
  let audio;
  document.querySelector = (selector) => selector === "#skip" ? skip : { decode: () => decode };
  document.body = { classList: { add: (name) => classes.add(name) } };
  new Script(code).runInNewContext({
    exports: {}, document, window: new EventTarget(),
    require: (name) => {
      assert.equal(name, "@tauri-apps/api/core");
      return {
        isTauri: () => true,
        invoke: async (command) => {
          calls.push(command);
          return command === "show_intro" ? !alreadyOpen : undefined;
        },
      };
    },
    Audio: class extends EventTarget {
      constructor() { super(); audio = this; this.played = false; this.paused = false; }
      async play() { this.played = true; if (blockedAudio) throw new Error("Device unavailable"); }
      pause() { this.paused = true; }
    },
    setTimeout: (callback, delay) => { assert.ok(delay > 0 && delay <= 5000); timers.set(1, callback); return 1; },
    clearTimeout: (id) => timers.delete(id),
  });
  return { calls, timers, skip, document, classes, get audio() { return audio; } };
}

test("audio completion reveals the manager once and stops playback", async () => {
  const intro = splash();
  await settle();
  assert.equal(intro.audio.played, true);
  assert.equal(intro.classes.has("playing"), true);
  intro.audio.dispatchEvent(new Event("ended"));
  intro.skip.dispatchEvent(new Event("click"));
  await settle();
  assert.deepEqual(intro.calls, ["show_intro", "finish_intro"]);
  assert.equal(intro.audio.paused, true);
  assert.equal(intro.timers.size, 0);
});

test("skipping before the logo loads does not start audio later", async () => {
  let loaded;
  const intro = splash({ decode: new Promise((resolve) => { loaded = resolve; }) });
  intro.skip.dispatchEvent(new Event("click"));
  loaded();
  await settle();
  assert.deepEqual(intro.calls, ["finish_intro"]);
  assert.equal(intro.audio.played, false);
});

test("missing artwork and blocked audio still reach the timed fallback", async () => {
  const intro = splash({ decode: Promise.reject(new Error("Missing image")), blockedAudio: true });
  await settle();
  assert.equal(intro.timers.size, 1);
  intro.timers.get(1)();
  await settle();
  assert.deepEqual(intro.calls, ["show_intro", "finish_intro"]);
  assert.equal(intro.audio.paused, true);
});

test("Escape and Enter both skip without duplicate completion", async () => {
  for (const key of ["Escape", "Enter"]) {
    const intro = splash();
    await settle();
    intro.document.dispatchEvent(Object.assign(new Event("keydown"), { key }));
    intro.audio.dispatchEvent(new Event("ended"));
    await settle();
    assert.deepEqual(intro.calls, ["show_intro", "finish_intro"]);
    assert.equal(intro.audio.paused, true);
  }
});

test("a native fallback that already opened the manager prevents a late intro", async () => {
  const intro = splash({ alreadyOpen: true });
  await settle();
  assert.deepEqual(intro.calls, ["show_intro"]);
  assert.equal(intro.audio.played, false);
  assert.equal(intro.timers.size, 0);
});
