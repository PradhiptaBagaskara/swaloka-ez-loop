#!/usr/bin/env node
// Samakan versi aplikasi di tiga tempat yang dibaca Tauri dan npm:
//
//   node scripts/set-version.mjs 6.1.0
//
// Dipakai workflow rilis sebelum build. `version` di tauri.conf.json adalah
// yang dipakai bundler dan updater; dua sisanya dijaga tetap sama supaya versi
// yang tampil di aplikasi (perintah `app_version`) dan metadata npm cocok.

import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!version) {
  console.error("Pakai: node scripts/set-version.mjs <versi>");
  process.exit(1);
}

for (const file of ["src-tauri/tauri.conf.json", "package.json"]) {
  const data = JSON.parse(readFileSync(file, "utf8"));
  data.version = version;
  writeFileSync(file, `${JSON.stringify(data, null, 2)}\n`);
}

// Hanya `version` di dalam blok [package] yang diganti, bukan versi dependency.
const cargoFile = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoFile, "utf8");
const updated = cargo.replace(/(\[package\][\s\S]*?^version = ")[^"]*(")/m, `$1${version}$2`);
if (updated === cargo) {
  console.error("Tidak menemukan field version di blok [package] Cargo.toml.");
  process.exit(1);
}
writeFileSync(cargoFile, updated);

console.log(`Versi diset ke ${version}.`);
