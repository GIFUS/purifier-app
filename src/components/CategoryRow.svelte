<script lang="ts">
  import Check from "@lucide/svelte/icons/check";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import { openInExplorer } from "../api";
  import { formatBytes, formatCount, pluralize } from "../format";
  import {
    categorySize,
    isCleanable,
    state,
    targetsOf,
    toggleExpanded,
  } from "../state.svelte";
  import type { CategoryDef, TargetInfo } from "../types";

  let { category }: { category: CategoryDef } = $props();

  const targets = $derived(targetsOf(category.id));
  const cleanable = $derived(isCleanable(category));
  const measured = $derived(state.targets.has(category.id));
  const expanded = $derived(state.expanded.has(category.id));
  const size = $derived(categorySize(category.id));
  const isSelected = $derived(state.selected.has(category.id));

  const anyStatus = $derived(targets.some((target) => describe(target).text !== ""));
  const targetGrid = $derived(
    anyStatus
      ? "grid-cols-[1fr_130px_190px_minmax(46px,max-content)]"
      : "grid-cols-[1fr_190px_minmax(46px,max-content)]",
  );

  const sizeText = $derived(
    !measured ? "…" : cleanable ? formatBytes(size) : "empty",
  );

  function onHeadingClick(): void {
    toggleExpanded(category.id);
  }

  function describe(target: TargetInfo): { className: string; text: string } {
    if (target.reparse) return { className: "skip", text: "junction · skipped" };
    if (!target.exists) return { className: "muted", text: "" };
    if (target.error) return { className: "warn", text: target.error };
    if (!target.writable) return { className: "warn", text: "read-only" };
    if (target.locked) {
      return {
        className: "warn",
        text: `in use · ${formatCount(target.file_count)} files`,
      };
    }
    return { className: "muted", text: "" };
  }

  function targetMeta(target: TargetInfo): string {
    if (!target.exists) return "not found";
    if (target.file_count > 0) {
      return `${formatBytes(target.size_bytes)} · ${pluralize(target.file_count, "file")}`;
    }
    return formatBytes(target.size_bytes);
  }

  function tooltip(target: TargetInfo, status: { text: string }): string {
    return [
      target.path,
      target.error,
      status.text,
      target.last_modified ? `Modified: ${target.last_modified}` : null,
    ]
      .filter(Boolean)
      .join("\n");
  }
</script>

<li
  class="border-y border-transparent hover:bg-bg-hover {isSelected
    ? 'bg-[color-mix(in_srgb,var(--color-accent)_12%,transparent)]'
    : ''}"
  class:opacity-50={measured && !cleanable}
  data-id={category.id}
>
  <div
    class="grid cursor-pointer grid-cols-[18px_24px_1fr_auto] items-center gap-2.5 px-4 py-2 select-none"
    role="button"
    tabindex="0"
    onclick={onHeadingClick}
    onkeydown={(event) => {
      if (event.target !== event.currentTarget) return;
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        toggleExpanded(category.id);
      }
    }}
  >
    <div class="relative grid size-[18px] place-items-center">
      <input
        type="checkbox"
        class="peer size-[18px] cursor-pointer appearance-none rounded-[5px] border border-border-strong bg-bg-raised transition-[border-color,background] duration-150 hover:not-disabled:border-accent checked:border-accent checked:bg-accent disabled:cursor-not-allowed"
        disabled={!cleanable}
        checked={isSelected}
        onclick={(event) => event.stopPropagation()}
        onchange={(event) => {
          const input = event.currentTarget as HTMLInputElement;
          if (input.checked) state.selected.add(category.id);
          else state.selected.delete(category.id);
        }}
      />
      <Check
        class="pointer-events-none absolute size-3 text-white opacity-0 peer-checked:opacity-100"
        strokeWidth={3.5}
      />
    </div>
    <button
      class="inline-flex w-6 cursor-pointer items-center justify-center border-0 bg-none p-0 text-text-faint disabled:cursor-default disabled:opacity-35"
      class:text-accent={expanded}
      aria-expanded={expanded}
      title="Show the folders behind this category"
      disabled={!measured}
      onclick={(event) => {
        event.stopPropagation();
        toggleExpanded(category.id);
      }}
    >
      {#if expanded}
        <ChevronDown size={16} strokeWidth={2} />
      {:else}
        <ChevronRight size={16} strokeWidth={2} />
      {/if}
    </button>
    <div class="flex min-w-0 flex-col">
      <span class="text-row">{category.label}</span>
      <span class="overflow-hidden text-[12px] text-text-faint [text-overflow:ellipsis] whitespace-nowrap">
        {category.description}
      </span>
    </div>
    <div class="flex items-center gap-2.5">
      {#if category.danger === "caution"}
        <span
          class="rounded-full border border-[color-mix(in_srgb,var(--color-warn)_45%,transparent)] px-[7px] py-0.5 text-[10px] font-semibold tracking-[0.4px] text-warn uppercase"
          title="Clearing this cache costs a re-download or a cold rebuild."
        >caution</span>
      {/if}
      <span
        data-size={sizeText}
        class="min-w-[76px] text-right text-label [font-variant-numeric:tabular-nums]"
        class:text-text={size > 0}
        class:text-text-faint={size <= 0}
      >{sizeText}</span>
    </div>
  </div>

  {#if expanded}
    <ul data-targets class="m-0 list-none pb-1.5 pl-[58px]">
      {#if targets.length === 0}
        <li class="muted">Nothing to show yet.</li>
      {:else}
        {#each targets as target (target.path)}
          {@const status = describe(target)}
          <li
            class="grid min-h-8 {targetGrid} items-center gap-2.5 rounded-md px-2 py-1 text-[12px] hover:bg-bg-raised"
            class:opacity-55={!target.exists}
            title={tooltip(target, status)}
          >
            <span
              class="overflow-hidden font-[Cascadia_Mono,Consolas,monospace] text-[11.5px] [text-overflow:ellipsis] whitespace-nowrap"
            >{target.path}</span>
            {#if anyStatus}
              <span
                class="overflow-hidden text-[11px] text-text-faint text-ellipsis whitespace-nowrap"
                class:text-warn={status.className === "warn"}
                class:text-danger={status.className === "skip"}
              >
                {status.text}
              </span>
            {/if}
            <span
              data-target-size
              class="text-right text-text-muted whitespace-nowrap [font-variant-numeric:tabular-nums]"
            >{targetMeta(target)}</span>
            {#if target.exists}
              <button
                class="cursor-pointer rounded-[5px] border border-border-strong bg-none px-2 py-0.5 text-[11px] whitespace-nowrap text-text-muted transition-colors duration-150 hover:bg-bg-raised hover:text-text"
                title="Show this folder in Explorer"
                onclick={() => void openInExplorer(target.path)}
              >Open</button>
            {/if}
          </li>
        {/each}
      {/if}
    </ul>
  {/if}
</li>