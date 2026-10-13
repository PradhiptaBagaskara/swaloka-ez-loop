//! Identifikasi hardware encoder dan deteksi lewat trial-encode.

use serde::{Deserialize, Serialize};

use crate::core::ffmpeg::runner::FFmpegRunner;

/// Encoder video yang bisa dipilih aplikasi, memakai penamaan FFmpeg.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareEncoder {
    #[serde(rename = "h264_videotoolbox")]
    VideoToolbox,
    #[serde(rename = "h264_nvenc")]
    Nvenc,
    #[serde(rename = "h264_amf")]
    Amf,
    #[serde(rename = "h264_qsv")]
    Qsv,
    #[serde(rename = "h264_mf")]
    Mf,
    #[serde(rename = "h264_vaapi")]
    Vaapi,
    #[serde(rename = "h264_v4l2m2m")]
    V4l2m2m,
    #[serde(rename = "libx264")]
    Software,
}

impl HardwareEncoder {
    pub fn value(self) -> &'static str {
        match self {
            HardwareEncoder::VideoToolbox => "h264_videotoolbox",
            HardwareEncoder::Nvenc => "h264_nvenc",
            HardwareEncoder::Amf => "h264_amf",
            HardwareEncoder::Qsv => "h264_qsv",
            HardwareEncoder::Mf => "h264_mf",
            HardwareEncoder::Vaapi => "h264_vaapi",
            HardwareEncoder::V4l2m2m => "h264_v4l2m2m",
            HardwareEncoder::Software => "libx264",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HardwareEncoder::VideoToolbox => "VideoToolbox (macOS)",
            HardwareEncoder::Nvenc => "NVIDIA NVENC",
            HardwareEncoder::Amf => "AMD AMF",
            HardwareEncoder::Qsv => "Intel QuickSync",
            HardwareEncoder::Mf => "Media Foundation",
            HardwareEncoder::Vaapi => "VA-API (Linux)",
            HardwareEncoder::V4l2m2m => "V4L2 M2M (Linux)",
            HardwareEncoder::Software => "Software (libx264)",
        }
    }

    pub fn is_hardware(self) -> bool {
        self != HardwareEncoder::Software
    }

    /// Semua varian, urut untuk dropdown pengaturan.
    pub fn all() -> Vec<HardwareEncoder> {
        vec![
            HardwareEncoder::VideoToolbox,
            HardwareEncoder::Nvenc,
            HardwareEncoder::Amf,
            HardwareEncoder::Qsv,
            HardwareEncoder::Mf,
            HardwareEncoder::Vaapi,
            HardwareEncoder::V4l2m2m,
            HardwareEncoder::Software,
        ]
    }

    /// Bangun dari nilai string FFmpeg.
    pub fn from_value(value: &str) -> Option<HardwareEncoder> {
        HardwareEncoder::all()
            .into_iter()
            .find(|encoder| encoder.value() == value)
    }
}

/// Memilih encoder terbaik yang bisa dipakai di platform ini.
pub struct EncoderDetector {
    platform: String,
}

impl EncoderDetector {
    pub fn new(platform: Option<String>) -> Self {
        Self {
            platform: platform.unwrap_or_else(|| std::env::consts::OS.to_string()),
        }
    }

    const MACOS_PRIORITY: [HardwareEncoder; 2] =
        [HardwareEncoder::VideoToolbox, HardwareEncoder::Software];
    const WINDOWS_PRIORITY: [HardwareEncoder; 5] = [
        HardwareEncoder::Nvenc,
        HardwareEncoder::Amf,
        HardwareEncoder::Qsv,
        HardwareEncoder::Mf,
        HardwareEncoder::Software,
    ];
    const LINUX_PRIORITY: [HardwareEncoder; 4] = [
        HardwareEncoder::Nvenc,
        HardwareEncoder::Vaapi,
        HardwareEncoder::V4l2m2m,
        HardwareEncoder::Software,
    ];

    pub fn priority(&self) -> &'static [HardwareEncoder] {
        if self.platform == "macos" || self.platform == "darwin" {
            &Self::MACOS_PRIORITY
        } else if self.platform.starts_with("win") {
            &Self::WINDOWS_PRIORITY
        } else {
            &Self::LINUX_PRIORITY
        }
    }

    /// Argumen trial-encode satu detik untuk membuktikan encoder bekerja.
    pub fn trial_args(encoder: HardwareEncoder) -> Vec<String> {
        [
            "-hide_banner",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x240:d=1",
            "-c:v",
            encoder.value(),
            "-an",
            "-f",
            "null",
            "-",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect()
    }

    /// Apakah encoder menyelesaikan trial-encode di mesin ini.
    pub async fn test(&self, runner: &FFmpegRunner, encoder: HardwareEncoder) -> bool {
        let executable = runner.ffmpeg_path();
        match runner
            .run_executable(
                &executable,
                Self::trial_args(encoder),
                None,
                None,
                false,
                true,
            )
            .await
        {
            Ok(result) => result.ok(),
            Err(_) => false,
        }
    }

    /// Encoder pertama yang bekerja menurut urutan prioritas platform.
    pub async fn detect(&self, runner: &FFmpegRunner) -> HardwareEncoder {
        for encoder in self.priority() {
            if *encoder == HardwareEncoder::Software {
                return HardwareEncoder::Software;
            }
            if self.test(runner, *encoder).await {
                return *encoder;
            }
        }
        HardwareEncoder::Software
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_matches_platform() {
        let mac = EncoderDetector::new(Some("darwin".into()));
        assert_eq!(mac.priority()[0], HardwareEncoder::VideoToolbox);
        let win = EncoderDetector::new(Some("windows".into()));
        assert_eq!(win.priority()[0], HardwareEncoder::Nvenc);
        let linux = EncoderDetector::new(Some("linux".into()));
        assert_eq!(linux.priority()[0], HardwareEncoder::Nvenc);
    }

    #[test]
    fn trial_args_use_encoder_value() {
        let args = EncoderDetector::trial_args(HardwareEncoder::Nvenc);
        assert_eq!(args[6], "h264_nvenc");
        assert_eq!(args.last().unwrap(), "-");
    }

    #[test]
    fn round_trips_values() {
        for encoder in HardwareEncoder::all() {
            assert_eq!(HardwareEncoder::from_value(encoder.value()), Some(encoder));
        }
        assert!(!HardwareEncoder::Software.is_hardware());
        assert!(HardwareEncoder::Nvenc.is_hardware());
    }
}
