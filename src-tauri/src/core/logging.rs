//! Logging berjenjang: pohon log yang bisa dimutasi, formatter, dan writer file.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Tingkat keparahan sebuah entri log.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl LogLevel {
    /// Ikon teks yang dipakai di UI dan file log.
    pub fn icon(self) -> &'static str {
        match self {
            LogLevel::Info => "·",
            LogLevel::Success => "✓",
            LogLevel::Warning => "!",
            LogLevel::Error => "✗",
        }
    }

    /// Urutan untuk filter level minimum.
    pub fn order(self) -> usize {
        match self {
            LogLevel::Info => 0,
            LogLevel::Success => 1,
            LogLevel::Warning => 2,
            LogLevel::Error => 3,
        }
    }
}

/// Entri log yang direkam untuk file `logs/` (berbentuk pohon).
#[derive(Debug)]
pub struct LogEntry {
    pub message: String,
    pub level: LogLevel,
    pub timestamp: DateTime<Local>,
    pub children: Vec<LogEntry>,
    pub duration: Option<Duration>,
}

impl LogEntry {
    pub fn new(message: impl Into<String>, level: LogLevel) -> Self {
        Self {
            message: message.into(),
            level,
            timestamp: Local::now(),
            children: Vec::new(),
            duration: None,
        }
    }

    /// Buat entri anak dan kembalikan referensinya.
    pub fn add(&mut self, message: impl Into<String>, level: LogLevel) -> &mut LogEntry {
        self.adopt(LogEntry::new(message, level))
    }

    /// Lampirkan entri yang sudah ada sebagai anak.
    pub fn adopt(&mut self, entry: LogEntry) -> &mut LogEntry {
        self.children.push(entry);
        self.children
            .last_mut()
            .expect("anak baru saja ditambahkan")
    }

    /// Catat durasi operasi yang digambarkan entri ini.
    pub fn finish(&mut self, duration: Option<Duration>) {
        self.duration = duration;
    }

    /// Telusuri entri ini dan seluruh turunannya secara depth-first.
    pub fn walk(&self) -> Vec<&LogEntry> {
        let mut out = vec![self];
        for child in &self.children {
            out.extend(child.walk());
        }
        out
    }

    /// Bentuk datar (kedalaman, entri) untuk dikirim ke UI.
    pub fn flatten(&self) -> Vec<(usize, &LogEntry)> {
        let mut out = Vec::new();
        self.flatten_into(0, &mut out);
        out
    }

    fn flatten_into<'a>(&'a self, depth: usize, out: &mut Vec<(usize, &'a LogEntry)>) {
        out.push((depth, self));
        for child in &self.children {
            child.flatten_into(depth + 1, out);
        }
    }

    /// Ubah entri ini menjadi record datar untuk event UI.
    pub fn record(&self, depth: usize) -> LogRecord {
        LogRecord {
            message: self.message.clone(),
            level: self.level,
            timestamp: format_timestamp(self.timestamp),
            depth,
            duration_seconds: self.duration.map(|value| value.as_secs_f64()),
        }
    }
}

/// Entri log datar yang dikirim lewat event ke frontend.
#[derive(Clone, Debug, Serialize)]
pub struct LogRecord {
    pub message: String,
    pub level: LogLevel,
    pub timestamp: String,
    pub depth: usize,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<f64>,
}

impl LogRecord {
    pub fn new(message: impl Into<String>, level: LogLevel, depth: usize) -> Self {
        Self {
            message: message.into(),
            level,
            timestamp: format_timestamp(Local::now()),
            depth,
            duration_seconds: None,
        }
    }
}

fn format_timestamp(moment: DateTime<Local>) -> String {
    format!(
        "{}.{:03}",
        moment.format("%H:%M:%S"),
        moment.timestamp_subsec_millis()
    )
}

/// Render pohon log menjadi teks polos untuk file `logs/`.
pub struct LogFormatter;

impl LogFormatter {
    pub fn format_entry(entry: &LogEntry, indent: usize) -> Vec<String> {
        let mut line = format!(
            "{}{} {} {}",
            "  ".repeat(indent),
            format_timestamp(entry.timestamp),
            entry.level.icon(),
            entry.message
        );
        if let Some(duration) = entry.duration {
            line.push_str(&format!(" [{:.2}s]", duration.as_secs_f64()));
        }
        let mut lines = vec![line];
        for child in &entry.children {
            lines.extend(Self::format_entry(child, indent + 1));
        }
        lines
    }

    pub fn format_tree(entries: &[LogEntry]) -> String {
        let mut lines: Vec<String> = Vec::new();
        for entry in entries {
            lines.extend(Self::format_entry(entry, 0));
        }
        lines.join("\n")
    }
}

/// Menulis pohon log satu kali run ke file di folder `logs/` proyek.
pub struct LogFileWriter {
    logs_dir: PathBuf,
}

impl LogFileWriter {
    pub fn new(logs_dir: impl AsRef<Path>) -> Self {
        Self {
            logs_dir: logs_dir.as_ref().to_path_buf(),
        }
    }

    pub fn write(&self, entries: &[LogEntry], base_name: &str) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.logs_dir)?;
        let stamp = Local::now().format("%Y%m%d_%H%M%S");
        let path = self.logs_dir.join(format!("{base_name}_{stamp}.log"));
        std::fs::write(&path, LogFormatter::format_tree(entries))?;
        Ok(path)
    }
}

/// Tulis daftar record log (hasil streaming ke UI) ke file teks berindentasi.
pub fn write_records(
    logs_dir: &Path,
    base_name: &str,
    records: &[LogRecord],
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(logs_dir)?;
    let stamp = Local::now().format("%Y%m%d_%H%M%S");
    let path = logs_dir.join(format!("{base_name}_{stamp}.log"));
    let text = records
        .iter()
        .map(|record| {
            format!(
                "{}{} {} {}",
                "  ".repeat(record.depth),
                record.timestamp,
                record.level.icon(),
                record.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, text)?;
    Ok(path)
}

/// Logger konsol tingkat sistem.
pub struct AppLogger {
    pub enabled: bool,
    pub min_level: LogLevel,
}

impl AppLogger {
    pub fn new(enabled: bool, min_level: LogLevel) -> Self {
        Self { enabled, min_level }
    }

    pub fn log(&self, message: &str, level: LogLevel) {
        if !self.enabled || level.order() < self.min_level.order() {
            return;
        }
        println!("{} {}", level.icon(), message);
    }

    pub fn info(&self, message: &str) {
        self.log(message, LogLevel::Info);
    }

    pub fn success(&self, message: &str) {
        self.log(message, LogLevel::Success);
    }

    pub fn warning(&self, message: &str) {
        self.log(message, LogLevel::Warning);
    }

    pub fn error(&self, message: &str) {
        self.log(message, LogLevel::Error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_tree_with_depth() {
        let mut root = LogEntry::new("root", LogLevel::Info);
        root.add("child", LogLevel::Info);
        root.add("child2", LogLevel::Warning)
            .add("grand", LogLevel::Info);
        let flat = root.flatten();
        assert_eq!(flat.len(), 4);
        assert_eq!(flat[0].0, 0);
        assert_eq!(flat[1].0, 1);
        assert_eq!(flat[3].0, 2);
    }

    #[test]
    fn formats_tree_text() {
        let mut root = LogEntry::new("command", LogLevel::Info);
        root.add("line", LogLevel::Warning);
        let text = LogFormatter::format_tree(std::slice::from_ref(&root));
        assert!(text.contains("command"));
        assert!(text.contains("! line"));
    }
}
