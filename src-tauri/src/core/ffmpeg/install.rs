//! Pemasangan FFmpeg bawaan aplikasi: memilih sumber per platform, mengunduh
//! dengan progres byte, mengekstrak arsip Windows, lalu memverifikasi versinya.
//!
//! Semua platform memakai alur yang sama supaya aplikasi tidak bergantung pada
//! FFmpeg yang terpasang di komputer pengguna. Sumber per platform dipatok ke
//! FFmpeg 8.1.2 karena versi 9 membuat sebagian perintah yang dipakai aplikasi
//! gagal.

use std::fs::{self, File};
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::storage::paths;

/// Versi FFmpeg yang dipatok aplikasi.
pub const PINNED_VERSION: &str = "8.1.2";

/// Awalan versi yang diterima saat verifikasi biner hasil unduhan.
pub const ACCEPTED_PREFIX: &str = "8.1";

/// Pesan yang dipakai saat pengguna menghentikan unduhan.
pub const CANCELLED_MESSAGE: &str = "Unduhan FFmpeg dibatalkan";

const GYAN_BASE: &str = "https://github.com/GyanD/codexffmpeg/releases/download/8.1.2";
const SHAKA_BASE: &str =
    "https://github.com/shaka-project/static-ffmpeg-binaries/releases/download/n8.1.2-1";

const WINDOWS_ARCHIVE_FILE: &str = "ffmpeg-8.1.2-essentials_build.7z";
const WINDOWS_ARCHIVE_BYTES: u64 = 33_876_939;
const WINDOWS_ARCHIVE_SHA: &str =
    "e25b682664025d49034c981afb4bae36238a40f29a3cc1c713ad9a8b5b3528f6";
/// Nama berkas di dalam arsip Windows. Ditulis eksplisit (bukan lewat
/// `BinaryKind::file_name`) supaya jalur ekstraksi ini tetap bisa diuji dari
/// platform lain.
const WINDOWS_ARCHIVE_MEMBERS: [&str; 2] = ["ffmpeg.exe", "ffprobe.exe"];

/// Ukuran buffer unduhan: 1 MB supaya progres bergerak halus tanpa membebani.
const BUFFER_BYTES: usize = 1 << 20;

/// Biner yang dipasang aplikasi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryKind {
    Ffmpeg,
    Ffprobe,
}

impl BinaryKind {
    /// Nama berkas di folder pemasangan.
    pub fn file_name(self) -> String {
        let stem = match self {
            BinaryKind::Ffmpeg => "ffmpeg",
            BinaryKind::Ffprobe => "ffprobe",
        };
        if cfg!(windows) {
            format!("{stem}.exe")
        } else {
            stem.to_string()
        }
    }
}

/// Satu berkas yang harus diunduh apa adanya lalu dipakai langsung.
#[derive(Clone, Debug)]
pub struct RawFile {
    pub url: String,
    pub sha256: Option<String>,
    pub kind: BinaryKind,
    pub bytes: u64,
}

/// Arsip yang berisi kedua biner (dipakai untuk build Windows).
#[derive(Clone, Debug)]
pub struct ArchiveFile {
    pub url: String,
    pub sha256: Option<String>,
    pub bytes: u64,
    /// Nama berkas yang harus ada di dalam arsip.
    pub members: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum InstallStep {
    Raw(RawFile),
    SevenZip(ArchiveFile),
}

/// Rencana unduhan untuk satu platform.
#[derive(Clone, Debug)]
pub struct InstallPlan {
    pub version: String,
    pub steps: Vec<InstallStep>,
}

impl InstallPlan {
    /// Perkiraan total byte yang diunduh (dipakai untuk persen dan teks UI).
    pub fn total_bytes(&self) -> u64 {
        self.steps
            .iter()
            .map(|step| match step {
                InstallStep::Raw(file) => file.bytes,
                InstallStep::SevenZip(file) => file.bytes,
            })
            .sum()
    }

