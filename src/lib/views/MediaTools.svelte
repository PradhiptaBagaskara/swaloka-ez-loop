<script lang="ts">
  import { onDestroy } from "svelte";

  import {
    api,
    errorMessage,
    type AudioOverlay,
    type EncoderOption,
    type LiveDefaults,
    type LiveStandardOptions,
    type VideoInfo
  } from "$lib/api";
  import DropZone from "$lib/components/bins/DropZone.svelte";
  import FileRow from "$lib/components/bins/FileRow.svelte";
  import SingleSlot from "$lib/components/bins/SingleSlot.svelte";
  import PreviewModal from "$lib/components/PreviewModal.svelte";
  import Card from "$lib/components/ui/Card.svelte";
  import DataGrid from "$lib/components/ui/DataGrid.svelte";
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import PageHeader from "$lib/components/ui/PageHeader.svelte";
  import Segmented from "$lib/components/ui/Segmented.svelte";
  import StatusBadge from "$lib/components/ui/StatusBadge.svelte";
  import { baseName, fileStem, timestampSlug } from "$lib/format";
  import { pickMedia } from "$lib/picker";
  import { app } from "$lib/stores/app.svelte";
  import { audioPreview } from "$lib/stores/audioPreview.svelte";
  import { session } from "$lib/stores/session.svelte";
  import { tasks } from "$lib/stores/task.svelte";

  const form = session.media;

  const TABS = [
    { value: "audio", label: "Audio" },
    { value: "live", label: "Standar Live" },
    { value: "info", label: "Info Video" }
  ];

  const TAB_ACTIONS: Record<string, string | null> = {
    audio: "Proses audio",
    live: "Encode",
    info: null
  };

  const AUDIO_FORMATS = ["m4a", "mp3", "wav", "flac"];

  const inputClass =
    "w-full rounded-field border border-line bg-input px-3 py-2 text-sm text-text outline-none focus:border-accent";
  const labelClass = "block text-xs font-medium text-muted";

  let previewOpen = $state(false);
  let previewPath = $state<string | null>(null);
  let previewKind = $state<"audio" | "video">("video");
  let info = $state<VideoInfo | null>(null);
  let infoLoading = $state(false);
  let liveResult = $state<string>("");
  let liveDefaults = $state<LiveDefaults | null>(null);
  let encoders = $state<EncoderOption[]>([]);

  // Encoder yang akan dipakai (dari Pengaturan) beserta labelnya.
  const encoderValue = $derived(app.prefs.hw_encoder ?? "libx264");
  const encoderLabel = $derived(
    encoders.find((item) => item.value === encoderValue)?.label ?? encoderValue
  );
  const usingHardware = $derived(encoderValue !== "libx264");
  const activeBitrate = $derived(form.liveBitrateMbps ?? liveDefaults?.defaultMbps ?? 8);

  $effect(() => {
    void api.ffmpegEncoders().then((list) => (encoders = list));
  });

  $effect(() => {
    void api
      .mediaLiveDefaults()
      .then((defaults) => (liveDefaults = defaults))
      .catch(() => (liveDefaults = null));
  });

  const outputDir = $derived(
    app.project
      ? app.project.customOutputPath || `${app.project.rootPath}/outputs`
      : ""
  );

  const tabReady = $derived(
    (form.tab === "audio" && form.audioBase.length > 0) ||
      (form.tab === "live" && form.liveVideos.length > 0)
  );

  function showPreview(path: string, kind: "audio" | "video") {
    previewPath = path;
    previewKind = kind;
    previewOpen = true;
  }

  async function addVideos() {
    const paths = await pickMedia("video", true);
    if (paths.length > 0) form.liveVideos = [...form.liveVideos, ...paths];
  }

  function removeLiveVideo(index: number) {
    form.liveVideos = form.liveVideos.filter((_, position) => position !== index);
  }

  async function addBaseAudio() {
    const paths = await pickMedia("audio", true);
    if (paths.length > 0) {
      audioPreview.stop();
      form.audioBase = [...form.audioBase, ...paths];
    }
  }

  async function addOverlay() {
    const paths = await pickMedia("audio", true);
    if (paths.length > 0) {
      audioPreview.stop();
      form.audioOverlays = [
        ...form.audioOverlays,
        ...paths.map((path) => ({ path, volume: 1 }))
      ];
    }
  }

  function removeBaseAudio(index: number) {
    audioPreview.stop();
    form.audioBase = form.audioBase.filter((_, position) => position !== index);
  }

  function removeOverlay(index: number) {
    audioPreview.stop();
    form.audioOverlays = form.audioOverlays.filter((_, position) => position !== index);
  }

  /** Putar/hentikan satu trek audio utama (pemutaran selalu satu per satu). */
  function toggleBaseAudio(index: number, path: string) {
    if (audioPreview.isPlaying(`base:${index}`)) audioPreview.stop();
    else audioPreview.playBase(`base:${index}`, path);
  }

  /** Putar/hentikan satu overlay dengan volumenya sendiri. */
  function toggleOverlay(index: number, overlay: AudioOverlay) {
    if (audioPreview.isPlaying(`overlay:${index}`)) audioPreview.stop();
    else audioPreview.playOverlay(index, overlay.path, overlay.volume);
  }

  /** Putar/hentikan campuran: audio utama berurutan + overlay di atasnya. */
  function toggleMix() {
    if (audioPreview.isPlaying("mix")) audioPreview.stop();
    else audioPreview.playMix([...form.audioBase], [...form.audioOverlays]);
  }

  function changeTab(value: string) {
    // Berhenti begitu meninggalkan tab Audio supaya tidak ada yang berbunyi di latar.
    if (value !== "audio") audioPreview.stop();
    form.tab = value as typeof form.tab;
  }

  onDestroy(() => audioPreview.stop());

  async function loadInfo() {
    if (!form.infoVideo) return;
    infoLoading = true;
    try {
      info = await api.mediaInfo(form.infoVideo);
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    } finally {
      infoLoading = false;
    }
  }


  async function runAudio() {
    if (form.audioBase.length === 0) return;
    const format = form.audioFormat;
    const name = form.audioName.trim() || "audio_mix";
    const output = `${outputDir}/${name}_${timestampSlug()}.${format}`;
    await tasks.run(
      () => api.mediaAudioProcess([...form.audioBase], [...form.audioOverlays], output),
      {
        staging: [[0, "Menyiapkan"], [1, "Memproses audio"]],
        done: () => ({ label: baseName(output), path: output })
      }
    );
  }

  async function savePreset() {
    if (form.audioOverlays.length === 0) {
      app.showToast("Tambahkan overlay dulu.", "danger");
      return;
    }
    const output = `${outputDir}/preset_${timestampSlug()}.wav`;
    await tasks.run(
      () => api.mediaAudioExportPreset([...form.audioOverlays], output),
      {
        staging: [[0, "Menyimpan preset"]],
        done: () => ({ label: baseName(output), path: output })
      }
    );
  }


  async function runLive() {
    if (form.liveVideos.length === 0) return;
    const stamp = timestampSlug();
    let produced = 0;

    for (let index = 0; index < form.liveVideos.length; index += 1) {
      const video = form.liveVideos[index];
      const output = `${outputDir}/${fileStem(video)}_live_${stamp}_${index}.mp4`;
      const options: LiveStandardOptions = {
        videoPath: video,
        outputPath: output,
        bitrateMbps: form.liveBitrateMbps,
        hwEncoder: app.prefs.hw_encoder
      };
      const result = await tasks.run(() => api.mediaEncodeLive(options), {
        staging: [[0, `Meng-encode ${index + 1}/${form.liveVideos.length}`]],
        done: () => ({ label: baseName(output), path: output })
      });
      if (result) {
        produced += 1;
      }
    }
    liveResult = produced > 0 ? `Selesai: ${produced} file` : "Tidak ada yang dikerjakan.";
    app.showToast(liveResult, produced > 0 ? "success" : "danger");
  }

  async function runCurrent() {
    if (form.tab === "audio") await runAudio();
    else if (form.tab === "live") await runLive();
  }

  const metadataRows = $derived(
    info?.metadata
      ? [
          { label: "Codec:", value: info.metadata.codec ?? "tidak ada" },
          {
            label: "Resolusi:",
            value:
              info.metadata.width && info.metadata.height
                ? `${info.metadata.width}x${info.metadata.height}`
                : "tidak ada"
          },
          { label: "Frame rate:", value: info.metadata.fps ?? "tidak ada" },
          { label: "Format piksel:", value: info.metadata.pixFmt ?? "tidak ada" },
          {
            label: "Durasi:",
            value: info.metadata.duration ? `${info.metadata.duration.toFixed(1)} dtk` : "tidak ada"
          },
          { label: "Audio:", value: info.metadata.audioCodec ?? "tidak ada" }
        ]
      : []
  );

  const youtubeTone = $derived(
    info?.youtube?.status === "passed"
      ? "success"
      : info?.youtube?.status === "warning"
        ? "warning"
        : "danger"
  );
