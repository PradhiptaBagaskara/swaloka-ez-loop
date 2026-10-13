<script lang="ts">
  import type { Snippet } from "svelte";

  import StatusBadge from "$lib/components/ui/StatusBadge.svelte";

  let {
    index,
    title,
    subtitle = "",
    state = "",
    tone = "gray",
    actions,
    children
  }: {
    index: number;
    title: string;
    subtitle?: string;
    state?: string;
    tone?: "gray" | "primary" | "success" | "warning" | "danger" | "info";
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<!-- Satu tahap alur channel: bernomor, punya status, dan aksinya sendiri. -->
<section class="overflow-hidden rounded-card border border-line bg-surface">
  <header class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-line-subtle px-4 py-3">
    <span
      class="grid size-6 shrink-0 place-items-center rounded-full bg-accent-soft text-[11px] font-semibold text-accent"
      aria-hidden="true">{index}</span
    >
    <div class="min-w-0">
      <h2 class="text-sm font-semibold text-text">{title}</h2>
      {#if subtitle}
        <p class="mt-0.5 text-[11px] text-muted">{subtitle}</p>
      {/if}
    </div>
    {#if state}
      <StatusBadge text={state} {tone} />
    {/if}
    {#if actions}
      <div class="ml-auto flex shrink-0 flex-wrap items-center gap-2">
        {@render actions()}
      </div>
    {/if}
  </header>
  <div class="px-4 py-3">
    {@render children()}
  </div>
</section>
