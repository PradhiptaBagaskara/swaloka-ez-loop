<script lang="ts">
  let {
    items,
    value,
    onchange,
    class: klass = ""
  }: {
    items: { value: string; label: string }[];
    value: string;
    onchange: (value: string) => void;
    class?: string;
  } = $props();
</script>

<div class="inline-flex flex-wrap gap-1 rounded-card border border-line bg-surface p-1 {klass}">
  {#each items as item (item.value)}
    <button
      type="button"
      class="rounded-field px-3 py-1.5 text-xs font-medium transition {item.value === value
        ? 'bg-accent text-white'
        : 'text-muted hover:bg-canvas hover:text-text'}"
      onclick={() => onchange(item.value)}
    >
      {item.label}
    </button>
  {/each}
</div>
