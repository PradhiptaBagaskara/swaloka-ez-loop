//! Perintah pipeline looping.

use tauri::State;

use crate::error::AppResult;
use crate::features::looping::{LoopRequest, LoopResult, PreflightChecker, PreflightReport};
use crate::state::AppState;

#[tauri::command]
pub async fn loop_run(state: State<'_, AppState>, request: LoopRequest) -> AppResult<LoopResult> {
    state.start_run("Menggabungkan konten")?;
    let workspace = state.workspace();
    let result = match workspace {
        Ok(workspace) => {
            let progress = state.telemetry.progress_callback();
            workspace
                .loop_service
                .process(&request, Some(progress))
                .await
        }
        Err(error) => Err(error),
    };
    state.finish_run("loop");
    result
}

/// Periksa konsistensi input sebelum merge dijalankan.
#[tauri::command]
pub async fn loop_preflight(
    state: State<'_, AppState>,
    background_path: String,
    intro_path: Option<String>,
) -> AppResult<PreflightReport> {
    let background = state.metadata.read(&state.runner, &background_path).await;
    let intro = match intro_path {
        Some(path) if !path.is_empty() => Some(state.metadata.read(&state.runner, &path).await),
        _ => None,
    };
    let checker = PreflightChecker::new();
    Ok(checker.check(&background, intro.as_ref(), true))
}

/// Metadata video untuk pratinjau/gulir info.
#[tauri::command]
pub async fn loop_metadata(
    state: State<'_, AppState>,
    path: String,
) -> AppResult<crate::core::ffmpeg::VideoMetadata> {
    Ok(state.metadata.read(&state.runner, &path).await)
}
