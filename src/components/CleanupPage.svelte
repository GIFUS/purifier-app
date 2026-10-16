<script lang="ts">
  import { runClean, runScan } from "../cleanup.svelte";
  import { formatBytes } from "../format";
  import { PAGE_LABELS } from "../nav";
  import {
    categorySize,
    clearSelection,
    groupCategories,
    groupLabels,
    selectAllCleanable,
  } from "../state.svelte";
  import CategoryRow from "./CategoryRow.svelte";
  import StatsBlock from "./StatsBlock.svelte";
  import SummaryBar from "./SummaryBar.svelte";

  function titleCase(value: string): string {
    return value.charAt(0).toUpperCase() + value.slice(1);
  }
</script>

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
