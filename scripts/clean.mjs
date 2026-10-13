#!/usr/bin/env node
// Bersihkan hasil build dev, dan (opsional) data aplikasi sampai ke titik nol.
//
//   node scripts/clean.mjs            hasil build saja        (pnpm clean)
//   node scripts/clean.mjs --user     data aplikasi saja      (pnpm clean:user)
//   node scripts/clean.mjs --all      keduanya                (pnpm clean:all)
//
// Dijalankan lewat Node, bukan perintah shell, supaya sama di Windows, macOS,
// dan Linux tanpa `rm`/`del`/`rd`.
//
// Yang dibuang di mode build semuanya bisa dibangun ulang. Mode --user membuang
// pemasangan FFmpeg (puluhan MB) dan preferensi (termasuk daftar proyek
// terakhir), jadi keduanya dipisah: `pnpm clean` tidak pernah menyentuhnya.
//
// `src-tauri/gen/schemas/` sengaja TIDAK dihapus karena berkasnya ikut
// di-commit (dipakai editor untuk autocomplete ACL).

import { existsSync, lstatSync, readdirSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));

const flags = new Set(process.argv.slice(2));
const wantsAll = flags.has("--all");
const wantsUser = flags.has("--user");
const includeBuild = wantsAll || !wantsUser;
const includeUser = wantsAll || wantsUser;

/** Folder data aplikasi, mengikuti `storage/paths.rs` di backend. */
function appDataDir() {
  if (process.platform === "win32") {
    const base = process.env.APPDATA ?? join(homedir(), "AppData", "Roaming");
    return join(base, "SwalokaLoopingTool");
  }
  if (process.platform === "darwin") {
    return join(homedir(), "Library", "Application Support", "SwalokaLoopingTool");
  }
  const base = process.env.XDG_CONFIG_HOME ?? join(homedir(), ".config");
  return join(base, "swaloka-looping-tool");
}

const buildTargets = [
  { path: join(root, "dist"), label: "hasil build Vite" },
  { path: join(root, "node_modules", ".vite"), label: "cache optimizer Vite" },
  {
    path: join(root, "src-tauri", "target"),
    label: "artefak build Rust (debug + release)"
  }
];

const userTargets = [
  {
    path: appDataDir(),
    label: "data aplikasi: pemasangan FFmpeg + preferensi"
  }
];

/** Ukuran berkas atau folder secara rekursif; taut simbolik dilewati. */
function sizeOf(path, seen = new Set()) {
  let stats;
  try {
    stats = lstatSync(path);
  } catch {
    return 0;
  }
  if (stats.isSymbolicLink()) return 0;
  if (stats.isFile()) {
    // Artefak cargo saling hard-link, jadi berkas yang sama bisa muncul di
    // beberapa folder. Hitung sekali saja supaya angkanya tidak membengkak.
    const identity = `${stats.dev}:${stats.ino}`;
    if (seen.has(identity)) return 0;
    seen.add(identity);
    return stats.size;
  }

  let total = 0;
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    total += sizeOf(join(path, entry.name), seen);
  }
  return total;
}

function formatSize(bytes) {
  const mb = bytes / 1_000_000;
  return `${mb.toLocaleString("id-ID", {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1
  })} MB`;
}

function clean(targets) {
  let freed = 0;
  let removed = 0;
  let failed = 0;

  for (const target of targets) {
    if (!existsSync(target.path)) {
      console.log(`• ${target.path} — belum ada, dilewati`);
      continue;
    }

    const bytes = sizeOf(target.path);
    try {
      rmSync(target.path, { recursive: true, force: true });
    } catch (error) {
      failed += 1;
      console.error(`✗ ${target.path} gagal dihapus: ${error.message}`);
      console.error("  Tutup aplikasi dev dan proses build yang masih berjalan, lalu ulangi.");
      continue;
    }

    freed += bytes;
    removed += 1;
    console.log(`• ${target.path} — ${formatSize(bytes)} dihapus (${target.label})`);
  }

  return { freed, removed, failed };
}

const results = [];
if (includeBuild) {
  console.log("Hasil build:");
  results.push(clean(buildTargets));
}
if (includeUser) {
  if (includeBuild) console.log("");
  console.log("Data aplikasi:");
  results.push(clean(userTargets));
}

const freed = results.reduce((total, result) => total + result.freed, 0);
const removed = results.reduce((total, result) => total + result.removed, 0);
const failed = results.reduce((total, result) => total + result.failed, 0);

console.log(
  `\nSelesai. ${removed} folder dibersihkan, ${formatSize(freed)} dibebaskan.`
);
if (removed > 0 && includeBuild) {
  console.log("Jalankan `pnpm tauri dev` untuk membangun ulang.");
}
if (includeUser && removed > 0) {
  console.log("Pemasangan FFmpeg dan preferensi hilang; aplikasi akan mengunduh FFmpeg lagi saat dibuka.");
}
if (!includeUser && removed > 0) {
  console.log("Pemasangan FFmpeg dan preferensi tidak disentuh — pakai `pnpm clean:user` untuk itu.");
}

process.exit(failed > 0 ? 1 : 0);
