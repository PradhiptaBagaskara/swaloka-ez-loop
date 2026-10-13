//! Perintah channel: definisi channel, deteksi + perencanaan, cek kapasitas, dan eksekusi.

use serde::Deserialize;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::features::channel::{
    Channel, ChannelCapacityReport, ChannelPlanner, ChannelStatus, DEFAULT_MIN_DURATION_MINUTES,
    DEFAULT_SUGGESTED_LOOP_CAP,
};
use crate::state::AppState;

#[tauri::command]
pub fn channel_list(state: State<'_, AppState>) -> AppResult<Vec<Channel>> {
    Ok(state.workspace()?.channel_repository.list())
}

#[tauri::command]
pub fn channel_load(state: State<'_, AppState>, channel_id: String) -> AppResult<Option<Channel>> {
    Ok(state.workspace()?.channel_repository.load(&channel_id))
}

/// Input pembuatan channel baru.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelCreateInput {
    pub name: String,
    #[serde(default)]
    pub audio_folder_path: String,
    #[serde(default)]
    pub video_folder_path: String,
    #[serde(default = "default_minutes")]
    pub min_duration_minutes: f64,
    /// Jumlah konten yang ingin dibuat; 0 berarti mengikuti jumlah video.
    #[serde(default)]
    pub total_content: i64,
    /// Target putar tiap lagu; 0 berarti memakai bawaan aplikasi.
    #[serde(default = "default_loop_cap")]
    pub suggested_loop_cap: i64,
}

fn default_loop_cap() -> i64 {
    DEFAULT_SUGGESTED_LOOP_CAP
}

fn default_minutes() -> f64 {
    DEFAULT_MIN_DURATION_MINUTES
}

#[tauri::command]
pub fn channel_create(state: State<'_, AppState>, input: ChannelCreateInput) -> AppResult<Channel> {
    let workspace = state.workspace()?;
    let project = state
        .project()
        .ok_or_else(|| AppError::new("belum ada proyek yang terbuka"))?;

    let millis = chrono::Local::now().timestamp_millis();
    let mut channel = Channel::create(
        format!("ch_{millis}"),
        input.name,
        project.root_path.clone(),
        project.output_dir().to_string_lossy().to_string(),
        millis,
    );
    channel.audio_folder_path = input.audio_folder_path;
    channel.video_folder_path = input.video_folder_path;
    channel.total_content = input.total_content.max(0);
    if input.suggested_loop_cap > 0 {
        channel.suggested_loop_cap = input.suggested_loop_cap;
    }
    if input.min_duration_minutes > 0.0 {
        channel.min_duration_minutes = input.min_duration_minutes;
    }
    workspace.channel_repository.save(&channel)?;
    Ok(channel)
}

#[tauri::command]
pub fn channel_save(state: State<'_, AppState>, channel: Channel) -> AppResult<Channel> {
    let workspace = state.workspace()?;
    let mut channel = channel;
    channel.touch();
    workspace.channel_repository.save(&channel)?;
    Ok(channel)
}

#[tauri::command]
pub fn channel_delete(state: State<'_, AppState>, channel_id: String) -> AppResult<()> {
    state.workspace()?.channel_repository.delete(&channel_id)?;
    Ok(())
}

/// Deteksi ulang aset channel lalu susun rencananya.
#[tauri::command]
pub async fn channel_plan(state: State<'_, AppState>, channel: Channel) -> AppResult<Channel> {
    state.start_run()?;
    let workspace = state.workspace();
    let result = match workspace {
        Ok(workspace) => match workspace.channel_planner.plan(&channel).await {
            Ok(planned) => {
                workspace.channel_repository.save(&planned)?;
                Ok(planned)
            }
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    state.finish_run("channel_plan");
    result
}

/// Pindai ulang folder video lalu pasangkan video bebas ke rencana yang menunggu.
#[tauri::command]
pub fn channel_rescan_videos(state: State<'_, AppState>, channel: Channel) -> AppResult<Channel> {
    let workspace = state.workspace()?;
    let refreshed = workspace.channel_detector.rescan_videos(&channel);
    let assigned = ChannelPlanner::assign_waiting_videos(&refreshed);
    workspace.channel_repository.save(&assigned)?;
    Ok(assigned)
}

/// Estimasi kebutuhan disk rencana terpilih dan ruang yang tersedia.
#[tauri::command]
pub async fn channel_preflight(
    state: State<'_, AppState>,
    channel: Channel,
) -> AppResult<ChannelCapacityReport> {
    let workspace = state.workspace()?;
    Ok(workspace.channel_capacity.report(&channel).await)
}

/// Proses rencana terpilih satu per satu.
#[tauri::command]
pub async fn channel_run(state: State<'_, AppState>, channel: Channel) -> AppResult<Channel> {
    state.start_run()?;
    let workspace = state.workspace();
    let result = match workspace {
        Ok(workspace) => {
            let mut running = channel;
            running.status = ChannelStatus::Running;
            let progress = state.telemetry.progress_callback();
            workspace
                .channel_executor
                .run_selection(&running, Some(progress))
                .await
        }
        Err(error) => Err(error),
    };
    state.finish_run("channel_run");
    result
}

#[tauri::command]
pub fn channel_cancel(state: State<'_, AppState>) -> AppResult<()> {
    if let Ok(workspace) = state.workspace() {
        workspace.channel_executor.request_cancel();
    }
    state.runner.cancel();
    Ok(())
}
