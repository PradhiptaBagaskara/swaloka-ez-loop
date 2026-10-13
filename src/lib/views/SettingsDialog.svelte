<script lang="ts">
  import { untrack } from "svelte";
  import { Modal } from "flowbite-svelte";

  import { api, errorMessage, type EncoderOption } from "$lib/api";
  import { pickDirectory, pickFile } from "$lib/picker";
  import { app } from "$lib/stores/app.svelte";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let ffmpegPath = $state("");
  let encoder = $state("");
  let theme = $state<"light" | "dark">("light");
  let outputDir = $state("");
  let encoders = $state<EncoderOption[]>([]);
  let detecting = $state(false);
  let saving = $state(false);

  // Muat nilai awal hanya saat dialog dibuka. `app.*` sengaja dibaca tanpa
  // dilacak (untrack) supaya perubahan preferensi di tengah proses simpan —
  // mis. dari setTheme() — tidak menimpa pilihan pengguna yang belum tersimpan.
  $effect(() => {
    if (!open) return;
    untrack(() => {
      ffmpegPath = app.prefs.ffmpeg_path ?? "";
      encoder = app.prefs.hw_encoder ?? "";
      theme = app.theme;
      outputDir = app.project?.customOutputPath ?? "";
    });
    void api.ffmpegEncoders().then((list) => (encoders = list));
  });

  async function chooseFfmpeg() {
    const selected = await pickFile("Pilih executable FFmpeg");
    if (selected) ffmpegPath = selected;
  }

  async function chooseOutput() {
    const selected = await pickDirectory("Pilih folder output proyek");
    if (selected) outputDir = selected;
  }

  async function detectEncoder() {
    detecting = true;
    try {
      encoder = await api.ffmpegDetectEncoder();
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    } finally {
      detecting = false;
    }
  }

  async function save() {
    saving = true;
    try {
      await app.setTheme(theme);
      await api.prefsSet({
        ...app.prefs,
        theme,
        ffmpeg_path: ffmpegPath.trim() || null,
        hw_encoder: encoder || null
      });
      if (app.project) {
        await api.projectSetOutput(outputDir.trim() || null);
      }
      await app.refreshPrefs();
      await app.checkFfmpeg();
      app.showToast("Pengaturan disimpan", "success");
      open = false;
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    } finally {
      saving = false;
    }
  }

  const inputClass =
    "w-full rounded-field border border-line bg-input px-3 py-2 text-sm text-text outline-none transition focus:border-accent";
  const labelClass = "mb-1 block text-xs font-medium text-muted";
</script>

<Modal bind:open title="Pengaturan" size="lg">
  <div class="space-y-4">
    <div>
      <label class={labelClass} for="setting-theme">Tema</label>
      <select id="setting-theme" class={inputClass} bind:value={theme}>
        <option value="light">Terang</option>
        <option value="dark">Gelap</option>
      </select>
    </div>

    <div>
      <label class={labelClass} for="setting-ffmpeg">Path FFmpeg</label>
      <div class="flex gap-2">
        <input
          id="setting-ffmpeg"
          class={inputClass}
          placeholder="Kosongkan untuk deteksi otomatis"
          bind:value={ffmpegPath}
        />
        <button
          type="button"
          class="shrink-0 rounded-field border border-line px-3 text-sm text-text transition hover:bg-canvas"
          title="Pilih executable FFmpeg"
          onclick={chooseFfmpeg}>📂</button
        >
      </div>
    </div>

    <div>
      <label class={labelClass} for="setting-encoder">Hardware encoder</label>
      <div class="flex gap-2">
        <select id="setting-encoder" class={inputClass} bind:value={encoder}>
          <option value="">Otomatis</option>
          {#each encoders as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
        <button
          type="button"
          class="shrink-0 rounded-field border border-line px-3 text-xs text-text transition hover:bg-canvas disabled:opacity-50"
          disabled={detecting}
          onclick={detectEncoder}
        >
          {detecting ? "Mendeteksi..." : "Deteksi otomatis"}
        </button>
      </div>
    </div>

    {#if app.project}
      <div>
        <label class={labelClass} for="setting-output">Folder output proyek</label>
        <div class="flex gap-2">
          <input
            id="setting-output"
            class={inputClass}
            placeholder="Bawaan: outputs/ di dalam proyek"
            bind:value={outputDir}
          />
          <button
            type="button"
            class="shrink-0 rounded-field border border-line px-3 text-sm text-text transition hover:bg-canvas"
            title="Pilih folder output"
            onclick={chooseOutput}>📂</button
          >
        </div>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <button
        type="button"
        class="rounded-field border border-line px-4 py-2 text-sm text-text transition hover:bg-canvas"
        onclick={() => (open = false)}
      >
        Batal
      </button>
      <button
        type="button"
        class="rounded-field bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50"
        disabled={saving}
        onclick={save}
      >
        Simpan
      </button>
    </div>
  {/snippet}
</Modal>
