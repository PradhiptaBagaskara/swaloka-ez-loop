<script lang="ts">
  import { app, NAV_SECTIONS } from "$lib/stores/app.svelte";

  const SYSTEM_ACTIONS = [
    { key: "settings", label: "Pengaturan", icon: "⚙" },
    { key: "clean", label: "Bersihkan cache", icon: "⌫" },
    { key: "support", label: "Dukungan", icon: "♥" }
  ] as const;

  let { onSystemAction }: { onSystemAction: (key: string) => void } = $props();
</script>

<aside class="flex w-60 shrink-0 flex-col border-r border-line bg-rail">
  <div class="flex items-center gap-2.5 px-4 py-4">
    <span
      class="flex size-8 items-center justify-center rounded-card bg-accent text-sm font-bold text-white"
      aria-hidden="true">S</span
    >
    <div>
      <p class="text-sm font-semibold leading-tight text-text">Swaloka</p>
      <p class="text-[11px] text-faint">v{app.version}</p>
    </div>
  </div>

  <nav class="flex-1 overflow-y-auto px-2 pb-2">
    {#each NAV_SECTIONS as section (section.title)}
      <p class="px-2 pb-1 pt-3 text-[11px] font-semibold uppercase tracking-wide text-faint">
        {section.title}
      </p>
      {#each section.entries as entry (entry.key)}
        <button
          type="button"
          class="mb-0.5 flex w-full items-center gap-2.5 rounded-field px-2.5 py-2 text-left text-sm transition {app.view ===
          entry.key
            ? 'bg-accent/10 font-semibold text-accent'
            : 'text-muted hover:bg-canvas hover:text-text'}"
          onclick={() => (app.view = entry.key)}
        >
          <span class="w-4 text-center" aria-hidden="true">{entry.icon}</span>
          {entry.label}
        </button>
      {/each}
    {/each}
  </nav>

  <!-- Aksi sistem duduk di dasar sidebar, terpisah dari ruang kerja. -->
  <div class="border-t border-line-subtle px-2 py-3">
    <p class="px-2 pb-1 text-[11px] font-semibold uppercase tracking-wide text-faint">Sistem</p>
    {#each SYSTEM_ACTIONS as action (action.key)}
      <button
        type="button"
        class="mb-0.5 flex w-full items-center gap-2.5 rounded-field px-2.5 py-2 text-left text-sm text-muted transition hover:bg-canvas hover:text-text"
        onclick={() => onSystemAction(action.key)}
      >
        <span class="w-4 text-center" aria-hidden="true">{action.icon}</span>
        {action.label}
      </button>
    {/each}
  </div>
</aside>
