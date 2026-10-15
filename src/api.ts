import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppError,
  AppInfo,
  CategoryDef,
  CleanReport,
  CleanupMode,
  ScanEntry,
  ScanProgress,
  Status,
} from "./types";

const EVENTS = {
  target: "scan://target",
  status: "scan://status",
} as const;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    if (raw && typeof raw === "object" && "message" in raw) {
      const { kind, message } = raw as AppError;
      throw new Error(message, { cause: kind });
    }
    throw new Error(typeof raw === "string" ? raw : String(raw));
  }
}

export const listCategories = () => call<CategoryDef[]>("list_categories");

export const listGroups = () => call<[string, string][]>("list_groups");

export const scanCategories = (ids?: string[], quiet = false) =>
  call<ScanEntry[]>("scan_categories", { ids: ids ?? null, quiet });

export const cleanCategories = (ids: string[], mode?: CleanupMode) =>
  call<CleanReport>("clean_categories", { ids, mode: mode ?? null });

export const openInExplorer = (path: string) => call<void>("open_in_explorer", { path });

export const openRepository = () => call<void>("open_repository");

export const appReady = () => call<void>("app_ready");

export const appInfo = () => call<AppInfo>("app_info");

export const onTarget = (handler: (payload: ScanProgress) => void): Promise<UnlistenFn> =>
  listen<ScanProgress>(EVENTS.target, (event) => handler(event.payload));

export const onStatus = (handler: (payload: Status) => void): Promise<UnlistenFn> =>
  listen<Status>(EVENTS.status, (event) => handler(event.payload));