    pub fn has_archive(&self) -> bool {
        self.steps
            .iter()
            .any(|step| matches!(step, InstallStep::SevenZip(_)))
    }
}

struct ShakaAsset {
    os: &'static str,
    arch: &'static str,
    tag: &'static str,
    ffmpeg_bytes: u64,
    ffmpeg_sha: &'static str,
    ffprobe_bytes: u64,
    ffprobe_sha: &'static str,
}

/// Aset binary mentah dari shaka-project, lengkap dengan ukuran dan hash-nya
/// sehingga progres bisa dihitung tanpa menunggu header Content-Length.
const SHAKA_ASSETS: [ShakaAsset; 4] = [
    ShakaAsset {
        os: "macos",
        arch: "aarch64",
        tag: "osx-arm64",
        ffmpeg_bytes: 34_074_040,
        ffmpeg_sha: "e7b9fcd97f95f333512d6e8b8ac24d9dbc08f189f36047695499bd7b57214b22",
        ffprobe_bytes: 33_882_408,
        ffprobe_sha: "ded4c698b8ff38d0bc1fd30fcc5e768dc46f58bc15a8dfd61f98615ba49cde5c",
    },
    ShakaAsset {
        os: "macos",
        arch: "x86_64",
        tag: "osx-x64",
        ffmpeg_bytes: 42_745_472,
        ffmpeg_sha: "62c87854d851f202fc4a29bdda0fe7b6ebcddd37b863482ce1bdc81151b03fe4",
        ffprobe_bytes: 42_555_344,
        ffprobe_sha: "d530823f480a3c7eb6334f18a00197d1e9f1070e86172b9aa89c4bf4022bd879",
    },
    ShakaAsset {
        os: "linux",
        arch: "x86_64",
        tag: "linux-x64",
        ffmpeg_bytes: 48_299_480,
        ffmpeg_sha: "9eac5b2b5076db5ff853a6fa0dcd6b8de7d0cac8481eadda6c47cd935825f1ee",
        ffprobe_bytes: 48_090_488,
        ffprobe_sha: "065d3c56926052a76e884c4e4b51b7d95248da9391ab7effdcca6b94ceab98cf",
    },
    ShakaAsset {
        os: "linux",
        arch: "aarch64",
        tag: "linux-arm64",
        ffmpeg_bytes: 36_523_320,
        ffmpeg_sha: "6e7b1d7d1aa8c35e3fedd78a140aa0968717aeb7386ecfb0ee00773d9f0a4503",
        ffprobe_bytes: 36_326_648,
        ffprobe_sha: "fd2aca1456f0261cabef4514b6d97a70fa342003347f51b39c473dd364328089",
    },
];

/// Rencana unduhan untuk platform dan arsitektur tertentu.
pub fn plan_for(platform: &str, arch: &str) -> AppResult<InstallPlan> {
    let os = normalize_platform(platform);

    if os == "windows" {
        if arch != "x86_64" {
            return Err(AppError::new(format!(
                "Belum ada FFmpeg {PINNED_VERSION} bawaan untuk Windows {arch}. Atur path FFmpeg secara manual di Pengaturan."
            )));
        }
        return Ok(InstallPlan {
            version: PINNED_VERSION.to_string(),
            steps: vec![InstallStep::SevenZip(ArchiveFile {
                url: format!("{GYAN_BASE}/{WINDOWS_ARCHIVE_FILE}"),
                sha256: Some(WINDOWS_ARCHIVE_SHA.to_string()),
                bytes: WINDOWS_ARCHIVE_BYTES,
                members: WINDOWS_ARCHIVE_MEMBERS
                    .iter()
                    .map(|name| name.to_string())
                    .collect(),
            })],
        });
    }

    let asset = SHAKA_ASSETS
        .iter()
        .find(|asset| asset.os == os && asset.arch == arch)
        .ok_or_else(|| {
            AppError::new(format!(
                "Belum ada FFmpeg bawaan untuk {os} {arch}. Atur path FFmpeg secara manual di Pengaturan."
            ))
        })?;

    Ok(InstallPlan {
        version: PINNED_VERSION.to_string(),
        steps: vec![
            InstallStep::Raw(RawFile {
                url: format!("{SHAKA_BASE}/ffmpeg-{}", asset.tag),
                sha256: Some(asset.ffmpeg_sha.to_string()),
                kind: BinaryKind::Ffmpeg,
                bytes: asset.ffmpeg_bytes,
            }),
            InstallStep::Raw(RawFile {
                url: format!("{SHAKA_BASE}/ffprobe-{}", asset.tag),
                sha256: Some(asset.ffprobe_sha.to_string()),
                kind: BinaryKind::Ffprobe,
                bytes: asset.ffprobe_bytes,
            }),
        ],
    })
}

fn normalize_platform(platform: &str) -> &str {
    if platform.starts_with("win") {
        "windows"
    } else if platform.starts_with("macos") || platform.starts_with("darwin") {
        "macos"
    } else if platform.starts_with("linux") {
        "linux"
    } else {
        platform
    }
}

/// Folder tempat aplikasi menyimpan FFmpeg bawaannya.
pub fn install_dir() -> PathBuf {
    paths::config_dir().join("ffmpeg")
}

pub fn binary_path(kind: BinaryKind) -> PathBuf {
    install_dir().join(kind.file_name())
}

/// Apakah kedua biner bawaan sudah ada di folder pemasangan.
pub fn is_installed() -> bool {
    binary_path(BinaryKind::Ffmpeg).is_file() && binary_path(BinaryKind::Ffprobe).is_file()
}

/// Perkiraan ukuran unduhan platform saat ini, untuk teks tombol di UI.
pub fn expected_bytes() -> Option<u64> {
    plan_for(std::env::consts::OS, std::env::consts::ARCH)
        .ok()
        .map(|plan| plan.total_bytes())
}

/// Progres pemasangan yang dikirim ke frontend lewat event `setup://progress`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    /// "download" | "extract" | "verify" | "done"
    pub phase: String,
    pub percent: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
}

