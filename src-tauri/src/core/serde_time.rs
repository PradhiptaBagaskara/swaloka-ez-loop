//! Helper serde untuk waktu lokal: ditulis sebagai ISO-8601 tanpa offset,
//! sama seperti format JSON yang sudah dipakai aplikasi lama.

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Deserializer, Serializer};

/// Serialisasi `DateTime<Local>` sebagai ISO-8601 tanpa offset.
pub mod local_iso {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &DateTime<Local>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.format("%Y-%m-%dT%H:%M:%S%.6f").to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DateTime<Local>, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(parse_time(&raw).unwrap_or_else(Local::now))
    }
}

/// Serialisasi waktu lokal opsional.
pub mod opt_local_iso {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &Option<DateTime<Local>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(moment) => {
                serializer.serialize_str(&moment.format("%Y-%m-%dT%H:%M:%S%.6f").to_string())
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<DateTime<Local>>, D::Error> {
        let raw = Option::<String>::deserialize(deserializer)?;
        Ok(raw.as_deref().and_then(parse_time))
    }
}

/// Baca waktu ISO-8601 dengan atau tanpa offset.
pub fn parse_time(text: &str) -> Option<DateTime<Local>> {
    if text.is_empty() {
        return None;
    }
    if let Ok(value) = DateTime::parse_from_rfc3339(text) {
        return Some(value.with_timezone(&Local));
    }
    let trimmed = text.trim_end_matches('Z');
    for format in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, format) {
            return Some(
                Local
                    .from_local_datetime(&naive)
                    .earliest()
                    .unwrap_or_else(Local::now),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iso_with_and_without_offset() {
        assert!(parse_time("2026-05-03T10:00:00.000000").is_some());
        assert!(parse_time("2026-05-03T10:00:00Z").is_some());
        assert!(parse_time("2026-05-03T10:00:00+07:00").is_some());
        assert!(parse_time("").is_none());
        assert!(parse_time("bukan waktu").is_none());
    }
}
