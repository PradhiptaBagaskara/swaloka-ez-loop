<script lang="ts">
  import { Modal } from "flowbite-svelte";

  import { api, errorMessage, type Channel } from "$lib/api";
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import PageHeader from "$lib/components/ui/PageHeader.svelte";
  import StatusBadge from "$lib/components/ui/StatusBadge.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { audioPreview } from "$lib/stores/audioPreview.svelte";
  import { session } from "$lib/stores/session.svelte";
  import { tasks } from "$lib/stores/task.svelte";

  const form = session.channel;

  const primaryBtn =
    "rounded-field bg-accent px-3 py-1.5 text-xs font-medium text-white transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-50";
  const inputClass =
    "w-full rounded-field border border-line bg-input px-3 py-2 text-sm text-text outline-none focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent/40";
  const labelClass = "block text-xs font-medium text-muted";
  const hintClass = "mt-1 block text-[11px] text-faint";

  let channels = $state<Channel[]>([]);
  let loading = $state(true);
  let createOpen = $state(false);
  let newName = $state("");
  let newTotalContent = $state<number | null>(30);
  let newMinMinutes = $state<number | null>(120);
  let newLoopCap = $state<number | null>(2);
  let deleteOpen = $state(false);
  let pendingDelete = $state<Channel | null>(null);

  // Kembali dari halaman proses atau selesai memproses: segarkan daftarnya.
  $effect(() => {
    void app.view;
    void form.channelId;
    void tasks.running;
    if (!form.channelId) void reload();
  });

  async function reload() {
    try {
      channels = await api.channelList();
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    } finally {
      loading = false;
    }
  }

  /** Status ringkas satu channel untuk kolom terakhir tabel. */
  function statusOf(channel: Channel): {
    text: string;
    tone: "gray" | "primary" | "success" | "danger";
  } {
    const plans = channel.plans.length;
    if (plans === 0) return { text: "Belum ada plan", tone: "gray" };
    const failed = channel.plans.filter((plan) => plan.status === "failed").length;
    const done = channel.plans.filter((plan) => plan.status === "done").length;
    if (failed > 0 && done + failed === plans) {
      return { text: `${failed} gagal`, tone: "danger" };
    }
    if (failed > 0) return { text: `${failed} gagal dari ${plans}`, tone: "danger" };
    if (done === plans) return { text: "Selesai semua", tone: "success" };
    if (done > 0) return { text: `${done} dari ${plans} selesai`, tone: "primary" };
    return { text: "Siap dijalankan", tone: "gray" };
  }

  /** Buka dialog pembuatan channel dengan nilai awal yang aman. */
  function openCreate() {
    newName = "";
    newTotalContent = 30;
    newMinMinutes = 120;
    newLoopCap = 2;
    createOpen = true;
  }

  async function createChannel() {
    try {
      const created = await api.channelCreate({
        name: newName.trim() || "Channel baru",
        audioFolderPath: "",
        videoFolderPath: "",
        minDurationMinutes: newMinMinutes && newMinMinutes > 0 ? newMinMinutes : 120,
        totalContent: newTotalContent && newTotalContent > 0 ? Math.trunc(newTotalContent) : 0,
        suggestedLoopCap: newLoopCap && newLoopCap > 0 ? Math.trunc(newLoopCap) : 2
      });
      createOpen = false;
      app.showToast("Channel dibuat", "success");
      // Langsung masuk ke halaman proses supaya folder bisa dipilih.
      form.channelId = created.id;
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    }
  }

  function openChannel(channel: Channel) {
    audioPreview.stop();
    form.channelId = channel.id;
  }

  async function deleteChannel() {
    const channel = pendingDelete;
    deleteOpen = false;
    if (!channel) return;
    try {
      await api.channelDelete(channel.id);
      await reload();
      app.showToast("Channel dihapus", "success");
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    }
  }
</script>

<PageHeader
  pretitle="Produksi konten"
  title="EZ Loop"
  subtitle="Kelola folder aset dan plan tiap channel. Klik barisnya untuk membuka halaman proses."
