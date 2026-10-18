//! State aplikasi: preferensi, proyek aktif, service, dan telemetri run.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::core::ffmpeg::runner::LogCallback;
use crate::core::ffmpeg::{
    install_dir, FFmpegLocator, FFmpegRunner, InstallCallback, InstallProgress, Installer,
    MetadataReader, BUNDLED_FFMPEG_ONLY,
};
use crate::core::logging::{write_records, LogRecord};
use crate::error::{AppError, AppResult};
use crate::features::channel::{
    ChannelCapacity, ChannelDetector, ChannelExecutor, ChannelPlanner, ChannelRepository,
};
use crate::features::looping::{AudioNormalizer, LoopMergeService};
use crate::features::media_tools::{AudioToolsService, VideoToolsService};
use crate::features::ProgressCallback;
use crate::mcp::McpServer;
use crate::storage::cache::AudioCache;
use crate::storage::migration::LegacyMigrator;
use crate::storage::prefs::{Preferences, PreferencesStore};
use crate::storage::project::{Project, ProjectStore};
use crate::storage::temp::TempManager;

/// Muatan event `task://started`.
#[derive(Clone, Serialize)]
pub struct RunStarted {
    pub label: String,
}

/// Muatan event `data://changed`.
#[derive(Clone, Serialize)]
pub struct DataChanged {
    pub scope: String,
}

/// Service yang aktif selama sebuah proyek terbuka.
pub struct Workspace {
    pub cache: Arc<AudioCache>,
    pub temp: Arc<TempManager>,
    pub loop_service: Arc<LoopMergeService>,
    pub audio_tools: Arc<AudioToolsService>,
    pub video_tools: Arc<VideoToolsService>,
    pub channel_repository: Arc<ChannelRepository>,
    pub channel_detector: Arc<ChannelDetector>,
    pub channel_planner: Arc<ChannelPlanner>,
    pub channel_capacity: Arc<ChannelCapacity>,
    pub channel_executor: Arc<ChannelExecutor>,
}

/// Telemetri run: aliran log, progres, dan flag "sedang berjalan".
pub struct Telemetry {
    app: AppHandle,
    logs: Mutex<Vec<LogRecord>>,
    progress: Mutex<f64>,
    running: AtomicBool,
}

