//! Model proyek dan tata letak folder di disk (format v2).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::storage::paths::{
    AUDIO_CACHE_DIR, AUDIO_CACHE_MANIFEST, CACHE_DIR, CHANNELS_DIR, LEGACY_PROJECT_FILE_NAME,
    LOGS_DIR, OUTPUTS_DIR, PROJECT_FILE_NAME, PROJECT_FORMAT_VERSION, TEMP_DIR,
};

/// Sebuah folder pilihan pengguna beserta pengaturannya.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub root_path: String,
    pub custom_output_path: Option<String>,
    pub enable_parallel_processing: bool,
    pub format_version: i32,
}

impl Project {
    pub fn new(name: impl Into<String>, root_path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            root_path: root_path.into(),
            custom_output_path: None,
            enable_parallel_processing: true,
            format_version: PROJECT_FORMAT_VERSION,
        }
    }

    pub fn root(&self) -> PathBuf {
        PathBuf::from(&self.root_path)
    }

    pub fn project_file(&self) -> PathBuf {
        self.root().join(PROJECT_FILE_NAME)
    }

    pub fn legacy_project_file(&self) -> PathBuf {
        self.root().join(LEGACY_PROJECT_FILE_NAME)
    }

    /// Folder output: path kustom bila diset, selain itu `outputs/` di dalam proyek.
    pub fn output_dir(&self) -> PathBuf {
        match &self.custom_output_path {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => self.root().join(OUTPUTS_DIR),
        }
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root().join(LOGS_DIR)
    }

    pub fn temp_dir(&self) -> PathBuf {
        self.root().join(TEMP_DIR)
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root().join(CACHE_DIR)
    }

    pub fn audio_cache_dir(&self) -> PathBuf {
        self.cache_dir().join(AUDIO_CACHE_DIR)
    }

    pub fn audio_cache_manifest(&self) -> PathBuf {
        self.cache_dir().join(AUDIO_CACHE_MANIFEST)
    }

    /// Folder definisi channel di dalam proyek.
    pub fn channels_dir(&self) -> PathBuf {
        self.root().join(CHANNELS_DIR)
    }

    /// Buat folder root proyek dan subfolder standarnya.
    pub fn ensure_structure(&self) -> std::io::Result<()> {
        let directories = [
            self.root(),
            self.root().join(OUTPUTS_DIR),
            self.logs_dir(),
            self.temp_dir(),
            self.cache_dir(),
            self.audio_cache_dir(),
            self.channels_dir(),
        ];
        for directory in directories {
            std::fs::create_dir_all(directory)?;
        }
        if let Some(custom) = &self.custom_output_path {
            if !custom.is_empty() {
                std::fs::create_dir_all(custom)?;
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "formatVersion": self.format_version,
            "name": self.name,
            "rootPath": self.root_path,
            "customOutputPath": self.custom_output_path,
            "enableParallelProcessing": self.enable_parallel_processing,
        })
    }

    /// Bangun proyek dari JSON dengan default yang sama seperti implementasi lama.
    pub fn from_json(data: &Value, default_root: &str) -> Self {
        let root_path = data
            .get("rootPath")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(default_root)
            .to_string();

        let name = data
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                Path::new(&root_path)
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| String::from("Project"));

        let custom_output_path = data
            .get("customOutputPath")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        Self {
            name,
            root_path,
            custom_output_path,
            enable_parallel_processing: data
                .get("enableParallelProcessing")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            format_version: data
                .get("formatVersion")
                .and_then(Value::as_i64)
                .unwrap_or(1) as i32,
        }
    }
}

/// Memuat, menyimpan, dan membuat file proyek (termasuk migrasi `.swaloka`).
#[derive(Default)]
pub struct ProjectStore {
    pub migrator: Option<crate::storage::migration::LegacyMigrator>,
}

impl ProjectStore {
    pub fn new(migrator: Option<crate::storage::migration::LegacyMigrator>) -> Self {
        Self { migrator }
    }

    pub fn create(&self, root_path: &str, name: Option<String>) -> std::io::Result<Project> {
        let root = PathBuf::from(root_path);
        let name = name
            .filter(|value| !value.is_empty())
            .or_else(|| root.file_name().map(|v| v.to_string_lossy().to_string()))
            .unwrap_or_else(|| String::from("Project"));
        let project = Project::new(name, root.to_string_lossy().to_string());
        project.ensure_structure()?;
        self.save(&project)?;
        Ok(project)
    }

    /// Muat proyek, impor file `.swaloka` lama bila ada.
    pub fn load(&self, root_path: &str) -> Result<Project, ProjectError> {
        let root = PathBuf::from(root_path);
        let json_path = root.join(PROJECT_FILE_NAME);
        let project = if json_path.exists() {
            let text = std::fs::read_to_string(&json_path)?;
            let data: Value = serde_json::from_str(&text)?;
            Project::from_json(&data, &root.to_string_lossy())
        } else if root.join(LEGACY_PROJECT_FILE_NAME).exists() {
            match &self.migrator {
                Some(migrator) => migrator.migrate(&root)?,
                None => return Err(ProjectError::MissingMigrator),
            }
        } else {
            return Err(ProjectError::NotFound(root.to_string_lossy().to_string()));
        };
        project.ensure_structure()?;
        Ok(project)
    }

    pub fn save(&self, project: &Project) -> std::io::Result<()> {
        std::fs::create_dir_all(project.root())?;
        let text =
            serde_json::to_string_pretty(&project.to_json()).unwrap_or_else(|_| String::from("{}"));
        std::fs::write(project.project_file(), text)
    }
}

/// Kesalahan saat memuat proyek.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("tidak ada file proyek di {0}")]
    NotFound(String),
    #[error("proyek lama memerlukan migrator")]
    MissingMigrator,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Migration(#[from] crate::storage::migration::MigrationError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_project() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProjectStore::default();
        let project = store
            .create(&dir.path().to_string_lossy(), Some("Uji".into()))
            .unwrap();
        let loaded = store.load(&dir.path().to_string_lossy()).unwrap();
        assert_eq!(loaded.name, "Uji");
        assert_eq!(loaded.format_version, PROJECT_FORMAT_VERSION);
        assert!(loaded.audio_cache_dir().exists());
        let _ = project;
    }

    #[test]
    fn existing_project_gains_channels_dir_without_touching_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProjectStore::default();
        let project = store
            .create(&dir.path().to_string_lossy(), Some("Uji".into()))
            .unwrap();

        // Proyek yang dibuat sebelum fitur channel belum punya folder ini.
        std::fs::remove_dir(project.channels_dir()).unwrap();
        assert!(!project.channels_dir().exists());
        let marker = project.output_dir().join("hasil.mp4");
        std::fs::write(&marker, b"isi lama").unwrap();

        let loaded = store.load(&dir.path().to_string_lossy()).unwrap();
        assert!(loaded.channels_dir().exists());
        assert_eq!(std::fs::read(&marker).unwrap(), b"isi lama");
    }

    #[test]
    fn missing_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProjectStore::default();
        assert!(store.load(&dir.path().to_string_lossy()).is_err());
    }
}
