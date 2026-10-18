/// State antrean tugas: progres, aliran log, dan hasil run.

import { listen } from "@tauri-apps/api/event";

import { errorMessage } from "../api";

export type LogLevel = "info" | "success" | "warning" | "error";

export interface LogRecord {
  message: string;
  level: LogLevel;
  timestamp: string;
  depth: number;
  durationSeconds: number | null;
}

export interface RunResult {
  label: string;
  path?: string;
}

export interface RunOptions {
  /** Titik henti progres -> label tahap, urut naik. */
  staging?: [number, string][];
  /** Bentuk baris hasil dari nilai balikan perintah. */
  done?: (value: unknown) => RunResult;
  /** Nama dasar file log (dipakai backend). */
  onLog?: (record: LogRecord) => void;
}

const DEFAULT_STAGING: [number, string][] = [
  [0, "Memulai..."],
  [1, "Bekerja..."]
];

/** Satu baris log di panel; `seq` jadi key stabil saat daftar lama dipangkas. */
export interface LogLine extends LogRecord {
  seq: number;
}

/**
 * Batas baris log yang disimpan di UI. FFmpeg bisa memuntahkan puluhan ribu
 * baris diagnostik dalam satu perintah; tanpa batas ini DOM membengkak dan
 * webview membeku (progres serta timer ikut berhenti).
 */
const MAX_LOG_LINES = 1000;

class TaskStore {
  running = $state(false);
  progress = $state(0);
  stage = $state("Memulai...");
  logs = $state<LogLine[]>([]);
  /** Jumlah baris lama yang dibuang karena melewati batas UI. */
  droppedLogs = $state(0);
  result = $state<RunResult | null>(null);
  error = $state<string | null>(null);
  collapsed = $state(false);
  /** Durasi run berjalan (berdetak tiap 200 ms), lalu dibekukan saat selesai. */
  elapsedSeconds = $state(0);

  private staging: [number, string][] = DEFAULT_STAGING;
  private startedAt: number | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;
  private nextSeq = 0;
  /** True bila run yang tampil dimulai dari luar antarmuka (mis. server MCP). */
  private external = false;
  private installed = false;
  private onLog: ((record: LogRecord) => void) | null = null;

  async run<T>(action: () => Promise<T>, options: RunOptions = {}): Promise<T | undefined> {
    if (this.running) return undefined;

    this.staging = (options.staging ?? DEFAULT_STAGING)
      .slice()
      .sort((left, right) => left[0] - right[0]);
    this.running = true;
    this.external = false;
    this.onLog = options.onLog ?? null;
    this.progress = 0;
    this.stage = this.staging[0][1];
    this.logs = [];
    this.droppedLogs = 0;
    this.nextSeq = 0;
    this.result = null;
    this.error = null;
    // Panel progres selalu terbuka saat run mulai; pengguna tetap bisa menutupnya.
    this.collapsed = false;
    this.startTimer();

    try {
      const value = await action();
      this.progress = 1;
      this.stage = this.staging[this.staging.length - 1][1];
      this.result = options.done
        ? options.done(value)
        : { label: "Selesai", path: typeof value === "string" ? value : undefined };
      return value;
    } catch (error) {
      this.error = errorMessage(error);
      return undefined;
    } finally {
      this.onLog = null;
      this.stopTimer();
      this.running = false;
    }
  }

  private startTimer() {
    this.stopTimer();
    this.startedAt = Date.now();
    this.elapsedSeconds = 0;
    this.timer = setInterval(() => {
      if (this.startedAt !== null) {
        this.elapsedSeconds = (Date.now() - this.startedAt) / 1000;
      }
    }, 200);
  }

  private stopTimer() {
    if (this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }
    // Bekukan durasi terakhir supaya hasilnya tetap terbaca setelah selesai.
    if (this.startedAt !== null) {
      this.elapsedSeconds = (Date.now() - this.startedAt) / 1000;
      this.startedAt = null;
    }
  }

  /**
   * Pasang listener run sekali saja, dipanggil dari `App.svelte` saat mount.
   * Listener global (bukan per-run) supaya run yang dimulai dari luar antarmuka
   * — mis. lewat server MCP — ikut tampil di queue card.
   */
  async install() {
    if (this.installed) return;
    this.installed = true;
    await Promise.all([
      listen<{ label?: string }>("task://started", (event) => this.onStarted(event.payload)),
      listen("task://finished", () => this.onFinished()),
      listen<LogRecord>("task://log", (event) => {
        if (!this.running) return;
        this.appendLog(event.payload);
        if (!this.external) this.onLog?.(event.payload);
      }),
      listen<number>("task://progress", (event) => {
        if (!this.running) return;
        this.setProgress(event.payload);
      })
    ]);
  }

  /**
   * Run dimulai dari luar antarmuka: tampilkan di queue card walau antarmuka
   * tidak memulainya. Run yang dimulai antarmuka sudah memegang kartu ini, jadi
   * diabaikan supaya tidak menimpa progresnya.
   */
  private onStarted(payload: { label?: string }) {
    if (this.running) return;
    this.external = true;
    this.running = true;
    this.progress = 0;
    this.logs = [];
    this.droppedLogs = 0;
    this.nextSeq = 0;
    this.result = null;
    this.error = null;
    this.collapsed = false;
    this.staging = DEFAULT_STAGING;
    this.stage = payload?.label ?? "Dikerjakan dari luar";
    this.startTimer();
  }

  /** Run luar selesai; run yang dimulai antarmuka membereskan dirinya sendiri. */
  private onFinished() {
    if (!this.external) return;
    this.external = false;
    this.progress = 1;
    this.stopTimer();
    this.running = false;
  }

  setProgress(value: number) {
    this.progress = Math.min(1, Math.max(0, value));
    let label = this.staging[0][1];
    for (const [threshold, text] of this.staging) {
      if (this.progress >= threshold) label = text;
    }
    this.stage = label;
  }

  /** Simpan baris log, buang yang paling lama bila melewati batas UI. */
  private appendLog(record: LogRecord) {
    this.logs.push({ ...record, seq: this.nextSeq++ });
    if (this.logs.length > MAX_LOG_LINES) {
      const excess = this.logs.length - MAX_LOG_LINES;
      this.droppedLogs += excess;
      this.logs = this.logs.slice(excess);
    }
  }
}

export const tasks = new TaskStore();
