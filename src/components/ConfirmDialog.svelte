<script lang="ts">
  import { formatBytes } from "../format";
  import type { ConfirmRequest } from "./confirm-dialog";

  let {
    request,
    onresolve,
  }: { request: ConfirmRequest; onresolve: (ok: boolean) => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let settled = false;

  const titleId = "confirm-title";
  const subtitleId = "confirm-sub";

  function finish(): void {
    if (settled) return;
    settled = true;
    onresolve(dialog?.returnValue === "confirm");
    dialog?.remove();
  }

  $effect(() => {
    dialog?.showModal();
    dialog?.querySelector<HTMLButtonElement>('[value="cancel"]')?.focus();
  });
</script>

<dialog
  bind:this={dialog}
  class="m-auto w-[min(660px,calc(100vw-48px))] max-h-[min(82vh,680px)] rounded-panel border border-border-strong bg-bg-raised p-0 text-text shadow-[0_24px_64px_#0000008c] backdrop:bg-[#00000094]"
  aria-labelledby={titleId}
  aria-describedby={subtitleId}
  onclose={finish}
>
  <form
    method="dialog"
    class="flex min-h-0 flex-col gap-3.5 pb-[18px] pr-[22px] pl-5 pt-5"
  >
    <h2 id={titleId} class="m-0 text-[17px] font-semibold">{request.title}</h2>
    <p id={subtitleId} class="-mt-2 mb-0 text-caption text-text-muted">{request.subtitle}</p>

    <div
      class="flex max-h-[42vh] flex-col overflow-y-auto rounded-app border border-border bg-bg"
    >
      {#if request.items.length === 0}
        <p class="muted">Nothing selected.</p>
      {:else}
        {#each request.items as item, index (item.id)}
          <div
            class="px-3 py-2.5"
            class:border-t={index > 0}
            class:border-border={index > 0}
            data-id={item.id}
          >
            <div class="flex items-center gap-2">
              <span
                class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-label"
              >{item.label}</span>
              {#if item.caution}
                <span
                  class="rounded-full border border-[color-mix(in_srgb,var(--color-warn)_45%,transparent)] px-[7px] py-0.5 text-[10px] font-semibold tracking-[0.4px] text-warn uppercase"
                  title="This cache has to be downloaded or rebuilt on the next run."
                >caution</span>
              {/if}
              <span class="text-label [font-variant-numeric:tabular-nums]">
                {formatBytes(item.sizeBytes)}
              </span>
            </div>
            {#if item.paths.length > 0}
              <ul class="mt-1.5 mb-0 list-none p-0">
                {#each item.paths as path (path)}
                  <li
                    class="font-[Cascadia_Mono,Consolas,monospace] text-[11.5px] leading-[1.7] wrap-anywhere whitespace-normal text-text-faint"
                    title={path}
                  >{path}</li>
                {/each}
              </ul>
            {/if}
          </div>
        {/each}
      {/if}
    </div>

    {#if request.note}
      <p class="m-0 text-[12px] text-warn">{request.note}</p>
    {/if}

    <div class="flex justify-end gap-2.5">
      <button
        class="btn border border-border-strong bg-none text-text hover:not-disabled:border-accent hover:not-disabled:text-accent"
        value="cancel">Cancel</button
      >
      <button
        class="btn border border-transparent bg-danger font-semibold text-[#1a0d0d] hover:not-disabled:bg-[#ff8484]"
        value="confirm">{request.confirmLabel}</button
      >
    </div>
  </form>
</dialog>