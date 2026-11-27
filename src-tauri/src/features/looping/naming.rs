//! Menyusun nama file output yang bebas tabrakan.

use std::path::{Path, PathBuf};

use crate::core::utils::safe_base_name;

pub const DEFAULT_BASE_NAME: &str = "video";

/// Membentuk path video output dari nama pengguna atau video latar.
///
/// Nama mengikuti video latar (atau nama yang diisi pengguna) tanpa akhiran
/// waktu, supaya berkas hasil mudah dicari. Berkas yang sudah ada tidak
/// ditimpa: `unique_path` menambahkan `_1`, `_2`, dan seterusnya.
pub struct OutputNamer;

impl OutputNamer {
    pub fn resolve(
        &self,
        output_dir: impl AsRef<Path>,
        output_name: &str,
        background_path: impl AsRef<Path>,
    ) -> std::io::Result<PathBuf> {
        let directory = output_dir.as_ref();
        std::fs::create_dir_all(directory)?;

        let base = Self::base_name(output_name, background_path);
        Ok(crate::core::utils::unique_path(
            &directory.join(format!("{base}.mp4")),
        ))
    }

    /// Basis nama berkas: nama pengguna bila ada, selain itu nama video latar.
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
    fn name_follows_the_background_without_a_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let first = OutputNamer.resolve(dir.path(), "", "/x/bg_01.mp4").unwrap();
        assert_eq!(
            first.file_name().unwrap().to_string_lossy(),
            "bg_01.mp4",
            "tanpa akhiran waktu"
        );

        std::fs::write(&first, b"x").unwrap();
        let second = OutputNamer.resolve(dir.path(), "", "/x/bg_01.mp4").unwrap();
        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            "bg_01_1.mp4",
            "berkas yang sudah ada tidak ditimpa"
        );
    }

    #[test]
    fn explicit_name_wins_over_the_background() {
        let dir = tempfile::tempdir().unwrap();
        let output = OutputNamer
            .resolve(dir.path(), "gabungan", "/x/bg_01.mp4")
            .unwrap();
        assert_eq!(
            output.file_name().unwrap().to_string_lossy(),
            "gabungan.mp4"
        );
    }
}
