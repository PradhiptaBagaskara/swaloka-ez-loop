<script lang="ts">
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import { api } from "$lib/api";
  import { formatDuration } from "$lib/format";
  import { tasks } from "$lib/stores/task.svelte";

  const levelIcon: Record<string, string> = {
    info: "·",
    success: "✓",
    warning: "!",
    error: "✗"
  };
  const levelClass: Record<string, string> = {
    info: "text-muted",
    success: "text-success",
    warning: "text-warning",
    error: "text-danger"
  };

  let logBox = $state<HTMLDivElement | null>(null);

  // Area log hanya bisa dibuka kalau memang ada log yang bisa dibaca.
  const hasLogs = $derived(tasks.logs.length > 0);
  const expanded = $derived(!tasks.collapsed && hasLogs);

  // Saat log disembunyikan, yang tersisa hanya bar progres + durasi.
  const showTiming = $derived(
    tasks.running || Boolean(tasks.result) || Boolean(tasks.error)
  );
  const percent = $derived(Math.round(tasks.progress * 100));

  const dotClass = $derived(
    tasks.error
      ? "bg-danger"
      : tasks.result
        ? "bg-success"
        : tasks.running
          ? "animate-pulse bg-accent"
          : "bg-muted"
  );

  const label = $derived(
    tasks.error
      ? `Gagal: ${tasks.error}`
      : tasks.result
        ? `Selesai: ${tasks.result.label}`
        : tasks.running
          ? tasks.stage
          : "Belum ada proses"
  );

  const labelClass = $derived(
    tasks.error ? "text-danger" : tasks.running || tasks.result ? "text-text" : "text-muted"
  );

  const elapsed = $derived(formatDuration(tasks.elapsedSeconds));

  // Gulir otomatis ke baris log terbaru.
  $effect(() => {
    void tasks.logs.length;
    if (logBox && expanded) {
      logBox.scrollTop = logBox.scrollHeight;
    }
  });

  async function openOutput(path: string) {
    try {
      await revealItemInDir(path);
    } catch {
      // Best-effort: membuka folder bukan hal kritis.
    }
  }

  // Pembatalan duduk di sini karena hanya relevan selama ada proses berjalan.
  async function cancelRun() {
    try {
      await api.cancelRun();
    } catch {
      // Best-effort: backend tetap menghentikan prosesnya sendiri.
    }
  }
</script>

<section class="shrink-0 border-t border-line bg-surface">
  <div class="mx-auto flex h-10 w-full max-w-[1100px] items-center gap-3 px-5">
    <span class="size-2 shrink-0 rounded-full {dotClass}" aria-hidden="true"></span>

    <p class="min-w-0 flex-1 truncate text-xs font-medium {labelClass}" title={label}>
      {label}
    </p>

    {#if showTiming}
      <div class="w-40 shrink-0">
        <div class="h-1.5 overflow-hidden rounded-full bg-input">
          <div
            class="h-full rounded-full transition-[width] duration-200 {tasks.error
              ? 'bg-danger'
              : 'bg-accent'}"
            style={`width:${percent}%`}
          ></div>
        </div>
      </div>
      <span class="w-9 shrink-0 text-right text-[11px] tabular-nums text-muted">
        {percent}%
      </span>
      <span
        class="w-20 shrink-0 text-right text-[11px] tabular-nums text-muted"
        title={tasks.running ? "Durasi berjalan" : "Total durasi"}
      >
        {elapsed}
      </span>
    {/if}

    {#if tasks.running}
      <button
        type="button"
        class="shrink-0 whitespace-nowrap rounded-field border border-danger/40 bg-danger/10 px-2.5 py-1 text-[11px] font-medium text-danger transition hover:bg-danger/20"
        title="Hentikan proses yang sedang berjalan"
        onclick={cancelRun}
      >
        Batalkan proses
      </button>
    {/if}

    {#if tasks.result?.path}
      <button
        type="button"
        class="shrink-0 whitespace-nowrap rounded-field border border-line px-2.5 py-1 text-[11px] font-medium text-text transition hover:bg-canvas"
        onclick={() => openOutput(tasks.result?.path ?? "")}
      >
        Buka folder output
      </button>
    {/if}

    <!-- Selalu tampil supaya kontrolnya mudah ditemukan; non-aktif saat belum ada log. -->
    <button
      type="button"
      class="flex shrink-0 items-center gap-1 whitespace-nowrap rounded-field border border-line px-2.5 py-1 text-[11px] font-medium text-text transition hover:bg-canvas disabled:cursor-default disabled:border-line-subtle disabled:text-faint disabled:hover:bg-transparent"
      title={!hasLogs
        ? "Belum ada log untuk ditampilkan"
        : expanded
          ? "Sembunyikan log, tampilkan progres saja"
          : "Tampilkan log"}
      aria-expanded={expanded}
      disabled={!hasLogs}
      onclick={() => (tasks.collapsed = !tasks.collapsed)}
    >
      {expanded ? "Sembunyikan log" : "Tampilkan log"}
      <span aria-hidden="true">{expanded ? "▾" : "▴"}</span>
    </button>
  </div>

  {#if expanded}
    <div class="border-t border-line bg-input">
      <div
        bind:this={logBox}
        class="mx-auto max-h-52 w-full max-w-[1100px] overflow-auto px-5 py-2 font-mono text-[11px] leading-relaxed"
      >
        {#if tasks.droppedLogs > 0}
          <p class="text-faint">
            … {tasks.droppedLogs} baris awal disembunyikan agar tampilan tetap ringan
          </p>
        {/if}
        {#each tasks.logs as record (record.seq)}
          <p class="whitespace-pre-wrap break-words {levelClass[record.level]}">
            {"  ".repeat(record.depth)}{levelIcon[record.level] ?? "·"}
            {record.message}
          </p>
        {/each}
      </div>
    </div>
  {/if}
</section>
