/// Pemutar audio inline untuk tab Audio: satu item pada satu waktu, atau campuran
/// audio utama + overlay yang bisa diubah volumenya selagi berbunyi.

import { convertFileSrc } from "@tauri-apps/api/core";

import type { AudioOverlay } from "../api";
import { baseName } from "../format";

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

class AudioPreviewStore {
  /** Kunci item yang sedang diputar: `base:<i>`, `overlay:<i>`, `mix`, atau null. */
  playing = $state<string | null>(null);
  /** Progres 0..1 dari item yang sedang diputar. */
  progress = $state(0);
  error = $state<string | null>(null);

  private disposables: HTMLAudioElement[] = [];
  private basePlayers: HTMLAudioElement[] = [];
  private basePaths: string[] = [];
  private overlayBindings: { index: number; element: HTMLAudioElement }[] = [];

  isPlaying(key: string): boolean {
    return this.playing === key;
  }

  /** Pemutaran inline selalu eksklusif: memulai satu item menghentikan yang lain. */
  playBase(key: string, path: string, volume = 1) {
    this.playElement(key, path, volume);
  }

  playOverlay(index: number, path: string, volume: number) {
    const element = this.playElement(`overlay:${index}`, path, volume);
    // Didaftarkan agar slider volume bisa mengubahnya selagi berbunyi.
    this.overlayBindings = [{ index, element }];
  }

  /** Putar audio utama berurutan dengan overlay berjalan di atasnya. */
  playMix(baseAudios: string[], overlays: AudioOverlay[]) {
    this.stop();
    this.basePaths = [...baseAudios];

    this.overlayBindings = overlays.map((overlay, index) => {
      const element = this.track(new Audio(convertFileSrc(overlay.path)));
      // Overlay diulang supaya menutupi seluruh durasi audio utama.
      element.loop = true;
      element.volume = clamp01(overlay.volume);
      element.onerror = () => this.fail(overlay.path);
      void element.play().catch(() => this.fail(overlay.path));
      return { index, element };
    });

    this.basePlayers = baseAudios.map((path) => this.track(new Audio(convertFileSrc(path))));
    this.playing = "mix";
    if (this.basePlayers.length > 0) {
      this.playBaseAt(0);
    }
  }

  /** Ikuti geseran slider volume overlay selagi berbunyi. */
  syncVolumes(overlays: AudioOverlay[]) {
    for (const { index, element } of this.overlayBindings) {
      const overlay = overlays[index];
      if (overlay) element.volume = clamp01(overlay.volume);
    }
  }

  stop() {
    for (const element of this.disposables) {
      element.onended = null;
      element.ontimeupdate = null;
      element.onerror = null;
      element.pause();
      element.removeAttribute("src");
      element.load();
    }
    this.disposables = [];
    this.basePlayers = [];
    this.basePaths = [];
    this.overlayBindings = [];
    this.playing = null;
    this.progress = 0;
    this.error = null;
  }

  private track(element: HTMLAudioElement): HTMLAudioElement {
    this.disposables.push(element);
    return element;
  }

  /** Buat pemutar tunggal untuk satu item dan langsung jalankan. */
  private playElement(key: string, path: string, volume: number): HTMLAudioElement {
    this.stop();
    const element = this.track(new Audio(convertFileSrc(path)));
    element.volume = clamp01(volume);
    element.ontimeupdate = () => {
      this.progress = element.duration ? element.currentTime / element.duration : 0;
    };
    element.onended = () => this.stop();
    element.onerror = () => this.fail(path);
    this.playing = key;
    void element.play().catch(() => this.fail(path));
    return element;
  }

  private playBaseAt(index: number) {
    if (index >= this.basePlayers.length) {
      // Audio utama habis: overlay ikut berhenti karena durasi output sepanjang audio utama.
      if (this.basePlayers.length > 0) this.stop();
      return;
    }
    const element = this.basePlayers[index];
    const total = this.basePlayers.length;
    element.onended = () => this.playBaseAt(index + 1);
    element.ontimeupdate = () => {
      const within = element.duration ? element.currentTime / element.duration : 0;
      this.progress = (index + within) / total;
    };
    element.onerror = () => this.fail(this.basePaths[index] ?? "");
    void element.play().catch(() => this.fail(this.basePaths[index] ?? ""));
  }

  private fail(path: string) {
    this.error = `Gagal memutar ${baseName(path)}`;
  }
}

export const audioPreview = new AudioPreviewStore();
