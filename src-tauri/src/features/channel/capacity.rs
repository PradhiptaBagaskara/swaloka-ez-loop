//! Cek kapasitas sebelum menjalankan rencana terpilih: berapa byte yang
//! dibutuhkan, berapa yang tersedia, dan apakah masih muat.

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;

use crate::core::disk_space::available_bytes;
use crate::core::ffmpeg::disk::{DiskEstimateInput, DiskEstimator};
use crate::core::ffmpeg::metadata::MetadataReader;
use crate::core::ffmpeg::runner::FFmpegRunner;
use crate::features::channel::models::{Channel, ChannelPlan};

/// Perkiraan satu rencana dalam satu batch.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanCapacity {
    pub plan_id: String,
    pub index: i64,
    pub duration_seconds: f64,
    pub output_bytes: u64,
    pub required_bytes: u64,
}

/// Laporan kapasitas untuk seluruh rencana terpilih sebuah channel.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelCapacityReport {
    pub plan_count: usize,
    pub total_seconds: f64,
    pub required_bytes: u64,
    /// Ruang bebas di lokasi output; `None` bila tidak bisa dibaca.
    pub available_bytes: Option<u64>,
    /// `false` hanya bila ruang terbaca dan memang tidak cukup.
    pub fits: bool,
    /// Ruang bebas tidak terbaca — jalankan tetap boleh, tapi harus diberitahu.
    pub space_unknown: bool,
    pub plans: Vec<PlanCapacity>,
}

/// Penghitung kebutuhan disk untuk rencana terpilih.
pub struct ChannelCapacity {
    metadata: Arc<MetadataReader>,
    runner: Arc<FFmpegRunner>,
    estimator: DiskEstimator,
}

impl ChannelCapacity {
    pub fn new(metadata: Arc<MetadataReader>, runner: Arc<FFmpegRunner>) -> Self {
        Self {
            metadata,
            runner,
            estimator: DiskEstimator::default(),
        }
    }

    /// Laporan dengan ruang bebas dibaca dari lokasi output channel.
    pub async fn report(&self, channel: &Channel) -> ChannelCapacityReport {
        let available = available_bytes(Path::new(&channel.output_directory_path));
        self.report_with_available(channel, available).await
    }

    /// Laporan dengan ruang bebas yang sudah diketahui, supaya keputusan
    /// "muat / tidak muat / tidak diketahui" bisa diuji tanpa bergantung disk.
    pub async fn report_with_available(
        &self,
        channel: &Channel,
        available: Option<u64>,
    ) -> ChannelCapacityReport {
        let selected = channel.selected_plans();
        let mut plans: Vec<PlanCapacity> = Vec::with_capacity(selected.len());
        let mut required_bytes: u64 = 0;
        let mut total_seconds: f64 = 0.0;

        for plan in selected {
            let input = self.input_for(plan).await;
            let estimate = self.estimator.estimate(input);
            required_bytes = required_bytes.saturating_add(estimate.required_bytes);
            total_seconds += estimate.duration_seconds;
            plans.push(PlanCapacity {
                plan_id: plan.id.clone(),
                index: plan.index,
                duration_seconds: estimate.duration_seconds,
                output_bytes: estimate.output_bytes,
                required_bytes: estimate.required_bytes,
            });
        }

        ChannelCapacityReport {
            plan_count: plans.len(),
            total_seconds,
            required_bytes,
            available_bytes: available,
            // Ruang yang tidak terbaca bukan alasan menolak menjalankan.
            fits: available.is_none_or(|free| required_bytes <= free),
            space_unknown: available.is_none(),
            plans,
        }
    }

    /// Argumen estimator untuk satu rencana, diambil dari metadata video latar
    /// dan intro bila terbaca.
    async fn input_for(&self, plan: &ChannelPlan) -> DiskEstimateInput {
        let background = self.read_video(&plan.background_video_path).await;
        let intro = match plan.intro_video_path.as_deref() {
            Some(path) if !path.is_empty() => Some(self.read_video(path).await),
            _ => None,
        };
        plan_input(
            plan,
            background.0,
            background.1,
            intro.and_then(|value| value.0),
        )
    }

    /// (bitrate kbps, durasi detik) sebuah video; `None` bila tidak terbaca.
    async fn read_video(&self, path: &str) -> (Option<i64>, Option<f64>) {
        if path.is_empty() {
            return (None, None);
        }
        let metadata = self.metadata.read(&self.runner, path).await;
        if !metadata.readable {
            return (None, None);
        }
        (metadata.bitrate_kbps, metadata.duration)
    }
}

