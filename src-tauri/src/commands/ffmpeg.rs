//! Perintah seputar deteksi, konfigurasi, dan pemasangan FFmpeg.

use std::path::Path;

use tauri::State;

use crate::core::ffmpeg::install::{self, BinaryKind};
use crate::core::ffmpeg::{plan_for, EncoderDetector, HardwareEncoder, PINNED_VERSION};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[tauri::command]
pub async fn ffmpeg_check(state: State<'_, AppState>) -> AppResult<bool> {
    let locator = state.locator();
    let available = locator.is_available(&state.runner).await;
    if available {
        state.configure_ffmpeg();
    }
    Ok(available)
}

/// Asal biner FFmpeg yang sedang dipakai aplikasi.
#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FfmpegSource {
    /// Dipasang aplikasi sendiri lewat layar setup.
    Installed,
    /// Path pilihan pengguna di Pengaturan.
    Custom,
    /// Belum ada sama sekali.
    Missing,
}

/// Status FFmpeg untuk layar setup dan Dasbor.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub ready: bool,
    pub version: Option<String>,
    pub source: FfmpegSource,
    pub path: Option<String>,
    /// Ukuran unduhan platform ini, untuk teks tombol di UI.
    pub expected_bytes: Option<u64>,
    pub pinned_version: String,
}

pub(crate) async fn status_of(state: &AppState) -> AppResult<FfmpegStatus> {
    let locator = state.locator();
    let ready = locator.is_available(&state.runner).await;
    let resolved = locator.resolve_ffmpeg();

    let custom = locator.custom_path().map(str::to_string);
    let installed = install::binary_path(BinaryKind::Ffmpeg);
    let (source, path) = if custom.is_some() {
        (FfmpegSource::Custom, custom)
    } else if installed.is_file() {
        (
            FfmpegSource::Installed,
            Some(installed.to_string_lossy().to_string()),
        )
    } else {
        (FfmpegSource::Missing, None)
    };

    let version = if ready {
        install::verify_version(Path::new(&resolved)).ok()
    } else {
        None
    };

    Ok(FfmpegStatus {
        ready,
        version,
        source,
        path: if ready { Some(resolved) } else { path },
        expected_bytes: install::expected_bytes(),
        pinned_version: PINNED_VERSION.to_string(),
    })
}

#[tauri::command]
pub async fn ffmpeg_status(state: State<'_, AppState>) -> AppResult<FfmpegStatus> {
    status_of(&state).await
}

/// Unduh dan pasang FFmpeg bawaan aplikasi; progres dikirim lewat event
/// `setup://progress` supaya layar setup bisa menampilkan bilah unduhan.
#[tauri::command]
pub async fn ffmpeg_install(state: State<'_, AppState>) -> AppResult<FfmpegStatus> {
    let plan = plan_for(std::env::consts::OS, std::env::consts::ARCH)?;
    let target = install::install_dir();
    let cancel = state.installer.begin()?;
    let progress = state.telemetry.install_callback();

    let result =
        tokio::task::spawn_blocking(move || install::install(&plan, &target, &progress, &cancel))
            .await
            .map_err(|error| AppError::new(format!("Pemasangan FFmpeg gagal: {error}")));

    state.installer.end();
    result??;

    state.configure_ffmpeg();
    status_of(&state).await
}

#[tauri::command]
pub fn ffmpeg_install_cancel(state: State<'_, AppState>) -> AppResult<()> {
    state.installer.request_cancel();
    Ok(())
}

#[tauri::command]
pub async fn ffmpeg_detect_encoder(state: State<'_, AppState>) -> AppResult<String> {
    let detector = EncoderDetector::new(None);
    let encoder = detector.detect(&state.runner).await;
    Ok(encoder.value().to_string())
}

#[tauri::command]
pub fn ffmpeg_encoders() -> AppResult<Vec<EncoderOption>> {
    Ok(HardwareEncoder::all()
        .into_iter()
        .map(|encoder| EncoderOption {
            value: encoder.value().to_string(),
            label: encoder.label().to_string(),
        })
        .collect())
}

#[tauri::command]
pub fn ffmpeg_configure(state: State<'_, AppState>) -> AppResult<bool> {
    Ok(state.configure_ffmpeg())
}

/// Opsi encoder untuk dropdown pengaturan.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderOption {
    pub value: String,
    pub label: String,
}
