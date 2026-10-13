//! Builder argumen FFmpeg murni untuk pipeline looping.

use std::path::Path;

/// Bendera metadata warna BT.709 yang dipakai jalur merge.
pub const COLOR_METADATA_FLAGS: [&str; 8] = [
    "-colorspace",
    "bt709",
    "-color_trc",
    "bt709",
    "-color_primaries",
    "bt709",
    "-color_range",
    "tv",
];

/// Argumen audio AAC standar pipeline looping.
pub const AAC_AUDIO_ARGS: [&str; 9] = [
    "-vn", "-c:a", "aac", "-b:a", "192k", "-ar", "44100", "-ac", "2",
];

/// Ubah daftar string statis menjadi `Vec<String>`.
fn statics(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

/// Membangun argument vector FFmpeg dan file concat-demuxer.
pub struct LoopCommandBuilder;

impl LoopCommandBuilder {
    /// Escape path untuk dipakai di dalam file daftar concat-demuxer.
    pub fn format_concat_path(path: impl AsRef<Path>) -> String {
        let text = path.as_ref().to_string_lossy().replace('\\', "/");
        text.replace('\'', "'\\''")
    }

    pub fn audio_concat_content(files: &[impl AsRef<Path>]) -> String {
        files
            .iter()
            .map(|path| format!("file '{}'", Self::format_concat_path(path)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn video_concat_content(
        intro: impl AsRef<Path>,
        background: impl AsRef<Path>,
        background_loops: i64,
    ) -> String {
        let mut lines = vec![format!("file '{}'", Self::format_concat_path(intro))];
        for _ in 0..background_loops.max(0) {
            lines.push(format!("file '{}'", Self::format_concat_path(&background)));
        }
        lines.join("\n")
    }

    pub fn audio_concat_args(
        concat_file: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Vec<String> {
        // Disalin, bukan di-encode ulang: normalisasi sudah memastikan setiap
        // berkas memakai codec, sample rate, dan jumlah kanal yang sama, jadi
        // penyambungannya aman dan audio tidak melewati encoder dua kali.
        let mut args = statics(&["-y", "-f", "concat", "-safe", "0", "-i"]);
        args.push(concat_file.as_ref().to_string_lossy().to_string());
        args.extend(statics(&["-vn", "-c", "copy"]));
        args.push(output.as_ref().to_string_lossy().to_string());
        args
    }

    /// Satu panggilan FFmpeg yang mengonversi setiap pasangan `(source, output)` ke AAC.
    pub fn normalize_batch_args(entries: &[(String, String)]) -> Vec<String> {
        let mut args: Vec<String> = vec![String::from("-y")];
        for (source, _) in entries {
            args.push(String::from("-i"));
            args.push(source.clone());
        }
        for (index, (_, output)) in entries.iter().enumerate() {
            args.push(String::from("-map"));
            args.push(format!("{index}:a"));
            args.extend(AAC_AUDIO_ARGS.iter().map(|value| value.to_string()));
            args.push(output.clone());
        }
        args
    }

    pub fn stream_loop_merge_args(
        background: impl AsRef<Path>,
        audio: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut args = statics(&["-y", "-hwaccel", "auto", "-stream_loop", "-1", "-i"]);
        args.push(background.as_ref().to_string_lossy().to_string());
        args.push(String::from("-i"));
        args.push(audio.as_ref().to_string_lossy().to_string());
        args.extend(statics(&["-map", "0:v", "-map", "1:a", "-c:v", "copy"]));
        args.extend(COLOR_METADATA_FLAGS.iter().map(|value| value.to_string()));
        args.extend(statics(&[
            "-c:a",
            "copy",
            "-shortest",
            "-movflags",
            "+faststart",
        ]));
        args.push(output.as_ref().to_string_lossy().to_string());
        args
    }

    pub fn preloop_args(
        background: impl AsRef<Path>,
        additional_loops: i64,
        output: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut args = statics(&["-y", "-hwaccel", "auto", "-stream_loop"]);
        args.push(additional_loops.to_string());
        args.push(String::from("-i"));
        args.push(background.as_ref().to_string_lossy().to_string());
        args.extend(statics(&["-map", "0:v:0", "-c:v", "copy", "-an"]));
        args.push(output.as_ref().to_string_lossy().to_string());
        args
    }

    pub fn concat_video_merge_args(
        concat_file: impl AsRef<Path>,
        audio: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Vec<String> {
        let mut args = statics(&["-y", "-hwaccel", "auto", "-f", "concat", "-safe", "0", "-i"]);
        args.push(concat_file.as_ref().to_string_lossy().to_string());
        args.push(String::from("-i"));
        args.push(audio.as_ref().to_string_lossy().to_string());
        args.extend(statics(&["-map", "0:v", "-map", "1:a", "-c:v", "copy"]));
        args.extend(COLOR_METADATA_FLAGS.iter().map(|value| value.to_string()));
        args.extend(statics(&[
            "-c:a",
            "copy",
            "-shortest",
            "-movflags",
            "+faststart",
        ]));
        args.push(output.as_ref().to_string_lossy().to_string());
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    // Argumen-argumen ini dipatok agar identik dengan implementasi Python lama
    // (padanan tests/test_documented_commands.py).
    #[test]
    fn stream_loop_merge_is_pinned() {
        assert_eq!(
            LoopCommandBuilder::stream_loop_merge_args("/bg.mp4", "/a.m4a", "/out.mp4"),
            strings(&[
                "-y",
                "-hwaccel",
                "auto",
                "-stream_loop",
                "-1",
                "-i",
                "/bg.mp4",
                "-i",
                "/a.m4a",
                "-map",
                "0:v",
                "-map",
                "1:a",
                "-c:v",
                "copy",
                "-colorspace",
                "bt709",
                "-color_trc",
                "bt709",
                "-color_primaries",
                "bt709",
                "-color_range",
                "tv",
                "-c:a",
                "copy",
                "-shortest",
                "-movflags",
                "+faststart",
                "/out.mp4",
            ])
        );
    }

    #[test]
    fn concat_video_merge_is_pinned() {
        assert_eq!(
            LoopCommandBuilder::concat_video_merge_args("/c.txt", "/a.m4a", "/out.mp4"),
            strings(&[
                "-y",
                "-hwaccel",
                "auto",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                "/c.txt",
                "-i",
                "/a.m4a",
                "-map",
                "0:v",
                "-map",
                "1:a",
                "-c:v",
                "copy",
                "-colorspace",
                "bt709",
                "-color_trc",
                "bt709",
                "-color_primaries",
                "bt709",
                "-color_range",
                "tv",
                "-c:a",
                "copy",
                "-shortest",
                "-movflags",
                "+faststart",
                "/out.mp4",
            ])
        );
    }

    #[test]
    fn preloop_and_audio_concat_are_pinned() {
        assert_eq!(
            LoopCommandBuilder::preloop_args("/bg.mp4", 3, "/pre.mp4"),
            strings(&[
                "-y",
                "-hwaccel",
                "auto",
                "-stream_loop",
                "3",
                "-i",
                "/bg.mp4",
                "-map",
                "0:v:0",
                "-c:v",
                "copy",
                "-an",
                "/pre.mp4",
            ])
        );
        assert_eq!(
            LoopCommandBuilder::audio_concat_args("/c.txt", "/m.m4a"),
            strings(&[
                "-y", "-f", "concat", "-safe", "0", "-i", "/c.txt", "-vn", "-c", "copy", "/m.m4a",
            ])
        );
    }

    #[test]
    fn normalize_batch_is_pinned() {
        assert_eq!(
            LoopCommandBuilder::normalize_batch_args(&[
                ("/a.mp3".into(), "/ca.m4a".into()),
                ("/b.wav".into(), "/cb.m4a".into()),
            ]),
            strings(&[
                "-y", "-i", "/a.mp3", "-i", "/b.wav", "-map", "0:a", "-vn", "-c:a", "aac", "-b:a",
                "192k", "-ar", "44100", "-ac", "2", "/ca.m4a", "-map", "1:a", "-vn", "-c:a", "aac",
                "-b:a", "192k", "-ar", "44100", "-ac", "2", "/cb.m4a",
            ])
        );
    }

    #[test]
    fn concat_content_escapes_paths() {
        assert_eq!(
            LoopCommandBuilder::audio_concat_content(&["/a b/c.mp3"]),
            "file '/a b/c.mp3'"
        );
        assert_eq!(
            LoopCommandBuilder::video_concat_content("/i.mp4", "/bg.mp4", 2),
            "file '/i.mp4'\nfile '/bg.mp4'\nfile '/bg.mp4'"
        );
    }
}
