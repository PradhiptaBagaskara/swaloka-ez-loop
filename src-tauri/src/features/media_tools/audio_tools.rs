//! Audio tools: penggabungan dan pencampuran overlay.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::ffmpeg::runner::{FFmpegRunner, LogCallback};
use crate::core::logging::{LogLevel, LogRecord};
use crate::error::AppError;
use crate::features::looping::audio::AudioNormalizer;
use crate::features::media_tools::commands::MediaCommandBuilder;
use crate::features::media_tools::models::AudioOverlay;
use crate::storage::temp::TempManager;

/// Menggabungkan dan mencampur file audio.
pub struct AudioToolsService {
    pub runner: Arc<FFmpegRunner>,
    pub temp_manager: Arc<TempManager>,
    normalizer: Option<Arc<AudioNormalizer>>,
    on_log: Option<LogCallback>,
}

impl AudioToolsService {
    pub fn new(
        runner: Arc<FFmpegRunner>,
        temp_manager: Arc<TempManager>,
        normalizer: Option<Arc<AudioNormalizer>>,
        on_log: Option<LogCallback>,
    ) -> Self {
        Self {
            runner,
            temp_manager,
            normalizer,
            on_log,
        }
    }

    /// Gabungkan file audio secara berurutan ke format output.
    pub async fn concat(
        &self,
        audio_paths: &[String],
        output_path: &str,
    ) -> Result<String, AppError> {
        if audio_paths.is_empty() {
            return Err(AppError::new("no audio files to concatenate"));
        }
        let sources = self.prepare(audio_paths).await?;
        let temp_dir = self.temp_manager.create("swaloka_tools")?;
        let result = async {
            let concat_file = temp_dir.join("concat_list.txt");
            std::fs::write(&concat_file, MediaCommandBuilder::concat_content(&sources))?;
            self.emit(
                format!("Menggabungkan {} berkas audio", sources.len()),
                LogLevel::Info,
                0,
            );
            self.runner
                .run(
                    MediaCommandBuilder::concat_audio_args(&concat_file, output_path),
                    self.on_log.clone(),
                    None,
                    true,
                    false,
                )
                .await?;
            Ok(output_path.to_string())
        }
        .await;
        self.temp_manager.cleanup(&temp_dir);
        result
    }

