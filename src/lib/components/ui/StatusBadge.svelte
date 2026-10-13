<script lang="ts">
  let {
    text,
    tone = "gray"
  }: {
    text: string;
    tone?: "gray" | "primary" | "success" | "warning" | "danger" | "info";
  } = $props();

  // Warna selalu dipasangkan dengan teks — bukan warna saja.
  const toneClass: Record<string, string> = {
    gray: "border-line bg-canvas text-muted",
    primary: "border-accent/30 bg-accent/10 text-accent",
    success: "border-success/30 bg-success/10 text-success",
    warning: "border-warning/30 bg-warning/10 text-warning",
    danger: "border-danger/30 bg-danger/10 text-danger",
    info: "border-info/30 bg-info/10 text-info"
  };
  const dotClass: Record<string, string> = {
    gray: "bg-muted",
    primary: "bg-accent",
    success: "bg-success",
    warning: "bg-warning",
    danger: "bg-danger",
    info: "bg-info"
  };
</script>

<span
  class="inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] font-medium {toneClass[
    tone
  ]}"
>
  <span class="size-1.5 rounded-full {dotClass[tone]}" aria-hidden="true"></span>
  {text}
</span>
