export interface Entry {
  path: string;
  enabled: boolean;
}
export interface Profile {
  id: number;
  name: string;
  revision: number;
  entries: Entry[];
}
export interface Settings {
  gameRoot: string;
  scanRoots: string[];
}
export interface FileStamp {
  path: string;
  size: number;
  modified: number;
}
export interface ModPackage {
  path: string;
  name: string;
  files: FileStamp[];
  workshopId: string | null;
  warnings: string[];
}
export interface Scan {
  mods: ModPackage[];
  warnings: string[];
  orphanFiles: string[];
}
export interface Detection {
  steamRoots: string[];
  gameRoots: string[];
  workshopRoots: string[];
  warnings: string[];
}
export interface Preview {
  token: string;
  target: string;
  profileName: string;
  before: string;
  after: string;
  changed: boolean;
  warnings: string[];
  enabledCount: number;
  files: { source: string; target: string; size: number; copy: boolean }[];
}
export interface Receipt {
  target: string;
  backupDir: string | null;
  changed: boolean;
  message: string;
}
export interface Bootstrap {
  acquisitions: Acquisition[];
  profiles: Profile[];
  settings: Settings;
  catalog: Scan;
  dataDir: string;
  detection: Detection;
  integrations: { name: string; available: boolean; detail: string }[];
}
export interface Acquisition {
  digest: string;
  path: string;
  provider: string;
  modId: string;
  fileId: string;
  name: string;
  created: number;
}
export interface UpdateCheck {
  digest: string;
  state: "available" | "current" | "unknown" | "untracked" | "error";
  message: string;
  checkedAt: number;
}
export interface LibraryDeletePreview {
  token: string;
  digest: string;
  folder: string;
  packages: string[];
  bytes: number;
  fileCount: number;
  profiles: string[];
}
