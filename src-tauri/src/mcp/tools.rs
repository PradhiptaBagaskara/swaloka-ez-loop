//! Katalog tool MCP: adaptor tipis di atas service yang sama dipakai antarmuka.
//!
//! Tool sengaja berorientasi aksi dan memakai kosakata domain: unit hasil merge
//! disebut "content" (`loop_create_content`), bukan "episode".

use std::path::{Component, Path, PathBuf};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::tool;
use rmcp::tool_router;
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::features::channel::{
    Channel, ChannelPlanner, ChannelStatus, DEFAULT_MIN_DURATION_MINUTES,
    DEFAULT_SUGGESTED_LOOP_CAP,
};
use crate::features::looping::{LoopRequest, PreflightChecker};
use crate::state::AppState;

/// Handler tool MCP. Setiap tool membaca [`AppState`] yang sama dengan antarmuka.
#[derive(Clone)]
pub struct SwalokaMcp {
    app: AppHandle,
}

impl SwalokaMcp {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }
}

fn ok(value: Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::text(
        value.to_string(),
    )]))
}

fn fail(message: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![ContentBlock::text(
        message.into(),
    )]))
}

fn to_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn default_one() -> i64 {
    1
}

fn default_minutes() -> f64 {
    DEFAULT_MIN_DURATION_MINUTES
}

fn default_loop_cap() -> i64 {
    DEFAULT_SUGGESTED_LOOP_CAP
}

/// Normalisasi leksikal: buang `.` dan selesaikan `..` tanpa menyentuh disk.
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

/// Tolak path yang keluar dari root proyek.
fn required_within(root: &Path, candidate: &str) -> Result<(), String> {
    let raw = if Path::new(candidate).is_absolute() {
        PathBuf::from(candidate)
    } else {
        root.join(candidate)
    };
    if normalize(&raw).starts_with(root) {
        Ok(())
    } else {
        Err(format!("path di luar proyek: {candidate}"))
    }
}

