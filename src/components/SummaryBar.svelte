<script lang="ts">
  import { formatBytes } from "../format";
  import {
    selectedCautionCount,
    selectedSize,
    state,
  } from "../state.svelte";

  interface Props {
    onScan: () => void;
    onClean: () => void;
    onSelectAll: () => void;
    onClear: () => void;
  }

  let { onScan, onClean, onSelectAll, onClear }: Props = $props();

  const count = $derived(state.selected.size);
  const busy = $derived(state.inFlight);
  const cautions = $derived(selectedCautionCount());
  const label = $derived(
    count === 0
      ? "Nothing selected"
      : `${count} categor${count === 1 ? "y" : "ies"} selected`,
  );
</script>

<footer class="border-t border-border bg-bg-raised">
  <div class="error {state.error === null ? 'hidden' : ''}" role="alert">
    {state.error ?? ""}
  </div>

  <div class="flex items-center justify-between gap-4 px-4 py-3">
    <div class="flex min-w-0 items-center gap-2.5">
      <span class="text-caption whitespace-nowrap text-text-muted">{label}</span>
      {#if cautions > 0}
        <span class="text-[12px] text-warn">{cautions} will need to be re-downloaded</span>
      {/if}
      <div class="flex gap-2.5">
        <button
          class="btn border-0 bg-none px-0.5 py-1 text-caption text-accent hover:text-accent-hover hover:underline {busy ? 'hidden' : ''}"
          onclick={onSelectAll}>Select all</button
        >
        <button
          class="btn border-0 bg-none px-0.5 py-1 text-caption text-accent hover:text-accent-hover hover:underline {busy || count === 0
            ? 'hidden'
            : ''}"
          onclick={onClear}>Clear</button
        >
      </div>
    </div>

    <div class="flex items-center gap-2.5">
      <strong class="min-w-20 text-right text-[17px] [font-variant-numeric:tabular-nums]">
        {formatBytes(selectedSize())}
      </strong>
      <button
        class="btn border border-transparent bg-accent font-semibold text-[#0d1117] hover:not-disabled:bg-accent-hover"
        disabled={busy || count === 0}
        onclick={onClean}
      >{state.cleaning ? "Cleaning…" : "Clean selected"}</button
      >
      <button
        class="btn border border-border-strong bg-none text-text hover:not-disabled:border-accent hover:not-disabled:text-accent"
        disabled={busy}
        onclick={onScan}>Rescan</button
      >
    </div>
  </div>
</footer>