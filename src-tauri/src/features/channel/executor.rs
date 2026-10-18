//! Eksekusi rencana channel satu per satu lewat pipeline loop merge.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::ffmpeg::runner::LogCallback;
use crate::core::logging::{LogLevel, LogRecord};
use crate::error::AppError;
use crate::features::channel::models::{Channel, ChannelPlan, ChannelPlanStatus, ChannelStatus};
use crate::features::channel::repository::ChannelRepository;
use crate::features::looping::models::LoopRequest;
use crate::features::looping::LoopMergeService;
use crate::features::ProgressCallback;

/// Pesan status untuk rencana yang dibatalkan pengguna.
pub const CANCELLED_MESSAGE: &str = "Dibatalkan pengguna";

/// Menjalankan rencana terpilih sebuah channel secara berurutan.
pub struct ChannelExecutor {
    repository: ChannelRepository,
    loop_service: Arc<LoopMergeService>,
    on_log: Option<LogCallback>,
    running: AtomicBool,
    cancel_requested: AtomicBool,
}

impl ChannelExecutor {
    pub fn new(
        repository: ChannelRepository,
        loop_service: Arc<LoopMergeService>,
        on_log: Option<LogCallback>,
    ) -> Self {
        Self {
            repository,
            loop_service,
            on_log,
            running: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Minta pembatalan: proses ffmpeg yang sedang jalan dihentikan.
    pub fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::SeqCst);
        self.loop_service.runner.cancel();
    }

    /// Proses semua rencana terpilih; tiap perubahan state langsung disimpan.
    pub async fn run_selection(
        &self,
        channel: &Channel,
        on_progress: Option<ProgressCallback>,
    ) -> Result<Channel, AppError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(AppError::new("Masih ada proses channel yang berjalan"));
        }
        self.cancel_requested.store(false, Ordering::SeqCst);
        let result = self.run_inner(channel, on_progress).await;
        self.running.store(false, Ordering::SeqCst);
        result
    }

    async fn run_inner(
        &self,
        channel: &Channel,
        on_progress: Option<ProgressCallback>,
    ) -> Result<Channel, AppError> {
        // Channel dari pemanggil adalah sumber kebenaran: pilihan rencana, intro,
        // dan pengaturan yang belum sempat tersimpan ikut dipakai. Ini juga
        // menghindari balapan dengan penyimpanan yang masih dalam perjalanan.
        let mut current = channel.clone();
        let total = selected_total(&current);
        if total == 0 {
            return Err(AppError::new("Tidak ada plan yang dipilih"));
        }

        current.status = ChannelStatus::Running;
        current.touch();
        self.save(&mut current)?;
        self.log(
            format!(
                "Memproses {total} plan terpilih untuk channel {}",
                current.name
            ),
            LogLevel::Info,
        );
        report(&on_progress, progress(&current, total));

        while let Some(plan_id) = next_plan(&current).map(|plan| plan.id.clone()) {
            self.mark_running(&mut current, &plan_id)?;
            let outcome = self.process_plan(&current, &plan_id).await;

            let cancelled = self.cancel_requested.load(Ordering::SeqCst);
            let (status, error, produced) = match outcome {
                Ok(output_path) => (ChannelPlanStatus::Done, None, Some(output_path)),
                Err(failure) => {
                    let (status, message) = failure_state(&failure, cancelled);
                    (status, Some(message), None)
                }
            };
            self.finish_plan(&mut current, &plan_id, status, error, produced)?;

            let done = progress(&current, total);
            report(&on_progress, done);
            if cancelled {
                self.log(
                    format!(
                        "Dibatalkan pengguna pada {}",
                        plan_label(&current, &plan_id)
                    ),
                    LogLevel::Warning,
                );
                break;
            }
        }

        finalize(&mut current);
        current.touch();
        self.save(&mut current)?;
        report(&on_progress, 1.0);
        self.log(summary(&current), LogLevel::Info);
        Ok(current.clone())
    }

    /// Jalankan satu rencana dan kembalikan path outputnya.
    async fn process_plan(&self, channel: &Channel, plan_id: &str) -> Result<String, AppError> {
        let plan = channel
            .plan_by_id(plan_id)
            .ok_or_else(|| AppError::new("Plan tidak ditemukan"))?;
        if plan.background_video_path.trim().is_empty() {
            return Err(AppError::new("Plan belum punya video latar"));
        }
        let audio_files = audio_paths_for(channel, plan);
        if audio_files.is_empty() {
            return Err(AppError::new("Tidak ada audio yang bisa dipakai"));
        }

        self.log(
            format!(
                "Menggabungkan {} — {} lagu · {}",
                plan_label(channel, plan_id),
                audio_files.len(),
                format_duration(plan.total_duration_seconds)
            ),
            LogLevel::Info,
        );

        let request = LoopRequest {
            background_path: plan.background_video_path.clone(),
            audio_files,
            output_dir: channel_output_dir(channel),
            output_name: output_name_for(plan),
            loop_count: 1,
            intro_path: plan
                .intro_video_path
                .clone()
                .filter(|path| !path.is_empty()),
            output_path: None,
        };
        // Progress per rencana sengaja tidak dipakai: nilainya akan mundur lagi
        // saat rencana berikutnya mulai. Progres batch dihitung di luar.
        let result = self.loop_service.process(&request, None).await?;
        Ok(result.output_path)
    }

    fn mark_running(&self, channel: &mut Channel, plan_id: &str) -> Result<(), AppError> {
        if let Some(plan) = channel.plans.iter_mut().find(|plan| plan.id == plan_id) {
            plan.status = ChannelPlanStatus::Running;
            plan.error = None;
            plan.processed_at = Some(chrono::Local::now());
        }
        channel.touch();
        self.save(channel)
    }

    fn finish_plan(
        &self,
        channel: &mut Channel,
        plan_id: &str,
        status: ChannelPlanStatus,
        error: Option<String>,
        output_path: Option<String>,
    ) -> Result<(), AppError> {
        if let Some(plan) = channel.plans.iter_mut().find(|plan| plan.id == plan_id) {
            plan.status = status;
            plan.error = error;
            if output_path.is_some() {
                plan.output_path = output_path;
            }
            plan.finished_at = Some(chrono::Local::now());
        }
        channel.touch();
        self.save(channel)
    }

    fn save(&self, channel: &mut Channel) -> Result<(), AppError> {
        self.repository
            .save(channel)
            .map_err(|error| AppError::new(format!("Gagal menyimpan channel: {error}")))
    }

    fn log(&self, message: impl Into<String>, level: LogLevel) {
        if let Some(callback) = &self.on_log {
            callback(LogRecord::new(message, level, 0));
        }
    }
}

