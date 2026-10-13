<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    pretitle = "",
    title,
    subtitle = "",
    actions
  }: {
    pretitle?: string;
    title: string;
    subtitle?: string;
    actions?: Snippet;
  } = $props();
</script>

<header class="flex flex-wrap items-start justify-between gap-3 pb-4">
  <div class="min-w-0">
    {#if pretitle}
      <p class="text-[11px] font-semibold uppercase tracking-wide text-muted">{pretitle}</p>
    {/if}
    <h1 class="text-xl font-semibold leading-tight text-text">{title}</h1>
    {#if subtitle}
      <p class="mt-1 text-sm text-muted">{subtitle}</p>
    {/if}
  </div>
  {#if actions}
    <div class="flex shrink-0 items-center gap-2">{@render actions()}</div>
  {/if}
</header>
