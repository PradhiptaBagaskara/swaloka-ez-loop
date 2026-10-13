//! Menyusun nama file output yang bebas tabrakan.

use chrono::{DateTime, Local, Utc};
use std::path::{Path, PathBuf};

use crate::core::utils::{safe_base_name, unique_path};

pub const DEFAULT_BASE_NAME: &str = "video";

/// Membentuk path video output dari nama pengguna atau video latar.
pub struct OutputNamer;

impl OutputNamer {
    pub fn resolve(
        &self,
        output_dir: impl AsRef<Path>,
        output_name: &str,
        background_path: impl AsRef<Path>,
        moment: Option<DateTime<Local>>,
    ) -> std::io::Result<PathBuf> {
        let directory = output_dir.as_ref();
        std::fs::create_dir_all(directory)?;

        let base = Self::base_name(output_name, background_path);
        let suffix = Self::suffix(moment);
        let candidate = directory.join(format!("{base}_{suffix}.mp4"));
        Ok(unique_path(&candidate))
    }

    pub fn base_name(output_name: &str, background_path: impl AsRef<Path>) -> String {
        let trimmed = output_name.trim();
        if !trimmed.is_empty() {
            return safe_base_name(trimmed, 64);
        }
        let stem = background_path
            .as_ref()
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        if stem.is_empty() {
            DEFAULT_BASE_NAME.to_string()
        } else {
            safe_base_name(&stem, 64)
        }
    }

    /// Empat digit terakhir milidetik epoch.
    pub fn suffix(moment: Option<DateTime<Local>>) -> String {
        let millis = moment
            .map(|value| value.timestamp_millis())
            .unwrap_or_else(|| Utc::now().timestamp_millis());
        format!("{:04}", millis % 10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_explicit_name() {
        assert_eq!(OutputNamer::base_name("My Video", "/x/bg.mp4"), "My_Video");
        assert_eq!(
            OutputNamer::base_name("", "/x/My Background.mov"),
            "My_Background"
        );
    }

    #[test]
    fn suffix_has_four_digits() {
        assert_eq!(OutputNamer::suffix(None).len(), 4);
    }

    #[test]
    fn avoids_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let namer = OutputNamer;
        let first = namer.resolve(dir.path(), "vid", "/x/bg.mp4", None).unwrap();
        std::fs::write(&first, b"x").unwrap();
        let second = namer.resolve(dir.path(), "vid", "/x/bg.mp4", None).unwrap();
        assert_ne!(first, second);
    }
}
