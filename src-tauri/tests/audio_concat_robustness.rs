//! Audio ADTS mentah (`.aac`) tetap harus menghasilkan output yang benar.
//!
//! Sebelumnya file `.aac` disalin apa adanya karena dianggap sudah AAC, lalu
//! penyambungan `-c copy` gagal di muxer (exit 183, "Error parsing ADTS frame
//! header") atau menghasilkan durasi yang salah. Tes ini menjaga jalur
//! normalisasi dan penyambungan tetap benar untuk sumber seperti itu.
//!
//! Test dilewati (bukan gagal) bila FFmpeg tidak terpasang.

use std::path::Path;
use std::sync::Arc;

use swaloka_lib::core::ffmpeg::discovery::which;
use swaloka_lib::core::ffmpeg::{FFmpegLocator, FFmpegRunner, MetadataReader};
use swaloka_lib::features::looping::{AudioNormalizer, LoopMergeService, LoopRequest};
use swaloka_lib::storage::cache::AudioCache;
use swaloka_lib::storage::project::Project;
use swaloka_lib::storage::temp::TempManager;

fn run_ffmpeg(ffmpeg: &str, args: &[&str]) {
    let status = std::process::Command::new(ffmpeg)
        .args(args)
        .status()
        .expect("ffmpeg bisa dijalankan");
    assert!(status.success(), "ffmpeg gagal: {args:?}");
}

#[tokio::test]
async fn raw_adts_audio_keeps_its_duration_through_normalize_and_concat() {
    let Some(ffmpeg) = which("ffmpeg") else {
        return;
    };

    let directory = tempfile::tempdir().unwrap();
    let project = Project::new("Uji Audio", directory.path().to_string_lossy().to_string());
    project.ensure_structure().unwrap();

    // Tiga detik ADTS mentah, tiga detik mp3, dan latar dua detik.
    let raw_aac = directory.path().join("lagu.aac");
    run_ffmpeg(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=3",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-f",
            "adts",
            raw_aac.to_str().unwrap(),
        ],
    );
    let mp3 = directory.path().join("lagu2.mp3");
    run_ffmpeg(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=520:duration=3",
            "-c:a",
            "libmp3lame",
            mp3.to_str().unwrap(),
        ],
    );
    let background = directory.path().join("bg.mp4");
    run_ffmpeg(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=navy:s=320x240:r=30:d=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            background.to_str().unwrap(),
        ],
    );

    let locator = FFmpegLocator::new(None);
    let ffprobe = locator.resolve_ffprobe(Some(&ffmpeg));
    let runner = Arc::new(FFmpegRunner::new(ffmpeg.clone(), Some(ffprobe)));
    let metadata = Arc::new(MetadataReader::new(true));
    let cache = Arc::new(AudioCache::new(project.clone()));
    let temp = Arc::new(TempManager::new(project.temp_dir()));
    let normalizer = Arc::new(AudioNormalizer::new(
        runner.clone(),
        metadata.clone(),
        cache.clone(),
        None,
    ));
    let service = LoopMergeService::new(runner.clone(), metadata.clone(), cache, temp, None);

    let normalized = normalizer
        .normalize(&[
            raw_aac.to_string_lossy().to_string(),
            mp3.to_string_lossy().to_string(),
        ])
        .await
        .expect("normalisasi berhasil");

    // ADTS mentah harus dikonversi, bukan disalin.
    assert_ne!(normalized[0], raw_aac.to_string_lossy().to_string());
    assert!(
        normalized[0].ends_with(".m4a"),
        "hasil normalisasi m4a: {}",
        normalized[0]
    );
    assert!(Path::new(&normalized[0]).is_file());

    // Berkas m4a 48 kHz tidak boleh disalin apa adanya: timebase-nya berbeda.
    let wide = directory.path().join("lagu3.m4a");
    run_ffmpeg(
        &ffmpeg,
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=660:duration=3",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-ar",
            "48000",
            "-ac",
            "2",
            wide.to_str().unwrap(),
        ],
    );
    let wide_normalized = normalizer
        .normalize(&[wide.to_string_lossy().to_string()])
        .await
        .expect("normalisasi berhasil");
    assert_ne!(
        wide_normalized[0],
        wide.to_string_lossy().to_string(),
        "m4a 48 kHz harus dinormalisasi ke 44,1 kHz"
    );
    let wide_metadata = metadata.read(&runner, &wide_normalized[0]).await;
    assert_eq!(wide_metadata.sample_rate, Some(44_100));

    let mut audio_files = normalized.clone();
    audio_files.push(wide_normalized[0].clone());

    let request = LoopRequest {
        background_path: background.to_string_lossy().to_string(),
        audio_files,
        output_dir: project.output_dir().to_string_lossy().to_string(),
        output_name: String::from("gabungan"),
        loop_count: 1,
        intro_path: None,
        output_path: None,
    };
    let result = service
        .process(&request, None)
        .await
        .expect("merge berhasil tanpa galat muxer");

    let duration = metadata
        .read_duration(&runner, &result.output_path)
        .await
        .expect("durasi output terbaca");
    assert!(
        (duration - 9.0).abs() < 0.6,
        "tiga lagu tiga detik harus menjadi sekitar sembilan detik, dapat {duration}"
    );
}
