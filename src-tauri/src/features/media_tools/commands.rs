//! Builder argumen FFmpeg murni untuk media tools (audio + standar siaran).

use std::path::Path;

/// Flags scaling untuk jalur siaran (selalu lanczos).
pub const LIVE_SCALING_FLAGS: &str = "lanczos";

/// Batas bawah bitrate siaran yang diterima aplikasi (Mbps). Tabel resmi punya
/// baris 0,4 Mbps untuk resolusi kecil, tetapi di bawah 3 Mbps kualitas siaran
/// sudah tidak masuk akal untuk konten musik.
pub const MIN_LIVE_BITRATE_MBPS: f64 = 3.0;

/// Bitrate bawaan yang dipakai bila pengguna tidak mengisinya (Mbps).
pub const DEFAULT_LIVE_BITRATE_MBPS: f64 = 8.0;

/// Metadata warna Rec.709 yang diwajibkan untuk siaran SDR.
pub const LIVE_COLOR_METADATA_FLAGS: [&str; 8] = [
    "-colorspace",
    "bt709",
    "-color_trc",
    "bt709",
    "-color_primaries",
    "bt709",
    "-color_range",
    "tv",
];

fn statics(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

/// Format float mengikuti gaya Python (`1.0`, bukan `1`).
fn fmt_float(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

/// Memetakan pasangan CRF/preset ke argumen FFmpeg spesifik encoder.
/// Argumen encoder untuk berkas yang harus lolos standar siaran YouTube.
///
/// Mengikuti tabel resmi "Choose live encoder settings": H.264, bitrate CBR,
/// keyframe 2 detik, closed GOP, 2 B-frame, 1 reference frame, CABAC, dan
/// piksel persegi. Nilai resmi juga membatasi keyframe maksimum 4 detik.
pub struct LiveStandardArgs;

impl LiveStandardArgs {
    /// Filter normalisasi untuk siaran: resolusi video sumber dipertahankan,
    /// hanya dimensi digenapkan (syarat yuv420p), warna Rec.709, piksel persegi,
    /// dan frame rate dibuat tetap.
    pub fn normalize_filter(fps: f64) -> String {
        format!(
            "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p,setsar=1,fps={}",
            fmt_float(fps)
        )
    }

    /// Keyframe interval yang diminta, dalam frame (2 detik mengikuti fps sumber).
    pub fn gop_frames(fps: f64) -> i64 {
        (fps * 2.0).round().max(1.0) as i64
    }

    /// Bitrate yang benar-benar dipakai: pilihan pengguna, atau bawaan aplikasi.
    pub fn resolve_bitrate(requested: Option<f64>) -> f64 {
        requested
            .filter(|value| *value > 0.0)
            .unwrap_or(DEFAULT_LIVE_BITRATE_MBPS)
    }

    /// Argumen video: standar penuh dengan libx264, atau sebisanya dengan encoder
    /// hardware yang dipilih di Pengaturan.
    pub fn video_args(encoder: Option<&str>, bitrate_mbps: f64, gop_frames: i64) -> Vec<String> {
        match encoder {
            Some(name) if name != "libx264" => Self::hardware_args(name, bitrate_mbps, gop_frames),
            _ => Self::software_args(bitrate_mbps, gop_frames),
        }
    }

    /// Argumen video untuk siaran dengan libx264: CBR, keyframe 2 detik, closed GOP.
    pub fn software_args(bitrate_mbps: f64, gop_frames: i64) -> Vec<String> {
        let rate = |value: f64| format!("{value:.1}M");
        let gop = gop_frames.to_string();
        statics(&[
            "-c:v",
            "libx264",
            "-profile:v",
            "high",
            "-preset",
            "veryfast",
            "-b:v",
            &rate(bitrate_mbps),
            "-minrate",
            &rate(bitrate_mbps),
            "-maxrate",
            &rate(bitrate_mbps),
            "-bufsize",
            &rate(bitrate_mbps * 2.0),
            "-g",
            &gop,
            "-keyint_min",
            &gop,
            "-sc_threshold",
            "0",
            "-bf",
            "2",
            "-refs",
            "1",
            "-coder",
            "1",
            "-flags",
            "+cgop",
            // Tanpa ini x264 tidak menuliskan primaries/transfer Rec.709 ke VUI.
            "-x264-params",
            "colorprim=bt709:transfer=bt709:colormatrix=bt709",
        ])
    }

    /// Argumen video untuk encoder hardware. Ditulis sebisanya sesuai standar,
    /// tetapi tiap encoder punya keterbatasan sendiri (mis. VideoToolbox tidak
    /// memegang target CBR dan tidak menulis primaries/transfer Rec.709).
    fn hardware_args(encoder: &str, bitrate_mbps: f64, gop_frames: i64) -> Vec<String> {
        let rate = |value: f64| format!("{value:.1}M");
        let gop = gop_frames.to_string();
        let common = |args: &mut Vec<String>| {
            args.extend(statics(&[
                "-b:v",
                &rate(bitrate_mbps),
                "-maxrate",
                &rate(bitrate_mbps),
                "-bufsize",
                &rate(bitrate_mbps * 2.0),
                "-g",
                &gop,
                "-profile:v",
                "high",
            ]));
        };

        let mut args = statics(&["-c:v", encoder]);
        match encoder {
            "h264_videotoolbox" => {
                common(&mut args);
                args.extend(statics(&["-allow_sw", "1"]));
            }
            "h264_nvenc" => {
                args.extend(statics(&["-rc", "cbr"]));
                common(&mut args);
                args.extend(statics(&["-bf", "2", "-strict_gop", "-preset", "p4"]));
            }
            "h264_amf" => {
                args.extend(statics(&[
                    "-usage",
                    "transcoding",
                    "-quality",
                    "quality",
                    "-rc",
                    "cbr",
                ]));
                common(&mut args);
                args.extend(statics(&["-bf", "2"]));
            }
            "h264_qsv" => {
                common(&mut args);
                args.extend(statics(&[
                    "-bf",
                    "2",
                    "-gop_mode",
                    "strict",
                    "-look_ahead",
                    "0",
                ]));
            }
            "h264_vaapi" => {
                args.extend(statics(&["-rc_mode", "CBR"]));
                common(&mut args);
            }
            _ => common(&mut args),
        }
        args
    }
}

pub struct MediaCommandBuilder;

impl MediaCommandBuilder {
    pub fn format_concat_path(path: impl AsRef<Path>) -> String {
        path.as_ref()
            .to_string_lossy()
            .replace('\\', "/")
            .replace('\'', "'\\''")
    }

    pub fn concat_content(paths: &[impl AsRef<Path>]) -> String {
        paths
            .iter()
            .map(|path| format!("file '{}'", Self::format_concat_path(path)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn audio_codec_args(output_path: impl AsRef<Path>) -> Vec<String> {
        let suffix = output_path
            .as_ref()
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
            .unwrap_or_default();
        match suffix.as_str() {
            ".mp3" => statics(&["-c:a", "libmp3lame", "-b:a", "192k"]),
            ".wav" => statics(&["-c:a", "pcm_s16le"]),
            ".flac" => statics(&["-c:a", "flac"]),
            ".ogg" => statics(&["-c:a", "libvorbis", "-qscale:a", "4"]),
            _ => statics(&["-c:a", "aac", "-b:a", "384k", "-ar", "48000", "-ac", "2"]),
        }
    }

    pub fn concat_audio_args(
        concat_file: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut args = statics(&["-y", "-f", "concat", "-safe", "0", "-i"]);
        args.push(concat_file.as_ref().to_string_lossy().to_string());
        args.extend(Self::audio_codec_args(&output));
        args.push(output.as_ref().to_string_lossy().to_string());
        args
    }

    pub fn batch_reencode_audio_args(
        inputs: &[String],
        outputs: &[String],
        output_path: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut args = statics(&["-y"]);
        for source in inputs {
            args.push(String::from("-i"));
            args.push(source.clone());
        }
        let codec = Self::audio_codec_args(&output_path);
        for (index, target) in outputs.iter().enumerate() {
            args.push(String::from("-map"));
            args.push(format!("{index}:a"));
            args.extend(statics(&["-vn"]));
            args.extend(codec.iter().cloned());
            args.push(target.clone());
        }
        args
    }

    pub fn overlay_merge_args(overlays: &[(String, f64)], output: impl AsRef<Path>) -> Vec<String> {
        let mut inputs: Vec<String> = Vec::new();
        for (path, _) in overlays {
            inputs.push(String::from("-i"));
            inputs.push(path.clone());
        }
        let mut parts: Vec<String> = Vec::new();
        let mut labels: Vec<String> = Vec::new();
        for (index, (_, volume)) in overlays.iter().enumerate() {
            parts.push(format!(
                "[{index}:a]volume={}[a{index}]",
                fmt_float(*volume)
            ));
            labels.push(format!("[a{index}]"));
        }
        parts.push(format!(
            "{}amerge=inputs={}[aout]",
            labels.join(""),
            overlays.len()
        ));
        let mut args = statics(&["-y"]);
        args.extend(inputs);
        args.extend(statics(&["-filter_complex"]));
        args.push(parts.join(";"));
        args.extend(statics(&["-map", "[aout]"]));
        args.extend(Self::audio_codec_args(&output));
        args.push(output.as_ref().to_string_lossy().to_string());
        args
    }

    pub fn premix_overlays_args(
        overlays: &[(String, f64)],
        output_wav: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut inputs: Vec<String> = Vec::new();
        for (path, _) in overlays {
            inputs.push(String::from("-i"));
            inputs.push(path.clone());
        }
        let mut parts: Vec<String> = Vec::new();
        for (index, (_, volume)) in overlays.iter().enumerate() {
            parts.push(format!(
                "[{index}:a]volume={}[v{index}]",
                fmt_float(*volume)
            ));
        }
        let amix_inputs: String = (0..overlays.len()).map(|i| format!("[v{i}]")).collect();
        let compensation = format!("{:.1}", overlays.len() as f64);
        parts.push(format!(
            "{amix_inputs}amix=inputs={}:duration=longest[overlay_mix];[overlay_mix]volume={compensation}[overlay_mix]",
            overlays.len()
        ));
        let mut args = statics(&["-y", "-vn"]);
        args.extend(inputs);
        args.extend(statics(&["-filter_complex"]));
        args.push(parts.join(";"));
        args.extend(statics(&["-map", "[overlay_mix]", "-c:a", "pcm_s16le"]));
        args.push(output_wav.as_ref().to_string_lossy().to_string());
        args
    }

    pub fn base_with_overlay_args(
        concat_file: impl AsRef<Path>,
        overlay_mix: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut args = statics(&["-y", "-vn", "-f", "concat", "-safe", "0", "-i"]);
        args.push(concat_file.as_ref().to_string_lossy().to_string());
        args.extend(statics(&["-stream_loop", "-1", "-i"]));
        args.push(overlay_mix.as_ref().to_string_lossy().to_string());
        args.extend(statics(&[
            "-filter_complex",
            "[0:a][1:a]amix=inputs=2:duration=first[out];[out]volume=2.0[out]",
            "-map",
            "[out]",
        ]));
        args.extend(Self::audio_codec_args(&output));
        args.push(output.as_ref().to_string_lossy().to_string());
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_audio_args_for_m4a() {
        assert_eq!(
            MediaCommandBuilder::concat_audio_args("/c.txt", "/out.m4a"),
            statics(&[
                "-y", "-f", "concat", "-safe", "0", "-i", "/c.txt", "-c:a", "aac", "-b:a", "384k",
                "-ar", "48000", "-ac", "2", "/out.m4a",
            ])
        );
    }

    #[test]
    fn audio_codec_args_follow_extension() {
        assert_eq!(
            MediaCommandBuilder::audio_codec_args("/x.mp3"),
            statics(&["-c:a", "libmp3lame", "-b:a", "192k"])
        );
        assert_eq!(
            MediaCommandBuilder::audio_codec_args("/x.flac"),
            statics(&["-c:a", "flac"])
        );
    }

    #[test]
    fn live_encode_args_are_pinned() {
        // 1080p30 pada bitrate yang disarankan tabel resmi (14 Mbps).
        assert_eq!(
            LiveStandardArgs::software_args(14.0, 60),
            statics(&[
                "-c:v",
                "libx264",
                "-profile:v",
                "high",
                "-preset",
                "veryfast",
                "-b:v",
                "14.0M",
                "-minrate",
                "14.0M",
                "-maxrate",
                "14.0M",
                "-bufsize",
                "28.0M",
                "-g",
                "60",
                "-keyint_min",
                "60",
                "-sc_threshold",
                "0",
                "-bf",
                "2",
                "-refs",
                "1",
                "-coder",
                "1",
                "-flags",
                "+cgop",
                "-x264-params",
                "colorprim=bt709:transfer=bt709:colormatrix=bt709",
            ])
        );
    }

    #[test]
    fn live_hardware_args_keep_cbr_and_keyframes() {
        // VideoToolbox: CBR + keyframe 2 detik sebisanya (tanpa B-frame).
        assert_eq!(
            LiveStandardArgs::video_args(Some("h264_videotoolbox"), 14.0, 60),
            statics(&[
                "-c:v",
                "h264_videotoolbox",
                "-b:v",
                "14.0M",
                "-maxrate",
                "14.0M",
                "-bufsize",
                "28.0M",
                "-g",
                "60",
                "-profile:v",
                "high",
                "-allow_sw",
                "1",
            ])
        );
        // NVENC menambahkan CBR eksplisit, strict GOP, dan 2 B-frame.
        let nvenc = LiveStandardArgs::video_args(Some("h264_nvenc"), 14.0, 60);
        assert!(nvenc.contains(&"cbr".to_string()));
        assert!(nvenc.contains(&"-strict_gop".to_string()));
        assert!(nvenc.contains(&"-bf".to_string()));
        // Tanpa encoder (atau libx264) tetap memakai jalur standar penuh.
        assert_eq!(
            LiveStandardArgs::video_args(None, 14.0, 60),
            LiveStandardArgs::software_args(14.0, 60)
        );
        assert_eq!(
            LiveStandardArgs::video_args(Some("libx264"), 14.0, 60),
            LiveStandardArgs::software_args(14.0, 60)
        );
    }

    #[test]
    fn bitrate_defaults_to_the_app_default() {
        assert_eq!(LiveStandardArgs::resolve_bitrate(None), 8.0);
        assert_eq!(LiveStandardArgs::resolve_bitrate(Some(0.0)), 8.0);
        assert_eq!(LiveStandardArgs::resolve_bitrate(Some(12.5)), 12.5);
    }

    #[test]
    fn live_gop_is_two_seconds() {
        assert_eq!(LiveStandardArgs::gop_frames(30.0), 60);
        assert_eq!(LiveStandardArgs::gop_frames(60.0), 120);
        assert_eq!(LiveStandardArgs::gop_frames(29.97), 60);
        assert_eq!(LiveStandardArgs::gop_frames(0.0), 1);
    }

    #[test]
    fn live_normalize_filter_keeps_source_resolution() {
        assert_eq!(
            LiveStandardArgs::normalize_filter(30.0),
            "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p,setsar=1,fps=30.0"
        );
        assert_eq!(
            LiveStandardArgs::normalize_filter(29.97),
            "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p,setsar=1,fps=29.97"
        );
    }

    #[test]
    fn base_with_overlay_is_pinned() {
        assert_eq!(
            MediaCommandBuilder::base_with_overlay_args("/c.txt", "/mix.wav", "/out.m4a"),
            statics(&[
                "-y",
                "-vn",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                "/c.txt",
                "-stream_loop",
                "-1",
                "-i",
                "/mix.wav",
                "-filter_complex",
                "[0:a][1:a]amix=inputs=2:duration=first[out];[out]volume=2.0[out]",
                "-map",
                "[out]",
                "-c:a",
                "aac",
                "-b:a",
                "384k",
                "-ar",
                "48000",
                "-ac",
                "2",
                "/out.m4a",
            ])
        );
    }

    #[test]
    fn formats_floats_like_python() {
        assert_eq!(fmt_float(1.0), "1.0");
        assert_eq!(fmt_float(0.5), "0.5");
        assert_eq!(fmt_float(0.75), "0.75");
    }
}
