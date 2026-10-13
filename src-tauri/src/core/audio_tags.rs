//! Baca durasi audio lokal tanpa spawn ffprobe (jalur cepat).

use lofty::file::AudioFile;
use lofty::probe::Probe;
use std::path::Path;

/// Durasi file audio dalam detik, atau `None` bila tak terbaca.
///
/// Dipakai sebagai jalur cepat sebelum ffprobe: membaca properti file secara
/// lokal. Pemanggil fallback ke ffprobe bila hasilnya `None`.
pub fn read_audio_duration(path: &Path) -> Option<f64> {
    let tagged = Probe::open(path).ok()?.read().ok()?;
    let seconds = tagged.properties().duration().as_secs_f64();
    if seconds <= 0.0 {
        return None;
    }
    Some(seconds)
}