/// Rencana berikutnya: yang terpilih dan belum diproses, urut indeks.
pub fn next_plan(channel: &Channel) -> Option<&ChannelPlan> {
    channel
        .plans
        .iter()
        .filter(|plan| plan.selected && plan.status == ChannelPlanStatus::NotProcessed)
        .min_by_key(|plan| plan.index)
}

/// Jumlah rencana terpilih.
pub fn selected_total(channel: &Channel) -> usize {
    channel.selected_plans().len()
}

/// Progres batch: rencana selesai (berhasil atau gagal) dibanding yang dipilih.
pub fn progress(channel: &Channel, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let finished = channel
        .plans
        .iter()
        .filter(|plan| plan.selected && plan.is_finished())
        .count();
    finished as f64 / total as f64
}

/// Path audio yang benar-benar ada untuk satu rencana; id yang tidak ketemu dilewati.
pub fn audio_paths_for(channel: &Channel, plan: &ChannelPlan) -> Vec<String> {
    plan.audio_item_ids
        .iter()
        .filter_map(|id| channel.audio_by_id(id))
        .map(|item| item.path.clone())
        .collect()
}

/// Folder output satu channel: folder output channel + nama channel yang sudah bersih.
pub fn channel_output_dir(channel: &Channel) -> String {
    PathBuf::from(&channel.output_directory_path)
        .join(sanitize(&channel.name))
        .to_string_lossy()
        .to_string()
}

/// Nama output satu rencana: nama berkas video latar tanpa ekstensi.
pub fn output_name_for(plan: &ChannelPlan) -> String {
    if plan.background_video_path.is_empty() {
        return format!("plan_{:02}", plan.index + 1);
    }
    Path::new(&plan.background_video_path)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("plan_{:02}", plan.index + 1))
}