/// Argumen estimator untuk satu rencana — murni supaya bisa diuji tanpa I/O.
pub fn plan_input(
    plan: &ChannelPlan,
    background_bitrate_kbps: Option<i64>,
    background_seconds: Option<f64>,
    intro_bitrate_kbps: Option<i64>,
) -> DiskEstimateInput {
    DiskEstimateInput {
        audio_file_count: plan.audio_item_ids.len() as i64,
        // Output sepanjang playlist rencana; video latar yang mengulang mengisi.
        audio_seconds: plan.total_duration_seconds,
        loop_count: 1,
        background_bitrate_kbps,
        background_seconds,
        intro_bitrate_kbps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::channel::models::{Channel, ChannelAudioItem, ChannelPlan};

    fn capacity() -> ChannelCapacity {
        ChannelCapacity::new(
            Arc::new(MetadataReader::new(true)),
            Arc::new(FFmpegRunner::new("ffmpeg", None)),
        )
    }

    fn plan(id: &str, index: i64, seconds: f64, selected: bool) -> ChannelPlan {
        ChannelPlan {
            index,
            total_duration_seconds: seconds,
            audio_item_ids: vec![format!("audio_{}", index + 1)],
            selected,
            background_video_path: format!("/video/tidak-ada-{index}.mp4"),
            ..ChannelPlan::new(id.to_string(), index)
        }
    }

    fn channel_with_plans(plans: Vec<ChannelPlan>) -> Channel {
        let mut channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge"),
            String::from("/p"),
            String::from("/p/outputs"),
            1,
        );
        channel.audio_items = vec![ChannelAudioItem {
            id: String::from("audio_1"),
            path: String::from("/musik/lagu.mp3"),
            duration_seconds: Some(300.0),
        }];
        channel.plans = plans;
        channel
    }

    #[tokio::test]
    async fn sums_only_the_selected_plans() {
        let channel = channel_with_plans(vec![
            plan("plan_1", 0, 7200.0, true),
            plan("plan_2", 1, 5400.0, true),
            plan("plan_3", 2, 3600.0, false),
        ]);
        let report = capacity().report_with_available(&channel, None).await;

        assert_eq!(report.plan_count, 2, "rencana tak terpilih tidak dihitung");
        assert_eq!(report.total_seconds, 12600.0);
        assert_eq!(report.plans.len(), 2);
        assert_eq!(report.plans[0].plan_id, "plan_1");

        let expected: u64 = report.plans.iter().map(|plan| plan.required_bytes).sum();
        assert_eq!(report.required_bytes, expected);
        assert!(report.plans[0].required_bytes > report.plans[0].output_bytes);
    }

    #[tokio::test]
    async fn unreadable_background_uses_the_default_bitrate() {
        let channel = channel_with_plans(vec![plan("plan_1", 0, 7200.0, true)]);
        let report = capacity().report_with_available(&channel, None).await;

        // Berkas tidak ada, jadi estimator memakai bitrate cadangan 8 Mbps.
        let expected = DiskEstimator::default().estimate(DiskEstimateInput {
            audio_file_count: 1,
            audio_seconds: 7200.0,
            loop_count: 1,
            background_bitrate_kbps: None,
            background_seconds: None,
            intro_bitrate_kbps: None,
        });
        assert_eq!(report.plans[0].required_bytes, expected.required_bytes);
        assert_eq!(report.plans[0].output_bytes, expected.output_bytes);
    }

    #[test]
    fn plan_input_maps_the_plan_fields() {
        let mut value = plan("plan_1", 0, 7200.0, true);
        value.audio_item_ids = vec![String::from("a1"), String::from("a2")];
        let input = plan_input(&value, Some(4000), Some(600.0), None);
        assert_eq!(input.audio_file_count, 2);
        assert_eq!(input.audio_seconds, 7200.0);
        assert_eq!(input.loop_count, 1);
        assert_eq!(input.background_bitrate_kbps, Some(4000));
        assert_eq!(input.background_seconds, Some(600.0));
    }

    #[test]
    fn intro_raises_the_estimated_video_bitrate() {
        let value = plan("plan_1", 0, 7200.0, true);
        let without = DiskEstimator::default().estimate(plan_input(&value, Some(4000), None, None));
        let with =
            DiskEstimator::default().estimate(plan_input(&value, Some(4000), None, Some(9000)));
        assert_eq!(without.video_bitrate_kbps, 4000);
        assert_eq!(
            with.video_bitrate_kbps, 9000,
            "intro dengan bitrate lebih tinggi dipakai"
        );
        assert!(with.required_bytes > without.required_bytes);
    }

    #[tokio::test]
    async fn decides_fits_not_fits_and_unknown() {
        let channel = channel_with_plans(vec![plan("plan_1", 0, 7200.0, true)]);

        let unknown = capacity().report_with_available(&channel, None).await;
        assert!(unknown.space_unknown);
        assert!(unknown.fits, "ruang tak terbaca tidak menolak jalan");
        assert_eq!(unknown.available_bytes, None);

        let plenty = capacity()
            .report_with_available(&channel, Some(unknown.required_bytes + 1))
            .await;
        assert!(plenty.fits);
        assert!(!plenty.space_unknown);

        let tight = capacity()
            .report_with_available(&channel, Some(unknown.required_bytes - 1))
            .await;
        assert!(!tight.fits, "kurang satu byte berarti tidak muat");
        assert_eq!(tight.available_bytes, Some(unknown.required_bytes - 1));
    }

    #[tokio::test]
    async fn empty_selection_needs_nothing() {
        let channel = channel_with_plans(vec![plan("plan_1", 0, 7200.0, false)]);
        let report = capacity().report_with_available(&channel, Some(0)).await;
        assert_eq!(report.plan_count, 0);
        assert_eq!(report.required_bytes, 0);
        assert!(
            report.fits,
            "tanpa rencana terpilih tidak ada yang perlu ruang"
        );
    }
}
