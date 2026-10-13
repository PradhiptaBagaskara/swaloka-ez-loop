//! Tipe data untuk media tools.

use serde::{Deserialize, Serialize};

use crate::core::ffmpeg::metadata::VideoMetadata;
use crate::core::ffmpeg::youtube::YouTubeResult;

fn one_f64() -> f64 {
    1.0
}

/// Trek overlay beserta volume mix-nya (0.0–1.0).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOverlay {
    pub path: String,
    #[serde(default = "one_f64")]
    pub volume: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStandardOptions {
    pub video_path: String,
    pub output_path: String,
    /// Bitrate CBR (Mbps). Bila kosong, dipakai bawaan aplikasi.
    #[serde(default)]
    pub bitrate_mbps: Option<f64>,
    /// Encoder pilihan pengguna; `None`/`libx264` berarti libx264.
    #[serde(default)]
    pub hw_encoder: Option<String>,
}

/// Metadata plus hasil validasi YouTube untuk tampilan info.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub path: String,
    pub size_bytes: u64,
    pub metadata: Option<VideoMetadata>,
    pub youtube: Option<YouTubeResult>,
}