/// Status dan pesan untuk rencana yang gagal.
pub fn failure_state(error: &AppError, cancelled: bool) -> (ChannelPlanStatus, String) {
    if cancelled {
        (ChannelPlanStatus::Failed, String::from(CANCELLED_MESSAGE))
    } else {
        (ChannelPlanStatus::Failed, error.message.clone())
    }
}

/// Status channel setelah batch berhenti.
pub fn finalize(channel: &mut Channel) {
    let done = channel.done_count();
    let failed = channel.failed_count();
    channel.status = if done == 0 && failed == 0 {
        ChannelStatus::Ready
    } else if failed == 0 {
        ChannelStatus::Completed
    } else if done == 0 {
        ChannelStatus::Failed
    } else {
        ChannelStatus::PartiallyDone
    };
}

fn plan_label(channel: &Channel, plan_id: &str) -> String {
    match channel.plan_by_id(plan_id) {
        Some(plan) => format!("plan #{}", plan.index + 1),
        None => String::from(plan_id),
    }
}

fn summary(channel: &Channel) -> String {
    format!(
        "Selesai: {} berhasil, {} gagal dari {} plan terpilih",
        channel.done_count(),
        channel.failed_count(),
        selected_total(channel)
    )
}

fn report(callback: &Option<ProgressCallback>, value: f64) {
    if let Some(callback) = callback {
        callback(value.clamp(0.0, 1.0));
    }
}

fn format_duration(seconds: f64) -> String {
    let total = seconds.round() as i64;
    format!("{} jam {} menit", total / 3600, (total % 3600) / 60)
}

