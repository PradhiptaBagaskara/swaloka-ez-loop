//! Estimasi pemakaian disk output dan temp sebelum merge.

use serde::Serialize;

pub const ESTIMATED_AUDIO_BITRATE_KBPS: i64 = 192;
pub const DEFAULT_VIDEO_BITRATE_KBPS: i64 = 8000;
pub const SAFETY_MULTIPLIER: f64 = 1.2;
pub const PREPROCESS_MIN_SECONDS: f64 = 600.0;
pub const FALLBACK_AUDIO_SECONDS: f64 = 180.0;

/// Prediksi ukuran output, puncak temp, dan total ruang yang dibutuhkan.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskEstimate {
    pub output_bytes: u64,
    pub temp_peak_bytes: u64,
    pub required_bytes: u64,
    pub duration_seconds: f64,
    pub video_bitrate_kbps: i64,
    pub audio_bitrate_kbps: i64,
}

/// Estimasi ukuran murni dari durasi dan bitrate.
pub struct DiskEstimator {
    pub audio_bitrate_kbps: i64,
    pub safety_multiplier: f64,
    pub preprocess_min_seconds: f64,
}

impl Default for DiskEstimator {
    fn default() -> Self {
        Self {
            audio_bitrate_kbps: ESTIMATED_AUDIO_BITRATE_KBPS,
            safety_multiplier: SAFETY_MULTIPLIER,
            preprocess_min_seconds: PREPROCESS_MIN_SECONDS,
        }
    }
}

/// Argumen estimasi disk.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskEstimateInput {
    pub audio_file_count: i64,
    pub audio_seconds: f64,
    pub loop_count: i64,
    pub background_bitrate_kbps: Option<i64>,
    pub background_seconds: Option<f64>,
    pub intro_bitrate_kbps: Option<i64>,
}

impl DiskEstimator {
    /// Estimasi byte yang akan dihasilkan dan dibutuhkan di ruang temp.
    pub fn estimate(&self, input: DiskEstimateInput) -> DiskEstimate {
        let mut duration_seconds = input.audio_seconds * input.loop_count as f64;
        if duration_seconds <= 0.0 {
            duration_seconds = std::cmp::max(1, input.audio_file_count * input.loop_count) as f64
                * FALLBACK_AUDIO_SECONDS;
        }

        let background_bitrate = input
            .background_bitrate_kbps
            .unwrap_or(DEFAULT_VIDEO_BITRATE_KBPS);
        let selected_video_bitrate = match input.intro_bitrate_kbps {
            None => background_bitrate,
            Some(intro) => std::cmp::max(background_bitrate, intro),
        };

        let output_bytes = bytes_for(
            duration_seconds,
            selected_video_bitrate + self.audio_bitrate_kbps,
        );
        let mut temp_peak_bytes = bytes_for(duration_seconds, self.audio_bitrate_kbps);

        if input.intro_bitrate_kbps.is_some() {
            if let Some(background_seconds) = input.background_seconds {
                if background_seconds > 0.0 && background_seconds < self.preprocess_min_seconds {
                    temp_peak_bytes += bytes_for(self.preprocess_min_seconds, background_bitrate);
                }
            }
        }

        let required_bytes =
            ((output_bytes + temp_peak_bytes) as f64 * self.safety_multiplier).ceil() as u64;

        DiskEstimate {
            output_bytes,
            temp_peak_bytes,
            required_bytes,
            duration_seconds,
            video_bitrate_kbps: selected_video_bitrate,
            audio_bitrate_kbps: self.audio_bitrate_kbps,
        }
    }
}

fn bytes_for(duration_seconds: f64, bitrate_kbps: i64) -> u64 {
    ((duration_seconds * bitrate_kbps as f64 * 1000.0) / 8.0).ceil() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_when_duration_unknown() {
        let estimator = DiskEstimator::default();
        let estimate = estimator.estimate(DiskEstimateInput {
            audio_file_count: 3,
            audio_seconds: 0.0,
            loop_count: 2,
            background_bitrate_kbps: None,
            background_seconds: None,
            intro_bitrate_kbps: None,
        });
        // 3 file * 2 loop * 180 detik.
        assert_eq!(estimate.duration_seconds, 1080.0);
        assert_eq!(estimate.video_bitrate_kbps, DEFAULT_VIDEO_BITRATE_KBPS);
    }

    #[test]
    fn picks_max_bitrate_with_intro() {
        let estimator = DiskEstimator::default();
        let estimate = estimator.estimate(DiskEstimateInput {
            audio_file_count: 1,
            audio_seconds: 600.0,
            loop_count: 1,
            background_bitrate_kbps: Some(4000),
            background_seconds: Some(600.0),
            intro_bitrate_kbps: Some(9000),
        });
        assert_eq!(estimate.video_bitrate_kbps, 9000);
        assert!(estimate.required_bytes > estimate.output_bytes);
    }
}
