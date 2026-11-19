import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { execFileSync } from "node:child_process";
import assert from "node:assert/strict";

const metadata = JSON.parse(
  execFileSync(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--format-version",
      "1",
      "--locked",
      "--filter-platform",
      "x86_64-pc-windows-msvc",
    ],
    { encoding: "utf8", maxBuffer: 30_000_000 },
  ),
);
const selected = new Set(metadata.resolve.nodes.map((n) => n.id));
const records = metadata.packages
  .filter((p) => p.source && selected.has(p.id))
  .map((p) => ({
    name: `${p.name}@${p.version}`,
    license: p.license ?? "See license file",
    dir: dirname(p.manifest_path),
    licenseFile: p.license_file,
    source: `https://crates.io/api/v1/crates/${p.name}/${p.version}/download`,
  }));
const lock = JSON.parse(readFileSync("package-lock.json", "utf8"));
for (const [path, pkg] of Object.entries(lock.packages)) {
  if (!path || pkg.dev) continue;
  records.push({
    name: `${path.replace(/^node_modules\//, "")}@${pkg.version}`,
    license: pkg.license ?? "See license file",
    dir: path,
    source: pkg.resolved,
  });
}
assert(records.some((r) => r.name.startsWith("tauri@")));
assert(records.some((r) => r.name.startsWith("react@")));
let output =
  "THIRD-PARTY DEPENDENCIES — WINDOWS BUILD\n\nGenerated from Cargo.lock and package-lock.json. Includes applicable build dependencies as well as runtime dependencies. Original license and notice text follows each package. Rust standard libraries are distributed under MIT/Apache-2.0; the separately installed Microsoft WebView2 runtime has its own Microsoft terms.\n\n";
const missing = [];
for (const r of records.sort((a, b) => a.name.localeCompare(b.name))) {
  output += `\n${"=".repeat(78)}\n${r.name}\nDeclared license: ${r.license}\nUnmodified source: ${r.source}\n`;
  const names = readdirSync(r.dir).filter((n) =>
    /^(licen[sc]e|copying|copyright|notice|patents)([._-]|$)/i.test(n),
  );
  if (r.licenseFile && !names.includes(r.licenseFile))
    names.push(r.licenseFile);
  let count = 0;
  for (const name of names) {
    try {
      output += `\n--- ${name} ---\n${readFileSync(join(r.dir, name), "utf8")}\n`;
      count++;
    } catch {
      /* Some crates use a directory of licenses; reported below for review. */
    }
  }
  if (!count) {
    const name = r.name.split("@")[0];
    const fallback = name.startsWith("webview2-com") ? "webview2" : name;
    try {
      output += `\n--- Upstream license ---\n${readFileSync(`licenses/${fallback}.txt`, "utf8")}\n`;
    } catch {
      missing.push(r.name);
    }
  }
}
assert.equal(missing.length, 0, `Missing license text: ${missing.join(", ")}`);
writeFileSync("THIRD_PARTY_LICENSES.txt", output);
console.log(
  JSON.stringify(
    {
      packages: records.length,
      licenseBytes: Buffer.byteLength(output),
      missingLicenseText: missing,
    },
    null,
    2,
  ),
);
