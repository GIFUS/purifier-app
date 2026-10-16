<script lang="ts">
  import { onMount } from "svelte";

  import {
    appReady,
    cleanCategories,
    listCategories,
    listGroups,
    onStatus,
    onTarget,
    scanCategories,
  } from "./api";
  import AboutPage from "./components/AboutPage.svelte";
  import AutorunPage from "./components/AutorunPage.svelte";
  import CategoryRow from "./components/CategoryRow.svelte";
  import { confirmDialog } from "./components/confirm-dialog";
  import Sidebar from "./components/Sidebar.svelte";
  import StatsBlock from "./components/StatsBlock.svelte";
  import SummaryBar from "./components/SummaryBar.svelte";
  import { formatBytes } from "./format";
  import { PAGE_LABELS } from "./nav";
  import {
    begin as progressBegin,
    finish as progressFinish,
    report as progressReport,
  } from "./progress.svelte";
  import type { CategoryDef } from "./types";
  import {
    appendTarget,
    categorySize,
    clearSelection,
    countsTowardTotal,
    groupCategories,
    groupLabels,
    pruneSelection,
    resetScan,
    selectAllCleanable,
    setError,
    state,
    targetsOf,
  } from "./state.svelte";

  const REPORT_HOLD_MS = 5000;
  const STATUS_FADE_MS = 1000;

  let runId = 0;

  function lockContextMenu(): void {
    if (!import.meta.env.PROD) return;
    document.addEventListener("contextmenu", (event) => event.preventDefault());
  }

  function revealWindow(): void {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        void appReady();
      });
    });
  }

  function describe(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  function titleCase(value: string): string {
    return value.charAt(0).toUpperCase() + value.slice(1);
  }

  async function boot(): Promise<void> {
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
      await runScan();
    } catch (error) {
      setError(describe(error));
      revealWindow();
    }
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

  async function runScan(): Promise<void> {
    if (state.inFlight) return;
    const run = ++runId;
    state.inFlight = true;
    progressBegin();
    setError(null);

    try {
      await scanInto();
    } catch (error) {
      if (run === runId) setError(describe(error));
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

  async function runClean(): Promise<void> {
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
      if (run === runId) setError(describe(error));
    } finally {
      if (run !== runId) return;
      state.inFlight = false;
      state.cleaning = false;
      progressFinish();
    }
  }

  onMount(() => {
    lockContextMenu();
    void boot();
  });
</script>

<div class="flex h-full overflow-hidden">
  <Sidebar />
  <div
    class="grid h-full min-w-0 flex-1 grid-rows-1 overflow-hidden rounded-tl-panel bg-bg"
  >
    {#if state.page === "cleanup"}
      <section class="grid h-full min-h-0 grid-rows-[auto_1fr_auto]">
        <header class="border-b border-border px-4 pb-3 pt-[18px]">
          <div class="flex min-w-0 items-center justify-between gap-4">
            <div class="min-w-0">
              <h1 class="m-0 text-title font-semibold tracking-[0.2px]">{PAGE_LABELS.cleanup}</h1>
              <p class="mt-1 text-caption text-text-muted">
                Measure first, then delete. Nothing is removed without your say-so.
              </p>
            </div>
          </div>
          <StatsBlock />
        </header>

        <main class="overflow-y-auto py-2 pb-4 [scrollbar-gutter:stable]">
          <div class="flex flex-col">
            {#if groupCategories().size === 0}
              <p class="muted">Loading the category catalog…</p>
            {:else}
              {#each [...groupCategories().entries()] as [group, categories] (group)}
                {@const groupSize = categories.reduce(
                  (sum, category) => sum + categorySize(category.id),
                  0,
                )}
                <section class="mt-2">
                  <h2
                    class="m-0 flex items-baseline justify-between px-4 py-2 text-[11px] font-semibold tracking-[0.9px] text-text-faint uppercase"
                  >
                    <span>{groupLabels().get(group) ?? titleCase(group)}</span>
                    {#if groupSize > 0}
                      <span class="[font-variant-numeric:tabular-nums]">{formatBytes(groupSize)}</span>
                    {/if}
                  </h2>
                  <ul class="m-0 list-none p-0">
                    {#each categories as category (category.id)}
                      <CategoryRow {category} />
                    {/each}
                  </ul>
                </section>
              {/each}
            {/if}
          </div>
        </main>

        <SummaryBar
          onScan={() => void runScan()}
          onClean={() => void runClean()}
          onSelectAll={() => selectAllCleanable()}
          onClear={() => clearSelection()}
        />
      </section>
    {:else if state.page === "autostart"}
      <AutorunPage />
    {:else}
      <AboutPage />
    {/if}
  </div>
</div>