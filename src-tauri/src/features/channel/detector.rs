//! Deteksi aset channel: memindai folder audio dan video, lalu membaca durasi
//! tiap lagu supaya perencanaan bisa menghitung total durasi.

use std::path::Path;
use std::sync::Arc;

use crate::core::ffmpeg::metadata::MetadataReader;
use crate::core::ffmpeg::runner::{FFmpegRunner, LogCallback};
use crate::core::logging::{LogLevel, LogRecord};
use crate::error::AppError;
use crate::features::channel::models::{Channel, ChannelAudioItem, FALLBACK_AUDIO_SECONDS};

/// Ekstensi audio yang dikenali — samakan dengan `src/lib/picker.ts`.
pub const AUDIO_EXTENSIONS: [&str; 6] = ["mp3", "wav", "m4a", "aac", "ogg", "flac"];
/// Ekstensi video yang dikenali — samakan dengan `src/lib/picker.ts`.
pub const VIDEO_EXTENSIONS: [&str; 7] = ["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv"];

/// Hasil pemindaian folder aset sebuah channel.
#[derive(Clone, Debug, Default)]
pub struct DetectedAssets {
    pub audio: Vec<ChannelAudioItem>,
    pub videos: Vec<String>,
    /// Berkas audio yang durasinya tidak terbaca sehingga memakai durasi cadangan.
    pub fallback_audio: Vec<String>,
}

impl DetectedAssets {
    pub fn audio_seconds(&self) -> f64 {
        self.audio
            .iter()
            .map(ChannelAudioItem::effective_duration)
            .sum()
    }
}

/// Pemindai folder aset channel.
pub struct ChannelDetector {
    metadata: Arc<MetadataReader>,
    runner: Arc<FFmpegRunner>,
    on_log: Option<LogCallback>,
}

impl ChannelDetector {
    pub fn new(
        metadata: Arc<MetadataReader>,
        runner: Arc<FFmpegRunner>,
        on_log: Option<LogCallback>,
    ) -> Self {
        Self {
            metadata,
            runner,
            on_log,
        }
    }

    /// Berkas dengan ekstensi yang dikenali di dalam satu folder, terurut
    /// tanpa membedakan huruf besar/kecil. Subfolder diabaikan.
    pub fn scan_folder(folder: &str, extensions: &[&str]) -> Vec<String> {
        let mut paths: Vec<String> = Vec::new();
        if folder.trim().is_empty() {
            return paths;
        }
        let Ok(entries) = std::fs::read_dir(Path::new(folder)) else {
            return paths;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            if extensions
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            {
                paths.push(path.to_string_lossy().to_string());
            }
        }
        paths.sort_by_key(|path| path.to_lowercase());
        paths
    }

    /// Baca durasi tiap lagu; yang gagal memakai durasi cadangan dan dilaporkan.
    pub async fn hydrate_audio(
        &self,
        items: &[ChannelAudioItem],
    ) -> (Vec<ChannelAudioItem>, Vec<String>) {
        let mut resolved: Vec<ChannelAudioItem> = Vec::with_capacity(items.len());
        let mut fallbacks: Vec<String> = Vec::new();
        for item in items {
            match self.metadata.read_duration(&self.runner, &item.path).await {
                Some(duration) if duration > 0.0 => resolved.push(item.with_duration(duration)),
                _ => {
                    fallbacks.push(item.path.clone());
                    resolved.push(item.clone());
                }
            }
        }
        (resolved, fallbacks)
    }

    /// Pindai ulang folder video saja — dipakai setelah pengguna menambahkan
    /// video baru ke folder, tanpa menyentuh daftar lagunya.
    pub fn rescan_videos(&self, channel: &Channel) -> Channel {
        let mut updated = channel.clone();
        if !channel.video_folder_path.trim().is_empty() {
            updated.video_items = Self::scan_folder(&channel.video_folder_path, &VIDEO_EXTENSIONS);
        }
        updated.touch();
        updated
    }

    /// Pindai folder channel dan perbarui `audioItems` serta `videoItems`.
    pub async fn detect(&self, channel: &Channel) -> Result<Channel, AppError> {
        if channel.audio_folder_path.trim().is_empty() {
            return Err(AppError::new("Folder audio channel belum dipilih"));
        }
        if channel.video_folder_path.trim().is_empty() {
            return Err(AppError::new("Folder video channel belum dipilih"));
        }

        let audio_paths = Self::scan_folder(&channel.audio_folder_path, &AUDIO_EXTENSIONS);
        if audio_paths.is_empty() {
            return Err(AppError::new(format!(
                "Tidak ada berkas audio yang didukung di folder audio: {}",
                channel.audio_folder_path
            )));
        }
        let video_paths = Self::scan_folder(&channel.video_folder_path, &VIDEO_EXTENSIONS);
        if video_paths.is_empty() {
            return Err(AppError::new(format!(
                "Tidak ada berkas video di folder video: {}",
                channel.video_folder_path
            )));
        }

        let candidates: Vec<ChannelAudioItem> = audio_paths
            .iter()
            .enumerate()
            .map(|(position, path)| ChannelAudioItem {
                // Indeks audio mulai dari 0: audio pertama adalah audio_0.
                id: format!("audio_{position}"),
                path: path.clone(),
                duration_seconds: None,
            })
            .collect();
        let (audio, fallback_audio) = self.hydrate_audio(&candidates).await;

        let assets = DetectedAssets {
            audio,
            videos: video_paths,
            fallback_audio,
        };
        if !assets.fallback_audio.is_empty() {
            self.emit(
                format!(
                    "Durasi {} lagu tidak terbaca, dipakai {FALLBACK_AUDIO_SECONDS:.0} detik: {}",
                    assets.fallback_audio.len(),
                    assets.fallback_audio.join(", ")
                ),
                LogLevel::Warning,
                1,
            );
        }
        self.emit(
            format!(
                "Terdeteksi {} lagu ({:.0} menit) dan {} video — target {} plan",
                assets.audio.len(),
                assets.audio_seconds() / 60.0,
                assets.videos.len(),
                assets.videos.len()
            ),
            LogLevel::Info,
            0,
        );

        let mut updated = channel.clone();
        updated.audio_items = assets.audio;
        updated.video_items = assets.videos;
        updated.touch();
        Ok(updated)
    }

