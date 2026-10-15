<script lang="ts">
  import { progress } from "../progress.svelte";

  const label = $derived(progress.view?.label ?? "");
  const indeterminate = $derived(progress.view?.fraction === null);
  const tone = $derived(
    progress.view?.tone === "ok"
      ? "bg-ok"
      : progress.view?.tone === "warn"
        ? "bg-warn"
        : "bg-accent",
  );
  const width = $derived(
    !progress.view || progress.view.fraction === null
      ? indeterminate
        ? ""
        : "width:0"
      : `width:${(progress.view.fraction * 100).toFixed(2)}%`,
  );
  const ease = $derived(
    progress.instantOff || progress.view?.snap
      ? "transition-none"
      : "transition-[width] duration-180 ease-out",
  );
  const fade = $derived(
    progress.instantOff
      ? "transition-none"
      : "transition-opacity duration-300 ease-out motion-reduce:transition-none",
  );
</script>

<div
  class="min-w-[190px] flex-[1_1_auto] {fade}"
  class:opacity-0={!progress.on}
  data-phase={progress.view?.phase ?? ""}
  aria-hidden={!progress.on}
>
  <div class="h-1 overflow-hidden rounded-full bg-border" role="progressbar">
    <div
      class="h-full rounded-[inherit] {tone} {ease}"
      class:w-[35%]={indeterminate}
      class:animate-sweep={indeterminate}
      style={width}
    ></div>
  </div>
  <span
    class="mt-1.5 block overflow-hidden text-[12px] text-text-muted [font-variant-numeric:tabular-nums] [text-overflow:ellipsis] whitespace-nowrap"
    title={progress.on ? label : ""}
  >{label}</span>
</div>