impl Telemetry {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            logs: Mutex::new(Vec::new()),
            progress: Mutex::new(0.0),
            running: AtomicBool::new(false),
        }
    }

    /// Callback log yang meneruskan record ke frontend dan menyimpannya untuk file log.
    pub fn log_callback(self: &Arc<Self>) -> LogCallback {
        let telemetry = Arc::clone(self);
        Arc::new(move |record: LogRecord| {
            {
                let mut logs = telemetry
                    .logs
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                logs.push(record.clone());
            }
            let _ = telemetry.app.emit("task://log", &record);
        })
    }

    /// Callback progres yang menyimpan nilai terakhir lalu memancarkannya ke
    /// frontend. Nilai tersimpan dipakai pembaca non-UI seperti server MCP.
    pub fn progress_callback(self: &Arc<Self>) -> ProgressCallback {
        let telemetry = Arc::clone(self);
        Arc::new(move |value: f64| {
            *telemetry
                .progress
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = value;
            let _ = telemetry.app.emit("task://progress", value);
        })
    }

    /// Callback progres pemasangan FFmpeg untuk layar setup (fase + byte).
    pub fn install_callback(self: &Arc<Self>) -> InstallCallback {
        let telemetry = Arc::clone(self);
        Arc::new(move |progress: InstallProgress| {
            let _ = telemetry.app.emit("setup://progress", &progress);
        })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn begin_run(&self) {
        self.running.store(true, Ordering::SeqCst);
        *self
            .progress
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = 0.0;
        self.logs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clear();
    }

    pub fn end_run(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Tulis log run terakhir ke folder `logs/` proyek.
    pub fn write_log(&self, logs_dir: &std::path::Path, base_name: &str) {
        let logs = self
            .logs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if logs.is_empty() {
            return;
        }
        let _ = write_records(logs_dir, base_name, &logs);
    }

    /// Hapus buffer log setelah run selesai dibaca.
    pub fn clear_logs(&self) {
        self.logs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clear();
    }

    /// Progres run terakhir (0.0..1.0), untuk pembaca non-UI seperti MCP.
    pub fn current_progress(&self) -> f64 {
        *self
            .progress
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    /// `limit` record log terbaru, untuk pembaca non-UI seperti MCP.
    pub fn recent_logs(&self, limit: usize) -> Vec<LogRecord> {
        let logs = self.logs.lock().unwrap_or_else(|error| error.into_inner());
        let start = logs.len().saturating_sub(limit);
        logs[start..].to_vec()
    }

    /// Beri tahu antarmuka bahwa sebuah run dimulai — termasuk run yang dimulai
    /// dari luar antarmuka, mis. lewat server MCP, supaya queue card ikut tampil.
    pub fn notify_started(&self, label: &str) {
        let _ = self.app.emit(
            "task://started",
            RunStarted {
                label: label.to_string(),
            },
        );
    }

    /// Beri tahu antarmuka bahwa run terakhir selesai.
    pub fn notify_finished(&self) {
        let _ = self.app.emit("task://finished", ());
    }

    /// Beri tahu antarmuka bahwa data domain (`channels`/`project`/`prefs`)
    /// berubah di luar dirinya, supaya tampilan yang relevan memuat ulang.
    pub fn notify_data(&self, scope: &str) {
        let _ = self.app.emit(
            "data://changed",
            DataChanged {
                scope: scope.to_string(),
            },
        );
    }
}

/// Folder tempat aplikasi mencari FFmpeg bawaannya, berurutan: folder
/// pemasangan aplikasi (hasil unduhan lewat layar setup), folder exe, lalu
/// folder resource bundle (macOS/Linux).
pub fn bundled_ffmpeg_dirs(app: &AppHandle) -> Vec<String> {
    let mut dirs: Vec<String> = vec![install_dir().to_string_lossy().to_string()];
    if let Some(parent) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        let parent = parent.to_string_lossy().to_string();
        if !dirs.contains(&parent) {
            dirs.push(parent);
        }
    }
    if let Ok(resource) = app.path().resource_dir() {
        let resource = resource.to_string_lossy().to_string();
        if !dirs.contains(&resource) {
            dirs.push(resource);
        }
    }
    dirs
}

/// State global aplikasi.
pub struct AppState {
    pub telemetry: Arc<Telemetry>,
    pub prefs_store: PreferencesStore,
    prefs: Mutex<Preferences>,
    project: Mutex<Option<Project>>,
    workspace: Mutex<Option<Arc<Workspace>>>,
    pub runner: Arc<FFmpegRunner>,
    pub metadata: Arc<MetadataReader>,
    /// Penjaga pemasangan FFmpeg dari layar setup (unduhan berprogres).
    pub installer: Installer,
    /// Server MCP (Kontrol AI): dibagikan dengan antarmuka, mati kecuali bila
    /// diaktifkan pengguna.
    pub mcp: Arc<McpServer>,
    bundled_dirs: Vec<String>,
}

impl AppState {
    pub fn new(app: AppHandle) -> Self {
        // Sisa file runtime MCP dari sesi sebelumnya (app ditutup paksa atau
        // di-crash) dibuang supaya tidak ada port/token basi yang menyesatkan
        // jembatan stdio. Server tidak menyala otomatis, jadi file ini memang
        // belum seharusnya ada saat aplikasi baru mulai.
        let _ = std::fs::remove_file(crate::storage::paths::mcp_runtime_file());
        let prefs_store = PreferencesStore::new(None);
        let prefs = prefs_store.load();
        let bundled_dirs = bundled_ffmpeg_dirs(&app);
        let locator =
            FFmpegLocator::new(prefs.ffmpeg_path.clone()).with_bundled_dirs(bundled_dirs.clone());
        let ffmpeg = locator.resolve_ffmpeg();
        let ffprobe = locator.resolve_ffprobe(Some(&ffmpeg));
        Self {
            telemetry: Arc::new(Telemetry::new(app.clone())),
            prefs_store,
            prefs: Mutex::new(prefs),
            project: Mutex::new(None),
            workspace: Mutex::new(None),
            runner: Arc::new(FFmpegRunner::new(ffmpeg, Some(ffprobe))),
            metadata: Arc::new(MetadataReader::new(true)),
            installer: Installer::default(),
            mcp: Arc::new(McpServer::new(app)),
            bundled_dirs,
        }
    }

    pub fn prefs(&self) -> Preferences {
        self.prefs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn set_prefs(&self, prefs: Preferences) -> AppResult<Preferences> {
        self.prefs_store.save(&prefs)?;
        *self.prefs.lock().unwrap_or_else(|error| error.into_inner()) = prefs.clone();
        Ok(prefs)
    }

    pub fn update_prefs<F>(&self, mutate: F) -> Preferences
    where
        F: FnOnce(Preferences) -> Preferences,
    {
        let current = self.prefs();
        let updated = mutate(current);
        let _ = self.prefs_store.save(&updated);
        *self.prefs.lock().unwrap_or_else(|error| error.into_inner()) = updated.clone();
        updated
    }

    pub fn project(&self) -> Option<Project> {
        self.project
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn workspace(&self) -> AppResult<Arc<Workspace>> {
        self.workspace
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            .ok_or_else(|| AppError::new("belum ada proyek yang terbuka"))
    }

    pub fn project_store(&self) -> ProjectStore {
        ProjectStore::new(Some(LegacyMigrator))
    }

    /// Aktifkan sebuah proyek: susun semua service dan catat ke preferensi.
    pub fn activate_project(&self, project: Project) -> AppResult<Project> {
        project.ensure_structure()?;
        let log = self.telemetry.log_callback();

        let cache = Arc::new(AudioCache::new(project.clone()));
        let temp = Arc::new(TempManager::new(project.temp_dir()));
        let normalizer = Arc::new(AudioNormalizer::new(
            self.runner.clone(),
            self.metadata.clone(),
            cache.clone(),
            Some(log.clone()),
        ));
        let loop_service = Arc::new(LoopMergeService::new(
            self.runner.clone(),
            self.metadata.clone(),
            cache.clone(),
            temp.clone(),
            Some(log.clone()),
        ));
        let audio_tools = Arc::new(AudioToolsService::new(
            self.runner.clone(),
            temp.clone(),
            Some(normalizer),
            Some(log.clone()),
        ));
        let video_tools = Arc::new(VideoToolsService::new(
            self.runner.clone(),
            self.metadata.clone(),
            temp.clone(),
            Some(log.clone()),
        ));
        let channel_repository = Arc::new(ChannelRepository::new(project.clone()));
        let channel_detector = Arc::new(ChannelDetector::new(
            self.metadata.clone(),
            self.runner.clone(),
            Some(log.clone()),
        ));
        let channel_planner = Arc::new(ChannelPlanner::new(
            self.metadata.clone(),
            self.runner.clone(),
            Some(log.clone()),
        ));
        let channel_capacity = Arc::new(ChannelCapacity::new(
            self.metadata.clone(),
            self.runner.clone(),
        ));
        let channel_executor = Arc::new(ChannelExecutor::new(
            ChannelRepository::new(project.clone()),
            loop_service.clone(),
            Some(log.clone()),
        ));

        let workspace = Arc::new(Workspace {
            cache,
            temp,
            loop_service,
            audio_tools,
            video_tools,
            channel_repository,
            channel_detector,
            channel_planner,
            channel_capacity,
            channel_executor,
        });

        *self
            .project
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(project.clone());
        *self
            .workspace
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(workspace);

        self.update_prefs(|prefs| prefs.add_recent(&project.root_path));
        self.telemetry.notify_data("project");
        Ok(project)
    }

    /// Tutup proyek: batalkan run, bersihkan temp, lepas service.
    pub fn close_project(&self) {
        self.runner.cancel();
        if let Some(workspace) = self
            .workspace
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            workspace.temp.clean_stale();
        }
        *self
            .project
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        self.telemetry.notify_data("project");
    }

    /// Locator FFmpeg dari preferensi saat ini plus folder bawaan aplikasi.
    /// Semua pemeriksaan ketersediaan FFmpeg harus lewat sini supaya biner yang
    /// dipasang installer ikut terdeteksi.
    pub fn locator(&self) -> FFmpegLocator {
        let locator = FFmpegLocator::new(self.prefs().ffmpeg_path.clone())
            .with_bundled_dirs(self.bundled_dirs.clone());
        if BUNDLED_FFMPEG_ONLY {
            locator.without_system_fallback()
        } else {
            locator
        }
    }

    /// Resolusi ulang path FFmpeg dari preferensi saat ini.
    pub fn configure_ffmpeg(&self) -> bool {
        let locator = self.locator();
        let ffmpeg = locator.resolve_ffmpeg();
        let ffprobe = locator.resolve_ffprobe(Some(&ffmpeg));
        self.runner.configure(ffmpeg, Some(ffprobe));
        true
    }

    /// Mulai sebuah run; tolak bila masih ada run lain yang berjalan. `label`
    /// ikut dipancarkan supaya antarmuka bisa menampilkan run yang bukan
    /// dimulainya sendiri (mis. dari server MCP).
    pub fn start_run(&self, label: &str) -> AppResult<()> {
        if self.telemetry.is_running() {
            return Err(AppError::new("Ada proses lain yang sedang berjalan"));
        }
        self.telemetry.begin_run();
        self.telemetry.notify_started(label);
        Ok(())
    }

    /// Selesaikan run: tulis log ke proyek, bersihkan buffer, lepas flag, lalu
    /// beri tahu antarmuka bahwa run sudah berakhir.
    pub fn finish_run(&self, base_name: &str) {
        if let Some(project) = self.project() {
            self.telemetry.write_log(&project.logs_dir(), base_name);
        }
        self.telemetry.clear_logs();
        self.telemetry.end_run();
        self.telemetry.notify_finished();
    }
}

/// Ringkasan statistik proyek untuk halaman Dasbor.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub output_count: usize,
    pub cache_count: usize,
    pub ffmpeg_ready: bool,
}