fn sanitize(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    for character in value.trim().chars() {
        if matches!(
            character,
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
        ) {
            cleaned.push('_');
        } else {
            cleaned.push(character);
        }
    }
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        String::from("channel")
    } else {
        collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::channel::models::{Channel, ChannelAudioItem};

    fn plan(id: &str, index: i64, status: ChannelPlanStatus, selected: bool) -> ChannelPlan {
        ChannelPlan {
            index,
            status,
            selected,
            total_duration_seconds: 7200.0,
            background_video_path: format!("/video/bg_{:02}.mp4", index + 1),
            audio_item_ids: vec![format!("audio_{}", index + 1)],
            ..ChannelPlan::new(id.to_string(), index)
        }
    }

    fn channel(plans: Vec<ChannelPlan>) -> Channel {
        let mut channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge Daily"),
            String::from("/p"),
            String::from("/p/outputs"),
            1,
        );
        channel.audio_items = (0..6)
            .map(|index| ChannelAudioItem {
                id: format!("audio_{}", index + 1),
                path: format!("/musik/lagu_{:02}.mp3", index + 1),
                duration_seconds: Some(300.0),
            })
            .collect();
        channel.plans = plans;
        channel
    }

    #[test]
    fn next_plan_takes_only_selected_unprocessed_in_index_order() {
        let value = channel(vec![
            plan("plan_1", 0, ChannelPlanStatus::Done, true),
            plan("plan_2", 1, ChannelPlanStatus::NotProcessed, true),
            plan("plan_3", 2, ChannelPlanStatus::NotProcessed, false),
            plan("plan_4", 3, ChannelPlanStatus::NotProcessed, true),
        ]);
        let next = next_plan(&value).unwrap();
        assert_eq!(next.id, "plan_2", "yang belum diproses dan terpilih");

        // Indeks 1 sudah selesai: berikutnya harus plan_4.
        let mut advanced = value.clone();
        advanced.plans[1].status = ChannelPlanStatus::Done;
        assert_eq!(next_plan(&advanced).unwrap().id, "plan_4");

        let all_done = channel(vec![plan("plan_1", 0, ChannelPlanStatus::Done, true)]);
        assert!(next_plan(&all_done).is_none());
    }

    #[test]
    fn progress_counts_finished_over_selected() {
        let five = channel(vec![
            plan("plan_1", 0, ChannelPlanStatus::Done, true),
            plan("plan_2", 1, ChannelPlanStatus::Failed, true),
            plan("plan_3", 2, ChannelPlanStatus::NotProcessed, true),
            plan("plan_4", 3, ChannelPlanStatus::NotProcessed, true),
            plan("plan_5", 4, ChannelPlanStatus::NotProcessed, true),
        ]);
        let total = selected_total(&five);
        assert_eq!(total, 5);
        assert!(
            (progress(&five, total) - 0.4).abs() < f64::EPSILON,
            "2 dari 5"
        );

        // Rencana tak terpilih tidak menambah penyebut maupun pembilang.
        let mut mixed = five.clone();
        mixed.plans[2].selected = false;
        let total = selected_total(&mixed);
        assert_eq!(total, 4);
        assert!(
            (progress(&mixed, total) - 0.5).abs() < f64::EPSILON,
            "2 dari 4"
        );

        assert_eq!(progress(&channel(Vec::new()), 0), 0.0);
    }

    #[test]
    fn cancelled_plan_is_marked_with_the_user_message() {
        let error = AppError::new("ffmpeg gagal (kode 1)");
        let (status, message) = failure_state(&error, true);
        assert_eq!(status, ChannelPlanStatus::Failed);
        assert_eq!(message, CANCELLED_MESSAGE);

        let (status, message) = failure_state(&error, false);
        assert_eq!(status, ChannelPlanStatus::Failed);
        assert_eq!(message, "ffmpeg gagal (kode 1)");
    }

    #[test]
    fn audio_paths_skip_unresolved_ids() {
        let mut value = channel(vec![plan(
            "plan_1",
            0,
            ChannelPlanStatus::NotProcessed,
            true,
        )]);
        value.plans[0].audio_item_ids = vec![
            String::from("audio_1"),
            String::from("tidak_ada"),
            String::from("audio_3"),
        ];
        let paths = audio_paths_for(&value, &value.plans[0]);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/musik/lagu_01.mp3");
        assert_eq!(paths[1], "/musik/lagu_03.mp3");

        value.plans[0].audio_item_ids = vec![String::from("hilang")];
        assert!(audio_paths_for(&value, &value.plans[0]).is_empty());
    }

    #[test]
    fn output_folder_and_name_are_sane() {
        // Susun harapan lewat `Path::join` supaya separator mengikuti platform.
        let expected = |name: &str| {
            Path::new("/p/outputs")
                .join(name)
                .to_string_lossy()
                .into_owned()
        };

        let value = channel(Vec::new());
        assert_eq!(channel_output_dir(&value), expected("Lounge Daily"));

        let mut nasty = channel(Vec::new());
        nasty.name = String::from("  Channel: A/B?  ");
        assert_eq!(channel_output_dir(&nasty), expected("Channel_ A_B_"));

        let mut empty = channel(Vec::new());
        empty.name = String::from("   ");
        assert_eq!(channel_output_dir(&empty), expected("channel"));

        assert_eq!(
            output_name_for(&plan("plan_1", 0, ChannelPlanStatus::NotProcessed, true)),
            "bg_01"
        );
        let mut no_background = plan("plan_2", 1, ChannelPlanStatus::NotProcessed, true);
        no_background.background_video_path = String::new();
        assert_eq!(output_name_for(&no_background), "plan_02");
    }

    #[test]
    fn finalize_reflects_the_batch_outcome() {
        let mut all_done = channel(vec![
            plan("plan_1", 0, ChannelPlanStatus::Done, true),
            plan("plan_2", 1, ChannelPlanStatus::Done, true),
        ]);
        finalize(&mut all_done);
        assert_eq!(all_done.status, ChannelStatus::Completed);

        let mut partial = channel(vec![
            plan("plan_1", 0, ChannelPlanStatus::Done, true),
            plan("plan_2", 1, ChannelPlanStatus::Failed, true),
        ]);
        finalize(&mut partial);
        assert_eq!(partial.status, ChannelStatus::PartiallyDone);

        let mut all_failed = channel(vec![plan("plan_1", 0, ChannelPlanStatus::Failed, true)]);
        finalize(&mut all_failed);
        assert_eq!(all_failed.status, ChannelStatus::Failed);

        let mut untouched = channel(vec![plan(
            "plan_1",
            0,
            ChannelPlanStatus::NotProcessed,
            true,
        )]);
        finalize(&mut untouched);
        assert_eq!(untouched.status, ChannelStatus::Ready);
    }
}
