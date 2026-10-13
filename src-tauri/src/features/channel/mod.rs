//! Fitur channel: definisi channel dari folder aset, deteksi aset, perencanaan
//! episode yang seimbang, dan eksekusi rencana secara berurutan.

pub mod capacity;
pub mod detector;
pub mod executor;
pub mod models;
pub mod planner;
pub mod repository;

pub use capacity::{plan_input, ChannelCapacity, ChannelCapacityReport, PlanCapacity};
pub use detector::{ChannelDetector, DetectedAssets, AUDIO_EXTENSIONS, VIDEO_EXTENSIONS};
pub use executor::{finalize, ChannelExecutor, CANCELLED_MESSAGE};
pub use models::{
    Channel, ChannelAudioItem, ChannelPlan, ChannelPlanStatus, ChannelStatus,
    DEFAULT_MIN_DURATION_MINUTES, DEFAULT_SUGGESTED_LOOP_CAP, FALLBACK_AUDIO_SECONDS,
};
pub use planner::{ChannelPlanner, PlaylistDraft};
pub use repository::ChannelRepository;
