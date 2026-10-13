//! Helper bersama: format, hashing, dan nama file yang aman.

use chrono::{DateTime, Local};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

/// Format jumlah byte memakai satuan biner (B, KB, MB, GB, TB).
pub fn format_bytes(size: f64) -> String {
    let mut value = size;
    for unit in ["B", "KB", "MB", "GB", "TB"] {
        if value.abs() < 1024.0 || unit == "TB" {
            if unit == "B" {
                return format!("{} B", value as i64);
            }
            return format!("{value:.1} {unit}");
        }
        value /= 1024.0;
    }
    format!("{value:.1} TB")
}

/// Format durasi seperti ringkasan log FFmpeg (ms / s / m s).
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs_f64();
    if total_seconds < 1.0 {
        return format!("{}ms", duration.as_millis());
    }
    if total_seconds < 60.0 {
        return format!("{total_seconds:.1}s");
    }
    let minutes = (total_seconds / 60.0) as u64;
    let seconds = (total_seconds % 60.0) as u64;
    format!("{minutes}m {seconds}s")
}

/// Slug `YYYY_MM_DD_HH_MM_SS_ms` untuk nama file.
pub fn timestamp_slug(moment: Option<DateTime<Local>>) -> String {
    let moment = moment.unwrap_or_else(Local::now);
    let millis = moment.timestamp_subsec_millis();
    format!("{}_{:03}", moment.format("%Y_%m_%d_%H_%M_%S"), millis)
}

/// Format detik menjadi `M:SS` atau `H:MM:SS` untuk sidecar timestamp.
pub fn format_clock(seconds: f64) -> String {
    let total = seconds as i64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        return format!("{hours}:{minutes:02}:{secs:02}");
    }
    if minutes == 0 {
        return format!("0:{secs:02}");
    }
    format!("{minutes}:{secs:02}")
}

/// FNV-1a 64-bit, identik dengan kunci cache lama.
pub fn fnv1a64(value: &str) -> u64 {
    let mut hash_value: u64 = 0xCBF2_9CE4_8422_2325;
    let prime: u64 = 0x0000_0100_0000_01B3;
    let mask: u64 = 0x7FFF_FFFF_FFFF_FFFF;
    for ch in value.chars() {
        hash_value ^= ch as u64;
        hash_value = hash_value.wrapping_mul(prime) & mask;
    }
    hash_value
}

fn unsafe_name() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^a-zA-Z0-9_\-]").expect("regex nama aman valid"))
}

/// Bersihkan stem file agar aman dipakai di nama file cache.
pub fn safe_base_name(name: &str, max_length: usize) -> String {
    let cleaned = unsafe_name().replace_all(name, "_").to_string();
    if cleaned.is_empty() {
        // Pertahankan perilaku Python: hasil kosong -> "audio".
        return String::from("audio");
    }
    cleaned.chars().take(max_length).collect()
}

/// Kembalikan `path` bila bebas, atau tambahkan `_1`, `_2`, ... sebelum ekstensi.
pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let suffix = path
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    let mut counter = 1;
    loop {
        let candidate = parent.join(format!("{stem}_{counter}{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes() {
        assert_eq!(format_bytes(512.0), "512 B");
        assert_eq!(format_bytes(2048.0), "2.0 KB");
        assert_eq!(format_bytes(5.0 * 1024.0 * 1024.0), "5.0 MB");
    }

    #[test]
    fn formats_clock() {
        assert_eq!(format_clock(5.0), "0:05");
        assert_eq!(format_clock(75.0), "1:15");
        assert_eq!(format_clock(3675.0), "1:01:15");
    }

    #[test]
    fn hashes_match_legacy() {
        // Nilai acuan dihitung dari implementasi Python `fnv1a64`.
        assert_eq!(fnv1a64(""), 0xCBF2_9CE4_8422_2325);
        assert_eq!(fnv1a64("a"), 0x2F63_DC4C_8601_EC8C);
        assert_eq!(fnv1a64("hello"), 0x2430_D846_80AA_BD0B);
        assert_eq!(fnv1a64("/Users/x/My Song.m4a"), 0x318B_85BF_7E19_6A63);
    }

    #[test]
    fn sanitizes_names() {
        assert_eq!(safe_base_name("my song! (1)", 48), "my_song___1_");
        assert_eq!(safe_base_name("ok-name_1", 48), "ok-name_1");
        assert_eq!(safe_base_name("", 48), "audio");
    }
}
