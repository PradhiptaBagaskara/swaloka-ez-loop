<script lang="ts">
  let {
    label,
    hint = "",
    onPick,
    icon = "＋",
    compact = false
  }: {
    label: string;
    hint?: string;
    onPick: () => void;
    icon?: string;
    compact?: boolean;
  } = $props();
</script>

<button
  type="button"
  class="flex w-full items-center gap-3 rounded-field border border-dashed border-line bg-canvas text-left transition hover:border-accent hover:bg-accent/5 {compact
    ? 'px-3 py-2'
    : 'px-3 py-4'}"
  onclick={onPick}
>
  <span
    class="flex size-8 shrink-0 items-center justify-center rounded-full bg-surface text-sm text-muted"
    aria-hidden="true">{icon}</span
  >
  <span class="min-w-0">
    <span class="block text-sm font-medium text-text">{label}</span>
    {#if hint}
      <span class="block text-[11px] text-muted">{hint}</span>
    {/if}
  </span>
</button>
