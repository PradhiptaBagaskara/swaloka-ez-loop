<script lang="ts">
  let {
    rows,
    labelWidth = "150px"
  }: {
    rows: { label: string; value: string; tone?: string }[];
    labelWidth?: string;
  } = $props();
</script>

<dl class="divide-y divide-line-subtle">
  {#each rows as row (row.label)}
    <div class="flex items-start gap-3 py-2">
      <dt class="shrink-0 text-xs text-muted" style={`width:${labelWidth}`}>{row.label}</dt>
      <dd class="min-w-0 flex-1 break-words text-sm text-text {row.tone ?? ''}">
        {row.value}
      </dd>
    </div>
  {/each}
</dl>
