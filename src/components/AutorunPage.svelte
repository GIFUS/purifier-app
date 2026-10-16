<script lang="ts">
  import { onMount } from "svelte";

  import Check from "@lucide/svelte/icons/check";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import {
    listAutorun,
    listAutorunTasks,
    setAutorunEnabled,
    setAutorunTaskEnabled,
  } from "../api";
  import { formatNumber } from "../format";
  import { PAGE_LABELS } from "../nav";
  import {
    autorunCount,
    autorunKey,
    autorunRows,
    autorunScopes,
    missingRows,
    state,
    taskKey,
  } from "../state.svelte";
  import type { AutorunEntry, AutorunTask } from "../types";

  const SCOPE_LABELS: Record<string, string> = {
    user: "Current user",
    machine: "All users",
  };

  const scopes = $derived(autorunScopes());
  const missing = $derived(missingRows());
  const counts = $derived(autorunCount());

  function isTask(row: AutorunEntry | AutorunTask): row is AutorunTask {
    return !("source" in row);
  }

  function describe(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  function scopeLabel(scope: string): string {
    return SCOPE_LABELS[scope] ?? scope;
  }

  async function load(): Promise<void> {
    state.autorunLoading = true;
    state.autorunError = null;
    try {
      const [entries, scheduled] = await Promise.all([listAutorun(), listAutorunTasks()]);
      state.autorun = entries;
      state.autorunTasks = scheduled;
      state.autorunLoaded = true;
    } catch (error) {
      state.autorunError = describe(error);
    } finally {
      state.autorunLoading = false;
    }
  }

  async function run(action: () => Promise<void>): Promise<void> {
    state.autorunError = null;
    try {
      await action();
      await load();
    } catch (error) {
      state.autorunError = describe(error);
    }
  }

  function toggleEntry(entry: AutorunEntry, enabled: boolean): Promise<void> {
    const key = autorunKey(entry);
    if (state.autorunBusy.has(key)) return Promise.resolve();
    state.autorunBusy.add(key);
    return run(() =>
      setAutorunEnabled(entry.source, entry.scope, entry.path, entry.name, enabled),
    ).finally(() => state.autorunBusy.delete(key));
  }

  function toggleTask(task: AutorunTask, enabled: boolean): Promise<void> {
    const key = taskKey(task);
    if (state.autorunBusy.has(key)) return Promise.resolve();
    state.autorunBusy.add(key);
    return run(() => setAutorunTaskEnabled(task.path, task.name, enabled)).finally(() =>
      state.autorunBusy.delete(key),
    );
  }

  onMount(() => {
    void load();
  });
</script>

<section class="grid h-full min-h-0 grid-rows-[auto_1fr_auto]">
  <header class="border-b border-border px-4 pb-3 pt-[18px]">
    <h1 class="m-0 text-title font-semibold tracking-[0.2px]">{PAGE_LABELS.autostart}</h1>
    <p class="mt-1 text-caption text-text-muted">
      Programs Windows starts when you sign in. Changes apply at the next sign-in.
    </p>
  </header>

  {#snippet entryRow(row: AutorunEntry | AutorunTask)}
  {@const task = isTask(row)}
  {@const busyKey = task ? taskKey(row as AutorunTask) : autorunKey(row as AutorunEntry)}
  {@const checked = row.enabled}
  {@const busy = state.autorunBusy.has(busyKey)}
  <li class="border-y border-transparent" class:opacity-55={!checked}>
    <div
      class="grid grid-cols-[18px_1fr_auto] items-center gap-2.5 px-4 py-2 hover:bg-bg-raised"
      class:cursor-pointer={!busy}
      class:cursor-not-allowed={busy}
      role="button"
      tabindex="0"
      aria-pressed={checked}
      aria-label={row.name}
      onclick={() => {
        if (busy) return;
        if (task) void toggleTask(row as AutorunTask, !checked);
        else void toggleEntry(row as AutorunEntry, !checked);
      }}
      onkeydown={(event) => {
        if (event.target !== event.currentTarget) return;
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          if (busy) return;
          if (task) void toggleTask(row as AutorunTask, !checked);
          else void toggleEntry(row as AutorunEntry, !checked);
        }
      }}
    >
      <div class="relative grid size-[18px] place-items-center">
        <input
          type="checkbox"
          class="peer size-[18px] cursor-pointer appearance-none rounded-[5px] border border-border-strong bg-bg-raised transition-[border-color,background] duration-150 hover:not-disabled:border-accent checked:border-accent checked:bg-accent disabled:cursor-not-allowed"
          checked={checked}
          disabled={busy}
          title={checked ? "Do not start this program" : "Start this program"}
          aria-label={row.name}
          onclick={(event) => event.stopPropagation()}
          onchange={(event) => {
            const input = event.currentTarget as HTMLInputElement;
            const next = input.checked;
            if (task) void toggleTask(row as AutorunTask, next);
            else void toggleEntry(row as AutorunEntry, next);
          }}
        />
        <Check
          class="pointer-events-none absolute size-3 text-white opacity-0 peer-checked:opacity-100"
          strokeWidth={3.5}
        />
      </div>

      <div class="flex min-w-0 flex-col">
        <span class="text-row">{row.name}</span>
        <span
          class="overflow-hidden font-[Cascadia_Mono,Consolas,monospace] text-[11.5px] text-text-faint [text-overflow:ellipsis] whitespace-nowrap"
          title={row.command}
        >{row.command}</span>
      </div>

      {#if !row.exists}
        <span
          class="inline-flex items-center gap-1 rounded-full border border-[color-mix(in_srgb,var(--color-warn)_45%,transparent)] px-[7px] py-0.5 text-[10px] font-semibold tracking-[0.4px] text-warn uppercase"
          title="The program this entry points at is no longer on disk."
        >
          <TriangleAlert size={11} strokeWidth={2.5} />
          missing
        </span>
      {/if}
    </div>
  </li>
{/snippet}

{#snippet groupRows(rows: (AutorunEntry | AutorunTask)[])}
  <ul class="m-0 list-none p-0">
    {#each rows as row (`${row.name} ${row.command}`)}
      {@render entryRow(row)}
    {/each}
  </ul>
{/snippet}

<main class="overflow-y-auto py-2 pb-4 [scrollbar-gutter:stable]">
  {#if state.autorunLoading && !state.autorunLoaded}
    <p class="muted">Reading startup entries…</p>
  {:else if scopes.length === 0 && missing.length === 0}
    <p class="muted">Nothing starts automatically on this machine.</p>
  {:else}
    {#each scopes as scope (scope)}
      {@const rows = autorunRows(scope)}
      {#if rows.length > 0}
        <section class="mt-2">
          <h2
            class="m-0 flex items-baseline justify-between px-4 py-2 text-[11px] font-semibold tracking-[0.9px] text-text-faint uppercase"
          >
            <span>{scopeLabel(scope)}</span>
            <span class="[font-variant-numeric:tabular-nums]">
              {formatNumber(rows.filter((row) => row.enabled).length)} / {formatNumber(rows.length)}
            </span>
          </h2>
          {@render groupRows(rows)}
        </section>
      {/if}
    {/each}

    {#if missing.length > 0}
      <section class="mt-2">
        <h2
          class="m-0 flex items-baseline justify-between px-4 py-2 text-[11px] font-semibold tracking-[0.9px] text-text-faint uppercase"
        >
          <span>Not installed</span>
          <span class="[font-variant-numeric:tabular-nums]">{formatNumber(missing.length)}</span>
        </h2>
        {@render groupRows(missing)}
      </section>
    {/if}
  {/if}
</main>

  <footer class="border-t border-border bg-bg-raised">
    <div class="error {state.autorunError === null ? 'hidden' : ''}" role="alert">
      {state.autorunError ?? ""}
    </div>

    <div class="flex items-center justify-between gap-4 px-4 py-3">
      <span class="text-caption whitespace-nowrap text-text-muted">
        {formatNumber(counts.enabled)} of {formatNumber(counts.total)} enabled
      </span>
      <button
        class="btn border border-border-strong bg-none text-text hover:not-disabled:border-accent hover:not-disabled:text-accent"
        disabled={state.autorunLoading}
        onclick={() => void load()}>Rescan</button
      >
    </div>
  </footer>
</section>