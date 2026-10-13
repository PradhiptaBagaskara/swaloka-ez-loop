//! Folder temp per-run dan pembersihan temp yang tertinggal.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Membuat folder temp per-run di dalam `temp/` proyek.
pub struct TempManager {
    temp_dir: PathBuf,
}

impl TempManager {
    pub fn new(temp_dir: impl AsRef<Path>) -> Self {
        Self {
            temp_dir: temp_dir.as_ref().to_path_buf(),
        }
    }

    /// Buat dan kembalikan folder temp baru dengan nama unik.
    pub fn create(&self, prefix: &str) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.temp_dir)?;
        let moment = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or(0);
        let mut path = self.temp_dir.join(format!("{prefix}_{moment}"));
        let mut suffix = 0;
        while path.exists() {
            suffix += 1;
            path = self.temp_dir.join(format!("{prefix}_{moment}_{suffix}"));
        }
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Hapus folder temp per-run bila ada.
    pub fn cleanup(&self, path: &Path) {
        if path.exists() {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    /// Hapus seluruh isi `temp/`; kembalikan jumlah entri yang dihapus.
    pub fn clean_stale(&self) -> usize {
        if !self.temp_dir.exists() {
            return 0;
        }
        let mut removed = 0;
        if let Ok(entries) = std::fs::read_dir(&self.temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ok = if path.is_dir() {
                    std::fs::remove_dir_all(&path).is_ok()
                } else {
                    std::fs::remove_file(&path).is_ok()
                };
                if ok {
                    removed += 1;
                }
            }
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_unique_dirs_and_cleans() {
        let dir = tempfile::tempdir().unwrap();
        let manager = TempManager::new(dir.path());
        let first = manager.create("run").unwrap();
        let second = manager.create("run").unwrap();
        assert_ne!(first, second);
        assert_eq!(manager.clean_stale(), 2);
        assert!(!first.exists());
    }
}
