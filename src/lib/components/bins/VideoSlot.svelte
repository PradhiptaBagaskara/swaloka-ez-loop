<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";

  import { baseName } from "$lib/format";

  let {
    caption,
    path = null,
    label = "",
    onPreview,
    onAdd,
    onClear
  }: {
    caption: string;
    path?: string | null;
    /** Konteks tambahan untuk aria-label, mis. "rencana #01". */
    label?: string;
    onPreview: () => void;
    onAdd: () => void;
    onClear: () => void;
  } = $props();

  const suffix = $derived(label ? ` ${label}` : "");
  const lower = $derived(caption.toLowerCase());

  /**
   * Bingkai pratinjau diambil oleh webview sendiri: cukup seek ke sekitar 10%
   * durasi supaya bukan layar hitam/gradasi pembuka. Tidak ada ekstraksi
   * thumbnail lewat FFmpeg, jadi tidak ada berkas cache dan tidak ada langkah
   * yang bisa gagal diam-diam.
   */
  function seekToRepresentativeFrame(event: Event) {
    const element = event.currentTarget as HTMLVideoElement;
    const duration = element.duration;
    if (!Number.isFinite(duration) || duration <= 0) return;
    try {
      element.currentTime = Math.min(Math.max(duration * 0.1, 1), 60);
    } catch {
      // Sebagian format tidak bisa di-seek; bingkai pertamanya tetap dipakai.
    }
  }
</script>

<!-- Satu kotak video: pratinjau bila ada, tombol tambah bila kosong. -->
<div class="min-w-0">
  <div class="relative">
    {#if path}
      <!-- Ada video: klik untuk pratinjau. -->
      <button
        type="button"
        class="relative block aspect-video w-full cursor-pointer overflow-hidden rounded-field border border-line bg-stage focus-visible:ring-2 focus-visible:ring-accent"
        title={`Pratinjau ${lower}`}
        aria-label={`Pratinjau ${lower}${suffix}`}
        onclick={onPreview}
      >
        <!-- Label di belakang video: tetap terbaca kalau formatnya tidak bisa
             diputar webview (mkv/avi/wmv lewat jalur ini memang tanpa bingkai). -->
        <span class="absolute inset-0 grid place-items-center text-[10px] text-faint" aria-hidden="true">
          pratinjau
        </span>
        <video
          src={convertFileSrc(path)}
          class="relative size-full object-cover"
          muted
          playsinline
          preload="metadata"
          tabindex="-1"
          aria-hidden="true"
          onloadedmetadata={seekToRepresentativeFrame}
        ></video>
        <span
          class="absolute bottom-1 right-1 rounded bg-black/60 px-1 text-[10px] leading-4 text-white"
          aria-hidden="true">▶</span
        >
      </button>
      <!-- Tombol hapus di pojok kanan atas video. -->
      <button
        type="button"
        class="absolute right-1 top-1 z-10 grid size-5 cursor-pointer place-items-center rounded-full border border-white/25 bg-black/60 text-[10px] leading-none text-white transition hover:border-danger hover:bg-danger focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
        title={`Hapus ${lower}`}
        aria-label={`Hapus ${lower}${suffix}`}
        onclick={onClear}
      >
        ✕
      </button>
    {:else}
      <!-- Belum ada video: klik untuk menambahkan. -->
      <button
        type="button"
        class="grid aspect-video w-full cursor-pointer place-items-center rounded-field border border-dashed border-line bg-canvas text-[10px] text-faint transition hover:border-accent hover:text-accent focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
        title={`Tambah ${lower}`}
        onclick={onAdd}
      >
        ＋ {caption}
      </button>
    {/if}
  </div>
  <p class="mt-1 truncate text-[10px] text-faint" title={path ?? ""}>
    {caption}{path ? ` · ${baseName(path)}` : ""}
  </p>
</div>
