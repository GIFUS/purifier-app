import { SvelteMap, SvelteSet } from "svelte/reactivity";

import type { CategoryDef, TargetInfo } from "./types";
import type { PageId } from "./nav";

interface State {
  categories: CategoryDef[];
  groups: [string, string][];
  targets: SvelteMap<string, TargetInfo[]>;
  selected: SvelteSet<string>;
  expanded: SvelteSet<string>;
  cleaning: boolean;
  inFlight: boolean;
  error: string | null;
  page: PageId;
  sidebarExpanded: boolean;
}

export const state: State = $state({
  categories: [],
  groups: [],
  targets: new SvelteMap<string, TargetInfo[]>(),
  selected: new SvelteSet<string>(),
  expanded: new SvelteSet<string>(),
  cleaning: false,
  inFlight: false,
  error: null,
  page: "cleanup",
  sidebarExpanded: false,
});

export function groupCategories(): Map<string, CategoryDef[]> {
  const groups = new Map<string, CategoryDef[]>();
  for (const category of state.categories) {
    const bucket = groups.get(category.group);
    if (bucket) bucket.push(category);
    else groups.set(category.group, [category]);
  }
  return groups;
}

export function targetsOf(categoryId: string): TargetInfo[] {
  return state.targets.get(categoryId) ?? [];
}

export function countsTowardTotal(target: TargetInfo): boolean {
  return target.exists && !target.reparse;
}

export function categorySize(categoryId: string): number {
  return targetsOf(categoryId)
    .filter(countsTowardTotal)
    .reduce((sum, target) => sum + target.size_bytes, 0);
}

function hasSomethingToDelete(target: TargetInfo): boolean {
  if (!target.exists || target.reparse) return false;
  if (target.error) return true;
  return target.size_bytes > 0 || target.file_count > 0;
}

export function isCleanable(category: CategoryDef): boolean {
  return targetsOf(category.id).some(hasSomethingToDelete);
}

function cleanableIds(): string[] {
  return state.categories.filter(isCleanable).map((category) => category.id);
}

export function toggleExpanded(id: string): void {
  if (state.expanded.has(id)) state.expanded.delete(id);
  else state.expanded.add(id);
}

export function selectAllCleanable(): void {
  for (const id of cleanableIds()) state.selected.add(id);
}

export function clearSelection(): void {
  state.selected.clear();
}

export function pruneSelection(): void {
  for (const category of state.categories) {
    if (state.selected.has(category.id) && !isCleanable(category)) {
      state.selected.delete(category.id);
    }
  }
}

export function resetScan(ids?: readonly string[]): void {
  if (ids) {
    for (const id of ids) state.targets.delete(id);
  } else {
    state.targets.clear();
  }
  state.error = null;
}

export function appendTarget(categoryId: string, target: TargetInfo, index: number): void {
  const bucket = state.targets.get(categoryId);
  if (!bucket || index === 0) state.targets.set(categoryId, [target]);
  else state.targets.set(categoryId, [...bucket, target]);
}

export function setError(message: string | null): void {
  state.error = message;
}

export function setPage(page: PageId): void {
  if (state.page === page) return;
  state.page = page;
}

export function toggleSidebar(): void {
  state.sidebarExpanded = !state.sidebarExpanded;
}

export function selectedSize(): number {
  let total = 0;
  for (const id of state.selected) total += categorySize(id);
  return total;
}

function cleanableCount(): number {
  return state.categories.filter(isCleanable).length;
}

interface Totals {
  bytes: number;
  files: number;
  cleanableCategories: number;
  totalCategories: number;
}

export function totals(): Totals {
  let bytes = 0;
  let files = 0;

  for (const targets of state.targets.values()) {
    for (const target of targets) {
      if (!countsTowardTotal(target)) continue;
      bytes += target.size_bytes;
      files += target.file_count;
    }
  }

  return {
    bytes,
    files,
    cleanableCategories: cleanableCount(),
    totalCategories: state.categories.length,
  };
}

export function selectedCautionCount(): number {
  return state.categories.filter(
    (category) => state.selected.has(category.id) && category.danger === "caution",
  ).length;
}

export function groupLabels(): Map<string, string> {
  return new Map(state.groups);
}