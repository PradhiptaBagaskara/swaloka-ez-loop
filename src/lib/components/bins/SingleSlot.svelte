<script lang="ts">
  import DropZone from "./DropZone.svelte";
  import FileRow from "./FileRow.svelte";

  let {
    label,
    path,
    kind = "video",
    emptyHint = "",
    onPick,
    onClear,
    onPreview
  }: {
    label: string;
    path: string | null;
    kind?: "audio" | "video";
    emptyHint?: string;
    onPick: () => void;
    onClear: () => void;
    onPreview?: () => void;
  } = $props();
</script>

<div class="space-y-2">
  <p class="text-xs font-medium text-muted">{label}</p>
  {#if path}
    <FileRow {path} {kind} onRemove={onClear} {onPreview} />
  {:else}
    <DropZone label={`Pilih ${label.toLowerCase()}`} hint={emptyHint} onPick={onPick} />
  {/if}
</div>
