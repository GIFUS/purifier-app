<script lang="ts">
  import {
    NAV_GROUPS,
    SECONDARY_NAV,
    SIDEBAR_EXPAND_LABEL,
    SIDEBAR_TOGGLE_LABEL,
    sidebarToggleIcon,
  } from "../nav";
  import { setPage, state, toggleSidebar } from "../state.svelte";
  import type { PageId } from "../nav";

  const expanded = $derived(state.sidebarExpanded);
  const toggleLabel = $derived(expanded ? SIDEBAR_TOGGLE_LABEL : SIDEBAR_EXPAND_LABEL);
  const ToggleIcon = $derived(sidebarToggleIcon(expanded));

  const itemLayout = $derived(expanded ? "pr-3" : "pr-0");
  const labelReveal = $derived(
    expanded
      ? "max-w-[150px] translate-x-0 opacity-100 visible"
      : "max-w-0 translate-x-[-4px] opacity-0 invisible",
  );
  const groupReveal = $derived(
    expanded
      ? "max-h-5 mt-1 translate-y-0 opacity-100 visible"
      : "max-h-0 translate-y-[-3px] opacity-0 invisible",
  );
  const railWidth = $derived(expanded ? "w-[240px]" : "w-[62px]");

  function itemClass(): string {
    return (
      "group relative flex h-[38px] w-full items-center gap-2.5 rounded-app border-0 bg-transparent pl-3 text-left text-row text-text-muted " +
      "transition-[background-color,color,padding] duration-240 ease-rail " +
      "hover:not-data-active:bg-bg-hover hover:not-data-active:text-text " +
      "data-active:bg-bg-hover data-active:text-text data-active:font-semibold " +
      "data-active:before:absolute data-active:before:left-0 data-active:before:h-[22px] data-active:before:w-[3px] " +
      "data-active:before:rounded-full data-active:before:bg-accent data-active:before:content-[''] " +
      "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent " +
      itemLayout
    );
  }

  function labelClass(): string {
    return (
      "min-w-0 overflow-hidden text-ellipsis whitespace-nowrap " +
      "transition-[max-width,opacity,transform,visibility] duration-240 ease-rail " +
      labelReveal
    );
  }

  function tipClass(): string {
    return (
      "pointer-events-none absolute top-1/2 left-[calc(100%+10px)] z-20 translate-y-[-50%] translate-x-[-4px] " +
      "rounded-md border border-border bg-bg-hover px-2 py-1 text-[12px] leading-[1.4] whitespace-nowrap text-text " +
      "opacity-0 transition-[opacity,transform] duration-120 group-hover:translate-x-0 group-hover:opacity-100 " +
      (expanded ? "hidden" : "")
    );
  }
</script>

{#snippet navItem(id: PageId, label: string, icon: import("svelte").Component)}
  {@const Icon = icon}
  <button
    type="button"
    class={itemClass()}
    data-tip={label}
    aria-label={label}
    aria-current={state.page === id ? "page" : undefined}
    data-active={state.page === id ? "" : undefined}
    onclick={() => setPage(id)}
  >
    <span class="grid size-[22px] flex-none place-items-center text-current">
      <Icon size={19} strokeWidth={1.7} />
    </span>
    <span class={labelClass()} aria-hidden="true">{label}</span>
    <span class={tipClass()} aria-hidden="true">{label}</span>
  </button>
{/snippet}

<aside
  class="flex h-full flex-none flex-col overflow-hidden border-r border-border bg-bg-raised py-2.5 pr-2 pb-3.5 pl-2 transition-[width,min-width] duration-240 ease-rail {railWidth}"
>
  <nav class="flex min-h-0 flex-col gap-2.5 overflow-hidden" aria-label="Purifier">
    {#each NAV_GROUPS as group (group.id)}
      <div class="flex min-w-0 flex-col" role="group" aria-label={group.label}>
        <span
          class="overflow-hidden px-2.5 text-[11px] leading-5 font-semibold tracking-[0.04em] whitespace-nowrap text-text-faint transition-[max-height,opacity,transform,visibility] duration-240 ease-rail {groupReveal}"
          aria-hidden="true"
        >{group.label}</span>
        <div class="flex flex-col gap-0.5">
          {#each group.items as item (item.id)}
            {@render navItem(item.id, item.label, item.icon)}
          {/each}
        </div>
      </div>
    {/each}
  </nav>

  <div class="mt-auto flex flex-col gap-[3px] pt-2.5">
    {#each SECONDARY_NAV as item (item.id)}
      {@render navItem(item.id, item.label, item.icon)}
    {/each}

    <div class="mt-[3px]">
      <button
        type="button"
        class="group relative flex h-8 w-full items-center gap-2.5 rounded-app border-0 bg-transparent pl-3 text-left text-caption text-text-faint transition-[background-color,color,padding] duration-240 ease-rail hover:bg-bg-hover {itemLayout}"
        data-tip={toggleLabel}
        aria-label={toggleLabel}
        aria-expanded={expanded}
        onclick={toggleSidebar}
      >
        <span class="grid size-[22px] flex-none place-items-center text-current">
          <ToggleIcon size={19} strokeWidth={1.7} />
        </span>
        <span class={labelClass()} aria-hidden="true">{SIDEBAR_TOGGLE_LABEL}</span>
        <span class={tipClass()} aria-hidden="true">{toggleLabel}</span>
      </button>
    </div>
  </div>
</aside>