import {
  appReady,
  cleanCategories,
  listCategories,
  listGroups,
  onStatus,
  onTarget,
  scanCategories,
} from "./api";
import { confirmDialog } from "./components/confirm-dialog";
import { errorMessage } from "./errors";
import { formatBytes } from "./format";
import {
  begin as progressBegin,
  finish as progressFinish,
  report as progressReport,
} from "./progress.svelte";
import {
  appendTarget,
  categorySize,
  countsTowardTotal,
  pruneSelection,
  resetScan,
  setError,
  state,
  targetsOf,
} from "./state.svelte";
import type { CategoryDef } from "./types";

const REPORT_HOLD_MS = 5000;
const STATUS_FADE_MS = 1000;

let runId = 0;

export function boot(): Promise<void> {
  return start();
}

export function lockContextMenu(): void {
  if (!import.meta.env.PROD) return;
  document.addEventListener("contextmenu", (event) => event.preventDefault());
}

export function runScan(): Promise<void> {
  return scan();
}

export function runClean(): Promise<void> {
  return clean();
}

async function start(): Promise<void> {
  await onTarget((payload) => {
    appendTarget(payload.category_id, payload.target, payload.index);
  });

  await onStatus((status) => {
    state.cleaning = status.phase === "cleaning";
    progressReport(status);
  });

  try {
    const [categories, groups] = await Promise.all([listCategories(), listGroups()]);
    state.categories = categories;
    state.groups = groups;
    revealWindow();
    await scan();
  } catch (error) {
    setError(errorMessage(error));
    revealWindow();
  }
}

function revealWindow(): void {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      void appReady();
    });
  });
}

async function scanInto(ids?: string[], quiet = false): Promise<void> {
  const entries = await scanCategories(ids, quiet);
  for (const entry of entries) {
    if (entry.targets.length === 0 || !state.targets.has(entry.category.id)) {
      state.targets.set(entry.category.id, []);
    }
  }
  pruneSelection();
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function scan(): Promise<void> {
  if (state.inFlight) return;
  const run = ++runId;
  state.inFlight = true;
  progressBegin();
  setError(null);

  try {
    await scanInto();
  } catch (error) {
    if (run === runId) setError(errorMessage(error));
  } finally {
    if (run !== runId) return;
    state.inFlight = false;
    state.cleaning = false;
    await sleep(STATUS_FADE_MS);
    if (run === runId) progressFinish();
  }
}

async function confirmClean(ids: string[]): Promise<boolean> {
  const categories = ids
    .map((id) => state.categories.find((candidate) => candidate.id === id))
    .filter((candidate): candidate is CategoryDef => candidate !== undefined);

  const bytes = ids.reduce((sum, id) => sum + categorySize(id), 0);
  const cautions = categories.filter((category) => category.danger === "caution").length;
  const plural = ids.length === 1 ? "category" : "categories";

  return confirmDialog({
    title: `Delete ${formatBytes(bytes)} from ${ids.length} ${plural}?`,
    subtitle: "This cannot be undone. Deleted files are not sent to the Recycle Bin.",
    items: categories.map((category) => ({
      id: category.id,
      label: category.label,
      sizeBytes: categorySize(category.id),
      caution: category.danger === "caution",
      paths: targetsOf(category.id)
        .filter(countsTowardTotal)
        .map((target) => target.path),
    })),
    note:
      cautions > 0
        ? `${cautions} of these ${cautions === 1 ? "is a cache that" : "are caches that"} ` +
          "must be re-downloaded or rebuilt on the next run."
        : undefined,
    confirmLabel: `Delete ${formatBytes(bytes)}`,
  });
}

async function clean(): Promise<void> {
  if (state.inFlight) return;
  const ids = [...state.selected];
  if (ids.length === 0) return;

  if (!(await confirmClean(ids))) return;
  if (state.inFlight) return;

  const run = ++runId;
  state.inFlight = true;
  state.cleaning = true;
  progressBegin();
  setError(null);
  resetScan(ids);

  try {
    await cleanCategories(ids);

    for (const id of ids) state.selected.delete(id);
    resetScan(ids);
    await scanInto(ids, true);

    if (run === runId) {
      state.inFlight = false;
      state.cleaning = false;
    }

    await sleep(REPORT_HOLD_MS);
  } catch (error) {
    if (run === runId) setError(errorMessage(error));
  } finally {
    if (run !== runId) return;
    state.inFlight = false;
    state.cleaning = false;
    progressFinish();
  }
}
