//! Tipe data untuk pipeline looping.

use serde::{Deserialize, Serialize};

use crate::core::ffmpeg::youtube::YouTubeResult;

/// Input untuk satu merge looping.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopRequest {
    pub background_path: String,
    #[serde(default)]
    pub audio_files: Vec<String>,
    pub output_dir: String,
    #[serde(default)]
    pub output_name: String,
    #[serde(default = "default_loop_count")]
    pub loop_count: i64,
    #[serde(default)]
    pub intro_path: Option<String>,
    #[serde(default)]
    pub output_path: Option<String>,
}

fn default_loop_count() -> i64 {
    1
}

impl LoopRequest {
    pub fn can_generate(&self) -> bool {
        !self.background_path.is_empty() && !self.audio_files.is_empty() && self.loop_count >= 1
    }
}

/// Output dari satu merge looping.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopResult {
    pub output_path: String,
    pub timestamp_path: Option<String>,
    pub playlist: Vec<String>,
}

/// Peringatan yang diangkat sebelum merge dimulai.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub codec_mismatch: bool,
    pub mismatch_fields: Vec<String>,
    pub youtube: Option<YouTubeResult>,
}

impl PreflightReport {
    pub fn has_warnings(&self) -> bool {
        if self.codec_mismatch || !self.mismatch_fields.is_empty() {
            return true;
        }
        self.youtube
            .as_ref()
            .map(|result| !result.issues.is_empty())
            .unwrap_or(false)
    }
}
