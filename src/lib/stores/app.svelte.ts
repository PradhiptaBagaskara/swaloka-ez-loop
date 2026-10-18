/// State aplikasi: proyek aktif, preferensi, status FFmpeg, dan navigasi.

import { api, errorMessage, type FfmpegSource, type Preferences, type Project } from "../api";

export type ViewKey = "dashboard" | "channel" | "looping" | "media" | "ai";

export interface NavEntry {
  key: ViewKey;
  label: string;
  icon: string;
}

export const NAV_SECTIONS: { title: string; entries: NavEntry[] }[] = [
  {
    title: "Ruang Kerja",
    entries: [
      { key: "dashboard", label: "Dasbor", icon: "▤" },
      { key: "channel", label: "EZ Loop", icon: "▣" },
      { key: "looping", label: "Looping", icon: "⟳" },
      { key: "media", label: "Media Tools", icon: "✂" },
      { key: "ai", label: "Kontrol AI", icon: "◈" },
    ],
  },
];

export const DEFAULT_PREFS: Preferences = {
  theme: "light",
  ffmpeg_path: null,
  hw_encoder: null,
  last_project_path: null,
  recent_projects: [],
  mcp_port: 8787,
};

class AppStore {
  project = $state<Project | null>(null);
  prefs = $state<Preferences>({ ...DEFAULT_PREFS });
  version = $state("");
  view = $state<ViewKey>("dashboard");
  ffmpegReady = $state(false);
  ffmpegChecked = $state(false);
  ffmpegVersion = $state<string | null>(null);
  ffmpegSource = $state<FfmpegSource>("missing");
  busy = $state(false);
  settingsOpen = $state(false);
  toast = $state<{ message: string; kind: "success" | "danger" } | null>(null);
  /**
   * Revisi data per scope (`channels`, `project`, `prefs`). View membacanya
   * sebagai dependensi supaya memuat ulang saat data berubah dari luar.
   */
  revisions = $state<Record<string, number>>({ channels: 0, project: 0, prefs: 0 });
  /** Layar root yang harus dirender. */
  get screen(): "loading" | "guide" | "landing" | "workspace" {
    if (!this.ffmpegChecked) return "loading";
    if (!this.ffmpegReady) return "guide";
    if (!this.project) return "landing";
    return "workspace";
  }

  get theme(): "light" | "dark" {
    return this.prefs.theme === "dark" ? "dark" : "light";
  }

  async bootstrap() {
    this.prefs = await api.prefsGet();
    this.version = await api.appVersion();
    this.applyTheme();
    this.project = await api.projectCurrent();
    await this.checkFfmpeg();
  }

  applyTheme() {
    const root = document.documentElement;
    root.classList.toggle("dark", this.theme === "dark");
  }

  async setTheme(theme: "light" | "dark") {
    this.prefs = { ...this.prefs, theme };
    this.applyTheme();
    this.prefs = await api.prefsSet(this.prefs);
  }

  async checkFfmpeg() {
    const status = await api.ffmpegStatus();
    this.ffmpegReady = status.ready;
    this.ffmpegVersion = status.version;
    this.ffmpegSource = status.source;
    this.ffmpegChecked = true;
  }

  async createProject(rootPath: string) {
    this.busy = true;
    try {
      const name = rootPath.split(/[\\/]/).filter(Boolean).pop() ?? "Project";
      this.project = await api.projectCreate(rootPath, name);
      this.view = "dashboard";
      this.showToast("Proyek dibuat", "success");
    } catch (error) {
      this.showToast(`Gagal membuat proyek: ${errorMessage(error)}`, "danger");
    } finally {
      this.busy = false;
    }
  }

  async openProject(rootPath: string) {
    this.busy = true;
    try {
      this.project = await api.projectOpen(rootPath);
      this.view = "dashboard";
      this.showToast("Proyek dibuka", "success");
    } catch (error) {
      this.showToast(`Gagal membuka proyek: ${errorMessage(error)}`, "danger");
    } finally {
      this.busy = false;
    }
  }

  async closeProject() {
    await api.projectClose();
    this.project = null;
    this.prefs = await api.prefsGet();
  }

  async refreshPrefs() {
    this.prefs = await api.prefsGet();
    this.applyTheme();
  }

  /**
   * Terapkan perubahan yang datang dari luar antarmuka (mis. channel dibuat
   * lewat server MCP): muat ulang data untuk scope itu dan naikkan revisinya
   * supaya view yang relevan ikut menyegarkan diri.
   */
  async applyExternalChange(scope: string) {
    try {
      if (scope === "project") {
        this.project = await api.projectCurrent();
      } else if (scope === "prefs") {
        this.prefs = await api.prefsGet();
        this.applyTheme();
      }
    } catch {
      /* Abaikan: perubahan berikutnya akan menyinkronkan lagi. */
    }
    this.revisions = { ...this.revisions, [scope]: (this.revisions[scope] ?? 0) + 1 };
  }

  async prefsRemove(path: string) {
    this.prefs = await api.removeRecent(path);
  }

  showToast(message: string, kind: "success" | "danger" = "success") {
    this.toast = { message, kind };
    setTimeout(() => {
      this.toast = null;
    }, 3200);
  }
}

export const app = new AppStore();
