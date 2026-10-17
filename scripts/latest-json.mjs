#!/usr/bin/env node
// Susun manifest `latest.json` untuk updater Tauri dari artifact hasil build.
//
//   ARTIFACTS_DIR=installers VERSION=6.1.0 \
//   BASE_URL=https://github.com/<owner>/<repo>/releases/download/v6.1.0 \
//   node scripts/latest-json.mjs
//
// Dipakai oleh workflow release: tiap artifact build tersimpan di subfolder
// sendiri (mis. `swaloka-macos-arm64`). Skrip ini mengambil berkas updater
// beserta berkas `.sig` pasangannya, lalu menulis manifest yang menunjuk ke
// aset GitHub Release.
//
// Manifest sengaja gagal keras bila ada platform yang artifact updater-nya
// tidak ketemu: Tauri memvalidasi seluruh berkas sebelum membaca field versi,
// jadi satu entri yang cacat membuat semua platform gagal diperbarui.

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

const subdirs = readdirSync(artifactsDir, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);

const platforms = {};
const missing = [];

for (const { key, slug, match } of PLATFORMS) {
  const subdir = subdirs.find((name) => name.includes(slug));
  const files = subdir
    ? readdirSync(join(artifactsDir, subdir)).filter((name) => !name.endsWith(".sig"))
    : [];
  const file = files.find(match);

  if (!subdir || !file) {
    missing.push(key);
    continue;
  }

  const signature = readFileSync(join(artifactsDir, subdir, `${file}.sig`), "utf8").trim();
  platforms[key] = { signature, url: `${baseUrl}/${encodeURIComponent(basename(file))}` };
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
