<script lang="ts">
  import type { ChannelAudioItem, ChannelPlan } from "$lib/api";
  import FileRow from "$lib/components/bins/FileRow.svelte";
  import VideoSlot from "$lib/components/bins/VideoSlot.svelte";
  import StatusBadge from "$lib/components/ui/StatusBadge.svelte";

  import { baseName, formatClock, formatDuration } from "$lib/format";
  import { audioPreview } from "$lib/stores/audioPreview.svelte";

  let {
    plan,
    songs,
    missing = 0,
    onUpdate,
    onPlaySong,
    onPreview,
    onPreviewOutput,
    onPickVideo,
    onOpenOutput,
    onRemovePlan,
  }: {
    plan: ChannelPlan;
    songs: ChannelAudioItem[];
    missing?: number;
    /** Satu pintu untuk mengubah rencana: seleksi, status, dan video. */
    onUpdate: (change: Partial<ChannelPlan>) => void;
    onPlaySong: (index: number, path: string) => void;
    /** Pratinjau video pembuka/latar. */
    onPreview: (path: string | null) => void;
    /** Pratinjau berkas hasil (urutannya diatur induk). */
    onPreviewOutput: () => void;
    /** Pilih berkas video: pembuka atau latar. */
    onPickVideo: (kind: "intro" | "background") => void;
    onOpenOutput: () => void;
    onRemovePlan: () => void;
  } = $props();

  const actionBtn =
    "cursor-pointer rounded-field border border-line px-2.5 py-1 text-[11px] text-text transition hover:bg-surface focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none";
  const dangerBtn =
    "cursor-pointer rounded-field border border-danger/40 px-2.5 py-1 text-[11px] font-medium text-danger transition hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none";

  const STATUS: Record<
    ChannelPlan["status"],
    { text: string; tone: "gray" | "info" | "success" | "danger" }
  > = {
    notProcessed: { text: "Belum diproses", tone: "gray" },
    running: { text: "Sedang diproses", tone: "info" },
    done: { text: "Selesai", tone: "success" },
    failed: { text: "Gagal", tone: "danger" },
  };

  const label = $derived(`#${String(plan.index + 1).padStart(2, "0")}`);
  const meta = $derived(STATUS[plan.status]);
  const songKey = (index: number) => `plan:${plan.id}:${index}`;
  /** Daftar lagu bisa panjang (satu lagu boleh terulang), jadi tertutup dulu. */
  let songsOpen = $state(false);
</script>

<!-- Satu rencana: dua kotak video (pembuka & latar), status di atas, sisanya di bawah. -->
<article
  class="overflow-hidden rounded-card border transition {plan.selected
    ? 'border-accent bg-accent-soft/40'
    : 'border-line bg-surface'}"