    /// Re-encode file unik sekali, lalu gabungkan dengan stream copy.
    pub async fn concat_optimized(
        &self,
        audio_paths: &[String],
        unique_paths: &[String],
        output_path: &str,
    ) -> Result<String, AppError> {
        let temp_dir = self.temp_manager.create("swaloka_optimized")?;
        let suffix = Path::new(output_path)
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy()))
            .unwrap_or_default();
        let result = async {
            let outputs: Vec<String> = (0..unique_paths.len())
                .map(|index| {
                    temp_dir
                        .join(format!("audio_part_{index}{suffix}"))
                        .to_string_lossy()
                        .to_string()
                })
                .collect();
            self.emit(
                format!("Meng-encode ulang {} berkas unik", unique_paths.len()),
                LogLevel::Info,
                0,
            );
            self.runner
                .run(
                    MediaCommandBuilder::batch_reencode_audio_args(
                        unique_paths,
                        &outputs,
                        output_path,
                    ),
                    self.on_log.clone(),
                    None,
                    true,
                    false,
                )
                .await?;

            let mapping: HashMap<String, String> = unique_paths
                .iter()
                .cloned()
                .zip(outputs.iter().cloned())
                .collect();
            let mapped: Vec<String> = audio_paths
                .iter()
                .filter_map(|path| mapping.get(path).cloned())
                .collect();
            let concat_file = temp_dir.join("concat_list.txt");
            std::fs::write(&concat_file, MediaCommandBuilder::concat_content(&mapped))?;
            self.emit(
                String::from("Menggabungkan audio hasil encode ulang"),
                LogLevel::Info,
                1,
            );
            let mut copy_args: Vec<String> = vec![
                String::from("-y"),
                String::from("-f"),
                String::from("concat"),
            ];
            copy_args.extend(["-safe", "0", "-i"].iter().map(|value| value.to_string()));
            copy_args.push(concat_file.to_string_lossy().to_string());
            copy_args.extend(
                ["-c", "copy", output_path]
                    .iter()
                    .map(|value| value.to_string()),
            );
            self.runner
                .run(copy_args, self.on_log.clone(), None, true, false)
                .await?;
            Ok(output_path.to_string())
        }
        .await;
        self.temp_manager.cleanup(&temp_dir);
        result
    }

    /// Campur beberapa trek overlay menjadi satu output memakai amerge.
    pub async fn apply_overlays(
        &self,
        overlays: &[AudioOverlay],
        output_path: &str,
    ) -> Result<String, AppError> {
        if overlays.is_empty() {
            return Err(AppError::new("no overlays provided"));
        }
        let specs = overlay_specs(overlays);
        self.emit(
            format!("Menerapkan {} overlay", overlays.len()),
            LogLevel::Info,
            0,
        );
        self.runner
            .run(
                MediaCommandBuilder::overlay_merge_args(&specs, output_path),
                self.on_log.clone(),
                None,
                true,
                false,
            )
            .await?;
        Ok(output_path.to_string())
    }

    /// Tulis overlay yang sudah di-premix menjadi preset WAV yang bisa dipakai ulang.
    pub async fn export_overlay_preset(
        &self,
        overlays: &[AudioOverlay],
        output_path: &str,
    ) -> Result<String, AppError> {
        if overlays.is_empty() {
            return Err(AppError::new("no overlays provided"));
        }
        let temp_dir = self.temp_manager.create("swaloka_preset")?;
        let result = async {
            let mix = self.premix_overlays(overlays, &temp_dir).await?;
            std::fs::copy(&mix, output_path)?;
            Ok(output_path.to_string())
        }
        .await;
        self.temp_manager.cleanup(&temp_dir);
        result
    }

    /// Ulang overlay di atas rangkaian audio dasar lalu campur keduanya.
    pub async fn process_with_overlays(
        &self,
        base_audios: &[String],
        overlays: &[AudioOverlay],
        output_path: &str,
    ) -> Result<String, AppError> {
        if base_audios.is_empty() {
            return Err(AppError::new("no base audios provided"));
        }
        if overlays.is_empty() {
            let unique = dedupe(base_audios);
            if unique.len() < base_audios.len() {
                return self
                    .concat_optimized(base_audios, &unique, output_path)
                    .await;
            }
            return self.concat(base_audios, output_path).await;
        }

        let temp_dir = self.temp_manager.create("swaloka_audio")?;
        let result = async {
            let overlay_mix: PathBuf = if overlays.len() == 1 && overlays[0].volume == 1.0 {
                PathBuf::from(&overlays[0].path)
            } else {
                self.premix_overlays(overlays, &temp_dir).await?
            };

            let concat_file = temp_dir.join("base_audios.txt");
            std::fs::write(
                &concat_file,
                MediaCommandBuilder::concat_content(base_audios),
            )?;
            self.emit(
                format!("Mencampur {} audio utama dengan overlay", base_audios.len()),
                LogLevel::Info,
                0,
            );
            self.runner
                .run(
                    MediaCommandBuilder::base_with_overlay_args(
                        &concat_file,
                        &overlay_mix,
                        output_path,
                    ),
                    self.on_log.clone(),
                    None,
                    true,
                    false,
                )
                .await?;
            Ok(output_path.to_string())
        }
        .await;
        self.temp_manager.cleanup(&temp_dir);
        result
    }

    async fn premix_overlays(
        &self,
        overlays: &[AudioOverlay],
        temp_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        let mix_path = temp_dir.join("overlay_mix.wav");
        let specs = overlay_specs(overlays);
        self.emit(
            format!("Mencampur {} overlay", overlays.len()),
            LogLevel::Info,
            0,
        );
        self.runner
            .run(
                MediaCommandBuilder::premix_overlays_args(&specs, &mix_path),
                self.on_log.clone(),
                None,
                true,
                false,
            )
            .await?;
        Ok(mix_path)
    }

    /// Pakai salinan ternormalisasi dari cache bila normalizer tersedia.
    async fn prepare(&self, audio_paths: &[String]) -> Result<Vec<String>, AppError> {
        match &self.normalizer {
            None => Ok(audio_paths.to_vec()),
            Some(normalizer) => normalizer.normalize(audio_paths).await,
        }
    }

    fn emit(&self, message: impl Into<String>, level: LogLevel, depth: usize) {
        if let Some(callback) = &self.on_log {
            callback(LogRecord::new(message, level, depth));
        }
    }
}

fn overlay_specs(overlays: &[AudioOverlay]) -> Vec<(String, f64)> {
    overlays
        .iter()
        .map(|overlay| (overlay.path.clone(), overlay.volume))
        .collect()
}

fn dedupe(values: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for value in values {
        if !seen.contains(value) {
            seen.push(value.clone());
        }
    }
    seen
}
