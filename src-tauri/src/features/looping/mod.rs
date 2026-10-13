//! Pipeline looping: video latar diulang di atas playlist audio.

pub mod audio;
pub mod builders;
pub mod models;
pub mod naming;
pub mod pipeline;
pub mod playlist;
pub mod preflight;

pub use audio::AudioNormalizer;
pub use builders::LoopCommandBuilder;
pub use models::{LoopRequest, LoopResult, PreflightReport};
pub use naming::OutputNamer;
pub use pipeline::LoopMergeService;
pub use playlist::PlaylistBuilder;
pub use preflight::PreflightChecker;
