/// State sesi per-alat: bertahan saat berpindah halaman.

import type { AudioOverlay } from "../api";

class SessionStore {
  /** Halaman Looping. */
  looping = $state({
    backgroundPath: null as string | null,
    introPath: null as string | null,
    audioFiles: [] as string[],
    loopCount: 1,
    outputName: ""
  });

  /** Halaman Media Tools. */
  media = $state({
    tab: "audio" as "audio" | "live" | "info",
    audioBase: [] as string[],
    audioOverlays: [] as AudioOverlay[],
    audioFormat: "m4a",
    audioName: "audio_mix",
    liveVideos: [] as string[],
    /** Bitrate CBR pilihan pengguna; null = pakai bitrate bawaan aplikasi. */
    liveBitrateMbps: null as number | null,
    infoVideo: null as string | null
  });

  /** Halaman Channel: hanya pilihan yang lengket, daftarnya dimuat di view. */
  channel = $state({
    channelId: null as string | null
  });
}

export const session = new SessionStore();
