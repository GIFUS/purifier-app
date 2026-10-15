<script lang="ts">
  import { onMount } from "svelte";

  import { appInfo, openRepository } from "../api";
  import githubMark from "../../assets/github-icon.svg?raw";
  import { PAGE_LABELS } from "../nav";
  import type { AppInfo } from "../types";

  let info = $state<AppInfo | null>(null);

  onMount(() => {
    appInfo()
      .then((value) => (info = value))
      .catch(() => (info = null));
  });

  const version = $derived(info?.version ?? "…");
  const identifier = $derived(info?.identifier ?? "…");
</script>

<section class="grid h-full min-h-0 grid-rows-[auto_auto_1fr] overflow-hidden px-4 pb-4 pt-[18px]">
  <header>
    <h1 class="m-0 text-title font-semibold tracking-[0.2px]">{PAGE_LABELS.about}</h1>
    <p class="mt-1 text-caption text-text-muted">
      A cleaner that shows its work before it deletes anything.
    </p>
  </header>

  <dl class="mt-[18px] grid gap-0">
    <div class="flex gap-4 border-b border-border py-2">
      <dt class="w-[150px] flex-none text-label text-text-muted">Version</dt>
      <dd class="m-0 wrap-anywhere text-label text-text">{version}</dd>
    </div>
    <div class="flex gap-4 border-b border-border py-2">
      <dt class="w-[150px] flex-none text-label text-text-muted">Identifier</dt>
      <dd class="m-0 wrap-anywhere text-label text-text">{identifier}</dd>
    </div>
    <div class="flex gap-4 border-b border-border py-2">
      <dt class="w-[150px] flex-none text-label text-text-muted">Platform</dt>
      <dd class="m-0 wrap-anywhere text-label text-text">
        Windows 10 or Windows 11 only
      </dd>
    </div>
  </dl>

  <div class="mt-auto flex items-center gap-2.5 pt-3.5">
    <span class="text-label text-text-muted">Source code</span>
    <button
      type="button"
      class="inline-flex size-7 cursor-pointer items-center justify-center rounded-app text-text-muted transition-colors duration-150 hover:bg-bg-hover hover:text-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent [&_path]:fill-current [&_svg]:size-[18px]"
      title="Open the repository on GitHub"
      aria-label="Purifier on GitHub"
      onclick={() => void openRepository()}
    >
      {@html githubMark}
    </button>
  </div>
</section>