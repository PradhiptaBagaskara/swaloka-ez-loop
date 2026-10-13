<script lang="ts">
  import { onMount } from "svelte";

  import { app } from "$lib/stores/app.svelte";
  import { setup } from "$lib/stores/setup.svelte";

  const platform = navigator.userAgent.includes("Win")
    ? "windows"
    : navigator.userAgent.includes("Mac")
      ? "macos"
      : "linux";

  const instructions: Record<string, { title: string; command: string }[]> = {
    windows: [
      { title: "winget", command: "winget install Gyan.FFmpeg" },
      { title: "Chocolatey", command: "choco install ffmpeg" },
      { title: "Scoop", command: "scoop install ffmpeg" }
    ],
    macos: [
      { title: "Homebrew", command: "brew install ffmpeg" },
      { title: "Manual", command: "unduh dari evermeet.cx/ffmpeg lalu taruh di PATH" }
    ],
    linux: [
      { title: "Debian/Ubuntu", command: "sudo apt install ffmpeg" },
      { title: "Fedora", command: "sudo dnf install ffmpeg" },
      { title: "Arch", command: "sudo pacman -S ffmpeg" }
    ]
  };

  onMount(() => {
    if (!setup.status) void setup.refresh();
  });

  async function recheck() {
    await app.checkFfmpeg();
    await setup.refresh();
    if (!app.ffmpegReady) {
      app.showToast("FFmpeg masih belum terdeteksi.", "danger");
    }
  }
</script>

<div class="flex h-full items-center justify-center bg-canvas p-6">
  <div class="w-full max-w-xl rounded-card border border-line bg-surface p-8">
    <div class="text-center">
      <p class="text-3xl" aria-hidden="true">⚠</p>
      <h1 class="mt-3 text-xl font-semibold text-text">FFmpeg belum terpasang</h1>
      <p class="mt-2 text-sm text-muted">
        Swaloka memakai FFmpeg bawaannya sendiri (v{setup.status?.pinnedVersion ?? "8.1.2"})
        supaya hasilnya sama di setiap komputer.
      </p>
      {#if app.ffmpegSource === "custom"}
        <p class="mt-2 text-xs text-danger">
          Path FFmpeg yang diisi di Pengaturan belum bisa dipakai.
        </p>
      {/if}
    </div>

    {#if setup.running}
      <div class="mt-6 space-y-2" role="status" aria-live="polite">
        <div class="flex items-baseline justify-between gap-3 text-xs">
          <span class="font-medium text-text">{setup.message}</span>
          <span class="shrink-0 tabular-nums text-muted">{setup.percentLabel}%</span>
        </div>
        <div class="h-2 overflow-hidden rounded-full bg-input">
          <div
            class="h-full rounded-full bg-accent transition-[width] duration-200 motion-reduce:transition-none"
            style={`width:${setup.percentLabel}%`}
          ></div>
        </div>
        <div class="flex items-center justify-between gap-3 text-[11px] text-muted">
          <span class="tabular-nums">{setup.transferredLabel}</span>
          <button
            type="button"
            class="rounded-field border border-line px-3 py-1 font-medium text-text transition hover:bg-canvas"
            onclick={() => setup.cancel()}
          >
            Batal
          </button>
        </div>
      </div>
    {:else}
      {#if setup.error}
        <div
          class="mt-5 rounded-field border border-danger/40 bg-danger/5 px-3 py-2 text-sm text-danger"
          role="alert"
        >
          {setup.error}
        </div>
      {/if}

      {#if setup.canInstall}
        <div class="mt-6 flex flex-col items-center gap-2">
          <button
            type="button"
            class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90"
            onclick={() => setup.install()}
          >
            {setup.error ? "Coba unduh lagi" : `Unduh FFmpeg ${setup.sizeLabel ?? ""}`}
          </button>
          <p class="text-[11px] text-faint">
            Unduhan hanya sekali; berkasnya disimpan di folder aplikasi.
          </p>
        </div>
      {:else}
        <div class="mt-5 rounded-field bg-input px-3 py-2 text-sm text-muted">
          Unduh otomatis belum tersedia untuk platform ini. Pasang FFmpeg sendiri, lalu
          arahkan path-nya di bawah.
        </div>
      {/if}
    {/if}

    <details class="mt-6 rounded-field border border-line px-3 py-2" open={!setup.canInstall}>
      <summary class="cursor-pointer text-xs font-semibold text-muted">
        Cara lain (pasang sendiri)
      </summary>
      <div class="mt-3 space-y-2">
        {#each instructions[platform] as item (item.title)}
          <div class="rounded-field bg-input px-3 py-2">
            <p class="text-[11px] font-semibold uppercase tracking-wide text-faint">{item.title}</p>
            <p class="font-mono text-xs text-text">{item.command}</p>
          </div>
        {/each}
      </div>
      <div class="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          class="rounded-field border border-line px-4 py-2 text-sm font-medium text-text transition hover:bg-canvas"
          onclick={recheck}
        >
          Periksa lagi
        </button>
        <button
          type="button"
          class="rounded-field border border-line px-4 py-2 text-sm font-medium text-text transition hover:bg-canvas"
          onclick={() => (app.settingsOpen = true)}
        >
          Atur path FFmpeg
        </button>
      </div>
    </details>
  </div>
</div>
