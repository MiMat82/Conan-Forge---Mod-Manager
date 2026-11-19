import { test } from "node:test";
import assert from "node:assert/strict";
import {
  moveEntry,
  addEntry,
  pathKey,
  fileName,
  parentPath,
  formatSize,
  acquisitionForPath,
} from "../src/domain.ts";

test("library actions match the exact owning folder across Windows casing and separators", () => {
  const records = [{ path: "C:\\Library\\transfer-one", digest: "one" }];
  assert.equal(acquisitionForPath(records, "c:/library/TRANSFER-ONE/sub/Mod.pak")?.digest, "one");
  assert.equal(acquisitionForPath(records, "C:/Library/transfer-one-extra/Mod.pak"), undefined);
  assert.equal(acquisitionForPath(records, "D:/Game/Mods/Mod.pak"), undefined);
});

test("moving entries preserves disabled state and does not mutate the saved profile", () => {
  const entries = [
    { path: "a.pak", enabled: false },
    { path: "b.pak", enabled: true },
    { path: "c.pak", enabled: true },
  ];
  const result = moveEntry(entries, 0, 2);
  assert.deepEqual(
    result.map((e) => e.path),
    ["b.pak", "c.pak", "a.pak"],
  );
  assert.equal(result[2].enabled, false);
  assert.equal(entries[0].path, "a.pak");
  assert.equal(moveEntry(entries, 0, -1), entries);
  assert.equal(moveEntry(entries, 2, 3), entries);
});
test("adding the same Windows path does not duplicate or re-enable an entry", () => {
  const entries = [{ path: "C:\\Mods\\A.PAK", enabled: false }];
  assert.equal(addEntry(entries, "c:/mods/a.pak"), entries);
  assert.deepEqual(addEntry(entries, "C:\\Mods\\B.pak")[1], {
    path: "C:\\Mods\\B.pak",
    enabled: true,
  });
});
test("Windows and Unicode paths retain useful labels and import base folders", () => {
  assert.equal(fileName("D:\\Mods\\Ørken.pak"), "Ørken.pak");
  assert.equal(parentPath("D:\\Game\\Mods\\modlist.txt"), "D:\\Game\\Mods");
  assert.equal(pathKey("D:\\Mods\\A.PAK"), "d:/mods/a.pak");
  assert.equal(formatSize(0), "0 B");
  assert.equal(formatSize(1024 ** 3), "1.00 GB");
});
