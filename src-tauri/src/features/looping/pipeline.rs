//! Pipeline loop-merge: video latar diulang di atas playlist audio batch.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::core::ffmpeg::metadata::MetadataReader;
use crate::core::ffmpeg::runner::{FFmpegRunner, LogCallback};
use crate::core::logging::{LogLevel, LogRecord};
use crate::core::utils::format_clock;
use crate::error::AppError;
use crate::features::looping::audio::AudioNormalizer;
use crate::features::looping::builders::LoopCommandBuilder;
use crate::features::looping::models::{LoopRequest, LoopResult};
use crate::features::looping::naming::OutputNamer;
use crate::features::looping::playlist::PlaylistBuilder;
use crate::features::ProgressCallback;
use crate::storage::cache::AudioCache;
use crate::storage::temp::TempManager;

pub const PREPROCESS_MIN_SECONDS: f64 = 600.0;

/// Menggabungkan video latar, intro opsional, dan playlist audio.
pub struct LoopMergeService {
    pub runner: Arc<FFmpegRunner>,
    pub metadata: Arc<MetadataReader>,
    pub cache: Arc<AudioCache>,
    pub temp_manager: Arc<TempManager>,
    pub on_log: Option<LogCallback>,
    normalizer: AudioNormalizer,
    playlist: Mutex<PlaylistBuilder>,
    namer: OutputNamer,
}

impl LoopMergeService {
    pub fn new(
        runner: Arc<FFmpegRunner>,
        metadata: Arc<MetadataReader>,
        cache: Arc<AudioCache>,
        temp_manager: Arc<TempManager>,
        on_log: Option<LogCallback>,
    ) -> Self {
        let normalizer = AudioNormalizer::new(
            runner.clone(),
            metadata.clone(),
            cache.clone(),
            on_log.clone(),
        );
        Self {
            runner,
            metadata,
            cache,
            temp_manager,
            on_log,
            normalizer,
            playlist: Mutex::new(PlaylistBuilder::new()),
            namer: OutputNamer,
        }
    }

    /// Jalankan merge dan kembalikan file yang dihasilkan.
    pub async fn process(
        &self,
        request: &LoopRequest,
        on_progress: Option<ProgressCallback>,
    ) -> Result<LoopResult, AppError> {
        self.emit(
            String::from("Menggabungkan video latar dengan playlist audio"),
            LogLevel::Info,
            0,
        );

        let temp_dir = self.temp_manager.create("swaloka_merger")?;
        let result = self.run(request, &temp_dir, &on_progress).await;
        self.temp_manager.cleanup(&temp_dir);
        result
    }

    async fn run(
        &self,
        request: &LoopRequest,
        temp_dir: &Path,
        on_progress: &Option<ProgressCallback>,
    ) -> Result<LoopResult, AppError> {
        report(on_progress, 0.1);
        let normalized = self.normalizer.normalize(&request.audio_files).await?;
        report(on_progress, 0.3);

        let (order, playlist_files, timestamp_sources) = {
            let mut playlist = self.playlist.lock().expect("playlist terkunci");
            let order = playlist.build_order(normalized.len(), request.loop_count);
            let playlist_files = PlaylistBuilder::apply(&normalized, &order);
            let timestamp_sources = PlaylistBuilder::apply(&request.audio_files, &order);
            (order, playlist_files, timestamp_sources)
        };
        let _ = order;

        // Tracklist disusun di sini: urutan sudah pasti dan berkas ternormalisasi
        // sudah tersedia, jadi durasinya bisa diukur sekarang. Filenya baru
        // ditulis setelah video jadi agar tidak ada berkas yatim bila render gagal.
        let timestamp_lines = self
            .build_timestamp_lines(&timestamp_sources, &playlist_files)
            .await;

        let merged_audio = self.merge_audio(temp_dir, &playlist_files).await?;
        report(on_progress, 0.5);

        let output = match &request.output_path {
            Some(path) if !path.is_empty() => {
                let path = PathBuf::from(path);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                path
            }
            _ => self.namer.resolve(
                &request.output_dir,
                &request.output_name,
                &request.background_path,
                None,
            )?,
        };

        match &request.intro_path {
            None => {
                self.merge_background(&request.background_path, &merged_audio, &output)
                    .await?;
            }
            Some(intro) => {
                report(on_progress, 0.6);
                self.merge_with_intro(
                    request,
                    temp_dir,
                    &merged_audio,
                    &output,
                    intro,
                    on_progress,
                )
                .await?;
            }
        }

        report(on_progress, 1.0);
        let timestamp_path = self.write_timestamp_file(&output, timestamp_lines)?;
        self.emit(
            format!("Video selesai: {}", output.display()),
            LogLevel::Success,
            0,
        );

        Ok(LoopResult {
            output_path: output.to_string_lossy().to_string(),
            timestamp_path: timestamp_path.map(|value| value.to_string_lossy().to_string()),
            playlist: playlist_files,
        })
    }

    async fn merge_audio(
        &self,
        temp_dir: &Path,
        playlist_files: &[String],
    ) -> Result<PathBuf, AppError> {
        if playlist_files.is_empty() {
            return Err(AppError::new("no audio files to merge"));
        }
        let concat_file = temp_dir.join("audio_concat.txt");
        std::fs::write(
            &concat_file,
            LoopCommandBuilder::audio_concat_content(playlist_files),
        )?;
        let merged = temp_dir.join("audio_merged.m4a");
        self.emit(
            String::from("Menggabungkan berkas audio"),
            LogLevel::Info,
            0,
        );
        self.runner
            .run(
                LoopCommandBuilder::audio_concat_args(&concat_file, &merged),
                self.on_log.clone(),
                None,
                true,
                false,
            )
            .await?;
        Ok(merged)
    }