/// Callback progres pemasangan.
pub type InstallCallback = Arc<dyn Fn(InstallProgress) + Send + Sync>;

/// Penjaga agar hanya satu pemasangan berjalan sekaligus, sekaligus membawa
/// flag pembatalan yang dipakai tombol Batal di layar setup.
#[derive(Default)]
pub struct Installer {
    cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl Installer {
    pub fn begin(&self) -> AppResult<Arc<AtomicBool>> {
        let mut slot = self
            .cancel
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if slot.is_some() {
            return Err(AppError::new("Unduhan FFmpeg sedang berjalan"));
        }
        let flag = Arc::new(AtomicBool::new(false));
        *slot = Some(Arc::clone(&flag));
        Ok(flag)
    }

    pub fn request_cancel(&self) {
        let slot = self
            .cancel
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(flag) = slot.as_ref() {
            flag.store(true, Ordering::SeqCst);
        }
    }

    pub fn end(&self) {
        let mut slot = self
            .cancel
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *slot = None;
    }

    pub fn is_running(&self) -> bool {
        let slot = self
            .cancel
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        slot.is_some()
    }
}

/// Unduh dan pasang FFmpeg sesuai rencana ke folder bawaan aplikasi.
///
/// Mengembalikan versi yang terpasang. Berkas sementara dibersihkan saat gagal
/// supaya percobaan berikutnya mulai dari keadaan bersih.
pub fn install(
    plan: &InstallPlan,
    target: &Path,
    on_progress: &InstallCallback,
    cancel: &AtomicBool,
) -> AppResult<String> {
    fs::create_dir_all(target)?;

    let total_bytes = plan.total_bytes().max(1);
    // Unduhan mengambil sebagian besar waktu; ekstraksi dan verifikasi kecil.
    let download_span = if plan.has_archive() { 0.85 } else { 0.92 };

    let report = |phase: &str, percent: f64, downloaded: u64, message: &str| {
        on_progress(InstallProgress {
            phase: phase.to_string(),
            percent: percent.clamp(0.0, 1.0),
            downloaded_bytes: downloaded,
            total_bytes,
            message: message.to_string(),
        });
    };

    report("download", 0.0, 0, "Menyiapkan unduhan FFmpeg");

    match run_steps(plan, target, total_bytes, download_span, &report, cancel) {
        Ok(()) => {
            report(
                "verify",
                (download_span + 0.06).min(0.98),
                total_bytes,
                "Memeriksa FFmpeg",
            );
            let binary = target.join(BinaryKind::Ffmpeg.file_name());
            match verify_version(&binary) {
                Ok(version) => {
                    on_progress(InstallProgress {
                        phase: "done".to_string(),
                        percent: 1.0,
                        downloaded_bytes: total_bytes,
                        total_bytes,
                        message: format!("FFmpeg {version} siap dipakai"),
                    });
                    Ok(version)
                }
                Err(error) => {
                    remove_binaries(target);
                    Err(error)
                }
            }
        }
        Err(error) => {
            cleanup_partials(target);
            Err(error)
        }
    }
}

fn run_steps(
    plan: &InstallPlan,
    target: &Path,
    total_bytes: u64,
    download_span: f64,
    report: &dyn Fn(&str, f64, u64, &str),
    cancel: &AtomicBool,
) -> AppResult<()> {
    let mut downloaded = 0u64;

    for step in &plan.steps {
        if cancel.load(Ordering::SeqCst) {
            return Err(AppError::new(CANCELLED_MESSAGE));
        }
        match step {
            InstallStep::Raw(file) => {
                let destination = target.join(file.kind.file_name());
                let partial = partial_path(target, file.kind);
                let base = downloaded;
                let step_bytes = file.bytes;
                download_file(&file.url, &partial, cancel, &|received| {
                    let done = base + received.min(step_bytes);
                    report(
                        "download",
                        download_span * done as f64 / total_bytes as f64,
                        done,
                        "Mengunduh FFmpeg",
                    );
                })?;
                verify_digest(&partial, file.sha256.as_deref())?;
                fs::rename(&partial, &destination)?;
                make_executable(&destination)?;
                downloaded += file.bytes;
            }
            InstallStep::SevenZip(archive) => {
                let path = target.join(format!("{WINDOWS_ARCHIVE_FILE}.part"));
                let base = downloaded;
                download_file(&archive.url, &path, cancel, &|received| {
                    let done = base + received.min(archive.bytes);
                    report(
                        "download",
                        download_span * done as f64 / total_bytes as f64,
                        done,
                        "Mengunduh FFmpeg",
                    );
                })?;
                verify_digest(&path, archive.sha256.as_deref())?;

                if cancel.load(Ordering::SeqCst) {
                    let _ = fs::remove_file(&path);
                    return Err(AppError::new(CANCELLED_MESSAGE));
                }
                report(
                    "extract",
                    download_span + 0.03,
                    base + archive.bytes,
                    "Mengekstrak arsip FFmpeg",
                );
                extract_seven_zip(&path, target, &archive.members)?;
                let _ = fs::remove_file(&path);
                downloaded += archive.bytes;
            }
        }
    }

    Ok(())
}

/// Unduh satu berkas ke `partial` sambil melaporkan byte terunduh.
///
/// Terbuka supaya jalur unduhan bisa diuji terpisah dari pemasangan lengkap
/// (mis. menguji ekstraksi arsip Windows dari platform lain).
pub fn download_file(
    url: &str,
    partial: &Path,
    cancel: &AtomicBool,
    on_chunk: &dyn Fn(u64),
) -> AppResult<()> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|error| AppError::new(format!("Gagal mengunduh {url}: {error}")))?;

    let mut reader = response.body_mut().as_reader();
    let mut file = File::create(partial)?;
    let mut buffer = vec![0u8; BUFFER_BYTES];
    let mut received = 0u64;

    loop {
        if cancel.load(Ordering::SeqCst) {
            drop(file);
            let _ = fs::remove_file(partial);
            return Err(AppError::new(CANCELLED_MESSAGE));
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|error| AppError::new(format!("Unduhan {url} terputus: {error}")))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        received += read as u64;
        on_chunk(received);
    }

    file.flush()?;
    Ok(())
}

