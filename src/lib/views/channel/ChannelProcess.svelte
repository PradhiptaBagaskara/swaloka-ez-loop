<script lang="ts">
  import { onDestroy } from "svelte";
  import { Modal } from "flowbite-svelte";
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

  import {
    api,
    errorMessage,
    type Channel,
    type ChannelAudioItem,
    type ChannelPlan,
    type ChannelPreflight
  } from "$lib/api";
  import FolderSlot from "$lib/components/channel/FolderSlot.svelte";
  import PlanCard from "$lib/components/channel/PlanCard.svelte";
  import StageCard from "$lib/components/channel/StageCard.svelte";
  import PreviewModal from "$lib/components/PreviewModal.svelte";
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import PageHeader from "$lib/components/ui/PageHeader.svelte";
  import { formatBytes, formatDuration } from "$lib/format";
  import { pickDirectory, pickMedia } from "$lib/picker";
  import { app } from "$lib/stores/app.svelte";
  import { audioPreview } from "$lib/stores/audioPreview.svelte";
  import { session } from "$lib/stores/session.svelte";
  import { tasks } from "$lib/stores/task.svelte";

  const form = session.channel;

  const inputClass =
    "w-full rounded-field border border-line bg-input px-3 py-2 text-sm text-text outline-none focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent/40";
  const labelClass = "block text-xs font-medium text-muted";
  const hintClass = "mt-1 block text-[11px] text-faint";
  const primaryBtn =
    "rounded-field bg-accent px-3 py-1.5 text-xs font-medium text-white transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-50";
  const secondaryBtn =
    "rounded-field border border-line px-2.5 py-1.5 text-xs text-text transition hover:bg-canvas focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-50";

  let channel = $state<Channel | null>(null);
  let loading = $state(true);
  let preflight = $state<ChannelPreflight | null>(null);
  /** Penjaga internal (tidak ditampilkan), jadi cukup variabel biasa. */
  let preflightBusy = false;
  /** Pengaturan aset berubah setelah rencana dibuat. */
  let settingsChanged = $state(false);
  let confirmOpen = $state(false);
  let previewOpen = $state(false);
  let previewPath = $state<string | null>(null);

  const plans = $derived(channel?.plans ?? []);
  const selectedPlans = $derived(plans.filter((plan) => plan.selected));
  /** Folder video sudah dipilih: syarat tombol periksa ulang folder. */
  const videoFolderReady = $derived(Boolean(channel?.videoFolderPath));
  const foldersReady = $derived(
    Boolean(channel?.audioFolderPath) && Boolean(channel?.videoFolderPath)
  );
  const audioIndex = $derived(
    new Map((channel?.audioItems ?? []).map((item) => [item.id, item] as const))
  );
  const videoCount = $derived(channel?.videoItems.length ?? 0);
  const audioSeconds = $derived(
    (channel?.audioItems ?? []).reduce((total, item) => total + (item.durationSeconds ?? 0), 0)
  );
  const selectedSeconds = $derived(
    selectedPlans.reduce((total, plan) => total + plan.totalDurationSeconds, 0)
  );
  const planSeconds = $derived(
    plans.reduce((total, plan) => total + plan.totalDurationSeconds, 0)
  );
  /** Jumlah rencana yang diminta; 0 berarti mengikuti jumlah video. */
  const target = $derived(channel?.totalContent ? channel.totalContent : videoCount);
  /** Rencana yang belum kebagian video latar. */
  const waiting = $derived(plans.filter((plan) => !plan.backgroundVideoPath).length);
  /** Rencana yang benar-benar bisa diproses sekarang. */
  const runnable = $derived(plans.filter((plan) => Boolean(plan.backgroundVideoPath)));
  const selectedRunnable = $derived(
    plans.filter((plan) => plan.selected && Boolean(plan.backgroundVideoPath))
  );
  /** Berapa video dari folder channel yang sudah dipakai plan. */
  const usedVideos = $derived(
    (channel?.videoItems ?? []).filter((video) =>
      plans.some((plan) => plan.backgroundVideoPath === video)
    ).length
  );

  const shortfall = $derived(plans.length > 0 && plans.length < target ? target - plans.length : 0);
  const outputRoot = $derived(
    app.project ? app.project.customOutputPath || `${app.project.rootPath}/outputs` : ""
  );
  const canPlan = $derived(foldersReady && !tasks.running);
  const canRun = $derived(
    Boolean(channel) && selectedRunnable.length > 0 && !tasks.running && (preflight?.fits ?? true)
  );

  const stageOne = $derived(
    foldersReady
      ? { text: "Siap", tone: "success" as const }
      : { text: "Folder belum lengkap", tone: "gray" as const }
  );
  const stageTwo = $derived(
    plans.length > 0
      ? { text: `${plans.length} plan`, tone: "success" as const }
      : { text: "Belum dibuat", tone: "gray" as const }
  );
  const stageThree = $derived(
    plans.length === 0
      ? { text: "Menunggu plan", tone: "gray" as const }
      : selectedPlans.length > 0
        ? { text: `${selectedPlans.length} dari ${plans.length} dipilih`, tone: "primary" as const }
        : { text: "Tidak ada yang dipilih", tone: "warning" as const }
  );

  $effect(() => {
    const id = form.channelId;
    // Halaman tetap dibuka walau ada proses lain yang berjalan; `tasks.running`
    // hanya jadi pemicu supaya data disegarkan lagi begitu proses selesai.
    void tasks.running;
    if (!id) return;
    void load(id);
  });

  // Perkiraan disk ikut berubah saat pilihan, intro, atau aset berubah.
  // `refreshPreflight` hanya menulis `preflight` (bukan dependensi efek ini) dan
  // penjaga `preflightBusy` yang non-reaktif, jadi efeknya tidak berputar.
  $effect(() => {
    const current = channel;
    const picked = current ? current.plans.filter((plan) => plan.selected).length : 0;
    if (!current || picked === 0) {
      preflight = null;
      return;
    }
    void refreshPreflight(current);
  });

  onDestroy(() => audioPreview.stop());

  async function load(id: string) {
    try {
      channel = await api.channelLoad(id);
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    } finally {
      loading = false;
    }
  }

  async function save(value: Channel) {
    channel = value;
    try {
      channel = await api.channelSave(value);
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
      if (form.channelId) void load(form.channelId);
    }
  }

  function patch(change: Partial<Channel>) {
    if (!channel) return;
    void save({ ...channel, ...change });
  }

  function withPlan(value: Channel, planId: string, change: Partial<ChannelPlan>): Channel {
    return {
      ...value,
      plans: value.plans.map((plan) => (plan.id === planId ? { ...plan, ...change } : plan))
    };
  }

  function songsOf(plan: ChannelPlan): ChannelAudioItem[] {
    return plan.audioItemIds
      .map((id) => audioIndex.get(id))
      .filter((item): item is ChannelAudioItem => Boolean(item));
  }

  function backToList() {
    audioPreview.stop();
    form.channelId = null;
  }

  async function pickAudioFolder() {
    const path = await pickDirectory("Pilih folder audio");
    if (!path || !channel) return;
    settingsChanged = plans.length > 0;
    patch({ audioFolderPath: path });
  }

  async function pickVideoFolder() {
    const path = await pickDirectory("Pilih folder video");
    if (!path || !channel) return;
    settingsChanged = plans.length > 0;
    patch({ videoFolderPath: path });
  }

  async function pickOutputFolder() {
    const path = await pickDirectory("Pilih folder output");
    if (!path) return;
    patch({ outputDirectoryPath: path });
  }

  async function planChannel() {
    const current = channel;
    if (!current) return;
    confirmOpen = false;
    const planned = await tasks.run(() => api.channelPlan(current), {
      staging: [
        [0, "Memindai folder aset..."],
        [0.3, "Membaca durasi lagu..."],
        [0.6, "Menyusun plan seimbang..."],
        [1, "Plan siap"]
      ],
      done: (value) => ({ label: `${(value as Channel).plans.length} plan` })
    });
    if (planned) {
      channel = planned as Channel;
      settingsChanged = false;
      preflight = null;
      app.showToast("Plan dibuat", "success");
    }
  }

  async function selectAll(selectedValue: boolean) {
    if (!channel) return;
    void save({
      ...channel,
      plans: channel.plans.map((plan) => ({
        ...plan,
        // Rencana yang belum punya video latar belum bisa diproses.
        selected: selectedValue && Boolean(plan.backgroundVideoPath)
      }))
    });
  }

  /** Pindai ulang folder video lalu pasangkan video baru ke rencana yang menunggu. */
  async function rescanVideos() {
    if (!channel) return;
    try {
      const refreshed = await api.channelRescanVideos(channel);
      const stillWaiting = refreshed.plans.filter((plan) => !plan.backgroundVideoPath).length;
      const assigned = refreshed.plans.filter(
        (plan) => Boolean(plan.backgroundVideoPath)
      ).length - plans.filter((plan) => Boolean(plan.backgroundVideoPath)).length;
      channel = refreshed;
      app.showToast(
        assigned > 0
          ? `${assigned} plan kebagian video`
          : stillWaiting > 0
            ? "Tidak ada video baru di folder"
            : "Semua plan sudah punya video",
        assigned > 0 ? "success" : "danger"
      );
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    }
  }

  /** Pasang atau ganti video pembuka/latar satu rencana. */
  async function pickVideo(plan: ChannelPlan, kind: "intro" | "background") {
    if (!channel) return;
    const [path] = await pickMedia("video");
    if (!path) return;
    const change =
      kind === "intro" ? { introVideoPath: path } : { backgroundVideoPath: path };
    void save(withPlan(channel, plan.id, change));
  }

  /** Buang satu rencana, lalu nomori ulang supaya urutannya tetap rapat. */
  async function removePlan(plan: ChannelPlan) {
    if (!channel) return;
    const remaining = channel.plans
      .filter((item) => item.id !== plan.id)
      .map((item, index) => ({ ...item, index }));
    void save({
      ...channel,
      plans: remaining,
      totalContent: Math.max(1, remaining.length)
    });
  }

  function showPreview(path: string | null) {
    if (!path) return;
    previewPath = path;
    previewOpen = true;
  }

  /**
   * Pratinjau hasil: default buka pemutar bawaan; kalau tak ada pemutar bawaan,
   * baru tampilkan modal pratinjau (dan modal itu punya fallback penjelasan).
   */
  async function previewOutput(plan: ChannelPlan) {
    if (!plan.outputPath) return;
    try {
      await openPath(plan.outputPath);
    } catch {
      previewPath = plan.outputPath;
      previewOpen = true;
    }
  }

  /** Tunjuk berkas hasil di Finder, dengan berkasnya tersorot. */
  async function revealOutput(plan: ChannelPlan) {
    if (!plan.outputPath) return;
    try {
      await revealItemInDir(plan.outputPath);
    } catch {
      app.showToast("Tidak bisa membuka folder hasil", "danger");
    }
  }

  function toggleSong(planId: string, index: number, path: string) {
    const key = `plan:${planId}:${index}`;
    if (audioPreview.isPlaying(key)) audioPreview.stop();
    else audioPreview.playBase(key, path);
  }

  async function refreshPreflight(value: Channel) {
    if (preflightBusy || tasks.running) return;
    preflightBusy = true;
    try {
      preflight = await api.channelPreflight(value);
    } catch {
      preflight = null;
    } finally {
      preflightBusy = false;
    }
  }

  async function runSelection() {
    if (!channel) return;
    const current = channel;
    await refreshPreflight(current);
    // Ruang yang kurang adalah penghalang: angka butuh/tersedia sudah tampil.
    if (preflight && !preflight.fits) return;
    const result = await tasks.run(() => api.channelRun(current), {
      staging: [
        [0, "Menyiapkan batch..."],
        [0.05, "Menggabungkan plan terpilih..."],
        [1, "Batch selesai"]
      ],
      done: (value) => {
        const finished = value as Channel;
        const done = finished.plans.filter((plan) => plan.status === "done").length;
        const failed = finished.plans.filter((plan) => plan.status === "failed").length;
        const last = [...finished.plans].reverse().find((plan) => plan.outputPath)?.outputPath;
        return { label: `${done} berhasil · ${failed} gagal`, path: last ?? undefined };
      }
    });
    if (result) {
      channel = result as Channel;
      settingsChanged = false;
    }
    if (form.channelId) await load(form.channelId);
  }

  async function cancelSelection() {
    try {
      await api.channelCancel();
    } catch {
      // Backend tetap menghentikan prosesnya sendiri.
    }
  }
