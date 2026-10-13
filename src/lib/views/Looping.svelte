<script lang="ts">
  import { onDestroy } from "svelte";
  import { Modal } from "flowbite-svelte";

  import { api, type LoopRequest, type LoopResult, type PreflightReport } from "$lib/api";
  import DropZone from "$lib/components/bins/DropZone.svelte";
  import FileRow from "$lib/components/bins/FileRow.svelte";
  import VideoSlot from "$lib/components/bins/VideoSlot.svelte";
  import PreviewModal from "$lib/components/PreviewModal.svelte";
  import Card from "$lib/components/ui/Card.svelte";
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import PageHeader from "$lib/components/ui/PageHeader.svelte";
  import { baseName, fileStem } from "$lib/format";
  import { pickMedia } from "$lib/picker";
  import { app } from "$lib/stores/app.svelte";
  import { audioPreview } from "$lib/stores/audioPreview.svelte";
  import { session } from "$lib/stores/session.svelte";
  import { tasks } from "$lib/stores/task.svelte";

  const form = session.looping;

  const STAGING: [number, string][] = [
    [0.1, "Menyiapkan"],
    [0.3, "Menormalkan audio"],
    [0.5, "Menggabungkan audio"],
    [0.7, "Menyusun video"],
    [1, "Menyelesaikan"]
  ];

  let previewPath = $state<string | null>(null);
  /** Nama output yang terakhir diisi otomatis, untuk tahu kapan boleh ditimpa lagi. */
  let autoOutputName = $state("");
  let previewOpen = $state(false);
  /** Indeks trek yang sedang diseret untuk mengubah urutan. */
  let dragFrom = $state<number | null>(null);
  let warningOpen = $state(false);
  let warningReport = $state<PreflightReport | null>(null);
  let pending = $state<LoopRequest | null>(null);

  const outputDir = $derived(
    app.project
      ? app.project.customOutputPath || `${app.project.rootPath}/outputs`
      : ""
  );
  const canGenerate = $derived(
    Boolean(form.backgroundPath) && form.audioFiles.length > 0 && form.loopCount >= 1
  );

  /** Video memakai modal pratinjau; audio diputar inline tanpa modal. */
  function showVideo(path: string | null) {
    if (!path) return;
    previewPath = path;
    previewOpen = true;
  }

  const audioKey = (index: number) => `loop:${index}`;

  function toggleAudio(index: number, path: string) {
    const key = audioKey(index);
    if (audioPreview.isPlaying(key)) audioPreview.stop();
    else audioPreview.playBase(key, path);
  }

  onDestroy(() => audioPreview.stop());

  /**
   * Pasang video latar dan isi otomatis nama output dari namanya.
   * Nama yang sudah diketik pengguna sendiri tidak ditimpa.
   */
  function applyBackground(path: string) {
    form.backgroundPath = path;
    const suggested = fileStem(path);
    if (form.outputName.trim() === "" || form.outputName === autoOutputName) {
      form.outputName = suggested;
      autoOutputName = suggested;
    }
  }

  async function pickBackground() {
    const [path] = await pickMedia("video");
    if (path) applyBackground(path);
  }

  async function pickIntro() {
    const [path] = await pickMedia("video");
    if (path) form.introPath = path;
  }

  async function pickAudio() {
    const paths = await pickMedia("audio", true);
    if (paths.length > 0) form.audioFiles = [...form.audioFiles, ...paths];
  }

  function removeAudio(index: number) {
    form.audioFiles = form.audioFiles.filter((_, position) => position !== index);
  }

  /** Jatuhkan di baris ini: pindahkan trek yang diseret ke posisi ini. */
  function dropAudio(to: number) {
    const from = dragFrom;
    dragFrom = null;
    // Kunci pemutaran berbasis indeks, jadi hentikan dulu agar tak salah sorot.
    audioPreview.stop();
    if (from === null || from === to) return;
    const next = [...form.audioFiles];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    form.audioFiles = next;
  }

  function buildRequest(): LoopRequest {
    return {
      backgroundPath: form.backgroundPath ?? "",
      audioFiles: [...form.audioFiles],
      outputDir,
      outputName: form.outputName,
      loopCount: Math.max(1, Math.floor(form.loopCount)),
      introPath: form.introPath,
      outputPath: null
    };
  }

  async function start() {
    if (!canGenerate || tasks.running) return;
    pending = buildRequest();
    try {
      const report = await api.loopPreflight(
        pending.backgroundPath,
        pending.introPath
      );
      const hasWarnings =
        report.codecMismatch ||
        report.mismatchFields.length > 0 ||
        (report.youtube?.issues.length ?? 0) > 0;
      if (hasWarnings) {
        warningReport = report;
        warningOpen = true;
        return;
      }
    } catch {
      // Pratinjau pra-terbang bersifat opsional; lanjut saja bila gagal.
    }
    await execute();
  }

  async function proceed() {
    warningOpen = false;
    await execute();
  }

  async function execute() {
    const request = pending;
    if (!request) return;
    await tasks.run(() => api.loopRun(request), {
      staging: STAGING,
      done: (value) => {
        const result = value as LoopResult;
        return { label: baseName(result.outputPath), path: result.outputPath };
      }
    });
  }
