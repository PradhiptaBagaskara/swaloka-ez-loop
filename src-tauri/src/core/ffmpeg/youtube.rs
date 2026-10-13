//! Validasi standar unggah YouTube dan parameter encoding.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::core::ffmpeg::metadata::VideoMetadata;

/// Standar siaran YouTube: 2 B-frame dan 1 reference frame.
pub const LIVE_B_FRAMES: i64 = 2;
pub const LIVE_REF_FRAMES: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum YouTubeStatus {
    Passed,
    Warning,
    Failed,
}

/// Hasil pemeriksaan standar YouTube.
#[derive(Clone, Debug, Serialize)]
pub struct YouTubeResult {
    pub status: YouTubeStatus,
    pub issues: Vec<String>,
    pub details: BTreeMap<String, String>,
}

/// Menilai media terhadap rekomendasi encoding unggahan YouTube.
pub struct YouTubeChecker;

impl YouTubeChecker {
    /// Bitrate minimum dan yang disarankan (Mbps) untuk siaran, dari kolom H.264
    /// pada tabel resmi "Choose live encoder settings". Baris terakhir jadi fallback.
    pub fn live_bitrates_mbps(height: i64, fps: f64) -> (f64, f64) {
        const TABLE: [(i64, f64, f64, f64, f64); 6] = [
            // (tinggi minimum, min@30, rec@30, min@60, rec@60)
            (2160, 11.0, 42.0, 14.0, 50.0),
            (1440, 7.0, 21.0, 8.0, 34.0),
            (1080, 5.0, 14.0, 6.0, 17.0),
            (720, 3.0, 8.0, 3.0, 8.0),
            (480, 0.4, 4.0, 0.4, 4.0),
            (0, 0.4, 4.0, 0.4, 4.0),
        ];
        let high_fps = fps >= 50.0;
        for (min_height, min30, rec30, min60, rec60) in TABLE {
            if height >= min_height {
                return if high_fps {
                    (min60, rec60)
                } else {
                    (min30, rec30)
                };
            }
        }
        (0.4, 4.0)
    }

