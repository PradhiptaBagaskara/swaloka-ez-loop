//! Regresi: setelah "Bersihkan cache", run berikutnya harus tetap bisa menulis
//! ke folder cache — bukan gagal karena foldernya ikut terhapus.
//!
//! Test yang memakai FFmpeg dilewati (bukan gagal) bila FFmpeg tidak terpasang.

use std::path::Path;
use std::sync::Arc;

use swaloka_lib::core::ffmpeg::discovery::which;
use swaloka_lib::core::ffmpeg::runner::FFmpegRunner;
use swaloka_lib::core::ffmpeg::MetadataReader;
use swaloka_lib::features::looping::audio::AudioNormalizer;
use swaloka_lib::storage::cache::AudioCache;
use swaloka_lib::storage::project::Project;

fn project_in(dir: &tempfile::TempDir) -> Project {
    let project = Project::new("Uji", dir.path().to_string_lossy());
    project.ensure_structure().expect("struktur proyek");
    project
}

#[test]
fn clearing_cache_keeps_the_cache_directory() {
    let dir = tempfile::tempdir().unwrap();
    let cache = AudioCache::new(project_in(&dir));
    let source = dir.path().join("lagu.mp3");
    std::fs::write(&source, b"x").unwrap();
    let target = cache.target_path(&source);
    std::fs::write(&target, b"y").unwrap();

    let result = cache.clear();

    assert_eq!(result.removed_audio, 1);
    assert!(
        target.parent().expect("target punya folder").exists(),
        "folder cache harus tetap ada setelah dibersihkan"
    );
}

#[tokio::test]
async fn normalizing_after_cache_clear_still_writes_the_target() {
    let Some(ffmpeg) = which("ffmpeg") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let project = project_in(&dir);
    let source = dir.path().join("lagu.wav");
    let runner = Arc::new(FFmpegRunner::new(ffmpeg, None));

    // Siapkan satu berkas audio sumber dengan FFmpeg itu sendiri.
    let make: Vec<String> = [
        "-hide_banner",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=0.4",
        "-c:a",
        "pcm_s16le",
    ]
    .iter()
    .map(|value| value.to_string())
    .chain(std::iter::once(source.to_string_lossy().to_string()))
    .collect();
    runner
        .run(make, None, None, true, false)
        .await
        .expect("gagal membuat berkas audio uji");

    let cache = Arc::new(AudioCache::new(project));
    cache.clear();

    let normalizer = AudioNormalizer::new(runner, Arc::new(MetadataReader::new(true)), cache, None);
    let normalized = normalizer
        .normalize(&[source.to_string_lossy().to_string()])
        .await
        .expect("normalisasi sesudah bersih cache harus berhasil");

    assert!(
        Path::new(&normalized[0]).exists(),
        "berkas hasil normalisasi harus ada di folder cache"
    );
}
