type Danger = "safe" | "caution";
export type CleanupMode = "clear_contents" | "remove_dir";
type TargetKind = "dir" | "file";
type Phase = "scanning" | "cleaning" | "done";

type Locator =
  | { Fixed: string }
  | { Env: string }
  | { Pattern: { base: string; pattern: string } }
  | { Join: [Locator[], string] };

export interface CategoryDef {
  id: string;
  group: string;
  label: string;
  description: string;
  danger: Danger;
  mode: CleanupMode;
  locators: Locator[];
}

export interface TargetInfo {
  path: string;
  kind: TargetKind;
  exists: boolean;
  size_bytes: number;
  file_count: number;
  last_modified: string | null;
  reparse: boolean;
  writable: boolean;
  locked: boolean;
  error: string | null;
}

export interface ScanEntry {
  category: CategoryDef;
  targets: TargetInfo[];
  error: string | null;
}

export interface ScanProgress {
  category_id: string;
  category_label: string;
  group: string;
  index: number;
  total: number;
  target: TargetInfo;
}

export interface Status {
  phase: Phase;
  done: number;
  total: number;
  indeterminate: boolean;
  message: string;
}

interface Failure {
  path: string;
  reason: string;
}

export interface CleanReport {
  failed: Failure[];
  freed_bytes: number;
  files_removed: number;
  blocked: number;
}

export interface AppError {
  kind: string;
  message: string;
}

export interface AppInfo {
  version: string;
  identifier: string;
}