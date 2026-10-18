//! Perintah Kontrol AI (MCP): status, nyalakan, dan matikan server.

use tauri::State;

use crate::error::AppResult;
use crate::mcp::McpStatus;
use crate::state::AppState;
use crate::storage::prefs::Preferences;

#[tauri::command]
pub fn mcp_status(state: State<'_, AppState>) -> AppResult<McpStatus> {
    Ok(state.mcp.status())
}

/// Nyalakan server MCP pada port loopback; port disimpan agar diingat sesi
/// berikutnya, tetapi server tidak pernah dinyalakan otomatis.
#[tauri::command]
pub fn mcp_start(state: State<'_, AppState>, port: Option<u16>) -> AppResult<McpStatus> {
    let port = port.unwrap_or_else(|| state.prefs().mcp_port);
    state.update_prefs(|prefs| Preferences {
        mcp_port: port,
        ..prefs
    });
    Ok(state.mcp.start(port))
}

#[tauri::command]
pub fn mcp_stop(state: State<'_, AppState>) -> AppResult<McpStatus> {
    Ok(state.mcp.stop())
}
