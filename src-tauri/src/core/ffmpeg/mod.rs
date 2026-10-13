//! Mesin FFmpeg: discovery, eksekusi, encoder, metadata, estimasi, standar.

pub mod discovery;
pub mod disk;
pub mod encoders;
pub mod install;
pub mod metadata;
pub mod runner;
pub mod youtube;

pub use discovery::{FFmpegLocator, BUNDLED_FFMPEG_ONLY};
pub use disk::{DiskEstimate, DiskEstimateInput, DiskEstimator};
pub use encoders::{EncoderDetector, HardwareEncoder};
pub use install::{
    expected_bytes, install_dir, is_installed, plan_for, InstallCallback, InstallProgress,
    Installer, PINNED_VERSION,
};
pub use metadata::{MetadataReader, VideoMetadata};
pub use runner::{FFmpegError, FFmpegRunner, LogCallback, ProcessResult};
pub use youtube::{YouTubeChecker, YouTubeResult, YouTubeStatus};
