<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { Modal } from "flowbite-svelte";

  import { baseName, extension } from "$lib/format";

  let {
    open = $bindable(false),
    path = null,
    kind = "audio"
  }: {
    open?: boolean;
    path?: string | null;
    kind?: "audio" | "video";
  } = $props();

  const EMBEDDED_AUDIO = [".mp3", ".wav", ".m4a", ".aac", ".ogg", ".flac"];
  const EMBEDDED_VIDEO = [".mp4", ".mov", ".webm", ".m4v"];

  /** Video besar (2 jam bisa >5 GB) sering tak selesai dimuat webview. */
  const LOAD_TIMEOUT_MS = 8000;

  const ext = $derived(path ? extension(path) : "");
  const supported = $derived(
    kind === "audio" ? EMBEDDED_AUDIO.includes(ext) : EMBEDDED_VIDEO.includes(ext)
  );
  const source = $derived(path ? convertFileSrc(path) : "");

  /** Gagal dimuat di webview; `noPlayer` = pemutar bawaan pun tak tersedia. */
  let failed = $state(false);
  let noPlayer = $state(false);
  let ready = false;
  /** Kunci konten terakhir: reset hanya saat modal/path benar-benar berganti. */
  let lastKey = "";

  // Setiap kali modal/path berganti, hitungan dimulai ulang.
  $effect(() => {
    const key = open && path ? `${path}|${supported}` : "";
    if (key === lastKey) return;
    lastKey = key;
    failed = false;
    noPlayer = false;
    ready = false;
    if (!key) return;
    // Format yang memang tak bisa dipratinjau langsung dilempar ke pemutar
    // bawaan (ditunda ke microtask agar tidak menulis state di dalam efek ini).
    if (!supported) {
      queueMicrotask(() => void toPlayer(path!));
      return;
    }
    const timer = setTimeout(() => {
      if (!ready) void toPlayer(path!);
    }, LOAD_TIMEOUT_MS);
    return () => clearTimeout(timer);
  });

  const externalBtn =
    "mt-2 cursor-pointer rounded-field border border-line bg-surface px-3 py-1.5 text-xs font-medium text-text transition hover:bg-canvas focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none";

  /**
   * Tidak bisa dipratinjau di app: lempar ke pemutar bawaan. Kalau tidak ada
   * pemutar bawaan, modal ini yang jadi jalan keluarnya.
   */
  async function toPlayer(target: string) {
    if (failed) return;
    failed = true;
    try {
      await openPath(target);
      open = false;
    } catch {
      noPlayer = true;
    }
  }

  async function reveal(target: string) {
    try {
      await revealItemInDir(target);
    } catch {
      // Best-effort: membuka Finder bukan hal kritis.
    }
  }
</script>

<Modal bind:open title="Pratinjau" size="xl">
  {#if path}
    <div class="space-y-3">
      <p class="truncate text-xs text-muted" title={path}>{baseName(path)}</p>
      {#if supported && kind === "audio"}
        <!-- svelte-ignore a11y_media_has_caption -->
        <audio class="w-full" src={source} controls autoplay></audio>
      {:else if supported && !failed}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          class="max-h-[60vh] w-full rounded-field bg-black"
          src={source}
          controls
          autoplay
          onerror={() => toPlayer(path)}
          onloadedmetadata={() => (ready = true)}
        ></video>
      {:else if noPlayer}
        <div class="rounded-field border border-line bg-canvas px-3 py-4 text-center">
          <p class="text-sm text-text">Berkas ini tidak bisa dipratinjau di aplikasi.</p>
          <p class="mt-1 text-[11px] text-muted">
            Ukurannya besar atau formatnya tidak didukung, dan aplikasi pemutar bawaan untuk berkas
            seperti ini juga tidak tersedia. Buka lewat Finder untuk memutarnya.
          </p>
          <button type="button" class={externalBtn} onclick={() => reveal(path)}>
            Buka di Finder
          </button>
        </div>
      {:else}
        <div class="rounded-field border border-line bg-canvas px-3 py-4 text-center">
          <p class="text-sm text-text">Berkas ini tidak bisa dipratinjau di aplikasi.</p>
          <p class="mt-1 text-[11px] text-muted">Sedang dibuka dengan pemutar bawaan…</p>
          <button type="button" class={externalBtn} onclick={() => reveal(path)}>
            Buka di Finder
          </button>
        </div>
      {/if}
    </div>
  {/if}
</Modal>