/// Batalkan run yang sedang berjalan (merge maupun batch channel).
fn cancel(state: &AppState) {
    if let Ok(workspace) = state.workspace() {
        workspace.channel_executor.request_cancel();
    }
    state.runner.cancel();
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ProjectOpenParams {
    root_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ProjectCreateParams {
    root_path: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct LoopPreflightParams {
    background_path: String,
    #[serde(default)]
    intro_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PathParams {
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct LoopCreateParams {
    background_path: String,
    #[serde(default)]
    audio_files: Vec<String>,
    output_dir: String,
    #[serde(default)]
    output_name: String,
    #[serde(default = "default_one")]
    loop_count: i64,
    #[serde(default)]
    intro_path: Option<String>,
    #[serde(default)]
    output_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ChannelIdParams {
    channel_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ChannelCreateParams {
    name: String,
    #[serde(default)]
    audio_folder_path: String,
    #[serde(default)]
    video_folder_path: String,
    #[serde(default = "default_minutes")]
    min_duration_minutes: f64,
    #[serde(default)]
    total_content: i64,
    #[serde(default = "default_loop_cap")]
    suggested_loop_cap: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ChannelSaveParams {
    channel: Value,
}

#[tool_router(server_handler)]
impl SwalokaMcp {
    #[tool(
        description = "Open an existing Swaloka project folder and make it the active project, shared with the running app."
    )]
    async fn project_open(
        &self,
        Parameters(params): Parameters<ProjectOpenParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        match state.project_store().load(&params.root_path) {
            Ok(project) => match state.activate_project(project) {
                Ok(project) => ok(to_value(&project)),
                Err(error) => fail(error.message),
            },
            Err(error) => fail(error.to_string()),
        }
    }

    #[tool(
        description = "Create a new Swaloka project at a folder and make it the active project."
    )]
    async fn project_create(
        &self,
        Parameters(params): Parameters<ProjectCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        if let Err(error) = std::fs::create_dir_all(&params.root_path) {
            return fail(error.to_string());
        }
        match state.project_store().create(&params.root_path, params.name) {
            Ok(project) => match state.activate_project(project) {
                Ok(project) => ok(to_value(&project)),
                Err(error) => fail(error.message),
            },
            Err(error) => fail(error.to_string()),
        }
    }

    #[tool(
        description = "Report the active project, its output location, FFmpeg readiness, and whether a job is running."
    )]
    async fn project_status(&self) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let project = state.project();
        let ffmpeg_ready = state.locator().is_available(&state.runner).await;
        ok(json!({
            "hasProject": project.is_some(),
            "name": project.as_ref().map(|value| value.name.clone()),
            "rootPath": project.as_ref().map(|value| value.root_path.clone()),
            "outputPath": project
                .as_ref()
                .map(|value| value.output_dir().to_string_lossy().to_string()),
            "ffmpegReady": ffmpeg_ready,
            "running": state.telemetry.is_running(),
        }))
    }

    #[tool(
        description = "Close the active project: cancels a running job and cleans temporary files."
    )]
    async fn project_close(&self) -> Result<CallToolResult, McpError> {
        let state = self.state();
        state.close_project();
        ok(json!({ "closed": true }))
    }

    #[tool(description = "Report FFmpeg readiness, version, source and path.")]
    async fn ffmpeg_status(&self) -> Result<CallToolResult, McpError> {
        match crate::commands::ffmpeg::status_of(&self.state()).await {
            Ok(status) => ok(to_value(&status)),
            Err(error) => fail(error.message),
        }
    }

    #[tool(
        description = "Check a background video (and optional intro) for codec or resolution mismatches before creating content."
    )]
    async fn loop_preflight(
        &self,
        Parameters(params): Parameters<LoopPreflightParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let background = state
            .metadata
            .read(&state.runner, &params.background_path)
            .await;
        let intro = match params.intro_path {
            Some(path) if !path.is_empty() => Some(state.metadata.read(&state.runner, &path).await),
            _ => None,
        };
        let report = PreflightChecker::new().check(&background, intro.as_ref(), true);
        ok(to_value(&report))
    }

    #[tool(
        description = "Read video metadata (codec, resolution, frame rate, duration, bitrate) for a file."
    )]
    async fn loop_metadata(
        &self,
        Parameters(params): Parameters<PathParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let metadata = state.metadata.read(&state.runner, &params.path).await;
        ok(to_value(&metadata))
    }

    #[tool(
        description = "Create one looping content file: a background video repeated over an audio playlist. Starts a run and returns immediately; follow it with run_status."
    )]
    async fn loop_create_content(
        &self,
        Parameters(params): Parameters<LoopCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let Some(project) = state.project() else {
            return fail("belum ada proyek yang terbuka");
        };
        let root = normalize(&project.root());
        for candidate in std::iter::once(&params.output_dir).chain(params.output_path.iter()) {
            if let Err(message) = required_within(&root, candidate) {
                return fail(message);
            }
        }

        let request = LoopRequest {
            background_path: params.background_path,
            audio_files: params.audio_files,
            output_dir: params.output_dir,
            output_name: params.output_name,
            loop_count: params.loop_count.max(1),
            intro_path: params.intro_path.filter(|value| !value.is_empty()),
            output_path: params.output_path,
        };
        if !request.can_generate() {
            return fail("backgroundPath, audioFiles, dan loopCount wajib diisi");
        }
        if let Err(error) = state.start_run("Menggabungkan konten") {
            return fail(error.message);
        }
        let workspace = match state.workspace() {
            Ok(workspace) => workspace,
            Err(error) => {
                state.finish_run("loop");
                return fail(error.message);
            }
        };
        let progress = state.telemetry.progress_callback();
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = workspace
                .loop_service
                .process(&request, Some(progress))
                .await;
            app.state::<AppState>().finish_run("loop");
        });
        ok(json!({ "started": true }))
    }

    #[tool(description = "List the project's EZ Loop channels, most recently changed first.")]
    async fn channel_list(&self) -> Result<CallToolResult, McpError> {
        let workspace = match self.state().workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        ok(to_value(&workspace.channel_repository.list()))
    }

    #[tool(description = "Load one channel definition by id.")]
    async fn channel_load(
        &self,
        Parameters(params): Parameters<ChannelIdParams>,
    ) -> Result<CallToolResult, McpError> {
        let workspace = match self.state().workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        match workspace.channel_repository.load(&params.channel_id) {
            Some(channel) => ok(to_value(&channel)),
            None => fail(format!("channel {} tidak ditemukan", params.channel_id)),
        }
    }

    #[tool(description = "Create a new channel definition.")]
    async fn channel_create(
        &self,
        Parameters(params): Parameters<ChannelCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let workspace = match state.workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        let Some(project) = state.project() else {
            return fail("belum ada proyek yang terbuka");
        };

        let millis = chrono::Local::now().timestamp_millis();
        let mut channel = Channel::create(
            format!("ch_{millis}"),
            params.name,
            project.root_path.clone(),
            project.output_dir().to_string_lossy().to_string(),
            millis,
        );
        channel.audio_folder_path = params.audio_folder_path;
        channel.video_folder_path = params.video_folder_path;
        channel.total_content = params.total_content.max(0);
        if params.suggested_loop_cap > 0 {
            channel.suggested_loop_cap = params.suggested_loop_cap;
        }
        if params.min_duration_minutes > 0.0 {
            channel.min_duration_minutes = params.min_duration_minutes;
        }
        match workspace.channel_repository.save(&channel) {
            Ok(()) => {
                state.telemetry.notify_data("channels");
                ok(to_value(&channel))
            }
            Err(error) => fail(error.to_string()),
        }
    }

    #[tool(
        description = "Save a full channel definition. Round-trip channel_load, change fields, then pass the object back here."
    )]
    async fn channel_save(
        &self,
        Parameters(params): Parameters<ChannelSaveParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let workspace = match state.workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        let mut channel: Channel = match serde_json::from_value(params.channel) {
            Ok(channel) => channel,
            Err(error) => return fail(format!("channel tidak valid: {error}")),
        };
        channel.touch();
        match workspace.channel_repository.save(&channel) {
            Ok(()) => {
                state.telemetry.notify_data("channels");
                ok(to_value(&channel))
            }
            Err(error) => fail(error.to_string()),
        }
    }

    #[tool(
        description = "Detect a channel's audio and video assets and build its balanced content plans."
    )]
    async fn channel_plan(
        &self,
        Parameters(params): Parameters<ChannelIdParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let workspace = match state.workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        let Some(channel) = workspace.channel_repository.load(&params.channel_id) else {
            return fail(format!("channel {} tidak ditemukan", params.channel_id));
        };
        if let Err(error) = state.start_run("Menyusun rencana") {
            return fail(error.message);
        }
        match workspace.channel_planner.plan(&channel).await {
            Ok(planned) => {
                let saved = workspace.channel_repository.save(&planned);
                state.telemetry.notify_data("channels");
                state.finish_run("channel_plan");
                match saved {
                    Ok(()) => ok(to_value(&planned)),
                    Err(error) => fail(error.to_string()),
                }
            }
            Err(error) => {
                state.finish_run("channel_plan");
                fail(error.message)
            }
        }
    }

    #[tool(
        description = "Rescan a channel's video folder and assign free videos to the plans that were waiting for one."
    )]
    async fn channel_rescan_videos(
        &self,
        Parameters(params): Parameters<ChannelIdParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let workspace = match state.workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        let Some(channel) = workspace.channel_repository.load(&params.channel_id) else {
            return fail(format!("channel {} tidak ditemukan", params.channel_id));
        };
        let refreshed = workspace.channel_detector.rescan_videos(&channel);
        let assigned = ChannelPlanner::assign_waiting_videos(&refreshed);
        match workspace.channel_repository.save(&assigned) {
            Ok(()) => {
                state.telemetry.notify_data("channels");
                ok(to_value(&assigned))
            }
            Err(error) => fail(error.to_string()),
        }
    }

    #[tool(
        description = "Estimate the disk space a channel's selected plans need and compare it with the free space at the output location."
    )]
    async fn channel_preflight(
        &self,
        Parameters(params): Parameters<ChannelIdParams>,
    ) -> Result<CallToolResult, McpError> {
        let workspace = match self.state().workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        let Some(channel) = workspace.channel_repository.load(&params.channel_id) else {
            return fail(format!("channel {} tidak ditemukan", params.channel_id));
        };
        let report = workspace.channel_capacity.report(&channel).await;
        ok(to_value(&report))
    }

    #[tool(
        description = "Process a channel's selected plans sequentially through the loop pipeline. Starts a batch and returns immediately; follow it with run_status."
    )]
    async fn channel_run(
        &self,
        Parameters(params): Parameters<ChannelIdParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let workspace = match state.workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        let Some(mut channel) = workspace.channel_repository.load(&params.channel_id) else {
            return fail(format!("channel {} tidak ditemukan", params.channel_id));
        };
        if channel.selected_plans().is_empty() {
            return fail("tidak ada plan yang dipilih pada channel ini");
        }
        if let Err(error) = state.start_run("Memproses konten") {
            return fail(error.message);
        }
        channel.status = ChannelStatus::Running;
        let progress = state.telemetry.progress_callback();
        let app = self.app.clone();
        let channel_id = params.channel_id;
        tauri::async_runtime::spawn(async move {
            let _ = workspace
                .channel_executor
                .run_selection(&channel, Some(progress))
                .await;
            let state = app.state::<AppState>();
            state.telemetry.notify_data("channels");
            state.finish_run("channel_run");
        });
        ok(json!({ "started": true, "channelId": channel_id }))
    }

    #[tool(description = "Cancel the running job (a merge or a channel batch).")]
    async fn channel_cancel(&self) -> Result<CallToolResult, McpError> {
        cancel(&self.state());
        ok(json!({ "cancelled": true }))
    }

    #[tool(
        description = "Read file size, video metadata, and YouTube-standard checks for a media file."
    )]
    async fn media_info(
        &self,
        Parameters(params): Parameters<PathParams>,
    ) -> Result<CallToolResult, McpError> {
        let workspace = match self.state().workspace() {
            Ok(workspace) => workspace,
            Err(error) => return fail(error.message),
        };
        match workspace.video_tools.info(&params.path).await {
            Ok(info) => ok(to_value(&info)),
            Err(error) => fail(error.message),
        }
    }

    #[tool(
        description = "Report whether a job is running, its latest progress (0..1), and recent log lines."
    )]
    async fn run_status(&self) -> Result<CallToolResult, McpError> {
        let state = self.state();
        let channel_running = state
            .workspace()
            .map(|workspace| workspace.channel_executor.is_running())
            .unwrap_or(false);
        ok(json!({
            "running": state.telemetry.is_running(),
            "progress": state.telemetry.current_progress(),
            "channelBatchRunning": channel_running,
            "logs": state.telemetry.recent_logs(50),
        }))
    }

    #[tool(description = "Cancel the running job (a merge or a channel batch).")]
    async fn run_cancel(&self) -> Result<CallToolResult, McpError> {
        cancel(&self.state());
        ok(json!({ "cancelled": true }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_curdir_and_parentdir() {
        assert_eq!(normalize(Path::new("/p/a/./b")), PathBuf::from("/p/a/b"));
        assert_eq!(normalize(Path::new("/p/a/../b")), PathBuf::from("/p/b"));
    }

    #[test]
    fn required_within_confines_writes_to_the_project_root() {
        let base = std::env::temp_dir().join("swaloka-mcp-confine");
        let root = normalize(&base);
        let inside = base.join("outputs").join("hasil.mp4");
        let outside = std::env::temp_dir()
            .join("swaloka-mcp-other")
            .join("hasil.mp4");

        assert!(required_within(&root, &inside.to_string_lossy()).is_ok());
        assert!(required_within(&root, &outside.to_string_lossy()).is_err());
    }

    #[test]
    fn required_within_rejects_parent_traversal() {
        let base = std::env::temp_dir().join("swaloka-mcp-confine");
        let root = normalize(&base);
        // Keluar dari root lewat `..` harus ditolak walau berkasnya tidak ada.
        let escape = base.join("..").join("swaloka-mcp-elsewhere").join("x.mp4");
        assert!(required_within(&root, &escape.to_string_lossy()).is_err());
    }
}
