//! Membaca metadata media dengan FFprobe.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;

use crate::core::audio_tags::read_audio_duration;
use crate::core::ffmpeg::runner::{FFmpegRunner, ProcessResult};

/// Pemetaan kunci tag ffprobe -> label yang ditampilkan.
const TAG_FIELDS: [(&str, &str); 31] = [
    ("title", "Title"),
    ("artist", "Artist"),
    ("author", "Author"),
    ("album_artist", "Album Artist"),
    ("album", "Album"),
    ("comment", "Comment"),
    ("description", "Description"),
    ("synopsis", "Synopsis"),
    ("copyright", "Copyright"),
    ("creation_time", "Creation Date"),
    ("date", "Date"),
    ("year", "Year"),
    ("encoder", "Encoder"),
    ("encoded_by", "Encoded By"),
    ("genre", "Genre"),
    ("track", "Track"),
    ("disc", "Disc"),
    ("publisher", "Publisher"),
    ("service_name", "Service Name"),
    ("service_provider", "Service Provider"),
    ("language", "Language"),
    ("rating", "Rating"),
    ("director", "Director"),
    ("producer", "Producer"),
    ("composer", "Composer"),
    ("performer", "Performer"),
    ("lyrics", "Lyrics"),
    ("network", "Network"),
    ("show", "Show"),
    ("episode_id", "Episode ID"),
    ("season_number", "Season Number"),
];

/// Properti stream video/audio plus payload FFprobe mentah.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub path: String,
    pub readable: bool,
    pub error: Option<String>,
    pub codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<String>,
    pub pix_fmt: Option<String>,
    pub duration: Option<f64>,
    pub has_audio: bool,
    pub audio_codec: Option<String>,
    /// Sample rate stream audio (Hz); dipakai normalizer untuk memutuskan
    /// berkas boleh disalin apa adanya saat penyambungan.
    pub sample_rate: Option<i64>,
    /// Jumlah kanal stream audio.
    pub channels: Option<i64>,
    pub tags: HashMap<String, String>,
    #[serde(skip_serializing)]
    pub raw: Option<Value>,
    pub color_space: Option<String>,
    pub color_primaries: Option<String>,
    pub color_transfer: Option<String>,
    pub color_range: Option<String>,
    pub profile: Option<String>,
    pub level: Option<String>,
    pub b_frames: Option<i64>,
    /// Jumlah reference frame (`refs` di ffprobe) — berbeda dari B-frame.
    pub ref_frames: Option<i64>,
    pub gop_size: Option<i64>,
    pub cabac: Option<bool>,
    pub interlaced: Option<bool>,
    pub bitrate_kbps: Option<i64>,
}

impl VideoMetadata {
    /// Metadata yang tidak terbaca (mis. file rusak).
    pub fn unreadable(path: &str, error: impl Into<String>) -> Self {
        Self {
            path: path.to_string(),
            readable: false,
            error: Some(error.into()),
            ..Default::default()
        }
    }

    /// Nilai fps sebagai angka, default 30.0.
    pub fn fps_value(&self) -> f64 {
        self.fps
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(30.0)
    }

    pub fn resolution(&self) -> Option<(u32, u32)> {
        Some((self.width?, self.height?))
    }
}

/// Membaca metadata lewat satu panggilan FFprobe, dengan cache in-memory.
pub struct MetadataReader {
    cache_enabled: bool,
    cache: Mutex<HashMap<String, VideoMetadata>>,
}

impl Default for MetadataReader {
    fn default() -> Self {
        Self::new(true)
    }
}

