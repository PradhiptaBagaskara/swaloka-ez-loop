//! Tipe error yang bisa diserialisasi untuk dikirim ke frontend.

use serde::Serialize;

/// Error aplikasi dengan satu pesan yang aman ditampilkan ke pengguna.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub message: String,
}

impl AppError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

impl From<crate::core::ffmpeg::runner::FFmpegError> for AppError {
    fn from(error: crate::core::ffmpeg::runner::FFmpegError) -> Self {
        Self::new(error.message)
    }
}

impl From<crate::storage::project::ProjectError> for AppError {
    fn from(error: crate::storage::project::ProjectError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<crate::storage::migration::MigrationError> for AppError {
    fn from(error: crate::storage::migration::MigrationError) -> Self {
        Self::new(error.to_string())
    }
}

/// Hasil standar untuk perintah Tauri.
pub type AppResult<T> = Result<T, AppError>;