</script>

<PageHeader
  pretitle="Alat"
  title="Looping"
  subtitle="Putar video latar berulang di atas playlist audio."
>
  {#snippet actions()}
    <button
      type="button"
      class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50"
      disabled={!canGenerate || tasks.running}
      onclick={start}
    >
      Buat video
    </button>
  {/snippet}
</PageHeader>

<Card title="Pengaturan" subtitle="Jumlah putaran dan nama file output.">
  <div class="grid gap-4 sm:grid-cols-[160px_1fr]">
    <label class="text-xs font-medium text-muted">
      Jumlah loop
      <input
        type="number"
        min="1"
        class="mt-1 w-full rounded-field border border-line bg-input px-3 py-2 text-sm text-text outline-none focus:border-accent"
        bind:value={form.loopCount}
      />
    </label>
    <label class="text-xs font-medium text-muted">
      Nama output
      <input
        type="text"
        placeholder="Bawaan: nama video latar"
        class="mt-1 w-full rounded-field border border-line bg-input px-3 py-2 text-sm text-text outline-none focus:border-accent"
        bind:value={form.outputName}
      />
    </label>
  </div>

  {#if !canGenerate}
    <ul class="mt-3 space-y-1 text-[11px] text-warning">
      {#if !form.backgroundPath}
        <li>• Tambahkan video latar untuk mengaktifkan Buat video.</li>
      {/if}
      {#if form.audioFiles.length === 0}
        <li>• Tambahkan minimal satu trek audio untuk mengaktifkan Buat video.</li>
      {/if}
    </ul>
  {/if}
</Card>

<div class="mt-4">
  <Card title="Video" subtitle="Video pembuka diputar sekali di awal, lalu video latar berulang.">
    <div class="grid grid-cols-2 gap-3">
      <VideoSlot
        caption="Video pembuka"
        path={form.introPath}
        onPreview={() => showVideo(form.introPath)}
        onAdd={pickIntro}
        onClear={() => (form.introPath = null)}
      />
      <VideoSlot
        caption="Video latar"
        path={form.backgroundPath}
        onPreview={() => showVideo(form.backgroundPath)}
        onAdd={pickBackground}
        onClear={() => (form.backgroundPath = null)}
      />
    </div>
  </Card>
</div>

<div class="mt-4">
  <Card title={`Trek audio (${form.audioFiles.length})`}>
    {#snippet actions()}
      {#if form.audioFiles.length > 0}
        <button
          type="button"
          class="text-xs text-muted transition hover:text-danger"
          onclick={() => (form.audioFiles = [])}
        >
          Hapus semua
        </button>
      {/if}
    {/snippet}

    <DropZone
      label="Tambah file audio"
      hint="MP3, WAV, M4A, AAC, OGG, FLAC"
      onPick={pickAudio}
    />

    {#if form.audioFiles.length === 0}
      <EmptyState
        title="Belum ada trek audio"
        subtitle="Tambahkan MP3, WAV, atau M4A ke playlist."
        icon="♪"
      />
    {:else}
      <div class="mt-3 space-y-2" role="list">
        {#each form.audioFiles as path, index (path + index)}
          <FileRow
            {path}
            kind="audio"
            index={index + 1}
            playing={audioPreview.isPlaying(audioKey(index))}
            progress={audioPreview.progress}
            reorderable
            dragging={dragFrom === index}
            onDragStart={() => (dragFrom = index)}
            onDragEnd={() => (dragFrom = null)}
            onDrop={() => dropAudio(index)}
            onPreview={() => toggleAudio(index, path)}
            onRemove={() => removeAudio(index)}
          />
        {/each}
      </div>
    {/if}
  </Card>
</div>

<PreviewModal bind:open={previewOpen} path={previewPath} kind="video" />

<Modal bind:open={warningOpen} title="Sebelum lanjut" size="md">
  <div class="space-y-3 text-sm text-text">
    {#if warningReport?.codecMismatch}
      <p>Video pembuka dan video latar memakai codec berbeda.</p>
    {/if}
    {#if warningReport && warningReport.mismatchFields.length > 0}
      <p>Tidak cocok: {warningReport.mismatchFields.join(", ")}.</p>
    {/if}
    {#if warningReport?.youtube && warningReport.youtube.issues.length > 0}
      <div>
        <p class="font-medium">Standar YouTube: {warningReport.youtube.status}</p>
        <ul class="mt-1 list-disc space-y-0.5 pl-5 text-xs text-muted">
          {#each warningReport.youtube.issues as issue (issue)}
            <li>{issue}</li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <button
        type="button"
        class="rounded-field border border-line px-4 py-2 text-sm text-text transition hover:bg-canvas"
        onclick={() => (warningOpen = false)}
      >
        Batal
      </button>
      <button
        type="button"
        class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90"
        onclick={proceed}
      >
        Lanjut
      </button>
    </div>
  {/snippet}
</Modal>