</script>

<button
  type="button"
  class="mb-2 inline-flex items-center gap-1 text-xs font-medium text-muted transition hover:text-accent focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
  onclick={backToList}
>
  <span aria-hidden="true">←</span> Semua channel
</button>

{#if loading}
  <p class="text-sm text-muted">Memuat channel…</p>
{:else if !channel}
  <EmptyState
    title="Channel tidak ditemukan"
    subtitle="Channel ini mungkin sudah dihapus. Kembali ke daftar untuk memilih yang lain."
    icon="▣"
    actionLabel="Kembali ke daftar"
    onAction={backToList}
  />
{:else}
  <PageHeader
    pretitle="Proses channel"
    title={channel.name}
    subtitle={`${channel.audioItems.length} lagu · ${videoCount} video · ${plans.length} plan`}
  >
    {#snippet actions()}
      <button
        type="button"
        class={primaryBtn}
        disabled={!canPlan}
        onclick={() => {
          if (plans.length > 0) {
            confirmOpen = true;
          } else {
            void planChannel();
          }
        }}
      >
        {plans.length > 0 ? "Buat ulang plan" : "Buat plan"}
      </button>
    {/snippet}
  </PageHeader>

  <div class="space-y-4">
    <StageCard
      index={1}
      title="Sumber"
      subtitle="Folder lagu, folder video, jumlah konten, dan lama tiap konten"
      state={stageOne.text}
      tone={stageOne.tone}
    >
      <div class="grid gap-3 md:grid-cols-2">
        <label class={labelClass}>
          Nama channel
          <input
            type="text"
            class={`${inputClass} mt-1`}
            value={channel.name}
            onchange={(event) => patch({ name: event.currentTarget.value.trim() || channel!.name })}
          />
        </label>
        <label class={labelClass}>
          Durasi minimum tiap konten (menit)
          <input
            type="number"
            min="1"
            step="5"
            class={`${inputClass} mt-1`}
            value={channel.minDurationMinutes}
            onchange={(event) => {
              const value = Number(event.currentTarget.value);
              if (Number.isFinite(value) && value > 0) {
                settingsChanged = plans.length > 0;
                patch({ minDurationMinutes: value });
              }
            }}
          />
        </label>

        <FolderSlot
          label="Folder audio"
          path={channel.audioFolderPath}
          icon="♪"
          emptyHint="Folder berisi lagu channel"
          onPick={pickAudioFolder}
          onClear={() => {
            settingsChanged = plans.length > 0;
            patch({ audioFolderPath: "" });
          }}
        >
          {#if channel.audioItems.length > 0}
            {channel.audioItems.length} lagu · total {formatDuration(audioSeconds)}
          {:else}
            Jumlah lagu dihitung saat plan dibuat
          {/if}
        </FolderSlot>
        <FolderSlot
          label="Folder video"
          path={channel.videoFolderPath}
          icon="▭"
          emptyHint="Folder berisi video latar"
          onPick={pickVideoFolder}
          onClear={() => {
            settingsChanged = plans.length > 0;
            patch({ videoFolderPath: "" });
          }}
        >
          {#if videoCount > 0}
            {videoCount} video · tiap video jadi satu plan
          {:else}
            Satu video dipakai untuk satu plan
          {/if}
        </FolderSlot>

        <label class={labelClass}>
          Total konten yang ingin dibuat
          <input
            type="number"
            min="1"
            step="1"
            class={`${inputClass} mt-1`}
            value={channel.totalContent > 0 ? channel.totalContent : ""}
            placeholder={videoCount > 0 ? String(videoCount) : "mengikuti jumlah video"}
            onchange={(event) => {
              const value = Number(event.currentTarget.value);
              settingsChanged = plans.length > 0;
              patch({ totalContent: Number.isFinite(value) && value > 0 ? Math.trunc(value) : 0 });
            }}
          />
          <span class={hintClass}>
            {#if videoCount === 0}
              Isi jumlah konten yang ingin dibuat; tiap plan butuh satu video latar.
            {:else if target > videoCount}
              {target} plan dibuat; {videoCount} bisa diproses sekarang, {Math.max(0, target - videoCount)} menunggu video.
            {:else}
              Kosongkan untuk mengikuti jumlah video ({videoCount} plan).
            {/if}
          </span>
        </label>
        <label class={labelClass}>
          Target putar per lagu
          <input
            type="number"
            min="1"
            step="1"
            class={`${inputClass} mt-1`}
            value={channel.suggestedLoopCap}
            onchange={(event) => {
              const value = Number(event.currentTarget.value);
              if (Number.isFinite(value) && value > 0) {
                settingsChanged = plans.length > 0;
                patch({ suggestedLoopCap: Math.trunc(value) });
              }
            }}
          />
          <span class={hintClass}>
            Kolam lagu diisi seluruh perpustakaan sebanyak {channel.suggestedLoopCap}×, lalu
            diambil sampai konten mencapai {channel.minDurationMinutes} menit. Lagu pembuka tiap
            konten diambil dari antrean supaya berbeda-beda.
          </span>
        </label>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2 rounded-field border border-line bg-canvas px-3 py-2">
        <span class="text-[11px] font-medium text-muted">Output</span>
        <span
          class="min-w-0 flex-1 truncate text-[11px] text-text"
          title={channel.outputDirectoryPath}
        >
          {channel.outputDirectoryPath}/{channel.name}
        </span>
        <button type="button" class={secondaryBtn} onclick={pickOutputFolder}>Ubah</button>
        {#if outputRoot && channel.outputDirectoryPath !== outputRoot}
          <button
            type="button"
            class={secondaryBtn}
            onclick={() => patch({ outputDirectoryPath: outputRoot })}
          >
            Pakai bawaan
          </button>
        {/if}
      </div>
    </StageCard>

    <StageCard
      index={2}
      title="Plan"
      subtitle="Susunan konten dari lagu yang terdeteksi"
      state={stageTwo.text}
      tone={stageTwo.tone}
    >
      {#snippet actions()}
        <button
          type="button"
          class="cursor-pointer rounded-field bg-accent px-2.5 py-1.5 text-xs font-medium text-white transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-50"
          disabled={!videoFolderReady}
          onclick={rescanVideos}
        >
          Periksa ulang folder
        </button>
      {/snippet}
      {#if !foldersReady}
        <p class="text-xs text-muted">
          Pilih folder audio dan folder video dulu di tahap 1. Aplikasi akan membuat satu plan
          untuk tiap video, dengan playlist yang durasinya merata dan tidak kembar.
        </p>
      {:else if plans.length === 0}
        <p class="text-xs text-muted">
          Siap menyusun {target > 0 ? `${target} plan` : "plan"} dari {channel.audioItems
            .length > 0
            ? `${channel.audioItems.length} lagu`
            : "folder audio"}, masing-masing minimal {channel.minDurationMinutes} menit dengan target
          putar {channel.suggestedLoopCap}× per lagu.
        </p>
      {:else}
        <dl class="grid gap-2 text-xs sm:grid-cols-3">
          <div>
            <dt class="text-muted">Plan</dt>
            <dd class="text-sm text-text">{plans.length}</dd>
          </div>
          <div>
            <dt class="text-muted">Rata-rata durasi</dt>
            <dd class="text-sm text-text">{formatDuration(planSeconds / plans.length)}</dd>
          </div>
          <div>
            <dt class="text-muted">Total durasi</dt>
            <dd class="text-sm text-text">{formatDuration(planSeconds)}</dd>
          </div>
        </dl>

        {#if shortfall > 0}
          <p
            class="mt-3 rounded-field border border-warning/30 bg-warning/10 px-2.5 py-2 text-[11px] text-warning"
          >
            Hanya {plans.length} dari {target} plan yang bisa dibuat karena variasi lagunya belum
            cukup ({shortfall} plan belum terbentuk). Tambah lagu, lalu buat ulang.
          </p>
        {/if}

        {#if settingsChanged}
          <p
            class="mt-3 rounded-field border border-warning/30 bg-warning/10 px-2.5 py-2 text-[11px] text-warning"
          >
            Folder, jumlah konten, atau target putar berubah setelah plan dibuat. Buat ulang agar
            plan mengikuti pengaturan terbaru.
          </p>
        {/if}

      <p class="mt-3 text-[11px] text-muted">
        {usedVideos} video latar dipakai dari folder
      </p>
      {/if}
    </StageCard>

    <StageCard
      index={3}
      title="Tinjau & pilih"
      subtitle="Tiap plan menampilkan video pembuka & latar. Klik kotak video untuk pratinjau, atau pasang/ganti videonya."
      state={stageThree.text}
      tone={stageThree.tone}
    >
      {#snippet actions()}
        {#if tasks.running}
          <button
            type="button"
            class="cursor-pointer rounded-field border border-danger/40 bg-danger/10 px-3 py-1.5 text-xs font-medium text-danger transition hover:bg-danger/20 focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
            onclick={cancelSelection}
          >
            Batalkan proses
          </button>
        {/if}
        <button type="button" class={primaryBtn} disabled={!canRun} onclick={runSelection}>
          Proses {selectedRunnable.length} plan terpilih
        </button>
      {/snippet}

      {#if plans.length === 0}
        <p class="text-xs text-muted">Belum ada plan untuk ditinjau.</p>
      {:else}
        {#if waiting > 0}
          <p
            class="mb-3 rounded-field border border-warning/30 bg-warning/10 px-2.5 py-2 text-[11px] text-warning"
          >
            {waiting} plan belum punya video latar. Tambahkan video ke folder lalu tekan "Periksa
            ulang folder", atau klik kotak "Video latar" pada plannya untuk memasang.
          </p>
        {/if}
        <div class="space-y-2">
          <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
            <p class="min-w-0 flex-1 text-xs text-muted">
              <span class="font-medium text-text">{selectedRunnable.length}</span> dari {runnable.length}
              plan siap dipilih
              {#if waiting > 0}
                · {waiting} menunggu video
              {/if}
              {#if selectedRunnable.length > 0}
                · total {formatDuration(selectedSeconds)}
              {/if}
            </p>
            <div class="ml-auto flex shrink-0 items-center gap-2">
              <button type="button" class={secondaryBtn} onclick={() => selectAll(true)}>
                Pilih semua
              </button>
              <button type="button" class={secondaryBtn} onclick={() => selectAll(false)}>
                Kosongkan
              </button>
            </div>
          </div>

          {#if selectedRunnable.length === 0}
            <p class="text-xs text-muted">
              {#if waiting > 0}
                Belum ada plan siap diproses: {waiting} plan masih menunggu video latar.
              {:else}
                Pilih minimal satu plan di bawah sebelum menjalankan.
              {/if}
            </p>
          {:else if preflight}
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
              <span class="text-muted"
                >Butuh <span class="font-medium text-text"
                  >{formatBytes(preflight.requiredBytes)}</span
                ></span
              >
              {#if preflight.availableBytes !== null}
                <span class="text-muted"
                  >Tersisa <span class="font-medium text-text"
                    >{formatBytes(preflight.availableBytes)}</span
                  ></span
                >
              {:else}
                <span class="text-muted">Ruang tersisa tidak terbaca</span>
              {/if}
              <span class="text-muted">
                · {preflight.planCount} plan · {formatDuration(preflight.totalSeconds)}
              </span>
            </div>

            {#if !preflight.fits}
              <p
                class="rounded-field border border-warning/30 bg-warning/10 px-2.5 py-2 text-[11px] text-warning"
              >
                Lokasi output tidak cukup menampung batch ini. Kurangi pilihan di tahap 3, atau
                pindahkan output ke drive lain.
              </p>
            {:else if preflight.spaceUnknown}
              <p class="rounded-field border border-line bg-canvas px-2.5 py-2 text-[11px] text-muted">
                Ruang bebas di lokasi output tidak bisa dibaca, jadi kecukupannya tidak bisa
                dipastikan. Proses tetap boleh dijalankan.
              </p>
            {/if}
          {:else}
            <p class="text-xs text-muted">Menghitung perkiraan kebutuhan disk…</p>
          {/if}
        </div>

        <div class="mt-3 grid gap-2 xl:grid-cols-2">
          {#each plans as plan (plan.id)}
            {@const songs = songsOf(plan)}
            <PlanCard
              {plan}
              {songs}
              missing={plan.audioItemIds.length - songs.length}
              onUpdate={(change) => save(withPlan(channel!, plan.id, change))}
              onPlaySong={(index, path) => toggleSong(plan.id, index, path)}
              onPreview={(path) => showPreview(path)}
              onPreviewOutput={() => previewOutput(plan)}
              onPickVideo={(kind) => pickVideo(plan, kind)}
              onOpenOutput={() => revealOutput(plan)}
              onRemovePlan={() => removePlan(plan)}
            />
          {/each}
        </div>
      {/if}
    </StageCard>
  </div>
{/if}

<PreviewModal bind:open={previewOpen} path={previewPath} kind="video" />

<Modal bind:open={confirmOpen} title="Buat ulang plan" size="md">
  <p class="text-sm text-text">
    Plan lama ({plans.length}) beserta status pengerjaannya akan diganti dengan susunan yang baru.
  </p>
  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <button
        type="button"
        class="rounded-field border border-line px-4 py-2 text-sm text-text transition hover:bg-canvas focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
        onclick={() => (confirmOpen = false)}
      >
        Batal
      </button>
      <button
        type="button"
        class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
        onclick={planChannel}
      >
        Buat ulang
      </button>
    </div>
  {/snippet}
</Modal>