>
  {#snippet actions()}
    <button type="button" class={primaryBtn} onclick={openCreate}>＋ Channel baru</button>
  {/snippet}
</PageHeader>

{#if loading}
  <p class="text-sm text-muted">Memuat channel…</p>
{:else if channels.length === 0}
  <EmptyState
    title="Belum ada channel"
    subtitle="Channel menghubungkan satu folder lagu dan satu folder video menjadi plan konten: tiap plan memakai satu video latar dan satu playlist yang durasinya merata."
    icon="▣"
    actionLabel="Buat channel pertama"
    onAction={openCreate}
  />
{:else}
  <section class="overflow-hidden rounded-card border border-line bg-surface">
    <div class="overflow-x-auto">
      <table class="w-full text-sm">
        <thead>
          <tr class="border-b border-line text-left text-[11px] uppercase tracking-wide text-faint">
            <th class="px-4 py-2 font-semibold">Nama</th>
            <th class="px-4 py-2 font-semibold">Lagu</th>
            <th class="px-4 py-2 font-semibold">Video</th>
            <th class="px-4 py-2 font-semibold">Plan</th>
            <th class="px-4 py-2 font-semibold">Status</th>
            <th class="px-4 py-2"><span class="sr-only">Buka</span></th>
          </tr>
        </thead>
        <tbody>
          {#each channels as channel (channel.id)}
            {@const done = channel.plans.filter((plan) => plan.status === "done").length}
            {@const status = statusOf(channel)}
            <!-- Klik di baris mana pun membuka channel; kontrol di dalamnya
                 menghentikan propagasi. Sengaja tanpa overlay `absolute`:
                 WebKit tidak menjadikan <tr> sebagai containing block, sehingga
                 pseudo `inset-0` melebar dan ikut menutupi header. -->
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
            <tr
              class="cursor-pointer border-b border-line-subtle transition last:border-b-0 hover:bg-canvas"
              onclick={() => openChannel(channel)}
            >
              <td class="px-4 py-2.5 text-text">
                <button
                  type="button"
                  class="text-left font-medium focus-visible:ring-2 focus-visible:ring-accent"
                  aria-label={`Buka channel ${channel.name}`}
                  onclick={(event) => {
                    event.stopPropagation();
                    openChannel(channel);
                  }}
                >
                  {channel.name}
                </button>
              </td>
              <td class="px-4 py-2.5 text-muted">{channel.audioItems.length}</td>
              <td class="px-4 py-2.5 text-muted">{channel.videoItems.length}</td>
              <td class="px-4 py-2.5 text-muted">
                {#if channel.plans.length === 0}
                  –
                {:else}
                  <span class="flex items-center gap-2">
                    <span class="tabular-nums">{done}/{channel.plans.length}</span>
                    <span class="h-1 w-16 overflow-hidden rounded-full bg-line-subtle">
                      <span
                        class="block h-full rounded-full bg-accent"
                        style={`width:${Math.round((done / channel.plans.length) * 100)}%`}
                      ></span>
                    </span>
                  </span>
                {/if}
              </td>
              <td class="px-4 py-2.5">
                <StatusBadge text={status.text} tone={status.tone} />
              </td>
              <td class="px-4 py-2.5">
                <div class="flex items-center justify-end gap-2">
                  <button
                    type="button"
                    class="rounded p-1 text-muted transition hover:bg-surface hover:text-danger focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
                    title="Hapus channel"
                    aria-label={`Hapus channel ${channel.name}`}
                    onclick={(event) => {
                      event.stopPropagation();
                      pendingDelete = channel;
                      deleteOpen = true;
                    }}>✕</button
                  >
                  <span class="text-faint" aria-hidden="true">›</span>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
{/if}

<Modal bind:open={deleteOpen} title="Hapus channel" size="md">
  <p class="text-sm text-text">
    Channel "{pendingDelete?.name ?? ""}" beserta {pendingDelete?.plans.length ?? 0} plan dan
    statusnya akan dihapus. File hasil yang sudah jadi tidak ikut terhapus.
  </p>
  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <button
        type="button"
        class="rounded-field border border-line px-4 py-2 text-sm text-text transition hover:bg-canvas"
        onclick={() => (deleteOpen = false)}
      >
        Batal
      </button>
      <button
        type="button"
        class="rounded-field bg-danger px-4 py-2 text-sm font-medium text-white transition hover:opacity-90"
        onclick={deleteChannel}
      >
        Hapus channel
      </button>
    </div>
  {/snippet}
</Modal>

<Modal bind:open={createOpen} title="Channel baru" size="md">
  <div class="space-y-3">
    <label class={labelClass}>
      Nama channel
      <input
        type="text"
        class={`${inputClass} mt-1`}
        bind:value={newName}
        placeholder="Channel baru"
        autocomplete="off"
      />
    </label>
    <label class={labelClass}>
      Total konten yang ingin dibuat
      <input
        type="number"
        min="0"
        step="1"
        class={`${inputClass} mt-1`}
        bind:value={newTotalContent}
        placeholder="0 = ikut jumlah video"
      />
      <span class={hintClass}>Kosongkan (0) untuk mengikuti jumlah video di folder.</span>
    </label>
    <label class={labelClass}>
      Durasi minimum tiap konten (menit)
      <input type="number" min="1" step="5" class={`${inputClass} mt-1`} bind:value={newMinMinutes} />
    </label>
    <label class={labelClass}>
      Target putar per lagu (loop)
      <input type="number" min="1" step="1" class={`${inputClass} mt-1`} bind:value={newLoopCap} />
    </label>
  </div>
  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <button
        type="button"
        class="rounded-field border border-line px-4 py-2 text-sm text-text transition hover:bg-canvas focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
        onclick={() => (createOpen = false)}
      >
        Batal
      </button>
      <button
        type="button"
        class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
        onclick={createChannel}
      >
        Buat channel
      </button>
    </div>
  {/snippet}
</Modal>
