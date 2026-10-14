//! Perintah sistem: batalkan run dan bersihkan cache.

use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::state::AppState;
use crate::storage::cache::CacheClearResult;

#[tauri::command]
pub fn cancel_run(state: State<'_, AppState>) -> AppResult<()> {
    state.runner.cancel();
    Ok(())
}

#[tauri::command]
pub fn clean_cache(state: State<'_, AppState>) -> AppResult<CacheClearResult> {
    let workspace = state.workspace()?;
    Ok(workspace.cache.clear())
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> AppResult<String> {
    Ok(app.package_info().version.to_string())
}
