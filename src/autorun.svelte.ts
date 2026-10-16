import type { AutorunEntry, AutorunTask } from "./types";
import { state } from "./state.svelte";

export type AutorunRow = AutorunEntry | AutorunTask;

function isTask(row: AutorunRow): row is AutorunTask {
  return !("source" in row);
}

export function autorunKey(entry: AutorunEntry): string {
  return `${entry.scope} ${entry.source} ${entry.path} ${entry.name}`;
}

export function taskKey(task: AutorunTask): string {
  return `${task.path} ${task.name}`;
}

export function rowKey(row: AutorunRow): string {
  return isTask(row) ? taskKey(row) : autorunKey(row);
}

export function autorunCount(): { enabled: number; total: number } {
  let enabled = 0;
  let total = 0;
  for (const row of allRows()) {
    total += 1;
    if (row.enabled) enabled += 1;
  }
  return { enabled, total };
}

export function autorunScopes(): string[] {
  const scopes = new Set<string>();
  for (const entry of state.autorun) scopes.add(entry.scope);
  for (const task of state.autorunTasks) scopes.add(task.scope);
  return [...scopes].sort();
}

export function autorunRows(scope: string): AutorunRow[] {
  const rows = allRows().filter((row) => row.scope === scope && row.exists);
  return byName(rows);
}

export function missingRows(): AutorunRow[] {
  return byName(allRows().filter((row) => !row.exists));
}

function allRows(): AutorunRow[] {
  return [...state.autorun, ...state.autorunTasks];
}

function byName(rows: AutorunRow[]): AutorunRow[] {
  return rows.sort((a, b) => (a.name.toLowerCase() < b.name.toLowerCase() ? -1 : 1));
}
