<script lang="ts">
  import { openPath } from "@tauri-apps/plugin-opener";

  import { app } from "$lib/stores/app.svelte";

  async function openRoot() {
    if (app.project) {
      // Best-effort: jangan blokir UI bila gagal.
      try {
        await openPath(app.project.rootPath);
      } catch {
        /* diabaikan */
      }
    }
  }
</script>

<header
  class="flex h-14 shrink-0 items-center gap-3 border-b border-line bg-surface px-4"
>
  <div class="min-w-0">
    <p class="truncate text-sm font-semibold text-text">{app.project?.name ?? "Swaloka"}</p>
    <button
      type="button"
      class="truncate text-[11px] text-muted transition hover:text-accent"
      title={app.project?.rootPath ?? ""}
      onclick={openRoot}
    >
      Buka folder
    </button>
  </div>

  <div class="flex-1"></div>

  <button
    type="button"
    class="rounded-field border border-line px-2.5 py-1.5 text-xs text-text transition hover:bg-canvas"
    title="Ganti tema"
    aria-label="Ganti tema"
    onclick={() => app.setTheme(app.theme === "dark" ? "light" : "dark")}
  >
    {app.theme === "dark" ? "☀" : "☾"}
  </button>

  <button
    type="button"
    class="rounded-field border border-line px-2.5 py-1.5 text-xs text-text transition hover:bg-canvas"
    title="Tutup proyek"
    onclick={() => app.closeProject()}
  >
    Tutup proyek
  </button>
</header>
