//! Konstanta tata letak filesystem dan direktori konfigurasi per platform.

use std::path::PathBuf;

pub const PROJECT_FILE_NAME: &str = "project.json";
pub const LEGACY_PROJECT_FILE_NAME: &str = "project.swaloka";
pub const PROJECT_FORMAT_VERSION: i32 = 2;

pub const OUTPUTS_DIR: &str = "outputs";
pub const LOGS_DIR: &str = "logs";
pub const TEMP_DIR: &str = "temp";
pub const CACHE_DIR: &str = "cache";
pub const AUDIO_CACHE_DIR: &str = "audios";
pub const AUDIO_CACHE_MANIFEST: &str = "audio_cache.json";
pub const CHANNELS_DIR: &str = "channels";

pub const APP_DIR_NAME: &str = "SwalokaLoopingTool";

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Direktori konfigurasi per pengguna untuk aplikasi.
pub fn config_dir() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| {
            home_dir()
                .join("AppData")
                .join("Roaming")
                .to_string_lossy()
                .to_string()
        });
        return PathBuf::from(base).join(APP_DIR_NAME);
    }
    if cfg!(target_os = "macos") {
        return home_dir()
            .join("Library")
            .join("Application Support")
            .join(APP_DIR_NAME);
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| home_dir().join(".config").to_string_lossy().to_string());
    PathBuf::from(base).join("swaloka-looping-tool")
}

pub fn preferences_file() -> PathBuf {
    config_dir().join("preferences.json")
}
