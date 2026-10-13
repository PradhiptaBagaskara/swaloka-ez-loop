<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { app } from "$lib/stores/app.svelte";
  import { pickDirectory } from "$lib/picker";

  const SAWERIA_URL = "https://saweria.co/masimas";

  async function createProject() {
    const directory = await pickDirectory("Pilih folder untuk proyek baru");
    if (directory) await app.createProject(directory);
  }

  async function openProject() {
    const directory = await pickDirectory("Pilih folder proyek yang ada");
    if (directory) await app.openProject(directory);
  }

  async function removeRecent(path: string) {
    await app.prefsRemove(path);
  }
</script>

<div class="min-h-full bg-canvas p-8">
  <div class="mx-auto w-full max-w-2xl space-y-5">
    <div class="text-center">
      <span
        class="mx-auto flex size-16 items-center justify-center rounded-card bg-accent text-2xl font-bold text-white"
        aria-hidden="true">S</span
      >
      <h1 class="mt-4 text-2xl font-semibold text-text">Swaloka Looping Tool</h1>
      <p class="mt-1 text-sm text-muted">
        Putar video latar berulang di atas playlist audio.
      </p>
    </div>

    <div class="grid gap-4 sm:grid-cols-2">
      <div class="rounded-card border border-line bg-surface p-5">
        <p class="text-lg" aria-hidden="true">🖼</p>
        <h2 class="mt-2 text-[15px] font-semibold text-text">Buat proyek</h2>
        <p class="mt-1 text-xs text-muted">Mulai proyek dari awal.</p>
        <button
          type="button"
          class="mt-4 w-full rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50"
          disabled={app.busy}
          onclick={createProject}
        >
          Buat proyek
        </button>
      </div>

      <div class="rounded-card border border-line bg-surface p-5">
        <p class="text-lg" aria-hidden="true">📂</p>
        <h2 class="mt-2 text-[15px] font-semibold text-text">Buka proyek</h2>
        <p class="mt-1 text-xs text-muted">Lanjutkan proyek yang tersimpan.</p>
        <button
          type="button"
          class="mt-4 w-full rounded-field border border-line px-4 py-2 text-sm font-medium text-text transition hover:bg-canvas disabled:opacity-50"
          disabled={app.busy}
          onclick={openProject}
        >
          Buka proyek
        </button>
      </div>
    </div>

    <section class="rounded-card border border-line bg-surface">
      <header class="border-b border-line px-4 py-3">
        <h2 class="text-[15px] font-semibold text-text">Proyek terkini</h2>
      </header>
      <div class="p-2">
        {#if app.prefs.recent_projects.length === 0}
          <p class="px-2 py-3 text-xs text-muted">Belum ada proyek terkini.</p>
        {:else}
          {#each app.prefs.recent_projects as path (path)}
            {@const name = path.split(/[\\/]/).filter(Boolean).pop() ?? path}
            <div class="relative flex items-center gap-2 rounded-field px-2 py-2 hover:bg-canvas">
              <span class="text-sm text-muted" aria-hidden="true">▭</span>
              <div class="min-w-0 flex-1">
                <!-- Tombol ini menutupi seluruh baris, jadi satu klik di mana pun membuka proyek. -->
                <button
                  type="button"
                  class="block w-full cursor-pointer truncate text-left text-sm text-text after:absolute after:inset-0 after:content-[''] focus-visible:ring-2 focus-visible:ring-accent"
                  title={name}
                  aria-label={`Buka proyek ${name}`}
                  onclick={() => app.openProject(path)}
                >
                  {name}
                </button>
                <p class="truncate text-[11px] text-faint" title={path}>{path}</p>
              </div>
              <button
                type="button"
                class="relative z-10 rounded p-1 text-muted transition hover:bg-surface hover:text-danger focus-visible:ring-2 focus-visible:ring-accent"
                title="Hapus dari terkini"
                aria-label={`Hapus ${name} dari terkini`}
                onclick={() => removeRecent(path)}>✕</button
              >
            </div>
          {/each}
        {/if}
      </div>
    </section>

    <div class="flex items-center justify-center gap-3 text-xs text-faint">
      <span>v{app.version}</span>
      <button
        type="button"
        class="transition hover:text-text"
        onclick={() => (app.settingsOpen = true)}>Pengaturan</button
      >
      <button type="button" class="transition hover:text-text" onclick={() => openUrl(SAWERIA_URL)}>
        Dukungan
      </button>
    </div>
  </div>
</div>
