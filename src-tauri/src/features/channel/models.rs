//! Model channel: definisi channel, daftar aset yang terdeteksi, rencana episode,
//! dan status pengerjaannya.

use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer, Serialize};

use crate::core::serde_time::{local_iso, opt_local_iso};

/// Durasi minimum bawaan untuk satu rencana (menit).
pub const DEFAULT_MIN_DURATION_MINUTES: f64 = 120.0;

/// Durasi cadangan saat durasi audio tidak terbaca (detik).
pub const FALLBACK_AUDIO_SECONDS: f64 = 180.0;

/// Target putar bawaan tiap lagu per rencana.
pub const DEFAULT_SUGGESTED_LOOP_CAP: i64 = 2;

/// Status keseluruhan sebuah channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChannelStatus {
    #[default]
    Draft,
    Ready,
    Running,
    PartiallyDone,
    Completed,
    Failed,
}

/// Status satu rencana episode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChannelPlanStatus {
    #[default]
    NotProcessed,
    Running,
    Done,
    Failed,
}

fn deserialize_channel_status<'de, D>(deserializer: D) -> Result<ChannelStatus, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.as_str() {
        "ready" => ChannelStatus::Ready,
        "running" => ChannelStatus::Running,
        "partiallyDone" => ChannelStatus::PartiallyDone,
        "completed" => ChannelStatus::Completed,
        "failed" => ChannelStatus::Failed,
        _ => ChannelStatus::Draft,
    })
}

fn deserialize_plan_status<'de, D>(deserializer: D) -> Result<ChannelPlanStatus, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.as_str() {
        "running" => ChannelPlanStatus::Running,
        "done" => ChannelPlanStatus::Done,
        "failed" => ChannelPlanStatus::Failed,
        _ => ChannelPlanStatus::NotProcessed,
    })
}

fn default_min_duration_minutes() -> f64 {
    DEFAULT_MIN_DURATION_MINUTES
}

fn default_loop_cap() -> i64 {
    DEFAULT_SUGGESTED_LOOP_CAP
}

fn default_true() -> bool {
    true
}

/// Satu lagu yang terdeteksi di folder audio channel.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAudioItem {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
}

impl ChannelAudioItem {
    pub fn with_duration(&self, duration: f64) -> Self {
        Self {
            duration_seconds: Some(duration),
            ..self.clone()
        }
    }

    /// Durasi yang dipakai perencanaan: nilai terbaca atau cadangan.
    pub fn effective_duration(&self) -> f64 {
        self.duration_seconds.unwrap_or(FALLBACK_AUDIO_SECONDS)
    }
}

/// Satu rencana episode: satu video latar dengan satu playlist lagu.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelPlan {
    pub id: String,
    #[serde(default)]
    pub index: i64,
    #[serde(default)]
    pub background_video_path: String,
    #[serde(default)]
    pub audio_item_ids: Vec<String>,
    #[serde(default)]
    pub total_duration_seconds: f64,
    #[serde(default)]
    pub max_usage: i64,
    #[serde(default)]
    pub min_usage: i64,
    #[serde(default = "default_true")]
    pub selected: bool,
    #[serde(default, deserialize_with = "deserialize_plan_status")]
    pub status: ChannelPlanStatus,
    #[serde(default)]
    pub intro_video_path: Option<String>,
    #[serde(default)]
    pub output_path: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default, with = "opt_local_iso")]
    pub processed_at: Option<DateTime<Local>>,
    #[serde(default, with = "opt_local_iso")]
    pub finished_at: Option<DateTime<Local>>,
}

impl ChannelPlan {
    /// Rencana baru yang belum diproses dan belum punya latar.
    pub fn new(id: String, index: i64) -> Self {
        Self {
            id,
            index,
            background_video_path: String::new(),
            audio_item_ids: Vec::new(),
            total_duration_seconds: 0.0,
            max_usage: 0,
            min_usage: 0,
            selected: true,
            status: ChannelPlanStatus::NotProcessed,
            intro_video_path: None,
            output_path: None,
            error: None,
            processed_at: None,
            finished_at: None,
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            ChannelPlanStatus::Done | ChannelPlanStatus::Failed
        )
    }
}

