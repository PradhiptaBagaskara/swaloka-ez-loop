//! Preferensi aplikasi (pengganti shared_preferences), disimpan sebagai JSON.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::storage::paths::preferences_file;

pub const RECENT_PROJECT_LIMIT: usize = 5;

/// Preferensi lintas proyek. Kunci JSON sengaja snake_case agar sama
/// dengan format lama sehingga file preferensi lama tetap terbaca.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub ffmpeg_path: Option<String>,
    #[serde(default)]
    pub hw_encoder: Option<String>,
    #[serde(default)]
    pub last_project_path: Option<String>,
    #[serde(default)]
    pub recent_projects: Vec<String>,
    /// Port loopback Kontrol AI (MCP). Server sendiri tidak dinyalakan otomatis
    /// saat aplikasi dibuka; pengguna yang mengaktifkannya tiap sesi.
    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,
}

fn default_theme() -> String {
    String::from("light")
}

fn default_mcp_port() -> u16 {
    crate::mcp::DEFAULT_PORT
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            ffmpeg_path: None,
            hw_encoder: None,
            last_project_path: None,
            recent_projects: Vec::new(),
            mcp_port: default_mcp_port(),
        }
    }
}

impl Preferences {
    /// Pindahkan `path` ke depan daftar terkini, dibatasi lima entri.
    pub fn add_recent(&self, path: &str) -> Self {
        let mut entries: Vec<String> = self
            .recent_projects
            .iter()
            .filter(|entry| entry.as_str() != path)
            .cloned()
            .collect();
        entries.insert(0, path.to_string());
        entries.truncate(RECENT_PROJECT_LIMIT);
        Self {
            recent_projects: entries,
            last_project_path: Some(path.to_string()),
            ..self.clone()
        }
    }

    pub fn remove_recent(&self, path: &str) -> Self {
        Self {
            recent_projects: self
                .recent_projects
                .iter()
                .filter(|entry| entry.as_str() != path)
                .cloned()
                .collect(),
            ..self.clone()
        }
    }

    /// Baca dari nilai JSON, mengabaikan entri tak dikenal / tak valid.
    pub fn from_json(data: &serde_json::Value) -> Self {
        let mut prefs: Preferences = serde_json::from_value(data.clone()).unwrap_or_default();
        prefs.recent_projects.truncate(RECENT_PROJECT_LIMIT);
        if prefs.theme.is_empty() {
            prefs.theme = default_theme();
        }
        prefs
    }
}

/// Memuat dan menyimpan [`Preferences`] sebagai JSON.
pub struct PreferencesStore {
    path: PathBuf,
}

impl PreferencesStore {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path: path.unwrap_or_else(preferences_file),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Preferences {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Preferences::default();
        };
        let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) else {
            return Preferences::default();
        };
        Preferences::from_json(&data)
    }

    pub fn save(&self, prefs: &Preferences) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(prefs).unwrap_or_else(|_| String::from("{}"));
        std::fs::write(&self.path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_recent_moves_to_front_and_caps() {
        let mut prefs = Preferences::default();
        for i in 0..7 {
            prefs = prefs.add_recent(&format!("/proj/{i}"));
        }
        assert_eq!(prefs.recent_projects.len(), RECENT_PROJECT_LIMIT);
        assert_eq!(prefs.recent_projects[0], "/proj/6");
        prefs = prefs.add_recent("/proj/3");
        assert_eq!(prefs.recent_projects[0], "/proj/3");
        assert_eq!(prefs.recent_projects.len(), RECENT_PROJECT_LIMIT);
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = PreferencesStore::new(Some(dir.path().join("preferences.json")));
        let prefs = Preferences {
            theme: "dark".into(),
            ffmpeg_path: Some("/usr/local/bin/ffmpeg".into()),
            ..Preferences::default()
        };
        store.save(&prefs).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.ffmpeg_path.as_deref(), Some("/usr/local/bin/ffmpeg"));
    }
}
