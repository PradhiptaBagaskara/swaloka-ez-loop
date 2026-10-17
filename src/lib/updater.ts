/// Pembaruan aplikasi dari dalam app (plugin updater Tauri).

import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

/** Ringkasan pembaruan yang tersedia dari endpoint. */
export interface UpdateInfo {
  version: string;
  body: string | null;
  date: string | null;
}

/** Pembaruan hasil pemeriksaan terakhir yang belum dipasang. */
let pending: Update | null = null;

/** Cek endpoint pembaruan; kembalikan info versi baru, atau null bila sudah terbaru. */
export async function checkUpdate(): Promise<UpdateInfo | null> {
  await pending?.close();
  pending = await check();
  if (!pending) return null;
  return {
    version: pending.version,
    body: pending.body ?? null,
    date: pending.date ?? null
  };
}

/** Unduh lalu pasang pembaruan tertunda. `onProgress` menerima persen 0-100. */
export async function installUpdate(onProgress: (percent: number) => void): Promise<void> {
  const update = pending;
  if (!update) throw new Error("Tidak ada pembaruan yang menunggu dipasang");

  let downloaded = 0;
  let total = 0;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? 0;
      onProgress(0);
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress(total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0);
    } else {
      onProgress(100);
    }
  });

  await update.close();
  pending = null;

  // Di Windows pemasang installer menutup lalu membuka ulang app sendiri.
  if (!navigator.userAgent.includes("Windows")) await relaunch();
}
