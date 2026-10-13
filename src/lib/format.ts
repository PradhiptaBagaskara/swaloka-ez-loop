/// Helper format yang sama dengan `core/utils.py` (byte, durasi, jam).

export function formatBytes(size: number): string {
  let value = size;
  for (const unit of ["B", "KB", "MB", "GB", "TB"]) {
    if (Math.abs(value) < 1024 || unit === "TB") {
      if (unit === "B") return `${Math.trunc(value)} B`;
      return `${value.toFixed(1)} ${unit}`;
    }
    value /= 1024;
  }
  return `${value.toFixed(1)} TB`;
}

export function formatClock(seconds: number): string {
  const total = Math.trunc(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  const pad = (value: number) => String(value).padStart(2, "0");
  if (hours > 0) return `${hours}:${pad(minutes)}:${pad(secs)}`;
  if (minutes === 0) return `0:${pad(secs)}`;
  return `${minutes}:${pad(secs)}`;
}

/** Durasi dalam satuan Indonesia, mis. "12 dtk" atau "1 mnt 5 dtk". */
export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  if (total < 60) return `${total} dtk`;
  const minutes = Math.floor(total / 60);
  if (minutes < 60) return `${minutes} mnt ${total % 60} dtk`;
  return `${Math.floor(minutes / 60)} jam ${minutes % 60} mnt`;
}

/** Nama file dari sebuah path (mendukung pemisah `/` dan `\\`). */
export function baseName(path: string): string {
  const index = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return index >= 0 ? path.slice(index + 1) : path;
}

/** Stem file tanpa ekstensi. */
export function fileStem(path: string): string {
  const name = baseName(path);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

/** Ekstensi huruf kecil termasuk titik, mis. `.mp4`. */
export function extension(path: string): string {
  const name = baseName(path);
  const dot = name.lastIndexOf(".");
  return dot >= 0 ? name.slice(dot).toLowerCase() : "";
}

/** Slug waktu `YYYYMMDD_HHMMSS` untuk nama file output. */
export function timestampSlug(date = new Date()): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  return (
    `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}` +
    `_${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}`
  );
}
