<script lang="ts">
  import { Modal } from "flowbite-svelte";

  import { update } from "$lib/stores/update.svelte";
</script>

<Modal bind:open={update.open} title="Pembaruan tersedia" size="md">
  {#if update.available}
    <div class="space-y-4">
      <p class="text-sm text-muted">
        Versi <span class="font-semibold text-text">{update.available.version}</span> sudah tersedia.
        Aplikasi akan dijalankan ulang setelah pemasangan selesai.
      </p>

      {#if update.available.body}
        <div class="max-h-48 overflow-y-auto rounded-field border border-line bg-canvas px-3 py-2">
          <pre class="whitespace-pre-wrap font-sans text-xs text-muted">{update.available.body}</pre>
        </div>
      {/if}

      {#if update.installing}
        <div class="space-y-2" role="status" aria-live="polite">
          <div class="flex items-baseline justify-between gap-3 text-xs">
            <span class="font-medium text-text">Mengunduh pembaruan…</span>
            <span class="shrink-0 tabular-nums text-muted">{update.percent}%</span>
          </div>
          <div class="h-2 overflow-hidden rounded-full bg-input">
            <div
              class="h-full rounded-full bg-accent transition-[width] duration-200 motion-reduce:transition-none"
              style={`width:${update.percent}%`}
            ></div>
          </div>
        </div>
      {/if}

      {#if update.error}
        <div
          class="rounded-field border border-danger/40 bg-danger/5 px-3 py-2 text-sm text-danger"
          role="alert"
        >
          {update.error}
        </div>
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <button
        type="button"
        class="rounded-field border border-line px-4 py-2 text-sm text-text transition hover:bg-canvas disabled:opacity-50"
        disabled={update.installing}
        onclick={() => (update.open = false)}
      >
        Nanti
      </button>
      <button
        type="button"
        class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50"
        disabled={update.installing}
        onclick={() => update.install()}
      >
        {update.error ? "Coba lagi" : "Perbarui"}
      </button>
    </div>
  {/snippet}
</Modal>
