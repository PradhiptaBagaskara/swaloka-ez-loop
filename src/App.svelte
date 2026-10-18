<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import Shell from "$lib/components/shell/Shell.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { tasks } from "$lib/stores/task.svelte";
  import { update } from "$lib/stores/update.svelte";
  import FfmpegGuide from "$lib/views/FfmpegGuide.svelte";
  import Landing from "$lib/views/Landing.svelte";
  import SettingsDialog from "$lib/views/SettingsDialog.svelte";
  import UpdateDialog from "$lib/views/UpdateDialog.svelte";

  onMount(() => {
    // Listener run dipasang sekali di sini supaya run yang dimulai dari luar
    // antarmuka (mis. lewat server MCP) ikut tampil di queue card.
    void tasks.install();
    void listen<{ scope: string }>("data://changed", (event) => {
      void app.applyExternalChange(event.payload.scope);
    });
    void app.bootstrap().then(() => update.check());
  });

  const toastClass = $derived(
    app.toast?.kind === "danger"
      ? "border-danger/40 bg-danger/10 text-danger"
      : "border-success/40 bg-success/10 text-success"
  );
</script>

{#if app.screen === "loading"}
  <div class="flex h-full items-center justify-center text-sm text-muted">
    Menyiapkan Swaloka...
  </div>
{:else if app.screen === "guide"}
  <FfmpegGuide />
{:else if app.screen === "landing"}
  <Landing />
{:else}
  <Shell />
{/if}

<SettingsDialog bind:open={app.settingsOpen} />
<UpdateDialog />

{#if app.toast}
  <div
    class="fixed left-1/2 top-4 z-50 -translate-x-1/2 rounded-card border px-4 py-2.5 text-sm shadow-lg {toastClass}"
  >
    {app.toast.message}
  </div>
{/if}