impl MetadataReader {
    pub fn new(cache: bool) -> Self {
        Self {
            cache_enabled: cache,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn invalidate(&self, path: Option<&str>) {
        let mut cache = self.cache.lock().expect("cache metadata terkunci");
        match path {
            Some(path) => {
                cache.remove(path);
            }
            None => cache.clear(),
        }
    }

    pub async fn read(&self, runner: &FFmpegRunner, path: &str) -> VideoMetadata {
        if self.cache_enabled {
            if let Some(found) = self.cache.lock().expect("cache terkunci").get(path) {
                return found.clone();
            }
        }
        let metadata = self.probe(runner, path).await;
        if self.cache_enabled {
            self.cache
                .lock()
                .expect("cache terkunci")
                .insert(path.to_string(), metadata.clone());
        }
        metadata
    }

    /// Durasi file: jalur cepat tag audio lokal, lalu ffprobe.
    pub async fn read_duration(&self, runner: &FFmpegRunner, path: &str) -> Option<f64> {
        if let Some(fast) = read_audio_duration(std::path::Path::new(path)) {
            return Some(fast);
        }
        self.read(runner, path).await.duration
    }

    async fn probe(&self, runner: &FFmpegRunner, path: &str) -> VideoMetadata {
        let args: Vec<String> = [
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path,
        ]
        .iter()
        .map(|value| value.to_string())
        .collect();

        let result = match runner.run_probe(args, None, None, false, true).await {
            Ok(result) => result,
            Err(error) => return VideoMetadata::unreadable(path, error.message),
        };

        if !result.ok() {
            return VideoMetadata::unreadable(path, failure_note(&result));
        }

        let joined = result.stdout.join("\n");
        let payload = if joined.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            match serde_json::from_str::<Value>(&joined) {
                Ok(value) => value,
                Err(error) => {
                    return VideoMetadata::unreadable(
                        path,
                        format!("invalid ffprobe output: {error}"),
                    )
                }
            }
        };
        build_metadata(path, &payload)
    }
}

/// Bangun [`VideoMetadata`] dari payload FFprobe.
pub fn build_metadata(path: &str, payload: &Value) -> VideoMetadata {
    let streams = payload
        .get("streams")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let video = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("video"));
    let audio = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"));
    let format = payload.get("format").cloned().unwrap_or(Value::Null);

    if video.is_none() && audio.is_none() {
        return VideoMetadata {
            path: path.to_string(),
            readable: false,
            error: Some(String::from("no media streams")),
            raw: Some(payload.clone()),
            ..Default::default()
        };
    }

    let stream = video.cloned().unwrap_or(Value::Null);
    let tags = extract_tags(format.get("tags").unwrap_or(&Value::Null));

    VideoMetadata {
        path: path.to_string(),
        readable: true,
        error: None,
        codec: stream
            .get("codec_name")
            .and_then(Value::as_str)
            .map(str::to_string),
        width: stream
            .get("width")
            .and_then(Value::as_u64)
            .map(|v| v as u32),
        height: stream
            .get("height")
            .and_then(Value::as_u64)
            .map(|v| v as u32),
        fps: stream
            .get("r_frame_rate")
            .and_then(Value::as_str)
            .and_then(format_frame_rate),
        pix_fmt: stream
            .get("pix_fmt")
            .and_then(Value::as_str)
            .map(str::to_string),
        duration: format_duration_value(&format, &stream),
        has_audio: audio.is_some(),
        sample_rate: audio
            .as_ref()
            .and_then(|value| value.get("sample_rate"))
            .and_then(Value::as_str)
            .and_then(|text| text.parse::<i64>().ok()),
        channels: audio
            .as_ref()
            .and_then(|value| value.get("channels"))
            .and_then(Value::as_i64),
        audio_codec: audio
            .and_then(|value| value.get("codec_name"))
            .and_then(Value::as_str)
            .map(str::to_string),
        tags,
        raw: Some(payload.clone()),
        color_space: string_field(&stream, "color_space"),
        color_primaries: string_field(&stream, "color_primaries"),
        color_transfer: string_field(&stream, "color_transfer"),
        color_range: string_field(&stream, "color_range"),
        profile: string_field(&stream, "profile"),
        level: stream
            .get("level")
            .and_then(Value::as_i64)
            .map(|value| format!("{:.1}", value as f64 / 10.0)),
        b_frames: stream.get("has_b_frames").and_then(Value::as_i64),
        ref_frames: stream.get("refs").and_then(Value::as_i64),
        gop_size: first_int(&stream, &["keyint", "g", "frame_gop_size"]),
        cabac: stream
            .get("coder")
            .and_then(Value::as_i64)
            .map(|value| value == 1),
        interlaced: stream
            .get("field_order")
            .and_then(Value::as_str)
            .map(|value| value != "progressive"),
        bitrate_kbps: format.get("bit_rate").and_then(bitrate_from_value),
    }
}

fn string_field(source: &Value, key: &str) -> Option<String> {
    source.get(key).and_then(Value::as_str).map(str::to_string)
}

