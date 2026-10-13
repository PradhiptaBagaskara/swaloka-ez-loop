<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title = "",
    subtitle = "",
    actions,
    footer,
    children,
    padded = true,
    class: klass = ""
  }: {
    title?: string;
    subtitle?: string;
    actions?: Snippet;
    footer?: Snippet;
    children: Snippet;
    padded?: boolean;
    class?: string;
  } = $props();
</script>

<section class="overflow-hidden rounded-card border border-line bg-surface {klass}">
  {#if title || actions}
    <header class="flex items-center justify-between gap-3 border-b border-line px-4 py-3">
      <div class="min-w-0">
        {#if title}
          <h2 class="truncate text-[15px] font-semibold text-text">{title}</h2>
        {/if}
        {#if subtitle}
          <p class="mt-0.5 text-xs text-muted">{subtitle}</p>
        {/if}
      </div>
      {#if actions}
        <div class="flex shrink-0 items-center gap-2">{@render actions()}</div>
      {/if}
    </header>
  {/if}

  {#if padded}
    <div class="p-4">{@render children()}</div>
  {:else}
    {@render children()}
  {/if}

  {#if footer}
    <footer class="border-t border-line bg-canvas/60 px-4 py-3">{@render footer()}</footer>
  {/if}
</section>
