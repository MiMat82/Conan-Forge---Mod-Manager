import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Prepares a candidate feed; publication remains a separate, verified release step.
const root = fileURLToPath(new URL('../', import.meta.url));
const [installer, signature, notes, output] = process.argv.slice(2);
assert(installer && signature && notes && output,
  'Usage: node scripts/prepare-update.mjs <setup.exe> <setup.exe.sig> <notes.txt> <candidate.json>');
const config = JSON.parse(await readFile(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const version = config.version;
assert.match(version, /^\d+\.\d+\.\d+$/);
assert.equal(path.basename(installer), `ConanForge-v${version}-windows-x64-setup.exe`);
const bytes = await readFile(installer);
assert(bytes.length > 0 && bytes.length < 128 * 1024 * 1024);
const sig = (await readFile(signature, 'utf8')).trim();
assert(sig.length > 100, 'Missing updater signature');
const manifest = {
  version,
  notes: (await readFile(notes, 'utf8')).trim(),
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: sig,
      url: `https://github.com/MiMat82/Conan-Forge---Mod-Manager/releases/download/v${version}/${path.basename(installer)}`,
    },
  },
};
await writeFile(output, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ candidate: output, version,
  sha256: createHash('sha256').update(bytes).digest('hex') }));
console.log('Verify with the Rust verify_update example before publishing this feed.');