    async fn merge_background(
        &self,
        background: &str,
        audio: &Path,
        output: &Path,
    ) -> Result<(), AppError> {
        self.emit(
            String::from("Memutar ulang video latar mengikuti durasi audio"),
            LogLevel::Info,
            0,
        );
        self.runner
            .run(
                LoopCommandBuilder::stream_loop_merge_args(background, audio, output),
                self.on_log.clone(),
                None,
                true,
                false,
            )
            .await?;
        Ok(())
    }

    async fn merge_with_intro(
        &self,
        request: &LoopRequest,
        temp_dir: &Path,
        merged_audio: &Path,
        output: &Path,
        intro: &str,
        on_progress: &Option<ProgressCallback>,
    ) -> Result<(), AppError> {
        self.emit(
            String::from("Menyusun video pembuka lalu latar berulang"),
            LogLevel::Info,
            0,
        );
        let prelooped = self
            .preloop_background(&request.background_path, temp_dir)
            .await?;
        report(on_progress, 0.7);

        let intro_seconds = self.duration(Some(intro)).await;
        let prelooped_text = prelooped.to_string_lossy();
        let merged_text = merged_audio.to_string_lossy();
        let background_seconds = self.duration(Some(prelooped_text.as_ref())).await;
        let audio_seconds = self.duration(Some(merged_text.as_ref())).await;
        let (Some(intro_seconds), Some(background_seconds), Some(audio_seconds)) =
            (intro_seconds, background_seconds, audio_seconds)
        else {
            return Err(AppError::new(format!(
                "Durasi media tidak terbaca untuk penggabungan dengan intro (intro: {}, latar: {}, audio: {})",
                describe_duration(intro_seconds),
                describe_duration(background_seconds),
                describe_duration(audio_seconds)
            )));
        };

        let remaining = audio_seconds - intro_seconds;
        let background_loops = std::cmp::max(1, (remaining / background_seconds).ceil() as i64);
        let concat_file = temp_dir.join("video_concat.txt");
        std::fs::write(
            &concat_file,
            LoopCommandBuilder::video_concat_content(intro, &prelooped, background_loops),
        )?;

        self.runner
            .run(
                LoopCommandBuilder::concat_video_merge_args(&concat_file, merged_audio, output),
                self.on_log.clone(),
                None,
                true,
                false,
            )
            .await?;
        Ok(())
    }

    async fn preloop_background(
        &self,
        background: &str,
        temp_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        let duration = self.duration(Some(background)).await;
        match duration {
            None => Ok(PathBuf::from(background)),
            Some(value) if value <= 0.0 || value >= PREPROCESS_MIN_SECONDS => {
                Ok(PathBuf::from(background))
            }
            Some(value) => {
                let repeat_count = (PREPROCESS_MIN_SECONDS / value).ceil() as i64;
                let additional_loops = std::cmp::max(0, repeat_count - 1);
                let prelooped = temp_dir.join("background_preloop.mp4");
                self.emit(
                    format!("Mengulang video latar {additional_loops} kali agar mencapai 10 menit"),
                    LogLevel::Info,
                    0,
                );
                self.runner
                    .run(
                        LoopCommandBuilder::preloop_args(background, additional_loops, &prelooped),
                        self.on_log.clone(),
                        None,
                        true,
                        false,
                    )
                    .await?;
                Ok(prelooped)
            }
        }
    }

    /// Susun baris tracklist: judul dari berkas sumber, durasi dari berkas
    /// **ternormalisasi** (yang benar-benar digabung; baru dibuat ffmpeg jadi
    /// hampir selalu terbaca lewat jalur cepat `lofty`, tanpa spawn ffprobe).
    /// Dihitung saat urutan playlist baru pasti, bukan setelah video jadi.
    async fn build_timestamp_lines(
        &self,
        sources: &[String],
        duration_sources: &[String],
    ) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut accumulated = 0.0f64;
        for (index, source) in sources.iter().enumerate() {
            let stem = Path::new(source)
                .file_stem()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default();
            lines.push(format!("{} - {}", format_clock(accumulated), stem));
            let measured = duration_sources.get(index).unwrap_or(source);
            accumulated += self.duration(Some(measured)).await.unwrap_or(0.0);
        }
        lines
    }

    /// Tulis berkas tracklist di samping video hasil (hanya setelah video jadi).
    fn write_timestamp_file(
        &self,
        output: &Path,
        lines: Vec<String>,
    ) -> Result<Option<PathBuf>, AppError> {
        if lines.is_empty() {
            return Ok(None);
        }
        self.emit(String::from("Menulis berkas timestamp"), LogLevel::Info, 0);
        let stem = output
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let path = output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{stem}_timestamp.txt"));
        std::fs::write(&path, lines.join("\n"))?;
        self.emit(
            format!("Berkas timestamp dibuat: {}", path.display()),
            LogLevel::Success,
            1,
        );
        Ok(Some(path))
    }

    async fn duration(&self, path: Option<&str>) -> Option<f64> {
        let path = path?;
        self.metadata.read_duration(&self.runner, path).await
    }

    fn emit(&self, message: impl Into<String>, level: LogLevel, depth: usize) {
        if let Some(callback) = &self.on_log {
            callback(LogRecord::new(message, level, depth));
        }
    }
}

/// Teks durasi untuk pesan galat: "tidak terbaca" atau jumlah detiknya.
fn describe_duration(value: Option<f64>) -> String {
    match value {
        Some(seconds) => format!("{seconds:.1} dtk"),
        None => String::from("tidak terbaca"),
    }
}

/// Laporkan progres 0.0..1.0 bila callback tersedia.
fn report(on_progress: &Option<ProgressCallback>, value: f64) {
    if let Some(callback) = on_progress {
        callback(value);
    }
}