    /// Validasi metadata dan kembalikan status beserta daftar masalah.
    pub fn check(&self, metadata: &VideoMetadata) -> YouTubeResult {
        let mut issues: Vec<String> = Vec::new();
        let mut details: BTreeMap<String, String> = BTreeMap::new();

        match &metadata.codec {
            Some(codec) => {
                details.insert("codec".into(), codec.clone());
                if codec != "h264" {
                    issues.push(format!("Video codec: {codec} (should be h264)"));
                }
            }
            None => issues.push(String::from("Video codec: not detected")),
        }

        if metadata.has_audio {
            match &metadata.audio_codec {
                Some(codec) => {
                    details.insert("audioCodec".into(), codec.clone());
                    if codec.to_lowercase() != "aac" {
                        issues.push(format!("Audio codec: {codec} (should be aac)"));
                    }
                }
                None => issues.push(String::from("Audio codec: not detected")),
            }
        } else {
            issues.push(String::from("Audio: no audio track"));
        }

        match &metadata.pix_fmt {
            Some(pix_fmt) => {
                details.insert("pixFmt".into(), pix_fmt.clone());
                if !pix_fmt.to_lowercase().contains("yuv420p") {
                    issues.push(format!("Pixel format: {pix_fmt} (should be yuv420p)"));
                }
            }
            None => issues.push(String::from("Pixel format: not detected")),
        }

        match &metadata.color_space {
            Some(color_space) => {
                details.insert("colorSpace".into(), color_space.clone());
                if color_space.to_lowercase() != "bt709" {
                    issues.push(format!("Color space: {color_space} (should be bt709)"));
                }
            }
            None => issues.push(String::from("Color space: metadata missing")),
        }

        details.insert(
            "colorPrimaries".into(),
            metadata
                .color_primaries
                .clone()
                .unwrap_or_else(|| String::from("N/A")),
        );
        details.insert(
            "colorTransfer".into(),
            metadata
                .color_transfer
                .clone()
                .unwrap_or_else(|| String::from("N/A")),
        );
        details.insert(
            "colorRange".into(),
            metadata
                .color_range
                .clone()
                .unwrap_or_else(|| String::from("N/A")),
        );

        match metadata.interlaced {
            Some(interlaced) => {
                details.insert(
                    "interlaced".into(),
                    if interlaced {
                        "Interlaced"
                    } else {
                        "Progressive"
                    }
                    .to_string(),
                );
                if interlaced {
                    issues.push(String::from("Scan: Interlaced (should be Progressive)"));
                }
            }
            None => {
                details.insert("interlaced".into(), "N/A".into());
            }
        }

        if metadata.codec.as_deref() == Some("h264") {
            match &metadata.profile {
                Some(profile) => {
                    details.insert("profile".into(), profile.clone());
                    if !profile.to_lowercase().contains("high") {
                        issues.push(format!("Profile: {profile} (should be High)"));
                    }
                }
                None => {
                    details.insert("profile".into(), "N/A".into());
                }
            }
            details.insert(
                "level".into(),
                metadata
                    .level
                    .clone()
                    .unwrap_or_else(|| String::from("N/A")),
            );
        }

        details.insert(
            "bFrames".into(),
            metadata
                .b_frames
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("N/A")),
        );
        if let Some(b_frames) = metadata.b_frames {
            if b_frames != LIVE_B_FRAMES {
                issues.push(format!(
                    "B-frame: {b_frames} (standar siaran memakai {LIVE_B_FRAMES})"
                ));
            }
        }
        details.insert(
            "refFrames".into(),
            metadata
                .ref_frames
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("N/A")),
        );
        if let Some(ref_frames) = metadata.ref_frames {
            if ref_frames != LIVE_REF_FRAMES {
                issues.push(format!(
                    "Reference frame: {ref_frames} (standar siaran memakai {LIVE_REF_FRAMES})"
                ));
            }
        }

        match (metadata.gop_size, metadata.fps.as_ref()) {
            (Some(gop_size), Some(_)) => {
                let fps = metadata.fps_value();
                let recommended = (fps * 2.0).round() as i64;
                let maximum = (fps * 4.0).round() as i64;
                details.insert("gopSize".into(), gop_size.to_string());
                if gop_size > maximum {
                    issues.push(format!(
                        "Keyframe interval: {gop_size} frame (lewat batas 4 detik; disarankan {recommended} frame = 2 detik)"
                    ));
                }
            }
            _ => {
                details.insert("gopSize".into(), "N/A".into());
            }
        }

        details.insert(
            "cabac".into(),
            match metadata.cabac {
                Some(true) => "Enabled".into(),
                Some(false) => "Disabled".into(),
                None => "N/A".into(),
            },
        );

        issues.extend(self.check_bitrate(metadata, &mut details));

        let status = Self::status(metadata, &issues);
        YouTubeResult {
            status,
            issues,
            details,
        }
    }

    fn check_bitrate(
        &self,
        metadata: &VideoMetadata,
        details: &mut BTreeMap<String, String>,
    ) -> Vec<String> {
        let Some(bitrate) = metadata.bitrate_kbps else {
            details.insert("bitrate".into(), "N/A".into());
            return Vec::new();
        };
        if metadata.height.is_none() || metadata.fps.is_none() {
            details.insert("bitrate".into(), format_bitrate(bitrate));
            return Vec::new();
        }

        let (minimum_mbps, recommended_mbps) =
            Self::live_bitrates_mbps(metadata.height.unwrap_or(0) as i64, metadata.fps_value());
        let minimum = (minimum_mbps * 1000.0).round() as i64;
        details.insert(
            "bitrate".into(),
            format!(
                "{} (minimum siaran: {})",
                format_bitrate(bitrate),
                format_bitrate(minimum)
            ),
        );
        if bitrate < minimum {
            return vec![format!(
                "Bitrate: {} di bawah minimum siaran {} (disarankan {:.0} Mbps)",
                format_bitrate(bitrate),
                format_bitrate(minimum),
                recommended_mbps
            )];
        }
        Vec::new()
    }

    fn status(metadata: &VideoMetadata, issues: &[String]) -> YouTubeStatus {
        if !issues.is_empty() {
            return if issues.len() <= 3 {
                YouTubeStatus::Warning
            } else {
                YouTubeStatus::Failed
            };
        }

        let mut unverified: Vec<&str> = Vec::new();
        if metadata.codec.as_deref() == Some("h264") {
            if metadata.profile.is_none() {
                unverified.push("Profile");
            }
            if metadata.interlaced.is_none() {
                unverified.push("Scan type");
            }
            if metadata.gop_size.is_none() {
                unverified.push("GOP size");
            }
            if metadata.cabac.is_none() {
                unverified.push("CABAC");
            }
        }
        if metadata.bitrate_kbps.is_none() {
            unverified.push("Bitrate");
        }

        if unverified.len() >= 3 {
            YouTubeStatus::Warning
        } else {
            YouTubeStatus::Passed
        }
    }
}

