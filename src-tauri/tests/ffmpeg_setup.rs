//! Uji pemasangan FFmpeg yang sungguh mengunduh dari internet.
//!
//! Dijalankan manual karena mengunduh puluhan MB:
//! `cargo test --test ffmpeg_setup -- --ignored --nocapture`

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use swaloka_lib::core::ffmpeg::install::{
    self, download_file, plan_for, verify_digest, BinaryKind, InstallCallback, InstallProgress,
    InstallStep,
};

fn collector() -> (InstallCallback, Arc<Mutex<Vec<InstallProgress>>>) {
    let seen: Arc<Mutex<Vec<InstallProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let callback: InstallCallback = Arc::new(move |progress| {
        sink.lock().expect("progres terkunci").push(progress);
    });
    (callback, seen)
}

fn size_of(path: &Path) -> u64 {
    std::fs::metadata(path).expect("berkas ada").len()
}

/// Pemasangan lengkap di platform ini: unduh, pasang, lalu pastikan binernya
/// benar-benar jalan dan versinya sesuai pin.
#[test]
#[ignore = "mengunduh FFmpeg dari internet"]
fn installs_the_pinned_build_for_this_platform() {
    let plan = plan_for(std::env::consts::OS, std::env::consts::ARCH)
        .expect("platform ini punya rencana unduhan");
    let dir = tempfile::tempdir().expect("folder sementara");
    let (progress, seen) = collector();

    let version = install::install(&plan, dir.path(), &progress, &AtomicBool::new(false))
        .expect("pemasangan selesai tanpa error");
    assert_eq!(version, install::PINNED_VERSION, "versi harus sesuai pin");
    println!(
        "terpasang FFmpeg {version} dari {} byte unduhan",
        plan.total_bytes()
    );

    for kind in [BinaryKind::Ffmpeg, BinaryKind::Ffprobe] {
        let path = dir.path().join(kind.file_name());
        assert!(path.is_file(), "{kind:?} harus ada setelah pemasangan");
        assert!(
            size_of(&path) > 1_000_000,
            "{kind:?} terlalu kecil, kemungkinan unduhan gagal"
        );
        if cfg!(unix) {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "{kind:?} harus executable");
        }
    }

    // Encoder yang dipakai aplikasi harus ada di build ini.
    let ffmpeg = dir.path().join(BinaryKind::Ffmpeg.file_name());
    let encoders = std::process::Command::new(&ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .expect("daftar encoder bisa dibaca");
    let text = String::from_utf8_lossy(&encoders.stdout);
    assert!(
        text.contains("libx264"),
        "build FFmpeg bawaan harus menyertakan libx264"
    );

    // Tidak boleh ada berkas sementara yang tertinggal.
    let leftovers: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".part") || name == "unpack")
        .collect();
    assert!(leftovers.is_empty(), "sisa berkas sementara: {leftovers:?}");

    // Progres harus naik monoton sampai selesai.
    let records = seen.lock().expect("progres terkunci");
    assert!(records.len() > 1, "progres harus dilaporkan bertahap");
    let mut previous = 0.0;
    for record in records.iter() {
        assert!(
            record.percent >= previous - 1e-9,
            "progres tidak boleh mundur: {} lalu {}",
            previous,
            record.percent
        );
        assert!(record.percent <= 1.0, "progres melebihi 100%");
        previous = record.percent;
    }
    let last = records.last().expect("ada progres terakhir");
    assert_eq!(last.phase, "done");
    assert!((last.percent - 1.0).abs() < 1e-9, "harus berakhir di 100%");
    assert_eq!(last.downloaded_bytes, plan.total_bytes());
}

/// Jalur arsip Windows diuji dari platform ini: `sevenz-rust` murni Rust, jadi
/// unduhan dan ekstraksinya bisa dibuktikan walau `.exe`-nya tidak bisa
/// dijalankan di sini.
#[test]
#[ignore = "mengunduh arsip FFmpeg Windows dari internet"]
fn extracts_the_windows_archive_without_running_it() {
    let plan = plan_for("windows", "x86_64").expect("rencana Windows terbentuk");
    let dir = tempfile::tempdir().expect("folder sementara");
    let archive = dir.path().join("windows-build.7z");

    let Some(InstallStep::SevenZip(file)) = plan.steps.first() else {
        panic!("rencana Windows harus memakai arsip");
    };

    download_file(&file.url, &archive, &AtomicBool::new(false), &|_| {}).expect("arsip terunduh");
    assert_eq!(size_of(&archive), file.bytes, "ukuran arsip harus persis");
    verify_digest(&archive, file.sha256.as_deref()).expect("hash arsip cocok");

    install::extract_seven_zip(&archive, dir.path(), &file.members).expect("arsip terekstrak");

    for name in &file.members {
        let extracted = dir.path().join(name);
        assert!(extracted.is_file(), "{name} harus ada hasil ekstraksi");
        assert!(
            size_of(&extracted) > 10_000_000,
            "{name} terlalu kecil untuk biner FFmpeg"
        );
    }
}

/// Hash yang salah harus ditolak, dan berkas rusaknya dibuang.
#[test]
#[ignore = "mengunduh berkas kecil dari internet"]
fn a_wrong_hash_is_rejected() {
    let dir = tempfile::tempdir().expect("folder sementara");
    let file = dir.path().join("payload.part");

    download_file(
        "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-8.1.2-essentials_build.7z.sha256",
        &file,
        &AtomicBool::new(false),
        &|_| {},
    )
    .expect("berkas kecil terunduh");

    let wrong = "0".repeat(64);
    assert!(verify_digest(&file, Some(&wrong)).is_err());
    assert!(!file.exists(), "berkas dengan hash salah harus dibuang");
}
