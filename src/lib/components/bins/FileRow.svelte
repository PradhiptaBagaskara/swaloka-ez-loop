<script lang="ts">
  import { baseName, formatBytes } from "$lib/format";

  let {
    path,
    kind = "audio",
    index,
    size,
    playing = false,
    progress = 0,
    reorderable = false,
    dragging = false,
    onPreview,
    onRemove,
    onDragStart,
    onDragEnd,
    onDrop
  }: {
    path: string;
    kind?: "audio" | "video";
    index?: number;
    size?: number;
    playing?: boolean;
    progress?: number;
    /** Tampilkan handle geser dan terima drop untuk mengubah urutan. */
    reorderable?: boolean;
    /** Baris ini sedang diseret. */
    dragging?: boolean;
    onPreview?: () => void;
    onRemove?: () => void;
    onDragStart?: () => void;
    onDragEnd?: () => void;
    /** Ada baris lain yang dijatuhkan pada baris ini. */
    onDrop?: () => void;
  } = $props();

  const icon = $derived(kind === "video" ? "▭" : "♪");
</script>

<div
  class="rounded-field border border-line bg-canvas px-2.5 py-2 transition {dragging
    ? 'opacity-50'
    : ''}"
  role={reorderable ? "listitem" : undefined}
  ondragover={(event) => {
    if (reorderable) event.preventDefault();
  }}
  ondrop={(event) => {
    if (!reorderable) return;
    event.preventDefault();
    onDrop?.();
  }}
>
  <div class="flex items-center gap-2">
    {#if reorderable}
      <!-- Handle geser: hanya ini yang bisa diseret, supaya klik tombol tetap aman. -->
      <span
        class="-ml-1 shrink-0 cursor-grab select-none rounded px-0.5 text-sm text-faint transition hover:text-text active:cursor-grabbing"
        draggable="true"
        title="Geser untuk ubah urutan"
        aria-hidden="true"
        ondragstart={(event) => {
          // WebKit (WKWebView macOS) butuh dataTransfer terisi, kalau tidak
          // drag dianggap tidak sah dan event `drop` tak pernah dipanggil.
          event.dataTransfer?.setData("text/plain", path);
          if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
          onDragStart?.();
        }}
        ondragend={() => onDragEnd?.()}
      >⠿</span>
    {/if}
    {#if index !== undefined}
      <span class="w-5 shrink-0 text-center text-[11px] font-semibold text-faint">{index}</span>
    {/if}
    <span class="shrink-0 text-sm text-muted" aria-hidden="true">{icon}</span>
    <div class="min-w-0 flex-1">
      <p class="truncate text-sm text-text" title={path}>{baseName(path)}</p>
      {#if size !== undefined}
        <p class="text-[11px] text-faint">{formatBytes(size)}</p>
      {/if}
    </div>

    <div class="flex shrink-0 items-center gap-1.5">
    {#if onPreview}
      <button
        type="button"
        class="rounded p-1 transition {playing
          ? 'text-accent'
          : 'text-muted hover:bg-surface hover:text-accent'}"
        title={playing ? "Hentikan" : "Pratinjau"}
        aria-label="{playing ? 'Hentikan' : 'Pratinjau'} {baseName(path)}"
        onclick={onPreview}>{playing ? "■" : "▶"}</button
      >
    {/if}
    {#if onRemove}
      <button
        type="button"
        class="ml-2 rounded border-l border-line p-1 pl-3 text-muted transition hover:bg-danger/10 hover:text-danger"
        title="Hapus"
        aria-label="Hapus {baseName(path)}"
        onclick={onRemove}>✕</button
      >
    {/if}
    </div>
  </div>

  {#if playing}
    <div class="mt-1.5 h-0.5 overflow-hidden rounded-full bg-line-subtle">
      <div
        class="h-full rounded-full bg-accent transition-[width] duration-200"
        style={`width:${Math.round(progress * 100)}%`}
      ></div>
    </div>
  {/if}
</div>
