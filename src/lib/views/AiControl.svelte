<script lang="ts">
  import { onMount } from "svelte";

  import { api, errorMessage } from "$lib/api";
  import PageHeader from "$lib/components/ui/PageHeader.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { mcp } from "$lib/stores/mcp.svelte";

  let port = $state(8787);
  let busy = $state(false);
  let exePath = $state("");

  onMount(async () => {
    await mcp.refresh();
    port = mcp.running ? mcp.port : app.prefs.mcp_port;
    try {
      exePath = await api.appExePath();
    } catch {
      /* path tak terbaca: bagian perintah stdio disembunyikan */
    }
  });

  // Perintah siap-tempel: path dikutip agar aman walau ada spasi.
  const stdioCommand = $derived(exePath ? `"${exePath}" --mcp-stdio` : "");
  const addCommand = $derived(
    exePath ? `command-code mcp add swaloka -- "${exePath}" --mcp-stdio` : ""
  );

  async function toggle() {
    if (busy) return;
    busy = true;
    try {
      if (mcp.running) {
        await mcp.stop();
      } else {
        await mcp.start(port);
      }
    } finally {
      busy = false;
    }
  }

  async function copy(value: string | null) {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value);
      app.showToast("Disalin", "success");
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    }
  }

  const inputClass =
    "w-full rounded-field border border-line bg-input px-3 py-2 text-sm text-text outline-none transition focus:border-accent";
  const labelClass = "mb-1 block text-xs font-medium text-muted";
  const ghostBtn =
    "shrink-0 rounded-field border border-line px-3 text-sm text-text transition hover:bg-canvas";
</script>

<PageHeader
  pretitle="Sistem"
  title="Kontrol AI"
  subtitle="Buka server MCP di loopback agar agen AI bisa mengendalikan aplikasi: membuat channel, menyusun rencana, dan menjalankan proses."
/>

<section class="rounded-card border border-line bg-surface p-5">
  <div class="flex items-start justify-between gap-4">
    <div class="min-w-0">
      <p class="flex items-center gap-2 text-sm font-medium text-text">
        <span
          class="size-2 rounded-full {mcp.running ? 'bg-success' : 'bg-faint'}"
          aria-hidden="true"
        ></span>
        {mcp.running ? "Aktif" : "Nonaktif"}
      </p>
      <p class="mt-1 max-w-prose text-xs text-muted">
        Server hanya mendengarkan di 127.0.0.1, wajib memakai token, dan tidak menyala otomatis saat
        aplikasi dibuka — aktifkan lagi setiap sesi. Saat aktif, pekerjaan agen tampil di queue card
        seperti pekerjaan manual.
      </p>
    </div>
    <button
      type="button"
      class="shrink-0 rounded-field px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50 {mcp.running
        ? 'bg-danger'
        : 'bg-accent'}"
      disabled={busy}
      onclick={toggle}
    >
      {mcp.running ? "Matikan Kontrol AI" : "Aktifkan Kontrol AI"}
    </button>
  </div>

  <div class="mt-5 max-w-xs">
    <label class={labelClass} for="mcp-port">Port loopback</label>
    <input
      id="mcp-port"
      type="number"
      min="1024"
      max="65535"
      class={inputClass}
      bind:value={port}
      disabled={mcp.running}
    />
    <p class="mt-1 text-[11px] text-faint">
      {mcp.running
        ? "Matikan dulu untuk mengubah port."
        : "Bawaan 8787. Ubah bila bentrok dengan aplikasi lain."}
    </p>
  </div>

  {#if mcp.running && mcp.endpoint && mcp.token}
    <div class="mt-5 space-y-4 border-t border-line pt-5">
      <div>
        <p class={labelClass}>Endpoint (Streamable HTTP)</p>
        <div class="flex gap-2">
          <input
            readonly
            class={inputClass}
            value={mcp.endpoint}
            onfocus={(event) => event.currentTarget.select()}
          />
          <button type="button" class={ghostBtn} onclick={() => copy(mcp.endpoint)}>Salin</button>
        </div>
      </div>
      <div>
        <p class={labelClass}>Token</p>
        <div class="flex gap-2">
          <input
            readonly
            class={inputClass}
            value={mcp.token}
            onfocus={(event) => event.currentTarget.select()}
          />
          <button type="button" class={ghostBtn} onclick={() => copy(mcp.token)}>Salin</button>
        </div>
      </div>
      <div class="rounded-field border border-line-subtle bg-canvas p-3 text-xs text-muted">
        <p class="font-medium text-text">Cara menyambungkan klien</p>
        <p class="mt-1">
          <span class="text-text">HTTP</span> — arahkan klien ke endpoint di atas dengan header
          <code class="rounded bg-input px-1">Authorization: Bearer &lt;token&gt;</code>.
        </p>
        <p class="mt-1">
          <span class="text-text">stdio</span> — jalankan aplikasi dengan argumen
          <code class="rounded bg-input px-1">--mcp-stdio</code>; jembatan membaca port dan token
          otomatis, jadi tak perlu menempel token. Aplikasi harus tetap terbuka dan Kontrol AI aktif.
        </p>

        {#if exePath}
          <div class="mt-3 space-y-2">
            <div>
              <p class="text-text">Perintah bridge (terminal)</p>
              <div class="mt-1 flex gap-2">
                <input
                  readonly
                  class={inputClass}
                  value={stdioCommand}
                  onfocus={(event) => event.currentTarget.select()}
                />
                <button type="button" class={ghostBtn} onclick={() => copy(stdioCommand)}
                  >Salin</button
                >
              </div>
            </div>
            <div>
              <p class="text-text">Daftarkan di Command Code</p>
              <div class="mt-1 flex gap-2">
                <input
                  readonly
                  class={inputClass}
                  value={addCommand}
                  onfocus={(event) => event.currentTarget.select()}
                />
                <button type="button" class={ghostBtn} onclick={() => copy(addCommand)}>Salin</button>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</section>
