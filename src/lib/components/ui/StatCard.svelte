<script lang="ts">
  let {
    label,
    value,
    hint = "",
    icon = "",
    tone = "primary"
  }: {
    label: string;
    value: string | number;
    hint?: string;
    icon?: string;
    tone?: "primary" | "success" | "warning" | "danger" | "info" | "gray";
  } = $props();

  const toneText: Record<string, string> = {
    primary: "text-accent",
    success: "text-success",
    warning: "text-warning",
    danger: "text-danger",
    info: "text-info",
    gray: "text-muted"
  };
</script>

<div class="rounded-card border border-line bg-surface p-4">
  <div class="flex items-start justify-between gap-2">
    <span class="text-[11px] font-semibold uppercase tracking-wide text-muted">{label}</span>
    {#if icon}
      <span class="text-lg {toneText[tone]}" aria-hidden="true">{icon}</span>
    {/if}
  </div>
  <p class="mt-2 text-2xl font-semibold leading-none text-text">{value}</p>
  {#if hint}
    <p class="mt-1.5 text-xs text-muted">{hint}</p>
  {/if}
</div>
