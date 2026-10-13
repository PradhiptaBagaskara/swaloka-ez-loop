<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    subtitle = "",
    icon = "▢",
    actionLabel = "",
    onAction,
    children
  }: {
    title: string;
    subtitle?: string;
    icon?: string;
    actionLabel?: string;
    onAction?: () => void;
    children?: Snippet;
  } = $props();
</script>

<div class="flex flex-col items-center justify-center gap-2 px-4 py-8 text-center">
  <span class="text-2xl text-faint" aria-hidden="true">{icon}</span>
  <p class="text-sm font-medium text-text">{title}</p>
  {#if subtitle}
    <p class="max-w-md text-xs text-muted">{subtitle}</p>
  {/if}
  {#if actionLabel && onAction}
    <button
      type="button"
      class="mt-1 rounded-field border border-line px-3 py-1.5 text-xs font-medium text-text transition hover:bg-canvas"
      onclick={onAction}
    >
      {actionLabel}
    </button>
  {/if}
  {#if children}
    {@render children()}
  {/if}
</div>