    fn emit(&self, message: impl Into<String>, level: LogLevel, depth: usize) {
        if let Some(callback) = &self.on_log {
            callback(LogRecord::new(message, level, depth));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detector() -> ChannelDetector {
        ChannelDetector::new(
            Arc::new(MetadataReader::new(true)),
            Arc::new(FFmpegRunner::new("ffmpeg", None)),
            None,
        )
    }

    fn channel_with(audio: &Path, video: &Path) -> Channel {
        let mut channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge"),
            String::from("/p"),
            String::from("/p/outputs"),
            1,
        );
        channel.audio_folder_path = audio.to_string_lossy().to_string();
        channel.video_folder_path = video.to_string_lossy().to_string();
        channel
    }

    fn touch(path: &Path, name: &str) {
        std::fs::write(path.join(name), b"").unwrap();
    }

    #[test]
    fn scan_filters_sorts_and_ignores_subfolders() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        touch(root, "b.mp3");
        touch(root, "A.MP3");
        touch(root, "c.txt");
        std::fs::create_dir(root.join("anak")).unwrap();
        touch(&root.join("anak"), "dalam.mp3");

        let found = ChannelDetector::scan_folder(&root.to_string_lossy(), &AUDIO_EXTENSIONS);
        assert_eq!(found.len(), 2, "hanya berkas audio di folder itu sendiri");
        assert!(found[0].to_lowercase().ends_with("a.mp3"));
        assert!(found[1].ends_with("b.mp3"));
        assert!(
            !found.iter().any(|path| path.contains("dalam")),
            "subfolder diabaikan"
        );
    }

    #[test]
    fn scan_of_missing_folder_is_empty() {
        assert!(ChannelDetector::scan_folder("/tidak/ada", &AUDIO_EXTENSIONS).is_empty());
        assert!(ChannelDetector::scan_folder("", &AUDIO_EXTENSIONS).is_empty());
    }

    #[tokio::test]
    async fn detect_reports_fallback_durations_by_name() {
        let audio = tempfile::tempdir().unwrap();
        let video = tempfile::tempdir().unwrap();
        touch(audio.path(), "lagu_01.mp3");
        touch(audio.path(), "lagu_02.mp3");
        touch(video.path(), "bg_01.mp4");

        let channel = channel_with(audio.path(), video.path());
        let detected = detector().detect(&channel).await.unwrap();

        assert_eq!(detected.audio_items.len(), 2);
        assert_eq!(detected.video_items.len(), 1);
        assert_eq!(detected.plan_target(), 1);
        // Berkas kosong tidak punya durasi terbaca, jadi cadangan yang dipakai.
        assert_eq!(detected.audio_items[0].duration_seconds, None);
        assert_eq!(
            detected.audio_items[0].effective_duration(),
            FALLBACK_AUDIO_SECONDS
        );
        assert_eq!(detected.audio_items[0].id, "audio_0");
        assert_eq!(detected.audio_items[1].id, "audio_1");
    }

    #[tokio::test]
    async fn detect_rejects_empty_folders_naming_them() {
        let audio = tempfile::tempdir().unwrap();
        let video = tempfile::tempdir().unwrap();
        touch(video.path(), "bg.mp4");

        let empty_audio = channel_with(audio.path(), video.path());
        let error = detector().detect(&empty_audio).await.unwrap_err();
        assert!(error.message.contains("folder audio"), "{}", error.message);
        assert!(
            error
                .message
                .contains(&audio.path().to_string_lossy().to_string()),
            "pesan menyebut folder yang bermasalah: {}",
            error.message
        );

        // Kasus kedua: folder audio berisi, folder video kosong.
        let full_audio = tempfile::tempdir().unwrap();
        let empty_video = tempfile::tempdir().unwrap();
        touch(full_audio.path(), "lagu.mp3");

        let case = channel_with(full_audio.path(), empty_video.path());
        let error = detector().detect(&case).await.unwrap_err();
        assert!(error.message.contains("folder video"), "{}", error.message);
        assert!(
            error
                .message
                .contains(&empty_video.path().to_string_lossy().to_string()),
            "pesan menyebut folder video: {}",
            error.message
        );
    }

    #[tokio::test]
    async fn detect_rejects_unset_folders() {
        let channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge"),
            String::from("/p"),
            String::from("/p/outputs"),
            1,
        );
        let error = detector().detect(&channel).await.unwrap_err();
        assert!(error.message.contains("Folder audio"), "{}", error.message);
    }
}
