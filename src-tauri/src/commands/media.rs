//! Perintah media tools.

use tauri::State;

use crate::error::AppResult;
use crate::features::media_tools::commands::{DEFAULT_LIVE_BITRATE_MBPS, MIN_LIVE_BITRATE_MBPS};
use crate::features::media_tools::{AudioOverlay, LiveStandardOptions, VideoInfo};
use crate::state::AppState;

#[tauri::command]
pub async fn media_encode_live(
    state: State<'_, AppState>,
    options: LiveStandardOptions,
) -> AppResult<Option<String>> {
    state.start_run()?;
    let workspace = state.workspace();
    let result = match workspace {
        Ok(workspace) => workspace.video_tools.encode_live(&options).await,
        Err(error) => Err(error),
    };
    state.finish_run("media_live");
    result
}

#[tauri::command]
pub async fn media_info(state: State<'_, AppState>, path: String) -> AppResult<VideoInfo> {
    let workspace = state.workspace()?;
    workspace.video_tools.info(&path).await
}

/// Bawaan encoding standar siaran: bitrate bawaan, batas bawah yang diterima,
/// dan interval keyframe (2 detik) — dipakai UI supaya angkanya satu sumber.
#[tauri::command]
pub fn media_live_defaults() -> AppResult<LiveDefaults> {
    Ok(LiveDefaults {
        default_mbps: DEFAULT_LIVE_BITRATE_MBPS,
        min_mbps: MIN_LIVE_BITRATE_MBPS,
        keyframe_seconds: 2,
    })
}

/// Bawaan encoding standar siaran yang ditampilkan di UI.
#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDefaults {
    pub default_mbps: f64,
    pub min_mbps: f64,
    pub keyframe_seconds: i64,
}

#[tauri::command]
pub async fn media_audio_process(
    state: State<'_, AppState>,
    base_audios: Vec<String>,
    overlays: Vec<AudioOverlay>,
    output_path: String,
) -> AppResult<String> {
    state.start_run()?;
    let workspace = state.workspace();
    let result = match workspace {
        Ok(workspace) => {
            workspace
                .audio_tools
                .process_with_overlays(&base_audios, &overlays, &output_path)
                .await
        }
        Err(error) => Err(error),
    };
    state.finish_run("media_audio");
    result
}

#[tauri::command]
pub async fn media_audio_concat(
    state: State<'_, AppState>,
    audio_paths: Vec<String>,
    output_path: String,
) -> AppResult<String> {
    state.start_run()?;
    let workspace = state.workspace();
    let result = match workspace {
        Ok(workspace) => {
            workspace
                .audio_tools
                .concat(&audio_paths, &output_path)
                .await
        }
        Err(error) => Err(error),
    };
    state.finish_run("media_audio_concat");
    result
}

#[tauri::command]
pub async fn media_audio_export_preset(
    state: State<'_, AppState>,
    overlays: Vec<AudioOverlay>,
    output_path: String,
) -> AppResult<String> {
    state.start_run()?;
    let workspace = state.workspace();
    let result = match workspace {
        Ok(workspace) => {
            workspace
                .audio_tools
                .export_overlay_preset(&overlays, &output_path)
                .await
        }
        Err(error) => Err(error),
    };
    state.finish_run("media_preset");
    result
}
