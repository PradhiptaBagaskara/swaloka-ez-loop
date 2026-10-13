/// Dialog pemilih file/folder (dibungkus agar konsisten di semua view).

import { open } from "@tauri-apps/plugin-dialog";

export const VIDEO_EXTENSIONS = ["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv"];
export const AUDIO_EXTENSIONS = ["mp3", "wav", "m4a", "aac", "ogg", "flac"];

/** Pilih satu atau banyak file media; kembalikan daftar path (bisa kosong). */
export async function pickMedia(
  kind: "video" | "audio",
  multiple = false
): Promise<string[]> {
  const extensions = kind === "video" ? VIDEO_EXTENSIONS : AUDIO_EXTENSIONS;
  const selected = await open({
    multiple,
    directory: false,
    filters: [{ name: kind === "video" ? "Video" : "Audio", extensions }]
  });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

/** Pilih satu file executable. */
export async function pickFile(title: string): Promise<string | null> {
  const selected = await open({ multiple: false, directory: false, title });
  return typeof selected === "string" ? selected : null;
}

/** Pilih satu folder. */
export async function pickDirectory(title: string): Promise<string | null> {
  const selected = await open({ multiple: false, directory: true, title });
  return typeof selected === "string" ? selected : null;
}
