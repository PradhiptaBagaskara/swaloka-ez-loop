//! Backend Swaloka Looping Tool.
//!
//! Struktur modul meniru versi Python lama supaya mudah dibandingkan saat
//! porting: `core/` (utilitas + mesin FFmpeg), `features/` (pipeline fitur),
//! dan `storage/` (proyek, preferensi, cache).

pub mod commands;
pub mod core;
pub mod error;
pub mod features;
pub mod mcp;
pub mod state;
pub mod storage;

use state::AppState;
use tauri::Manager;

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(AppState::new(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::project::project_current,
            commands::project::project_create,
            commands::project::project_open,
            commands::project::project_close,
            commands::project::project_set_output,
            commands::project::prefs_get,
            commands::project::prefs_set,
            commands::project::recent_projects,
            commands::project::remove_recent,
            commands::project::project_stats,
            commands::ffmpeg::ffmpeg_check,
            commands::ffmpeg::ffmpeg_status,
            commands::ffmpeg::ffmpeg_install,
            commands::ffmpeg::ffmpeg_install_cancel,
            commands::ffmpeg::ffmpeg_detect_encoder,
            commands::ffmpeg::ffmpeg_encoders,
            commands::ffmpeg::ffmpeg_configure,
            commands::looping::loop_run,
            commands::looping::loop_preflight,
            commands::looping::loop_metadata,
            commands::media::media_encode_live,
            commands::media::media_live_defaults,
            commands::media::media_info,
            commands::media::media_audio_process,
            commands::media::media_audio_concat,
            commands::media::media_audio_export_preset,
            commands::channel::channel_list,
            commands::channel::channel_load,
            commands::channel::channel_create,
            commands::channel::channel_save,
            commands::channel::channel_delete,
            commands::channel::channel_plan,
            commands::channel::channel_rescan_videos,
            commands::channel::channel_preflight,
            commands::channel::channel_run,
            commands::channel::channel_cancel,
            commands::system::cancel_run,
            commands::system::clean_cache,
            commands::system::app_version,
            commands::system::app_exe_path,
            commands::mcp::mcp_status,
            commands::mcp::mcp_start,
            commands::mcp::mcp_stop,
        ])
        .build(tauri::generate_context!())
        .expect("gagal menyiapkan aplikasi Tauri");

    app.run(|app_handle, event| {
        // Bersihkan file runtime MCP saat aplikasi keluar normal, supaya tidak
        // meninggalkan port/token basi. Kasus tutup paksa ditangani saat startup
        // berikutnya (lihat `AppState::new`).
        if let tauri::RunEvent::Exit = event {
            app_handle.state::<AppState>().mcp.stop();
        }
    });
}
