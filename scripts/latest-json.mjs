#!/usr/bin/env node
// Susun manifest `latest.json` untuk updater Tauri dari artifact hasil build.
//
//   ARTIFACTS_DIR=installers VERSION=6.1.0 \
//   BASE_URL=https://github.com/<owner>/<repo>/releases/download/v6.1.0 \
//   node scripts/latest-json.mjs
//
// Dipakai oleh workflow release: tiap artifact build tersimpan di subfolder
// sendiri (mis. `swaloka-macos-arm64`). Di dalamnya berkas masih terbagi per
// folder bundler (`macos/`, `nsis/`, `appimage/`, ...) karena upload-artifact
// menyimpan berkas pada leluhur bersama dari semua pola path-nya, jadi berkas
// updater dicari secara rekursif. Skrip ini mengambil berkas updater beserta
// berkas `.sig` pasangannya, lalu menulis manifest yang menunjuk ke aset
// GitHub Release.
//
// Manifest sengaja gagal keras bila ada platform yang artifact updater-nya
// tidak ketemu: Tauri memvalidasi seluruh berkas sebelum membaca field versi,
// jadi satu entri yang cacat membuat semua platform gagal diperbarui.
//
// URL di manifest memakai nama aset Release, yang BUKAN nama berkas lokal:
// GitHub mengganti setiap karakter di luar `[A-Za-z0-9._-]` menjadi titik, dan
// yang paling sering kena adalah spasi pada `productName` Tauri
// (`Swaloka EZ Loop_1.1.2_aarch64.dmg` tersimpan sebagai
// `Swaloka.EZ.Loop_1.1.2_aarch64.dmg`). Memakai nama lokal apa adanya membuat
// unduhan 404 meski versinya terdeteksi.

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const artifactsDir = process.env.ARTIFACTS_DIR ?? "installers";
const baseUrl = process.env.BASE_URL;
const version = process.env.VERSION;
const output = process.env.OUTPUT ?? "latest.json";

if (!baseUrl || !version) {
  console.error("BASE_URL dan VERSION wajib diisi.");
  process.exit(1);
}

/// Kunci platform Tauri -> penanda nama artifact + pola berkas updater.
const PLATFORMS = [
  { key: "darwin-aarch64", slug: "macos-arm64", match: (name) => name.endsWith(".app.tar.gz") },
  { key: "darwin-x86_64", slug: "macos-x64", match: (name) => name.endsWith(".app.tar.gz") },
  { key: "windows-x86_64", slug: "windows-x64", match: (name) => name.endsWith("-setup.exe") },
  { key: "linux-x86_64", slug: "linux-x64", match: (name) => name.endsWith(".AppImage") }
];

/// Semua berkas di bawah `dir`, rekursif.
function listFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? listFiles(path) : [path];
  });
}

/// Nama aset Release untuk sebuah berkas lokal: sama dengan perilaku GitHub.
function assetName(file) {
  return basename(file).replace(/[^A-Za-z0-9._-]/g, ".");
}

const subdirs = readdirSync(artifactsDir, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);

const platforms = {};
const missing = [];

for (const { key, slug, match } of PLATFORMS) {
  const subdir = subdirs.find((name) => name.includes(slug));
  const file = subdir
    ? listFiles(join(artifactsDir, subdir)).find(
        (path) => !path.endsWith(".sig") && match(basename(path))
      )
    : undefined;

  if (!subdir || !file) {
    missing.push(key);
    continue;
  }

  const signature = readFileSync(`${file}.sig`, "utf8").trim();
  platforms[key] = { signature, url: `${baseUrl}/${assetName(file)}` };
}

if (missing.length > 0) {
  console.error(`Artifact updater tidak ditemukan untuk: ${missing.join(", ")}`);
  process.exit(1);
}

const manifest = {
  version,
  notes: process.env.NOTES ?? `Swaloka ${version}`,
  pub_date: new Date().toISOString(),
  platforms
};

writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`${output} ditulis untuk ${Object.keys(platforms).join(", ")}.`);
