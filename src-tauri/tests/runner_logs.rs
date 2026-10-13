//! Verifikasi bahwa output mentah FFmpeg tidak lagi masuk ke log UI.
//!
//! Dua kontrak yang dijaga di sini:
//! 1. Run yang sukses tidak mengirim satu pun baris output FFmpeg ke UI.
//! 2. Run yang gagal mengirim tepat satu laporan error yang memuat ekornya.
//!
//! Test dilewati (bukan gagal) bila FFmpeg tidak terpasang di mesin ini.

use std::sync::{Arc, Mutex};

use swaloka_lib::core::ffmpeg::discovery::which;
use swaloka_lib::core::ffmpeg::runner::{FFmpegRunner, LogCallback};
use swaloka_lib::core::logging::{LogLevel, LogRecord};

fn collector() -> (LogCallback, Arc<Mutex<Vec<LogRecord>>>) {
    let records: Arc<Mutex<Vec<LogRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&records);
    let callback: LogCallback = Arc::new(move |record| {
        sink.lock().expect("kumpulan log terkunci").push(record);
    });
    (callback, records)
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[tokio::test]
async fn successful_run_emits_no_ffmpeg_output() {
    let Some(ffmpeg) = which("ffmpeg") else {
        return;
    };
    let runner = FFmpegRunner::new(ffmpeg, None);
    let (callback, records) = collector();

    let result = runner
        .run(
            args(&[
                "-hide_banner",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.3",
                "-f",
                "null",
                "-",
            ]),
            Some(callback),
            None,
            true,
            false,
        )
        .await
        .expect("perintah contoh harus sukses");

    assert!(result.ok());
    assert!(
        records.lock().expect("log terkunci").is_empty(),
        "log UI harus kosong untuk run yang sukses, bukan berisi output FFmpeg"
    );
}

#[tokio::test]
async fn failed_run_emits_exactly_one_error_report() {
    let Some(ffmpeg) = which("ffmpeg") else {
        return;
    };
    let runner = FFmpegRunner::new(ffmpeg, None);
    let (callback, records) = collector();

    let error = runner
        .run(
            args(&[
                "-hide_banner",
                "-i",
                "/berkas/tidak/ada.mp4",
                "-f",
                "null",
                "-",
            ]),
            Some(callback),
            None,
            true,
            false,
        )
        .await
        .expect_err("input yang tidak ada harus gagal");

    assert!(error.message.contains("gagal (kode"));

    let records = records.lock().expect("log terkunci");
    assert_eq!(records.len(), 1, "hanya satu laporan error yang dikirim");
    assert_eq!(records[0].level, LogLevel::Error);
    assert!(records[0].message.starts_with("ffmpeg gagal (kode"));
    assert!(
        records[0].message.contains("tidak/ada.mp4"),
        "laporan harus memuat bukti dari FFmpeg, dapat: {}",
        records[0].message
    );
}
