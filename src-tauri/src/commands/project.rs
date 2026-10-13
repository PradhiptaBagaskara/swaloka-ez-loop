//! Perintah seputar proyek, preferensi, dan statistik dasbor.

use std::path::Path;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::{AppState, ProjectStats};
use crate::storage::prefs::Preferences;
use crate::storage::project::Project;

#[tauri::command]
pub fn project_current(state: State<'_, AppState>) -> AppResult<Option<Project>> {
    Ok(state.project())
}

#[tauri::command]
pub fn project_create(
    state: State<'_, AppState>,
    root_path: String,
    name: Option<String>,
) -> AppResult<Project> {
    std::fs::create_dir_all(&root_path)?;
    let project = state.project_store().create(&root_path, name)?;
    state.activate_project(project)
}

#[tauri::command]
pub fn project_open(state: State<'_, AppState>, root_path: String) -> AppResult<Project> {
    let project = state.project_store().load(&root_path)?;
    state.activate_project(project)
}

#[tauri::command]
pub fn project_close(state: State<'_, AppState>) -> AppResult<()> {
    state.close_project();
    Ok(())
}

#[tauri::command]
pub fn project_set_output(
    state: State<'_, AppState>,
    output_path: Option<String>,
) -> AppResult<Project> {
    let mut project = state
        .project()
        .ok_or_else(|| AppError::new("belum ada proyek yang terbuka"))?;
    project.custom_output_path = output_path.filter(|value| !value.is_empty());
    state.project_store().save(&project)?;
    state.activate_project(project)
}

#[tauri::command]
pub fn prefs_get(state: State<'_, AppState>) -> AppResult<Preferences> {
    Ok(state.prefs())
}

#[tauri::command]
pub fn prefs_set(state: State<'_, AppState>, prefs: Preferences) -> AppResult<Preferences> {
    let saved = state.set_prefs(prefs)?;
    state.configure_ffmpeg();
    Ok(saved)
}

#[tauri::command]
pub fn recent_projects(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    Ok(state.prefs().recent_projects)
}

#[tauri::command]
pub fn remove_recent(state: State<'_, AppState>, path: String) -> AppResult<Preferences> {
    Ok(state.update_prefs(|prefs| prefs.remove_recent(&path)))
}

#[tauri::command]
pub async fn project_stats(state: State<'_, AppState>) -> AppResult<ProjectStats> {
    let Some(project) = state.project() else {
        return Ok(ProjectStats {
            output_count: 0,
            cache_count: 0,
            ffmpeg_ready: false,
        });
    };

    let output_count = std::fs::read_dir(project.output_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_file())
                .count()
        })
        .unwrap_or(0);

    let cache_count = std::fs::read_dir(project.audio_cache_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_file())
                .count()
        })
        .unwrap_or(0);

    let locator = state.locator();
    let ffmpeg_ready = locator.is_available(&state.runner).await;

    Ok(ProjectStats {
        output_count,
        cache_count,
        ffmpeg_ready,
    })
}

#[allow(dead_code)]
fn exists(path: &str) -> bool {
    Path::new(path).exists()
}
