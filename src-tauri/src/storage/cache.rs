//! Cache audio ternormalisasi yang dikunci oleh identitas file sumber.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::core::utils::{fnv1a64, safe_base_name};
use crate::storage::project::Project;

/// Hasil pembersihan cache.
#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheClearResult {
    pub removed_audio: usize,
    pub removed_logs: usize,
}

impl CacheClearResult {
    pub fn total(&self) -> usize {
        self.removed_audio + self.removed_logs
    }
}

/// Menyimpan salinan audio ternormalisasi dan manifest sumber -> cache.
pub struct AudioCache {
    project: Project,
}

impl AudioCache {
    pub fn new(project: Project) -> Self {
        Self { project }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.project.audio_cache_manifest()
    }

    pub fn load_manifest(&self) -> BTreeMap<String, String> {
        let path = self.manifest_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return BTreeMap::new();
        };
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) else {
            return BTreeMap::new();
        };
        map.into_iter()
            .map(|(key, value)| {
                let value = value.as_str().unwrap_or_default().to_string();
                (key, value)
            })
            .collect()
    }

    pub fn save_manifest(&self, manifest: &BTreeMap<String, String>) -> std::io::Result<()> {
        if let Some(parent) = self.manifest_path().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(manifest).unwrap_or_else(|_| String::from("{}"));
        std::fs::write(self.manifest_path(), text)
    }

    /// Identitas stabil sebuah file sumber (path kanonik, case-fold di macOS/Windows).
    pub fn cache_key(&self, source: &Path) -> String {
        let key = resolve_path(source).to_string_lossy().to_string();
        if cfg!(target_os = "macos") || cfg!(windows) {
            key.to_lowercase()
        } else {
            key
        }
    }

    /// File cache yang menjadi tujuan normalisasi sebuah sumber.
    pub fn target_path(&self, source: &Path) -> PathBuf {
        let key = self.cache_key(source);
        let stem = source
            .file_stem()
            .map(|value| safe_base_name(&value.to_string_lossy(), 48))
            .unwrap_or_else(|| String::from("audio"));
        self.project
            .audio_cache_dir()
            .join(format!("{stem}_{:x}.m4a", fnv1a64(&key)))
    }

    /// Kembalikan salinan cache sebuah sumber bila masih ada.
    pub fn lookup(&self, source: &Path) -> Option<PathBuf> {
        let manifest = self.load_manifest();
        let cached = manifest.get(&self.cache_key(source))?;
        if cached.is_empty() {
            return None;
        }
        let path = PathBuf::from(cached);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn record(&self, source: &Path, cached: &Path) -> std::io::Result<()> {
        let mut manifest = self.load_manifest();
        manifest.insert(self.cache_key(source), cached.to_string_lossy().to_string());
        self.save_manifest(&manifest)
    }

    /// Hapus audio cache dan file log, biarkan output dan pengaturan tetap utuh.
    pub fn clear(&self) -> CacheClearResult {
        let mut removed_audio = 0;
        let audio_dir = self.project.audio_cache_dir();
        if audio_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&audio_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_file() && std::fs::remove_file(entry.path()).is_ok() {
                        removed_audio += 1;
                    }
                }
            }
            // Foldernya sengaja dibiarkan: ia bagian dari tata letak proyek, dan
            // normalisasi berikutnya menulis ke sini lagi. Kalau dihapus, run
            // sesudah pembersihan gagal karena ffmpeg tak bisa membuka output.
        }

        let manifest = self.manifest_path();
        if manifest.exists() {
            let _ = std::fs::remove_file(&manifest);
        }

        let mut removed_logs = 0;
        let logs_dir = self.project.logs_dir();
        if logs_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&logs_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_file() && std::fs::remove_file(entry.path()).is_ok() {
                        removed_logs += 1;
                    }
                }
            }
        }

        CacheClearResult {
            removed_audio,
            removed_logs,
        }
    }
}

/// Kembalikan path absolut kanonik; fallback ke path absolut biasa bila file belum ada.
pub fn resolve_path(source: &Path) -> PathBuf {
    let expanded = if let Ok(rest) = source.strip_prefix("~") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(rest)
    } else {
        source.to_path_buf()
    };
    std::fs::canonicalize(&expanded).unwrap_or_else(|_| {
        if expanded.is_absolute() {
            expanded
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(expanded)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::project::Project;

    #[test]
    fn target_path_is_stable_and_suffixed() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::new("P", dir.path().to_string_lossy());
        project.ensure_structure().unwrap();
        let cache = AudioCache::new(project);
        let source = dir.path().join("My Song.mp3");
        let target = cache.target_path(&source);
        assert_eq!(target.extension().unwrap(), "m4a");
        assert!(target
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("My_Song_"));
        assert_eq!(target, cache.target_path(&source));
    }

    #[test]
    fn records_and_looks_up() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::new("P", dir.path().to_string_lossy());
        project.ensure_structure().unwrap();
        let cache = AudioCache::new(project);
        let source = dir.path().join("a.mp3");
        std::fs::write(&source, b"x").unwrap();
        let target = cache.target_path(&source);
        std::fs::write(&target, b"y").unwrap();
        cache.record(&source, &target).unwrap();
        assert_eq!(cache.lookup(&source), Some(target));
    }

    #[test]
    fn clear_empties_files_but_keeps_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::new("P", dir.path().to_string_lossy());
        project.ensure_structure().unwrap();
        let cache = AudioCache::new(project);
        let source = dir.path().join("a.mp3");
        std::fs::write(&source, b"x").unwrap();
        let target = cache.target_path(&source);
        std::fs::write(&target, b"y").unwrap();
        cache.record(&source, &target).unwrap();

        let result = cache.clear();

        assert_eq!(result.removed_audio, 1);
        assert!(!target.exists());
        assert!(
            cache
                .target_path(&source)
                .parent()
                .expect("target punya folder")
                .exists(),
            "folder cache harus tetap ada supaya run berikutnya bisa menulis"
        );
    }
}
