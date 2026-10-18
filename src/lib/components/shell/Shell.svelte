<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { api, errorMessage } from "$lib/api";
  import { app } from "$lib/stores/app.svelte";
  import AiControl from "$lib/views/AiControl.svelte";
  import Channel from "$lib/views/Channel.svelte";
  import Dashboard from "$lib/views/Dashboard.svelte";
  import Looping from "$lib/views/Looping.svelte";
  import MediaTools from "$lib/views/MediaTools.svelte";
  import TaskStrip from "$lib/components/TaskStrip.svelte";
  import Sidebar from "./Sidebar.svelte";
  import Topbar from "./Topbar.svelte";

  const SAWERIA_URL = "https://saweria.co/masimas";

  async function handleSystemAction(key: string) {
    if (key === "settings") {
      app.settingsOpen = true;
      return;
    }
    if (key === "support") {
      await openUrl(SAWERIA_URL);
      return;
    }
    if (key === "clean") {
      try {
        const result = await api.cleanCache();
        app.showToast(
          `Cache dibersihkan: ${result.removedAudio + result.removedLogs} file dihapus`,
          "success"
        );
      } catch (error) {
        app.showToast(errorMessage(error), "danger");
      }
    }
  }
</script>

<div class="flex h-full min-h-0">
  <Sidebar onSystemAction={handleSystemAction} />

  <div class="flex min-w-0 flex-1 flex-col">
    <Topbar />
    <main class="min-h-0 flex-1 overflow-y-auto bg-canvas p-5">
      <div class="mx-auto w-full max-w-[1100px] pb-10">
        {#if app.view === "dashboard"}
          <Dashboard />
        {:else if app.view === "channel"}
          <Channel />
        {:else if app.view === "looping"}
          <Looping />
        {:else if app.view === "media"}
          <MediaTools />
        {:else}
          <AiControl />
        {/if}
      </div>
    </main>
    <TaskStrip />
  </div>
</div>