fn format_bitrate(kbps: i64) -> String {
    if kbps > 1000 {
        format!("{:.1} Mbps", kbps as f64 / 1000.0)
    } else {
        format!("{kbps} kbps")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_metadata() -> VideoMetadata {
        VideoMetadata {
            path: "/x.mp4".into(),
            readable: true,
            codec: Some("h264".into()),
            width: Some(1920),
            height: Some(1080),
            fps: Some("30".into()),
            pix_fmt: Some("yuv420p".into()),
            duration: Some(10.0),
            has_audio: true,
            audio_codec: Some("aac".into()),
            color_space: Some("bt709".into()),
            profile: Some("High".into()),
            level: Some("4.1".into()),
            gop_size: Some(15),
            cabac: Some(true),
            interlaced: Some(false),
            bitrate_kbps: Some(9000),
            ..Default::default()
        }
    }

    #[test]
    fn passes_a_compliant_video() {
        let checker = YouTubeChecker;
        let result = checker.check(&good_metadata());
        assert_eq!(result.status, YouTubeStatus::Passed);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn warns_on_a_few_issues() {
        let checker = YouTubeChecker;
        let mut metadata = good_metadata();
        metadata.codec = Some("hevc".into());
        metadata.audio_codec = Some("opus".into());
        let result = checker.check(&metadata);
        assert_eq!(result.status, YouTubeStatus::Warning);
        assert_eq!(result.issues.len(), 2);
    }

    #[test]
    fn fails_on_many_issues() {
        let checker = YouTubeChecker;
        let metadata = VideoMetadata {
            path: "/x".into(),
            readable: true,
            codec: Some("hevc".into()),
            width: Some(1920),
            height: Some(1080),
            fps: Some("30".into()),
            pix_fmt: Some("yuv444p".into()),
            has_audio: false,
            color_space: Some("bt601".into()),
            profile: Some("Main".into()),
            gop_size: Some(1),
            bitrate_kbps: Some(100),
            ..Default::default()
        };
        let result = checker.check(&metadata);
        assert_eq!(result.status, YouTubeStatus::Failed);
        assert!(result.issues.len() >= 4);
    }

    #[test]
    fn recommends_bitrates_by_resolution() {
        // Tabel resmi "Choose live encoder settings", kolom H.264.
        assert_eq!(YouTubeChecker::live_bitrates_mbps(1080, 30.0), (5.0, 14.0));
        assert_eq!(YouTubeChecker::live_bitrates_mbps(1080, 60.0), (6.0, 17.0));
        assert_eq!(YouTubeChecker::live_bitrates_mbps(2160, 60.0), (14.0, 50.0));
        assert_eq!(YouTubeChecker::live_bitrates_mbps(360, 30.0), (0.4, 4.0));
    }
}
