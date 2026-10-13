//! Fitur pipeline: looping, media tools, dan channel.

pub mod channel;
pub mod looping;
pub mod media_tools;

use std::sync::Arc;

/// Callback progres bernilai 0.0..1.0.
pub type ProgressCallback = Arc<dyn Fn(f64) + Send + Sync>;
