/// Pembungkus bertipe untuk perintah Tauri dan tipe payload backend.

import { invoke } from "@tauri-apps/api/core";

export interface Project {
  name: string;
  rootPath: string;
  customOutputPath: string | null;
  enableParallelProcessing: boolean;
  formatVersion: number;
}

export interface Preferences {
  theme: string;
  ffmpeg_path: string | null;
  hw_encoder: string | null;
  last_project_path: string | null;
  recent_projects: string[];
  mcp_port: number;
}

export interface ProjectStats {
  outputCount: number;
  cacheCount: number;
  ffmpegReady: boolean;
}

export type YouTubeStatus = "passed" | "warning" | "failed";

export interface YouTubeResult {
  status: YouTubeStatus;
  issues: string[];
  details: Record<string, string>;
}

export interface VideoMetadata {
  path: string;
  readable: boolean;
  error: string | null;
  codec: string | null;
  width: number | null;
  height: number | null;
  fps: string | null;
  pixFmt: string | null;
  duration: number | null;
  hasAudio: boolean;
  audioCodec: string | null;
  tags: Record<string, string>;
  colorSpace: string | null;
  colorPrimaries: string | null;
  colorTransfer: string | null;
  colorRange: string | null;
  profile: string | null;
  level: string | null;
  bFrames: number | null;
  gopSize: number | null;
  cabac: boolean | null;
  interlaced: boolean | null;
  bitrateKbps: number | null;
}

export interface VideoInfo {
  path: string;
  sizeBytes: number;
  metadata: VideoMetadata | null;
  youtube: YouTubeResult | null;
}

export interface PreflightReport {
  codecMismatch: boolean;
  mismatchFields: string[];
  youtube: YouTubeResult | null;
}

export interface LoopRequest {
  backgroundPath: string;
  audioFiles: string[];
  outputDir: string;
  outputName: string;
  loopCount: number;
  introPath: string | null;
  outputPath: string | null;
}

export interface LoopResult {
  outputPath: string;
  timestampPath: string | null;
  playlist: string[];
}

export interface AudioOverlay {
  path: string;
  volume: number;
}

export interface LiveStandardOptions {
  videoPath: string;
  outputPath: string;
  /** Bitrate CBR (Mbps); null = pakai bawaan aplikasi. */
  bitrateMbps: number | null;
  /** Encoder pilihan pengguna dari Pengaturan (null/libx264 = software). */
  hwEncoder: string | null;
}

export interface LiveDefaults {
  /** Bitrate bawaan bila pengguna tidak mengisinya. */
  defaultMbps: number;
  /** Batas bawah yang diterima aplikasi. */
  minMbps: number;
  /** Interval keyframe standar siaran, dalam detik. */
  keyframeSeconds: number;
}

export interface EncoderOption {
  value: string;
  label: string;
}

/** Asal biner FFmpeg yang sedang dipakai aplikasi. */
export type FfmpegSource = "installed" | "custom" | "missing";

export interface FfmpegStatus {
  ready: boolean;
  version: string | null;
  source: FfmpegSource;
  path: string | null;
  /** Ukuran unduhan platform ini; null = belum ada sumber otomatis. */
  expectedBytes: number | null;
  pinnedVersion: string;
}

/** Progres pemasangan FFmpeg dari event `setup://progress`. */
export interface InstallProgress {
  phase: "download" | "extract" | "verify" | "done";
  percent: number;
  downloadedBytes: number;
  totalBytes: number;
  message: string;
}

export interface CacheClearResult {
  removedAudio: number;
  removedLogs: number;
}

/** Status Kontrol AI (MCP): loopback-only dan dijaga token per sesi. */
export interface McpStatus {
  running: boolean;
  port: number;
  endpoint: string | null;
  token: string | null;
}

export type ChannelStatus =
  | "draft"
  | "ready"
  | "running"
  | "partiallyDone"
  | "completed"
  | "failed";
export type ChannelPlanStatus = "notProcessed" | "running" | "done" | "failed";

/** Satu lagu yang terdeteksi di folder audio channel. */
export interface ChannelAudioItem {
  id: string;
  path: string;
  durationSeconds: number | null;
}

/** Satu rencana episode: satu video latar dengan satu playlist lagu. */
export interface ChannelPlan {
  id: string;
  index: number;
  backgroundVideoPath: string;
  audioItemIds: string[];
  totalDurationSeconds: number;
  maxUsage: number;
  minUsage: number;
  selected: boolean;
  status: ChannelPlanStatus;
  introVideoPath: string | null;
  outputPath: string | null;
  error: string | null;
  processedAt: string | null;
  finishedAt: string | null;
}

export interface Channel {
  id: string;
  name: string;
  projectRootPath: string;
  outputDirectoryPath: string;
  audioFolderPath: string;
  videoFolderPath: string;
  minDurationMinutes: number;
  /** Target putar tiap lagu di dalam satu episode. */
  suggestedLoopCap: number;
  /** Jumlah konten yang ingin dibuat; 0 = mengikuti jumlah video. */
  totalContent: number;
  seed: number;
  audioItems: ChannelAudioItem[];
  videoItems: string[];
  plans: ChannelPlan[];
  status: ChannelStatus;
  createdAt: string;
  updatedAt: string;
}

