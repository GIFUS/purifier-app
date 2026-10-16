<script lang="ts">
  import { onMount } from "svelte";

  import AboutPage from "./components/AboutPage.svelte";
  import AutorunPage from "./components/AutorunPage.svelte";
  import CleanupPage from "./components/CleanupPage.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import { boot, lockContextMenu } from "./cleanup.svelte";
  import { state } from "./state.svelte";

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
      <CleanupPage />
    {:else if state.page === "autostart"}
      <AutorunPage />
    {:else}
      <AboutPage />
    {/if}
  </div>
</div>
