//! Normalisasi audio input ke AAC, memakai hasil cache bila ada.

use std::path::Path;
use std::sync::Arc;

use crate::core::ffmpeg::metadata::MetadataReader;
use crate::core::ffmpeg::runner::{FFmpegRunner, LogCallback};
use crate::core::logging::{LogLevel, LogRecord};
use crate::error::AppError;
use crate::features::looping::builders::LoopCommandBuilder;
use crate::storage::cache::{resolve_path, AudioCache};

/// Container AAC yang aman disalin apa adanya. `.aac` mentah (ADTS) sengaja
/// TIDAK termasuk: ADTS tidak menyimpan timestamp, sehingga penyambungannya
/// memperkirakan durasi dari bitrate dan hasilnya bisa salah — atau bahkan
/// ditolak muxer sebagai "Error parsing ADTS frame header". File `.aac`
/// karena itu ikut dinormalisasi ke m4a.
const AAC_EXTENSIONS: [&str; 1] = [".m4a"];
/// Sample rate dan kanal hasil normalisasi. Penyambungan memakai `-c copy`,
/// jadi setiap berkas harus memakai timebase yang sama: `.m4a` dengan sample
/// rate lain ikut dinormalisasi, jika tidak durasi gabungannya meleset.
pub const NORMALIZED_SAMPLE_RATE: i64 = 44_100;
pub const NORMALIZED_CHANNELS: i64 = 2;

/// Memastikan setiap audio input berupa file AAC yang kompatibel `-c copy`.
pub struct AudioNormalizer {
    runner: Arc<FFmpegRunner>,
    metadata: Arc<MetadataReader>,
    cache: Arc<AudioCache>,
    on_log: Option<LogCallback>,
}

impl AudioNormalizer {
    pub fn new(
        runner: Arc<FFmpegRunner>,
        metadata: Arc<MetadataReader>,
        cache: Arc<AudioCache>,
        on_log: Option<LogCallback>,
    ) -> Self {
        Self {
            runner,
            metadata,
            cache,
            on_log,
        }
    }

    /// Berkas boleh disalin apa adanya hanya bila container AAC **dan**
    /// parameternya sudah sama dengan hasil normalisasi. `.m4a` dari sumber
    /// lain sering beda sample rate, dan perbedaan timebase itu yang membuat
    /// penyambungan `-c copy` menghasilkan durasi meleset.
    async fn can_reuse_directly(&self, path: &str) -> bool {
        if !Self::is_already_aac(path) {
            return false;
        }
        let metadata = self.metadata.read(&self.runner, path).await;
        metadata.readable
            && metadata.sample_rate == Some(NORMALIZED_SAMPLE_RATE)
            && metadata.channels == Some(NORMALIZED_CHANNELS)
    }

    pub fn is_already_aac(path: impl AsRef<Path>) -> bool {
        path.as_ref()
            .extension()
            .map(|value| {
                let ext = format!(".{}", value.to_string_lossy().to_lowercase());
                AAC_EXTENSIONS.contains(&ext.as_str())
            })
            .unwrap_or(false)
    }

    /// Kembalikan satu path ternormalisasi per input, dengan urutan yang sama.
    pub async fn normalize(&self, files: &[String]) -> Result<Vec<String>, AppError> {
        if files.is_empty() {
            return Ok(Vec::new());
        }

        let source_paths: Vec<String> = files
            .iter()
            .map(|path| resolve_path(Path::new(path)).to_string_lossy().to_string())
            .collect();

        let mut results: Vec<Option<String>> = vec![None; source_paths.len()];
        let mut pending: Vec<(usize, String, String)> = Vec::new();

        for (index, source) in source_paths.iter().enumerate() {
            if self.can_reuse_directly(source).await {
                results[index] = Some(source.clone());
            } else if let Some(cached) = self.cache.lookup(Path::new(source)) {
                results[index] = Some(cached.to_string_lossy().to_string());
            } else {
                let target = self.cache.target_path(Path::new(source));
                pending.push((index, source.clone(), target.to_string_lossy().to_string()));
            }
        }

        if !pending.is_empty() {
            self.convert(&pending).await?;
            for (index, source, output) in &pending {
                results[*index] = Some(output.clone());
                self.cache.record(Path::new(source), Path::new(output))?;
            }
        }

        if results.iter().any(Option::is_none) {
            return Err(AppError::new("failed to normalize all audio files"));
        }
        Ok(results.into_iter().flatten().collect())
    }

    async fn convert(&self, pending: &[(usize, String, String)]) -> Result<(), AppError> {
        let entries: Vec<(String, String)> = pending
            .iter()
            .map(|(_, source, output)| (source.clone(), output.clone()))
            .collect();
        // Folder cache bisa saja belum ada (mis. habis dibersihkan); pastikan
        // ada sebelum ffmpeg diminta menulis ke dalamnya.
        for (_, output) in &entries {
            if let Some(parent) = Path::new(output).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        self.emit(
            format!("Menyiapkan {} berkas audio", entries.len()),
            LogLevel::Info,
            0,
        );
        self.emit(String::from("Menormalkan audio ke AAC"), LogLevel::Info, 1);
        self.runner
            .run(
                LoopCommandBuilder::normalize_batch_args(&entries),
                self.on_log.clone(),
                None,
                true,
                false,
            )
            .await?;
        self.emit(
            String::from("Audio selesai dinormalkan"),
            LogLevel::Success,
            1,
        );
        Ok(())
    }

    fn emit(&self, message: impl Into<String>, level: LogLevel, depth: usize) {
        if let Some(callback) = &self.on_log {
            callback(LogRecord::new(message, level, depth));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aac_extensions() {
        assert!(AudioNormalizer::is_already_aac("/x/y.m4a"));
        assert!(AudioNormalizer::is_already_aac("/x/y.M4A"));
        assert!(!AudioNormalizer::is_already_aac("/x/y.mp3"));
        // ADTS mentah tidak punya timestamp: harus dinormalisasi dulu.
        assert!(!AudioNormalizer::is_already_aac("/x/y.aac"));
    }
}