/// Sebuah channel beserta aset dan rencananya.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub project_root_path: String,
    pub output_directory_path: String,
    #[serde(default)]
    pub audio_folder_path: String,
    #[serde(default)]
    pub video_folder_path: String,
    #[serde(default = "default_min_duration_minutes")]
    pub min_duration_minutes: f64,
    /// Target putar tiap lagu di dalam satu rencana.
    #[serde(default = "default_loop_cap")]
    pub suggested_loop_cap: i64,
    /// Jumlah konten yang ingin dibuat; 0 berarti mengikuti jumlah video.
    #[serde(default)]
    pub total_content: i64,
    #[serde(default)]
    pub seed: i64,
    #[serde(default)]
    pub audio_items: Vec<ChannelAudioItem>,
    #[serde(default)]
    pub video_items: Vec<String>,
    #[serde(default)]
    pub plans: Vec<ChannelPlan>,
    #[serde(default, deserialize_with = "deserialize_channel_status")]
    pub status: ChannelStatus,
    #[serde(default = "Local::now", with = "local_iso")]
    pub created_at: DateTime<Local>,
    #[serde(default = "Local::now", with = "local_iso")]
    pub updated_at: DateTime<Local>,
}

impl Channel {
    pub fn create(
        id: String,
        name: String,
        project_root_path: String,
        output_directory_path: String,
        seed: i64,
    ) -> Self {
        let now = Local::now();
        Self {
            id,
            name,
            project_root_path,
            output_directory_path,
            audio_folder_path: String::new(),
            video_folder_path: String::new(),
            min_duration_minutes: DEFAULT_MIN_DURATION_MINUTES,
            suggested_loop_cap: DEFAULT_SUGGESTED_LOOP_CAP,
            total_content: 0,
            seed,
            audio_items: Vec::new(),
            video_items: Vec::new(),
            plans: Vec::new(),
            status: ChannelStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    /// Tandai waktu pembaruan terakhir.
    pub fn touch(&mut self) {
        self.updated_at = Local::now();
    }

    /// Durasi minimum rencana dalam detik.
    pub fn min_duration_seconds(&self) -> f64 {
        (self.min_duration_minutes.max(0.0)) * 60.0
    }

    /// Jumlah plan yang harus dibuat: yang diminta pengguna, dan 0 berarti
    /// mengikuti jumlah video. Plan yang belum kebagian video tetap dibuat
    /// dan menunggu video.
    pub fn plan_target(&self) -> usize {
        if self.total_content > 0 {
            self.total_content as usize
        } else {
            self.video_items.len()
        }
    }

    /// Video yang sudah dipakai rencana, beserta rencananya.
    pub fn video_assignments(&self) -> std::collections::BTreeMap<&str, &str> {
        self.plans
            .iter()
            .filter(|plan| !plan.background_video_path.is_empty())
            .map(|plan| (plan.background_video_path.as_str(), plan.id.as_str()))
            .collect()
    }

    /// Rencana yang masih menunggu video latar.
    pub fn waiting_for_video(&self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.background_video_path.is_empty())
            .count()
    }

    /// Target putar tiap lagu per episode, minimal satu kali.
    pub fn loop_cap(&self) -> i64 {
        self.suggested_loop_cap.max(1)
    }

    pub fn audio_by_id(&self, id: &str) -> Option<&ChannelAudioItem> {
        self.audio_items.iter().find(|item| item.id == id)
    }

    pub fn plan_by_id(&self, id: &str) -> Option<&ChannelPlan> {
        self.plans.iter().find(|plan| plan.id == id)
    }

    /// Rencana yang dipilih untuk diproses, urut sesuai indeks.
    pub fn selected_plans(&self) -> Vec<&ChannelPlan> {
        let mut plans: Vec<&ChannelPlan> = self.plans.iter().filter(|plan| plan.selected).collect();
        plans.sort_by_key(|plan| plan.index);
        plans
    }

    pub fn done_count(&self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.status == ChannelPlanStatus::Done)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.plans
            .iter()
            .filter(|plan| plan.status == ChannelPlanStatus::Failed)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_selected_defaults_to_true() {
        let payload = json!({
            "id": "ch1",
            "name": "Lounge",
            "projectRootPath": "/p",
            "outputDirectoryPath": "/p/outputs",
            "plans": [
                {"id": "plan_1", "index": 0, "backgroundVideoPath": "/bg.mp4"},
                {"id": "plan_2", "index": 1, "selected": false}
            ],
            "createdAt": "2026-05-03T10:00:00.000000",
            "updatedAt": "2026-05-03T10:00:00.000000"
        });
        let channel: Channel = serde_json::from_value(payload).unwrap();
        assert!(
            channel.plans[0].selected,
            "rencana tanpa field selected terpilih"
        );
        assert!(
            !channel.plans[1].selected,
            "selected: false tetap dihormati"
        );
        assert_eq!(channel.min_duration_minutes, DEFAULT_MIN_DURATION_MINUTES);
        assert_eq!(channel.selected_plans().len(), 1);
    }

    #[test]
    fn unknown_plan_status_falls_back_to_not_processed() {
        let payload = json!({
            "id": "ch1", "name": "n",
            "projectRootPath": "/p", "outputDirectoryPath": "/o",
            "status": "aneh",
            "plans": [{"id": "plan_1", "index": 0, "status": "entah"}]
        });
        let channel: Channel = serde_json::from_value(payload).unwrap();
        assert_eq!(channel.status, ChannelStatus::Draft);
        assert_eq!(channel.plans[0].status, ChannelPlanStatus::NotProcessed);
    }

    #[test]
    fn round_trips_channel_json() {
        let mut channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge"),
            String::from("/p"),
            String::from("/p/outputs"),
            42,
        );
        channel.audio_folder_path = String::from("/musik");
        channel.plans = vec![ChannelPlan {
            total_duration_seconds: 7265.5,
            audio_item_ids: vec![String::from("a1"), String::from("a2")],
            max_usage: 4,
            min_usage: 3,
            status: ChannelPlanStatus::Done,
            output_path: Some(String::from("/p/outputs/Lounge/bg.mp4")),
            ..ChannelPlan::new(String::from("plan_1"), 0)
        }];

        let text = serde_json::to_string_pretty(&channel).unwrap();
        let back: Channel = serde_json::from_str(&text).unwrap();
        assert_eq!(back.id, channel.id);
        assert_eq!(back.seed, 42);
        assert_eq!(back.audio_folder_path, "/musik");
        assert_eq!(back.plans[0].status, ChannelPlanStatus::Done);
        assert_eq!(back.plans[0].max_usage, 4);
        assert!(back.plans[0].selected);

        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["plans"][0]["status"], json!("done"));
        assert_eq!(value["status"], json!("draft"));
    }

    #[test]
    fn plan_target_follows_the_request_and_the_video_count() {
        let mut channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge"),
            String::from("/p"),
            String::from("/p/outputs"),
            1,
        );
        channel.video_items = vec![String::from("/v/a.mp4"), String::from("/v/b.mp4")];

        // Tanpa permintaan eksplisit: ikut jumlah video.
        assert_eq!(channel.plan_target(), 2);
        // Minta lebih banyak dari video yang ada: rencananya tetap dibuat,
        // yang belum kebagian video tinggal menunggu.
        channel.total_content = 30;
        assert_eq!(channel.plan_target(), 30);
        // Minta lebih sedikit: permintaan yang dipakai.
        channel.total_content = 1;
        assert_eq!(channel.plan_target(), 1);
        // Tanpa video dan tanpa permintaan: tidak ada rencana.
        channel.video_items.clear();
        channel.total_content = 0;
        assert_eq!(channel.plan_target(), 0);

        assert_eq!(channel.loop_cap(), DEFAULT_SUGGESTED_LOOP_CAP);
        channel.suggested_loop_cap = 0;
        assert_eq!(channel.loop_cap(), 1, "target putar minimal satu kali");
    }

    #[test]
    fn marks_which_video_each_plan_uses() {
        let mut channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge"),
            String::from("/p"),
            String::from("/p/outputs"),
            1,
        );
        channel.audio_items = vec![ChannelAudioItem {
            id: String::from("audio_1"),
            path: String::from("/musik/a.mp3"),
            duration_seconds: Some(240.0),
        }];
        channel.video_items = vec![
            String::from("/v/a.mp4"),
            String::from("/v/b.mp4"),
            String::from("/v/c.mp4"),
        ];
        channel.plans = vec![
            ChannelPlan {
                background_video_path: String::from("/v/b.mp4"),
                ..ChannelPlan::new(String::from("plan_1"), 0)
            },
            ChannelPlan::new(String::from("plan_2"), 1),
        ];

        let assignments = channel.video_assignments();
        assert_eq!(assignments.get("/v/b.mp4"), Some(&"plan_1"));
        assert_eq!(
            assignments.get("/v/a.mp4"),
            None,
            "video bebas belum ditandai"
        );
        assert_eq!(channel.waiting_for_video(), 1);
        assert_eq!(channel.video_assignments().len(), 1);
        assert_eq!(channel.waiting_for_video(), 1);
    }

    #[test]
    fn effective_duration_uses_fallback() {
        let item = ChannelAudioItem {
            id: String::from("a1"),
            path: String::from("/a.mp3"),
            duration_seconds: None,
        };
        assert_eq!(item.effective_duration(), FALLBACK_AUDIO_SECONDS);
        assert_eq!(item.with_duration(120.0).effective_duration(), 120.0);
    }
}
