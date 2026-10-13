//! Pemeriksaan konsistensi pra-terbang antara background, intro, dan audio.

use crate::core::ffmpeg::metadata::VideoMetadata;
use crate::core::ffmpeg::youtube::YouTubeChecker;
use crate::features::looping::models::PreflightReport;

/// Memperingatkan ketidakcocokan intro/background dan masalah standar YouTube.
pub struct PreflightChecker {
    youtube: YouTubeChecker,
}

impl Default for PreflightChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl PreflightChecker {
    pub fn new() -> Self {
        Self {
            youtube: YouTubeChecker,
        }
    }

    pub fn check(
        &self,
        background: &VideoMetadata,
        intro: Option<&VideoMetadata>,
        check_youtube: bool,
    ) -> PreflightReport {
        let mut report = PreflightReport::default();

        if let Some(intro) = intro {
            report.codec_mismatch = background.codec.is_some()
                && intro.codec.is_some()
                && background.codec != intro.codec;
            report.mismatch_fields = Self::mismatch_fields(background, intro);
        }

        if check_youtube && background.readable {
            report.youtube = Some(self.youtube.check(background));
        }

        report
    }

    fn mismatch_fields(background: &VideoMetadata, intro: &VideoMetadata) -> Vec<String> {
        let mut fields: Vec<String> = Vec::new();
        if background.resolution() != intro.resolution() {
            fields.push(String::from("resolution"));
        }
        if background.fps != intro.fps {
            fields.push(String::from("frame rate"));
        }
        if background.pix_fmt != intro.pix_fmt {
            fields.push(String::from("pixel format"));
        }
        fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(codec: &str, width: u32, height: u32) -> VideoMetadata {
        VideoMetadata {
            path: "/x".into(),
            readable: true,
            codec: Some(codec.into()),
            width: Some(width),
            height: Some(height),
            fps: Some("30".into()),
            pix_fmt: Some("yuv420p".into()),
            has_audio: true,
            audio_codec: Some("aac".into()),
            color_space: Some("bt709".into()),
            ..Default::default()
        }
    }

    #[test]
    fn detects_mismatch() {
        let checker = PreflightChecker::new();
        let background = metadata("h264", 1920, 1080);
        let intro = metadata("hevc", 1280, 720);
        let report = checker.check(&background, Some(&intro), false);
        assert!(report.codec_mismatch);
        assert!(report.mismatch_fields.contains(&String::from("resolution")));
        assert!(report.has_warnings());
    }

    #[test]
    fn matches_when_identical() {
        let checker = PreflightChecker::new();
        let background = metadata("h264", 1920, 1080);
        let intro = metadata("h264", 1920, 1080);
        let report = checker.check(&background, Some(&intro), false);
        assert!(!report.codec_mismatch);
        assert!(report.mismatch_fields.is_empty());
        assert!(!report.has_warnings());
    }
}