/// Bandingkan hash berkas dengan yang diharapkan; menghapus berkas kalau beda
/// supaya tidak ada biner rusak yang tertinggal.
pub fn verify_digest(path: &Path, expected: Option<&str>) -> AppResult<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual = sha256_of(path)?;
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    let _ = fs::remove_file(path);
    Err(AppError::new(
        "Berkas FFmpeg yang diunduh rusak (hash tidak cocok). Coba unduh lagi.",
    ))
}

fn sha256_of(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// Ekstrak arsip `.7z` dan salin biner yang dibutuhkan ke folder pemasangan.
///
/// Pencarian dilakukan rekursif karena susunan folder di dalam arsip tidak
/// dijamin (mis. `ffmpeg-8.1.2-essentials_build/bin/ffmpeg.exe`).
pub fn extract_seven_zip(archive: &Path, target: &Path, members: &[String]) -> AppResult<()> {
    let staging = target.join("unpack");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let result = sevenz_rust::decompress_file(archive, &staging)
        .map_err(|error| AppError::new(format!("Gagal mengekstrak arsip FFmpeg: {error}")));
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    for member in members {
        let found = find_file(&staging, member)?.ok_or_else(|| {
            AppError::new(format!("{member} tidak ditemukan di dalam arsip FFmpeg"))
        })?;
        let destination = target.join(member);
        fs::copy(&found, &destination)?;
        make_executable(&destination)?;
    }

    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

/// Cari berkas menurut nama di dalam folder (termasuk sub-folder).
fn find_file(root: &Path, name: &str) -> AppResult<Option<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|value| value.to_str()) == Some(name) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

/// Jalankan `<path> -version` dan pastikan versinya sesuai pin aplikasi.
pub fn verify_version(path: &Path) -> AppResult<String> {
    let mut command = Command::new(path);
    command.arg("-version");
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);

    let output = command.output().map_err(|error| {
        AppError::new(format!(
            "FFmpeg hasil unduhan tidak bisa dijalankan: {error}"
        ))
    })?;

    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.lines().next().unwrap_or_default().trim().to_string();
    if !output.status.success() || !first.contains("ffmpeg version") {
        return Err(AppError::new(format!(
            "FFmpeg hasil unduhan tidak bisa dijalankan ({first})"
        )));
    }

    let reported = first.split_whitespace().nth(2).unwrap_or_default();
    // Build dari shaka menuliskan versinya sebagai `n8.1.2`, jadi awalan `n`
    // dilepas sebelum dibandingkan.
    let version = reported.trim_start_matches('n').to_string();
    if !version.starts_with(ACCEPTED_PREFIX) {
        return Err(AppError::new(format!(
            "Versi FFmpeg hasil unduhan {reported} tidak sesuai (dibutuhkan {ACCEPTED_PREFIX}.x)"
        )));
    }
    Ok(version)
}