</script>

<PageHeader
  pretitle="Alat"
  title="Media Tools"
  subtitle="Olah audio, encode video ke standar siaran, dan periksa standar YouTube."
>
  {#snippet actions()}
    {#if TAB_ACTIONS[form.tab]}
      <button
        type="button"
        class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50"
        disabled={!tabReady || tasks.running}
        onclick={runCurrent}
      >
        {TAB_ACTIONS[form.tab]}
      </button>
    {/if}
  {/snippet}
</PageHeader>

<Segmented items={TABS} value={form.tab} onchange={changeTab} />

{#if form.tab === "audio"}
  <div class="mt-4 grid gap-4 lg:grid-cols-2">
    <Card title="Audio & overlay">
      {#snippet actions()}
        <button
          type="button"
          class="rounded-field border border-line px-2.5 py-1 text-xs text-text transition hover:bg-canvas"
          onclick={addBaseAudio}
        >
          Tambah audio utama
        </button>
        <button
          type="button"
          class="rounded-field border border-line px-2.5 py-1 text-xs text-text transition hover:bg-canvas"
          onclick={addOverlay}
        >
          Tambah overlay
        </button>
        {#if form.audioOverlays.length > 0 || form.audioBase.length > 0}
          <button
            type="button"
            class="rounded-field border px-2.5 py-1 text-xs font-medium transition {audioPreview.isPlaying(
              'mix'
            )
              ? 'border-accent bg-accent/10 text-accent'
              : 'border-line text-text hover:bg-canvas'}"
            title="Putar audio utama berurutan dengan overlay di atasnya"
            onclick={toggleMix}
          >
            {audioPreview.isPlaying("mix") ? "Hentikan" : "Putar"}
          </button>
        {/if}
      {/snippet}

      <p class="text-xs font-medium text-muted">Audio utama ({form.audioBase.length})</p>
      {#if form.audioBase.length === 0}
        <EmptyState title="Belum ada audio utama" subtitle="Tambahkan trek utama dulu." icon="♪" />
      {:else}
        <div class="mt-2 space-y-2">
          {#each form.audioBase as path, index (path + index)}
            {@const playing = audioPreview.isPlaying(`base:${index}`)}
            <FileRow
              {path}
              kind="audio"
              index={index + 1}
              {playing}
              progress={audioPreview.progress}
              onPreview={() => toggleBaseAudio(index, path)}
              onRemove={() => removeBaseAudio(index)}
            />
          {/each}
        </div>
      {/if}

      <p class="mt-4 text-xs font-medium text-muted">Overlay</p>
      <p class="text-[11px] text-faint">
        Overlay diputar di atas audio utama sesuai volume masing-masing — slidernya bisa digeser selagi
        berbunyi.
      </p>

      {#if audioPreview.error}
        <div class="mt-2 rounded-field border border-danger/30 bg-danger/10 px-3 py-2 text-xs">
          {audioPreview.error}
        </div>
      {/if}

      {#if form.audioOverlays.length === 0}
        <p class="py-2 text-xs text-faint">Belum ada overlay.</p>
      {:else}
        <div class="mt-2 space-y-2">
          {#each form.audioOverlays as overlay, index (overlay.path + index)}
            {@const playing = audioPreview.isPlaying(`overlay:${index}`)}
            <div class="rounded-field border border-line bg-canvas px-3 py-2">
              <div class="flex items-center justify-between gap-2">
                <p class="truncate text-sm text-text" title={overlay.path}>
                  {baseName(overlay.path)}
                </p>
                <div class="flex shrink-0 items-center gap-1">
                  <button
                    type="button"
                    class="rounded p-1 transition {playing
                      ? 'text-accent'
                      : 'text-muted hover:bg-surface hover:text-accent'}"
                    title={playing ? "Hentikan" : "Putar overlay"}
                    aria-label="{playing ? 'Hentikan' : 'Putar'} overlay {baseName(overlay.path)}"
                    onclick={() => toggleOverlay(index, overlay)}>{playing ? "■" : "▶"}</button
                  >
                  <button
                    type="button"
                    class="ml-1 rounded border-l border-line p-1 pl-2 text-muted transition hover:bg-surface hover:text-danger"
                    title="Hapus"
                    aria-label="Hapus overlay"
                    onclick={() => removeOverlay(index)}>✕</button
                  >
                </div>
              </div>
              <label class="mt-1 flex items-center gap-2 text-[11px] text-muted">
                Volume {Math.round(overlay.volume * 100)}%
                <input
                  type="range"
                  min="0"
                  max="100"
                  step="5"
                  class="flex-1"
                  value={Math.round(overlay.volume * 100)}
                  oninput={(event) => {
                    const value = Number((event.currentTarget as HTMLInputElement).value) / 100;
                    form.audioOverlays = form.audioOverlays.map((item, position) =>
                      position === index ? { ...item, volume: value } : item
                    );
                    // Ikut berubah kalau overlay ini sedang berbunyi.
                    audioPreview.syncVolumes(form.audioOverlays);
                  }}
                />
              </label>
              {#if playing}
                <div class="mt-1.5 h-0.5 overflow-hidden rounded-full bg-line-subtle">
                  <div
                    class="h-full rounded-full bg-accent transition-[width] duration-200"
                    style={`width:${Math.round(audioPreview.progress * 100)}%`}
                  ></div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </Card>

    <Card title="Opsi audio">
      <div class="space-y-3">
        <label class={labelClass}>
          Format output
          <select class={`${inputClass} mt-1`} bind:value={form.audioFormat}>
            {#each AUDIO_FORMATS as format (format)}
              <option value={format}>{format}</option>
            {/each}
          </select>
        </label>
        <label class={labelClass}>
          Nama output
          <input type="text" class={`${inputClass} mt-1`} bind:value={form.audioName} />
        </label>
        <button
          type="button"
          class="rounded-field border border-line px-3 py-1.5 text-xs text-text transition hover:bg-canvas disabled:opacity-50"
          disabled={form.audioOverlays.length === 0}
          onclick={savePreset}
        >
          Simpan preset
        </button>
      </div>
    </Card>
  </div>
{:else if form.tab === "live"}
  <div class="mt-4 grid gap-4 lg:grid-cols-2">
    <Card title="Daftar video">
      <DropZone
        label="Tambah video"
        hint="Di-encode satu per satu ke standar siaran"
        onPick={addVideos}
      />
      {#if form.liveVideos.length === 0}
        <EmptyState
          title="Belum ada video"
          subtitle="Tambahkan klip untuk di-encode ke standar siaran."
          icon="▭"
        />
      {:else}
        <div class="mt-3 space-y-2">
          {#each form.liveVideos as path, index (path + index)}
            <FileRow
              {path}
              kind="video"
              index={index + 1}
              onPreview={() => showPreview(path, "video")}
              onRemove={() => removeLiveVideo(index)}
            />
          {/each}
        </div>
      {/if}
    </Card>

    <Card title="Opsi siaran" subtitle="Resolusi dan frame rate mengikuti tiap video.">
      <div class="space-y-3">
        <label class={labelClass}>
          Bitrate CBR (Mbps)
          <div class="mt-1 flex items-center gap-2">
            <input
              type="number"
              min={liveDefaults?.minMbps ?? 3}
              step="0.5"
              class={inputClass}
              value={form.liveBitrateMbps ?? liveDefaults?.defaultMbps ?? ""}
              onchange={(event) => {
                const value = Number((event.currentTarget as HTMLInputElement).value);
                if (!Number.isFinite(value) || value <= 0) {
                  form.liveBitrateMbps = null;
                } else {
                  // Batas bawah dipegang di sini supaya nilai tak masuk akal
                  // tidak sampai ke encoder.
                  form.liveBitrateMbps = Math.max(value, liveDefaults?.minMbps ?? 3);
                }
              }}
            />
            {#if form.liveBitrateMbps !== null}
              <button
                type="button"
                class="shrink-0 rounded-field border border-line px-2.5 py-1 text-[11px] text-text transition hover:bg-canvas"
                title="Kembali ke bitrate bawaan"
                onclick={() => (form.liveBitrateMbps = null)}
              >
                Bawaan
              </button>
            {/if}
          </div>
          {#if liveDefaults}
            <span class="mt-1 block text-[11px] text-faint">
              Bawaan {liveDefaults.defaultMbps} Mbps · minimum {liveDefaults.minMbps} Mbps
            </span>
          {/if}
        </label>

        <div class="rounded-field border border-line bg-canvas px-3 py-2 text-[11px] text-muted">
          <p class="font-medium text-text">Hasil encoding</p>
          <p class="mt-0.5">Encoder: {encoderLabel}</p>
          <p>CBR {activeBitrate} Mbps</p>
          <p>Keyframe tiap {liveDefaults?.keyframeSeconds ?? 2} detik</p>
          {#if usingHardware}
            <p>Parameter lain (B-frame, GOP tertutup, Rec.709) sebisanya</p>
          {:else}
            <p>H.264 High, 2 B-frame, 1 reference frame, closed GOP, Rec.709</p>
          {/if}
        </div>

        {#if activeBitrate < (liveDefaults?.minMbps ?? 3)}
          <p class="text-[11px] text-warning">
            Bitrate di bawah minimum yang diizinkan ({liveDefaults?.minMbps ?? 3} Mbps).
          </p>
        {/if}

        {#if usingHardware}
          <p class="text-[11px] text-warning">
            Encoder hardware tidak selalu memegang target CBR dan kerapian keyframe. Periksa
            hasilnya di tab Info Video.
          </p>
        {:else}
          <p class="text-[11px] text-faint">
            Mode ini memakai libx264 supaya semua parameter di atas dijamin sesuai standar siaran.
          </p>
        {/if}

        {#if liveResult}
          <p class="text-xs text-muted">{liveResult}</p>
        {/if}
      </div>
    </Card>
  </div>
{:else}
  <div class="mt-4 grid gap-4 lg:grid-cols-2">
    <Card title="Video">
      <SingleSlot
        label="Video"
        path={form.infoVideo}
        kind="video"
        emptyHint="Pilih satu video untuk diperiksa"
        onPick={async () => {
          const [path] = await pickMedia("video");
          if (path) {
            form.infoVideo = path;
            info = null;
            await loadInfo();
          }
        }}
        onClear={() => {
          form.infoVideo = null;
          info = null;
        }}
        onPreview={() => form.infoVideo && showPreview(form.infoVideo, "video")}
      />
    </Card>

    <Card title="Detail video">
      {#if infoLoading}
        <p class="text-sm text-muted">Memeriksa video...</p>
      {:else if info && info.metadata}
        <DataGrid rows={metadataRows} />
        {#if info.youtube}
          <div class="mt-3 border-t border-line pt-3">
            <div class="flex items-center gap-2">
              <span class="text-xs text-muted">Standar YouTube:</span>
              <StatusBadge text={info.youtube.status} tone={youtubeTone} />
            </div>
            {#if info.youtube.issues.length > 0}
              <ul class="mt-2 list-disc space-y-0.5 pl-5 text-xs text-muted">
                {#each info.youtube.issues as issue (issue)}
                  <li>{issue}</li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      {:else}
        <EmptyState
          title="Belum ada detail"
          subtitle="Pilih video untuk melihat codec, resolusi, dan standar YouTube."
          icon="▤"
        />
      {/if}
    </Card>
  </div>
{/if}

<PreviewModal bind:open={previewOpen} path={previewPath} kind={previewKind} />
