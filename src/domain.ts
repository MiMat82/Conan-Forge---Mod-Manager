import type { Acquisition, Entry } from "./types.ts";

export const pathKey = (path: string) =>
  path.replaceAll("\\", "/").toLowerCase();
export const fileName = (path: string) =>
  path.replaceAll("\\", "/").split("/").pop() || path;
export const parentPath = (path: string) =>
  path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));

export function acquisitionForPath(acquisitions: Acquisition[], path: string) {
  return acquisitions.find((record) =>
    pathKey(path).startsWith(pathKey(record.path).replace(/\/+$/, "") + "/"),
  );
}

export function moveEntry(entries: Entry[], from: number, to: number): Entry[] {
  if (
    from < 0 ||
    from >= entries.length ||
    to < 0 ||
    to >= entries.length ||
    from === to
  )
    return entries;
  const next = [...entries];
  const [entry] = next.splice(from, 1);
  next.splice(to, 0, entry);
  return next;
}
export function addEntry(entries: Entry[], path: string): Entry[] {
  return entries.some((e) => pathKey(e.path) === pathKey(path))
    ? entries
    : [...entries, { path, enabled: true }];
}
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}