fn partial_path(target: &Path, kind: BinaryKind) -> PathBuf {
    target.join(format!("{}.part", kind.file_name()))
}

fn cleanup_partials(target: &Path) {
    let Ok(entries) = fs::read_dir(target) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("part") {
            let _ = fs::remove_file(path);
        }
    }
    let _ = fs::remove_dir_all(target.join("unpack"));
}

fn remove_binaries(target: &Path) {
    let _ = fs::remove_file(target.join(BinaryKind::Ffmpeg.file_name()));
    let _ = fs::remove_file(target.join(BinaryKind::Ffprobe.file_name()));
}

#[cfg(unix)]
fn make_executable(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_uses_gyan_archive_on_windows_x64() {
        let plan = plan_for("windows", "x86_64").expect("rencana Windows terbentuk");
        assert_eq!(plan.steps.len(), 1);
        assert!(plan.has_archive());
        assert_eq!(plan.total_bytes(), WINDOWS_ARCHIVE_BYTES);
        match &plan.steps[0] {
            InstallStep::SevenZip(file) => {
                assert!(file.url.ends_with(WINDOWS_ARCHIVE_FILE));
                assert_eq!(file.sha256.as_deref(), Some(WINDOWS_ARCHIVE_SHA));
                assert_eq!(file.members.len(), 2);
            }
            other => panic!("Windows harus memakai arsip, dapat {other:?}"),
        }
    }

    #[test]
    fn plan_uses_raw_shaka_binaries_elsewhere() {
        for (os, arch, expected_name) in [
            ("macos", "aarch64", "ffmpeg-osx-arm64"),
            ("darwin", "x86_64", "ffmpeg-osx-x64"),
            ("linux", "x86_64", "ffmpeg-linux-x64"),
            ("linux", "aarch64", "ffmpeg-linux-arm64"),
        ] {
            let plan = plan_for(os, arch).unwrap_or_else(|_| panic!("rencana {os} {arch}"));
            assert!(!plan.has_archive(), "{os} {arch} tidak memakai arsip");
            assert_eq!(plan.steps.len(), 2);
            match &plan.steps[0] {
                InstallStep::Raw(file) => assert!(file.url.ends_with(expected_name)),
                other => panic!("{os} {arch} harus binary mentah, dapat {other:?}"),
            }
            match &plan.steps[1] {
                InstallStep::Raw(file) => {
                    assert_eq!(file.kind, BinaryKind::Ffprobe);
                    assert!(file.url.contains("ffprobe"));
                }
                other => panic!("berkas kedua harus ffprobe, dapat {other:?}"),
            }
            assert_eq!(
                plan.total_bytes(),
                SHAKA_ASSETS
                    .iter()
                    .find(|asset| asset.os == normalize_platform(os) && asset.arch == arch)
                    .map(|asset| asset.ffmpeg_bytes + asset.ffprobe_bytes)
                    .expect("aset terdaftar")
            );
        }
    }

    #[test]
    fn plan_rejects_unsupported_targets() {
        assert!(
            plan_for("windows", "aarch64").is_err(),
            "Windows arm64 belum didukung"
        );
        assert!(
            plan_for("freebsd", "x86_64").is_err(),
            "platform lain belum didukung"
        );
    }

    #[test]
    fn every_shaka_asset_has_a_hash_and_size() {
        for asset in &SHAKA_ASSETS {
            assert_eq!(asset.ffmpeg_sha.len(), 64);
            assert_eq!(asset.ffprobe_sha.len(), 64);
            assert!(asset.ffmpeg_bytes > 1_000_000);
            assert!(asset.ffprobe_bytes > 1_000_000);
        }
    }

    #[test]
    fn installer_allows_only_one_run_at_a_time() {
        let installer = Installer::default();
        assert!(!installer.is_running());

        let flag = installer.begin().expect("mulai pertama diterima");
        assert!(installer.is_running());
        assert!(installer.begin().is_err(), "pemasangan kedua ditolak");

        installer.request_cancel();
        assert!(flag.load(Ordering::SeqCst), "flag pembatalan diset");

        installer.end();
        assert!(!installer.is_running());
        assert!(installer.begin().is_ok(), "bisa mulai lagi setelah selesai");
    }

    #[test]
    fn install_dir_lives_under_config_dir() {
        let dir = install_dir();
        assert!(dir.starts_with(paths::config_dir()));
        assert!(binary_path(BinaryKind::Ffmpeg).starts_with(dir));
    }

    #[test]
    fn verify_digest_reports_mismatch_and_removes_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("payload.bin");
        fs::write(&file, b"bukan biner asli").unwrap();

        let wrong = "0".repeat(64);
        assert!(verify_digest(&file, Some(&wrong)).is_err());
        assert!(!file.exists(), "berkas rusak tidak boleh tertinggal");

        fs::write(&file, b"bukan biner asli").unwrap();
        let actual = sha256_of(&file).unwrap();
        assert!(verify_digest(&file, Some(&actual)).is_ok());
        assert!(file.exists());
        assert!(verify_digest(&file, None).is_ok());
    }

    #[test]
    fn find_file_walks_nested_folders() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("build").join("bin");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("ffmpeg.exe"), b"stub").unwrap();

        let found = find_file(dir.path(), "ffmpeg.exe").unwrap();
        assert_eq!(found, Some(nested.join("ffmpeg.exe")));
        assert!(find_file(dir.path(), "ffprobe.exe").unwrap().is_none());
    }

    #[test]
    fn extract_reports_a_clear_error_for_a_non_archive() {
        let dir = tempfile::tempdir().unwrap();
        let bogus = dir.path().join("bogus.7z");
        fs::write(&bogus, b"ini bukan arsip").unwrap();

        let error = extract_seven_zip(&bogus, dir.path(), &["ffmpeg".to_string()]).unwrap_err();
        assert!(
            error.message.contains("Gagal mengekstrak"),
            "pesan error harus menjelaskan kegagalan: {}",
            error.message
        );
    }

    #[cfg(unix)]
    fn write_fake_binary(path: &Path, first_line: &str) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, format!("#!/bin/sh\necho \"{first_line}\"\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn verify_version_accepts_the_pinned_line_and_rejects_others() {
        let dir = tempfile::tempdir().unwrap();

        let good = dir.path().join("ffmpeg-good");
        write_fake_binary(&good, "ffmpeg version 8.1.2 Copyright (c) 2000-2025");
        assert_eq!(verify_version(&good).unwrap(), "8.1.2");

        // Build shaka menuliskan versinya dengan awalan `n`; itu tetap diterima
        // dan dilaporkan tanpa awalan.
        let shaka = dir.path().join("ffmpeg-shaka");
        write_fake_binary(&shaka, "ffmpeg version n8.1.2 Copyright (c) 2000-2025");
        assert_eq!(verify_version(&shaka).unwrap(), "8.1.2");

        let too_new = dir.path().join("ffmpeg-9");
        write_fake_binary(&too_new, "ffmpeg version 9.0.2 Copyright (c) 2000-2025");
        let error = verify_version(&too_new).unwrap_err();
        assert!(error.message.contains("tidak sesuai"), "{}", error.message);

        let broken = dir.path().join("ffmpeg-broken");
        write_fake_binary(&broken, "bukan ffmpeg");
        assert!(verify_version(&broken).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn installed_binaries_become_executable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("ffmpeg");
        fs::write(&target, b"stub").unwrap();
        make_executable(&target).unwrap();

        let mode = fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }
}
