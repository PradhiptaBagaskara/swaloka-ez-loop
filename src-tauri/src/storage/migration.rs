//! Impor sekali jalan untuk proyek Flutter lama (`project.swaloka`).

use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::storage::paths::PROJECT_FORMAT_VERSION;
use crate::storage::project::Project;

/// Kesalahan saat file proyek lama tidak bisa diimpor.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct MigrationError(pub String);

/// Membaca proyek lama dan menuliskannya dalam format baru.
///
/// File lama tidak pernah diubah atau dihapus, sehingga migrasi yang gagal
/// tidak dapat merusak proyek asli pengguna.
#[derive(Default)]
pub struct LegacyMigrator;

impl LegacyMigrator {
    /// Apakah `root` menyimpan file proyek lama.
    pub fn detect(&self, root: &Path) -> bool {
        root.join("project.swaloka").exists()
    }

    /// Impor proyek lama dan tulis file format baru di sebelahnya.
    pub fn migrate(&self, root: &Path) -> Result<Project, MigrationError> {
        let legacy = root.join("project.swaloka");
        let text = std::fs::read_to_string(&legacy)
            .map_err(|error| MigrationError(format!("cannot read legacy project file: {error}")))?;
        let data: Value = serde_json::from_str(&text)
            .map_err(|error| MigrationError(format!("cannot read legacy project file: {error}")))?;

        let Some(name) = data.get("name").and_then(Value::as_str) else {
            return Err(MigrationError(String::from(
                "legacy project file is missing required fields",
            )));
        };

        let root_path = data
            .get("rootPath")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| root.to_string_lossy().to_string());

        let custom_output_path = data
            .get("customOutputPath")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let project = Project {
            name: name.to_string(),
            root_path,
            custom_output_path,
            enable_parallel_processing: data
                .get("enableParallelProcessing")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            format_version: PROJECT_FORMAT_VERSION,
        };
        let _ = project.ensure_structure();
        let text = serde_json::to_string_pretty(&project.to_json())
            .map_err(|error| MigrationError(error.to_string()))?;
        std::fs::write(project.project_file(), text)
            .map_err(|error| MigrationError(format!("cannot write project file: {error}")))?;
        Ok(project)
    }
}

#[allow(dead_code)]
fn legacy_path(root: &Path) -> PathBuf {
    root.join("project.swaloka")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_file_without_deleting_it() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("project.swaloka");
        std::fs::write(
            &legacy,
            r#"{"name":"Lama","customOutputPath":null,"enableParallelProcessing":false}"#,
        )
        .unwrap();
        let migrator = LegacyMigrator;
        assert!(migrator.detect(dir.path()));
        let project = migrator.migrate(dir.path()).unwrap();
        assert_eq!(project.name, "Lama");
        assert!(!project.enable_parallel_processing);
        assert!(legacy.exists());
        assert!(project.project_file().exists());
    }

    #[test]
    fn rejects_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("project.swaloka"), r#"{"rootPath":"/x"}"#).unwrap();
        assert!(LegacyMigrator.migrate(dir.path()).is_err());
    }
}
