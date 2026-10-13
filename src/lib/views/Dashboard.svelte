<script lang="ts">
  import { api, type ProjectStats } from "$lib/api";
  import Card from "$lib/components/ui/Card.svelte";
  import PageHeader from "$lib/components/ui/PageHeader.svelte";
  import StatCard from "$lib/components/ui/StatCard.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { tasks } from "$lib/stores/task.svelte";

  let stats = $state<ProjectStats | null>(null);

  // Muat ulang statistik saat halaman dibuka atau setelah sebuah run selesai.
  $effect(() => {
    void app.view;
    void tasks.running;
    void app.project?.customOutputPath;
    void api.projectStats().then((value) => (stats = value));
  });

  const quickStart = [
    {
      key: "looping" as const,
      title: "Looping",
      text: "Gabungkan video latar dengan playlist audio.",
      icon: "⟳"
    },
    {
      key: "media" as const,
      title: "Media Tools",
      text: "Encode video ke standar siaran YouTube.",
      icon: "✂"
    }
  ];
</script>

<PageHeader pretitle="Ruang Kerja" title="Dasbor" subtitle={`Ringkasan proyek ${app.project?.name ?? ""}.`}>
  {#snippet actions()}
    <button
      type="button"
      class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90"
      onclick={() => (app.view = "looping")}
    >
      Buat video
    </button>
  {/snippet}
</PageHeader>

<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
  <StatCard
    label="Output selesai"
    value={stats?.outputCount ?? 0}
    icon="▤"
    hint="File di folder output"
  />
  <StatCard
    label="File cache"
    value={stats?.cacheCount ?? 0}
    icon="⌫"
    tone="gray"
    hint="Audio ternormalisasi"
  />
  <StatCard
    label="FFmpeg"
    value={stats?.ffmpegReady ? "Siap" : "Bermasalah"}
    icon={stats?.ffmpegReady ? "✓" : "!"}
    tone={stats?.ffmpegReady ? "success" : "danger"}
    hint={stats?.ffmpegReady ? "FFmpeg terdeteksi" : "Periksa panduan instalasi"}
  />
</div>

<div class="mt-4 grid gap-4 sm:grid-cols-2">
  {#each quickStart as item (item.key)}
    <Card>
      <p class="text-lg" aria-hidden="true">{item.icon}</p>
      <h3 class="mt-2 text-[15px] font-semibold text-text">{item.title}</h3>
      <p class="mt-1 text-xs text-muted">{item.text}</p>
      <button
        type="button"
        class="mt-3 rounded-field border border-line px-3 py-1.5 text-xs font-medium text-text transition hover:bg-canvas"
        onclick={() => (app.view = item.key)}
      >
        Buka
      </button>
    </Card>
  {/each}
</div>
