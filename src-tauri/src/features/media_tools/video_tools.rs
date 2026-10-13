//! Video tools: encoding standar siaran dan pembacaan informasi video.

use std::path::Path;
use std::sync::Arc;

use crate::core::ffmpeg::metadata::MetadataReader;
use crate::core::ffmpeg::runner::{FFmpegRunner, LogCallback};
use crate::core::ffmpeg::youtube::YouTubeChecker;
use crate::core::logging::{LogLevel, LogRecord};
use crate::error::AppError;
use crate::features::media_tools::commands::{
    LiveStandardArgs, LIVE_COLOR_METADATA_FLAGS, MIN_LIVE_BITRATE_MBPS,
};
use crate::features::media_tools::models::{LiveStandardOptions, VideoInfo};
use crate::storage::temp::TempManager;

/// Meng-encode video ke standar siaran dan membaca informasinya.
pub struct VideoToolsService {
    pub runner: Arc<FFmpegRunner>,
    pub metadata: Arc<MetadataReader>,
    pub temp_manager: Arc<TempManager>,
    youtube: YouTubeChecker,
    on_log: Option<LogCallback>,
}

impl VideoToolsService {
    pub fn new(
        runner: Arc<FFmpegRunner>,
        metadata: Arc<MetadataReader>,
        temp_manager: Arc<TempManager>,
        on_log: Option<LogCallback>,
    ) -> Self {
        Self {
            runner,
            metadata,
            temp_manager,
            youtube: YouTubeChecker,
            on_log,
        }
    }

    // -- standar siaran ----------------------------------------------------

    /// Encode satu video ke standar siaran YouTube, memakai resolusi dan frame
    /// rate video sumbernya sendiri.
    pub async fn encode_live(
        &self,
        options: &LiveStandardOptions,
    ) -> Result<Option<String>, AppError> {
        if let Some(bitrate) = options.bitrate_mbps {
            if bitrate < MIN_LIVE_BITRATE_MBPS {
                return Err(AppError::new(format!(
                    "Bitrate minimum siaran {MIN_LIVE_BITRATE_MBPS} Mbps"
                )));
            }
        }

        // Resolusi dan fps mengikuti sumber, jadi metadata wajib terbaca.
        let source = self.metadata.read(&self.runner, &options.video_path).await;
        if !source.readable {
            return Err(AppError::new(format!(
                "Metadata video tidak terbaca: {}",
                source.error.unwrap_or_else(|| options.video_path.clone())
            )));
        }
        let fps = source.fps_value();

        let name = file_name(&options.video_path);
        self.emit(
            format!("Meng-encode {name} ke standar siaran"),
            LogLevel::Info,
            0,
        );
        self.runner
            .run(
                self.live_args(options, fps),
                self.on_log.clone(),
                None,
                true,
                false,
            )
            .await?;
        Ok(Some(options.output_path.clone()))
    }

    fn live_args(&self, options: &LiveStandardOptions, fps: f64) -> Vec<String> {
        let bitrate = LiveStandardArgs::resolve_bitrate(options.bitrate_mbps);
        let mut args = vec![
            String::from("-y"),
            String::from("-i"),
            options.video_path.clone(),
            String::from("-vf"),
            LiveStandardArgs::normalize_filter(fps),
        ];
        args.extend(LiveStandardArgs::video_args(
            options.hw_encoder.as_deref(),
            bitrate,
            LiveStandardArgs::gop_frames(fps),
        ));
        args.extend(["-movflags", "+faststart"].iter().map(|v| v.to_string()));
        args.extend(
            LIVE_COLOR_METADATA_FLAGS
                .iter()
                .map(|value| value.to_string()),
        );
        // Audio siaran: AAC 128 kbps, 44,1 kHz, stereo.
        args.extend(
            ["-c:a", "aac", "-b:a", "128k", "-ar", "44100", "-ac", "2"]
                .iter()
                .map(|v| v.to_string()),
        );
        args.push(options.output_path.clone());
        args
    }

    /// Kumpulkan info file, metadata, dan hasil standar YouTube.
    pub async fn info(&self, path: &str) -> Result<VideoInfo, AppError> {
        let metadata = self.metadata.read(&self.runner, path).await;
        let size = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
        let youtube = self.youtube.check(&metadata);
        Ok(VideoInfo {
            path: path.to_string(),
            size_bytes: size,
            metadata: Some(metadata),
            youtube: Some(youtube),
        })
    }

    // -- helper ------------------------------------------------------------

    fn emit(&self, message: impl Into<String>, level: LogLevel, depth: usize) {
        if let Some(callback) = &self.on_log {
            callback(LogRecord::new(message, level, depth));
        }
    }
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ffmpeg::metadata::MetadataReader;
    use crate::core::ffmpeg::runner::FFmpegRunner;
    use crate::storage::temp::TempManager;

    fn service() -> VideoToolsService {
        VideoToolsService::new(
            Arc::new(FFmpegRunner::new("ffmpeg", None)),
            Arc::new(MetadataReader::new(false)),
            Arc::new(TempManager::new(std::env::temp_dir())),
            None,
        )
    }

    fn options(bitrate: Option<f64>) -> LiveStandardOptions {
        LiveStandardOptions {
            video_path: String::from("/berkas/tidak/ada.mp4"),
            output_path: String::from("/tmp/keluaran.mp4"),
            bitrate_mbps: bitrate,
            hw_encoder: None,
        }
    }

    #[tokio::test]
    async fn rejects_bitrate_below_the_floor() {
        let error = service()
            .encode_live(&options(Some(1.0)))
            .await
            .expect_err("bitrate di bawah 3 Mbps harus ditolak");
        assert!(
            error.message.contains("minimum siaran 3"),
            "pesan harus menyebut batasnya, dapat: {}",
            error.message
        );
    }
}