>
  <div class="space-y-2.5 px-3 py-3">
    <!-- Status pengerjaan di atas video. -->
    <div class="flex items-center gap-2">
      <input
        type="checkbox"
        class="size-4 shrink-0 cursor-pointer accent-accent focus-visible:ring-2 focus-visible:ring-accent"
        checked={plan.selected}
        aria-label={`Pilih plan ${label}`}
        onchange={(event) =>
          onUpdate({ selected: event.currentTarget.checked })}
      />
      <span class="text-[11px] font-semibold text-faint">{label}</span>
      {#if !plan.backgroundVideoPath}
        <span
          class="rounded-full border border-warning/40 bg-warning/10 px-2 py-0.5 text-[10px] font-medium text-warning"
        >
          Perlu video
        </span>
      {/if}
      <span class="ml-auto"
        ><StatusBadge text={meta.text} tone={meta.tone} /></span
      >
    </div>

    <div class="grid grid-cols-2 gap-3">
      <VideoSlot
        caption="Video pembuka"
        path={plan.introVideoPath}
        label={`rencana ${label}`}
        onPreview={() => onPreview(plan.introVideoPath)}
        onAdd={() => onPickVideo("intro")}
        onClear={() => onUpdate({ introVideoPath: null })}
      />
      <VideoSlot
        caption="Video latar"
        path={plan.backgroundVideoPath || null}
        label={`rencana ${label}`}
        onPreview={() => onPreview(plan.backgroundVideoPath)}
        onAdd={() => onPickVideo("background")}
        // Tanpa video latar rencana belum bisa diproses, jadi ikut dilepas pilihannya.
        onClear={() => onUpdate({ backgroundVideoPath: "", selected: false })}
      />
    </div>

    <!-- Informasi rencana; tombol daftar lagu duduk sendiri di kanan. -->
    <div
      class="flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-muted"
    >
      <!-- Sampai detik: dua episode bisa berbeda beberapa detik saja, dan
           selisih itu yang membedakan durasinya. -->
      <span
        title={`${formatDuration(plan.totalDurationSeconds)} (${plan.totalDurationSeconds.toFixed(1)} detik)`}
      >
        {formatClock(plan.totalDurationSeconds)}
      </span>
      <span aria-hidden="true">·</span>
      <span class="rounded-full border border-line bg-surface px-2 py-0.5">
        loop {plan.minUsage}–{plan.maxUsage}×
      </span>
      {#if songs.length > 0}
        <!-- Gaya tombol (border + chevron) supaya jelas ini bisa dibuka/tutup. -->
        <button
          type="button"
          class="ml-auto inline-flex cursor-pointer items-center gap-1 rounded-full border border-line bg-surface px-2 py-0.5 font-medium text-text transition hover:border-accent hover:text-accent focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
          aria-expanded={songsOpen}
          title={songsOpen
            ? "Sembunyikan daftar lagu"
            : "Tampilkan daftar lagu"}
          onclick={() => (songsOpen = !songsOpen)}
        >
          Daftar lagu ({songs.length})
          <span aria-hidden="true">{songsOpen ? "▴" : "▾"}</span>
        </button>
      {:else}
        <span class="ml-auto">0 lagu</span>
      {/if}
    </div>

    {#if plan.outputPath}
      <div
        class="flex items-center gap-2 rounded-field border border-line bg-canvas px-2.5 py-2"
      >
        <span class="shrink-0 text-sm text-success" aria-hidden="true">✓</span>
        <p
          class="min-w-0 flex-1 truncate text-[11px] text-muted"
          title={plan.outputPath}
        >
          Hasil: {baseName(plan.outputPath)}
        </p>
        <button type="button" class={actionBtn} onclick={onOpenOutput}
          >Buka hasil</button
        >
        <button type="button" class={actionBtn} onclick={onPreviewOutput}
          >Pratinjau</button
        >
      </div>
    {/if}

    {#if missing > 0}
      <p
        class="rounded-field border border-warning/30 bg-warning/10 px-2.5 py-1.5 text-[11px] text-warning"
      >
        {missing} lagu tidak ditemukan lagi di folder audio
      </p>
    {/if}

    {#if songsOpen && songs.length > 0}
      <div class="max-h-40 space-y-1.5 overflow-auto rounded-field">
        <!-- Tanpa key: satu lagu boleh muncul berkali-kali dalam satu rencana (loop),
             jadi `song.id` tidak pernah unik di sini. -->
        {#each songs as song, index}
          <FileRow
            path={song.path}
            kind="audio"
            {index}
            playing={audioPreview.isPlaying(songKey(index))}
            progress={audioPreview.progress}
            onPreview={() => onPlaySong(index, song.path)}
          />
        {/each}
      </div>
    {/if}

    {#if plan.error}
      <p
        class="rounded-field border border-danger/30 bg-danger/10 px-2.5 py-1.5 text-[11px] text-danger"
      >
        {plan.error}
      </p>
    {/if}

    <div class="flex flex-wrap items-center gap-2">
      {#if plan.status === "done" || plan.status === "failed"}
        <!-- Cukup ubah statusnya: rencana bisa diproses lagi tanpa mengubah hasil lama. -->
        <button
          type="button"
          class={actionBtn}
          onclick={() => onUpdate({ status: "notProcessed" })}
        >
          Tandai belum diproses
        </button>
      {/if}
      <button type="button" class={dangerBtn} onclick={onRemovePlan}
        >Hapus plan</button
      >
    </div>
  </div>
</article>
