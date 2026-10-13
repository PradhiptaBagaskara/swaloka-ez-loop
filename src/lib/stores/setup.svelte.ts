/// State pemasangan FFmpeg bawaan aplikasi di layar setup.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { api, errorMessage, type FfmpegStatus, type InstallProgress } from "../api";
import { app } from "./app.svelte";

/** Ubah byte menjadi teks MB (satu desimal, pemisah desimal Indonesia). */
export function formatMb(bytes: number): string {
  return (bytes / 1_000_000).toLocaleString("id-ID", {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1
  });
}

class SetupStore {
  status = $state<FfmpegStatus | null>(null);
  running = $state(false);
  phase = $state<InstallProgress["phase"]>("download");
  percent = $state(0);
  downloadedBytes = $state(0);
  totalBytes = $state(0);
  message = $state("");
  error = $state<string | null>(null);

  private unlisten: UnlistenFn[] = [];
  private cancelled = false;

  /** Apakah platform ini punya sumber unduhan otomatis. */
  get canInstall(): boolean {
    return this.status?.expectedBytes != null;
  }

  get sizeLabel(): string | null {
    const bytes = this.status?.expectedBytes;
    return bytes == null ? null : `±${formatMb(bytes)} MB`;
  }

  get transferredLabel(): string {
    const total = this.totalBytes > 0 ? ` / ${formatMb(this.totalBytes)} MB` : "";
    return `${formatMb(this.downloadedBytes)} MB${total}`;
  }

  get percentLabel(): number {
    return Math.round(this.percent * 100);
  }

  async refresh() {
    this.status = await api.ffmpegStatus();
    return this.status;
  }

  async install() {
    if (this.running) return;

    this.error = null;
    this.cancelled = false;
    this.running = true;
    this.phase = "download";
    this.percent = 0;
    this.downloadedBytes = 0;
    this.totalBytes = this.status?.expectedBytes ?? 0;
    this.message = "Menyiapkan unduhan";

    this.unlisten = await Promise.all([
      listen<InstallProgress>("setup://progress", (event) => this.apply(event.payload))
    ]);

    try {
      this.status = await api.ffmpegInstall();
      await app.checkFfmpeg();
      app.showToast(`FFmpeg ${this.status.version ?? ""} siap dipakai`);
    } catch (error) {
      // Pembatalan oleh pengguna bukan kegagalan yang perlu dilaporkan.
      if (!this.cancelled) this.error = errorMessage(error);
    } finally {
      for (const unlisten of this.unlisten) unlisten();
      this.unlisten = [];
      this.running = false;
      this.percent = 0;
      this.downloadedBytes = 0;
    }
  }

  async cancel() {
    this.cancelled = true;
    await api.ffmpegInstallCancel();
  }

  private apply(progress: InstallProgress) {
    this.phase = progress.phase;
    this.percent = progress.percent;
    this.downloadedBytes = progress.downloadedBytes;
    this.totalBytes = progress.totalBytes;
    this.message = progress.message;
  }
}

export const setup = new SetupStore();
