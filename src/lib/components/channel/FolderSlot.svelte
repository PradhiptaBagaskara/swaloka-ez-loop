<script lang="ts">
  import type { Snippet } from "svelte";

  import DropZone from "$lib/components/bins/DropZone.svelte";
  import { baseName } from "$lib/format";

  let {
    label,
    path,
    icon = "▭",
    emptyHint = "",
    onPick,
    onClear,
    children
  }: {
    label: string;
    path: string;
    icon?: string;
    emptyHint?: string;
    onPick: () => void;
    onClear: () => void;
    children?: Snippet;
  } = $props();
</script>

<!-- Padanan SingleSlot untuk folder: menampilkan path plus ringkasan isinya. -->
<div class="space-y-2">
  <p class="text-xs font-medium text-muted">{label}</p>
  {#if path}
    <div class="rounded-field border border-line bg-canvas px-2.5 py-2">
      <div class="flex items-center gap-2">
        <span class="shrink-0 text-sm text-muted" aria-hidden="true">{icon}</span>
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm text-text" title={path}>{baseName(path)}</p>
          <p class="truncate text-[11px] text-faint" title={path}>{path}</p>
        </div>
        <div class="flex shrink-0 items-center gap-1">
          <button
            type="button"
            class="rounded border border-line px-2 py-1 text-[11px] text-text transition hover:bg-surface focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
            aria-label="Ganti {label}"
            onclick={onPick}
          >
            Ganti
          </button>
          <button
            type="button"
            class="ml-1 rounded border-l border-line p-1 pl-2 text-muted transition hover:bg-surface hover:text-danger focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
            title="Kosongkan"
            aria-label="Kosongkan {label}"
            onclick={onClear}
          >
            ✕
          </button>
        </div>
      </div>
      {#if children}
        <div class="mt-1.5 border-t border-line-subtle pt-1.5 text-[11px] text-muted">
          {@render children()}
        </div>
      {/if}
    </div>
  {:else}
    <DropZone
      label={`Pilih ${label.toLowerCase()}`}
      hint={emptyHint}
      {icon}
      compact
      onPick={onPick}
    />
  {/if}
</div>