fn format_frame_rate(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let result = if let Some((numerator, denominator)) = value.split_once('/') {
        let num: f64 = numerator.parse().ok()?;
        let den: f64 = denominator.parse().ok()?;
        if den == 0.0 {
            return None;
        }
        format!("{:.2}", num / den)
    } else {
        format!("{:.2}", value.parse::<f64>().ok()?)
    };
    if let Some(stripped) = result.strip_suffix(".00") {
        Some(stripped.to_string())
    } else {
        Some(result)
    }
}

fn format_duration_value(format: &Value, video: &Value) -> Option<f64> {
    for source in [format, video] {
        if let Some(value) = source.get("duration") {
            if let Some(text) = value.as_str() {
                if let Ok(parsed) = text.parse::<f64>() {
                    return Some(parsed);
                }
            } else if let Some(number) = value.as_f64() {
                return Some(number);
            }
        }
    }
    None
}

fn bitrate_from_value(value: &Value) -> Option<i64> {
    let raw = if let Some(text) = value.as_str() {
        text.parse::<i64>().ok()?
    } else {
        value.as_i64()?
    };
    Some((raw as f64 / 1000.0).round() as i64)
}

fn first_int(source: &Value, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(value) = source.get(*key) {
            if let Some(number) = value.as_i64() {
                return Some(number);
            }
            if let Some(text) = value.as_str() {
                if let Ok(parsed) = text.parse::<i64>() {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

fn extract_tags(tags: &Value) -> HashMap<String, String> {
    let mut found = HashMap::new();
    let Some(map) = tags.as_object() else {
        return found;
    };
    for (key, label) in TAG_FIELDS {
        if let Some(value) = map.get(key) {
            let text = match value {
                Value::String(text) => text.clone(),
                Value::Number(number) => number.to_string(),
                Value::Bool(flag) => flag.to_string(),
                _ => continue,
            };
            if !text.is_empty() {
                found.insert(label.to_string(), text);
            }
        }
    }
    found
}

fn failure_note(result: &ProcessResult) -> String {
    let source = if result.stderr.is_empty() {
        &result.stdout
    } else {
        &result.stderr
    };
    let start = source.len().saturating_sub(3);
    let detail = source[start..].join(" ");
    format!(
        "ffprobe exited with code {}: {}",
        result.returncode,
        detail.trim()
    )
    .trim()
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_frame_rate() {
        assert_eq!(format_frame_rate("30/1").as_deref(), Some("30"));
        assert_eq!(format_frame_rate("30000/1001").as_deref(), Some("29.97"));
        assert_eq!(format_frame_rate("25").as_deref(), Some("25"));
    }

    #[test]
    fn builds_metadata_from_payload() {
        let payload = json!({
            "streams": [
                {"codec_type": "video", "codec_name": "h264", "width": 1920, "height": 1080,
                 "r_frame_rate": "30/1", "pix_fmt": "yuv420p", "profile": "High", "level": 41,
                 "refs": 2, "keyint": 15, "coder": 1, "field_order": "progressive",
                 "has_b_frames": 2,
                 "color_space": "bt709"},
                {"codec_type": "audio", "codec_name": "aac"}
            ],
            "format": {"duration": "12.5", "bit_rate": "5000000", "tags": {"title": "Uji"}}
        });
        let metadata = build_metadata("/x.mp4", &payload);
        assert!(metadata.readable);
        assert_eq!(metadata.codec.as_deref(), Some("h264"));
        assert_eq!(metadata.resolution(), Some((1920, 1080)));
        assert_eq!(metadata.fps_value(), 30.0);
        assert!(metadata.has_audio);
        assert_eq!(metadata.audio_codec.as_deref(), Some("aac"));
        assert_eq!(metadata.gop_size, Some(15));
        assert_eq!(metadata.b_frames, Some(2));
        assert_eq!(metadata.ref_frames, Some(2));
        assert_eq!(metadata.cabac, Some(true));
        assert_eq!(metadata.interlaced, Some(false));
        assert_eq!(metadata.level.as_deref(), Some("4.1"));
        assert_eq!(metadata.bitrate_kbps, Some(5000));
        assert_eq!(metadata.tags.get("Title").map(String::as_str), Some("Uji"));
    }

    #[test]
    fn reports_missing_streams() {
        let metadata = build_metadata("/x", &json!({"streams": []}));
        assert!(!metadata.readable);
        assert_eq!(metadata.error.as_deref(), Some("no media streams"));
    }
}
