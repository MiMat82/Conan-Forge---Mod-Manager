import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

// This packaging addition must contain the already-published v0.4.1 binary.
// Rebuilt application versions need their own release qualification and hash.
const file = new URL('../src-tauri/target/release/conan-mod-manager.exe', import.meta.url);
const expected = 'c68d6813646c9041210cea60efc9976d6f38e8750b2769dd7aff4ca541e7279c';
const actual = createHash('sha256').update(await readFile(file)).digest('hex');
assert.equal(actual, expected,
  'Installer packaging requires the published v0.4.1 executable. See docs/INSTALLER.md.');
console.log('Published v0.4.1 executable verified: ' + actual);
