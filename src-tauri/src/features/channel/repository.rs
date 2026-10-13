//! Menyimpan definisi channel di dalam proyek.

use std::path::PathBuf;

use crate::features::channel::models::Channel;
use crate::storage::project::Project;

/// Membaca dan menulis file `<proyek>/channels/<id>.json`.
pub struct ChannelRepository {
    project: Project,
}

impl ChannelRepository {
    pub fn new(project: Project) -> Self {
        Self { project }
    }

    pub fn directory(&self) -> PathBuf {
        self.project.channels_dir()
    }

    pub fn path_for(&self, channel_id: &str) -> PathBuf {
        self.directory().join(format!("{channel_id}.json"))
    }

    pub fn save(&self, channel: &Channel) -> std::io::Result<()> {
        std::fs::create_dir_all(self.directory())?;
        let text = serde_json::to_string_pretty(channel).unwrap_or_else(|_| String::from("{}"));
        std::fs::write(self.path_for(&channel.id), text)
    }

    pub fn load(&self, channel_id: &str) -> Option<Channel> {
        let text = std::fs::read_to_string(self.path_for(channel_id)).ok()?;
        serde_json::from_str::<Channel>(&text).ok()
    }

    /// Semua channel, yang terakhir diubah lebih dulu.
    pub fn list(&self) -> Vec<Channel> {
        let mut channels: Vec<Channel> = Vec::new();
        let Ok(entries) = std::fs::read_dir(self.directory()) else {
            return channels;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Ok(channel) = serde_json::from_str::<Channel>(&text) {
                channels.push(channel);
            }
        }
        channels.sort_by_key(|channel| std::cmp::Reverse(channel.updated_at));
        channels
    }

    pub fn delete(&self, channel_id: &str) -> std::io::Result<()> {
        let path = self.path_for(channel_id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    use crate::features::channel::models::{ChannelPlan, ChannelPlanStatus};

    fn temp_project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::new("Uji", dir.path().to_string_lossy().to_string());
        project.ensure_structure().unwrap();
        (dir, project)
    }

    fn channel(id: &str, name: &str, seed: i64) -> Channel {
        Channel::create(
            id.to_string(),
            name.to_string(),
            String::from("/p"),
            String::from("/p/outputs"),
            seed,
        )
    }

    #[test]
    fn saves_lists_loads_and_deletes() {
        let (_dir, project) = temp_project();
        let repository = ChannelRepository::new(project.clone());

        let mut older = channel("ch_1", "Lounge", 1);
        older.updated_at = Local::now() - chrono::Duration::seconds(120);
        let newer = channel("ch_2", "Chill", 2);

        repository.save(&older).unwrap();
        repository.save(&newer).unwrap();

        let listed = repository.list();
        assert_eq!(listed.len(), 2);
        assert_eq!(
            listed[0].id, "ch_2",
            "yang terakhir diubah tampil lebih dulu"
        );
        assert_eq!(listed[1].id, "ch_1");

        let loaded = repository.load("ch_1").unwrap();
        assert_eq!(loaded.name, "Lounge");

        repository.delete("ch_1").unwrap();
        assert!(repository.load("ch_1").is_none());
        assert_eq!(repository.list().len(), 1);

        assert!(project.channels_dir().exists());
    }

    #[test]
    fn list_skips_malformed_files() {
        let (_dir, project) = temp_project();
        let repository = ChannelRepository::new(project.clone());
        repository.save(&channel("ch_ok", "Baik", 1)).unwrap();
        std::fs::write(repository.directory().join("rusak.json"), "{bukan json").unwrap();
        std::fs::write(repository.directory().join("catatan.txt"), "abaikan").unwrap();

        let listed = repository.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "ch_ok");
    }

    #[test]
    fn plan_state_and_selection_survive_a_round_trip() {
        let (_dir, project) = temp_project();
        let repository = ChannelRepository::new(project.clone());

        let mut value = channel("ch_1", "Lounge", 1);
        value.plans = vec![
            ChannelPlan {
                status: ChannelPlanStatus::Done,
                output_path: Some(String::from("/p/outputs/Lounge/bg-01.mp4")),
                selected: false,
                ..ChannelPlan::new(String::from("plan_1"), 0)
            },
            ChannelPlan::new(String::from("plan_2"), 1),
        ];
        repository.save(&value).unwrap();

        let reloaded = repository.load("ch_1").unwrap();
        assert_eq!(reloaded.plans[0].status, ChannelPlanStatus::Done);
        assert!(!reloaded.plans[0].selected);
        assert!(reloaded.plans[1].selected);
        assert_eq!(
            reloaded.plans[0].output_path.as_deref(),
            Some("/p/outputs/Lounge/bg-01.mp4")
        );
    }
}