export interface ChannelCreateInput {
  name: string;
  audioFolderPath: string;
  videoFolderPath: string;
  minDurationMinutes: number;
  /** Jumlah konten yang ingin dibuat; 0 = mengikuti jumlah video. */
  totalContent: number;
  /** Target putar tiap lagu; 0 = pakai bawaan aplikasi. */
  suggestedLoopCap: number;
}

/** Perkiraan disk untuk satu rencana. */
export interface ChannelPlanCapacity {
  planId: string;
  index: number;
  durationSeconds: number;
  outputBytes: number;
  requiredBytes: number;
}

/** Hasil cek kapasitas sebelum menjalankan rencana terpilih. */
export interface ChannelPreflight {
  planCount: number;
  totalSeconds: number;
  requiredBytes: number;
  availableBytes: number | null;
  fits: boolean;
  spaceUnknown: boolean;
  plans: ChannelPlanCapacity[];
}

export const api = {
  projectCurrent: () => invoke<Project | null>("project_current"),
  projectCreate: (rootPath: string, name?: string) =>
    invoke<Project>("project_create", { rootPath, name: name ?? null }),
  projectOpen: (rootPath: string) => invoke<Project>("project_open", { rootPath }),
  projectClose: () => invoke<void>("project_close"),
  projectSetOutput: (outputPath: string | null) =>
    invoke<Project>("project_set_output", { outputPath }),
  projectStats: () => invoke<ProjectStats>("project_stats"),

  prefsGet: () => invoke<Preferences>("prefs_get"),
  prefsSet: (prefs: Preferences) => invoke<Preferences>("prefs_set", { prefs }),
  recentProjects: () => invoke<string[]>("recent_projects"),
  removeRecent: (path: string) => invoke<Preferences>("remove_recent", { path }),

  ffmpegCheck: () => invoke<boolean>("ffmpeg_check"),
  ffmpegDetectEncoder: () => invoke<string>("ffmpeg_detect_encoder"),
  ffmpegEncoders: () => invoke<EncoderOption[]>("ffmpeg_encoders"),
  ffmpegConfigure: () => invoke<boolean>("ffmpeg_configure"),
  ffmpegStatus: () => invoke<FfmpegStatus>("ffmpeg_status"),
  ffmpegInstall: () => invoke<FfmpegStatus>("ffmpeg_install"),
  ffmpegInstallCancel: () => invoke<void>("ffmpeg_install_cancel"),

  loopRun: (request: LoopRequest) => invoke<LoopResult>("loop_run", { request }),
  loopPreflight: (backgroundPath: string, introPath: string | null) =>
    invoke<PreflightReport>("loop_preflight", { backgroundPath, introPath }),
  loopMetadata: (path: string) => invoke<VideoMetadata>("loop_metadata", { path }),

  mediaInfo: (path: string) => invoke<VideoInfo>("media_info", { path }),

  mediaEncodeLive: (options: LiveStandardOptions) =>
    invoke<string | null>("media_encode_live", { options }),
  mediaLiveDefaults: () => invoke<LiveDefaults>("media_live_defaults"),

  mediaAudioProcess: (baseAudios: string[], overlays: AudioOverlay[], outputPath: string) =>
    invoke<string>("media_audio_process", { baseAudios, overlays, outputPath }),
  mediaAudioConcat: (audioPaths: string[], outputPath: string) =>
    invoke<string>("media_audio_concat", { audioPaths, outputPath }),
  mediaAudioExportPreset: (overlays: AudioOverlay[], outputPath: string) =>
    invoke<string>("media_audio_export_preset", { overlays, outputPath }),

  channelList: () => invoke<Channel[]>("channel_list"),
  channelLoad: (channelId: string) => invoke<Channel | null>("channel_load", { channelId }),
  channelCreate: (input: ChannelCreateInput) => invoke<Channel>("channel_create", { input }),
  channelSave: (channel: Channel) => invoke<Channel>("channel_save", { channel }),
  channelDelete: (channelId: string) => invoke<void>("channel_delete", { channelId }),
  channelPlan: (channel: Channel) => invoke<Channel>("channel_plan", { channel }),
  channelRescanVideos: (channel: Channel) => invoke<Channel>("channel_rescan_videos", { channel }),
  channelPreflight: (channel: Channel) =>
    invoke<ChannelPreflight>("channel_preflight", { channel }),
  channelRun: (channel: Channel) => invoke<Channel>("channel_run", { channel }),
  channelCancel: () => invoke<void>("channel_cancel"),

  cancelRun: () => invoke<void>("cancel_run"),
  cleanCache: () => invoke<CacheClearResult>("clean_cache"),
  appVersion: () => invoke<string>("app_version"),
  appExePath: () => invoke<string>("app_exe_path"),

  mcpStatus: () => invoke<McpStatus>("mcp_status"),
  mcpStart: (port: number) => invoke<McpStatus>("mcp_start", { port }),
  mcpStop: () => invoke<McpStatus>("mcp_stop"),
};

/** Ubah pesan error apa pun menjadi teks yang bisa ditampilkan. */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
