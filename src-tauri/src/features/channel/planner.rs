//! Perencanaan episode channel: port langsung dari `sw-tauri/algo.py`.
//!
//! Urutan langkahnya sengaja sama dengan versi Python supaya mudah dibandingkan:
//! antrean lagu pembuka, kolam perpustakaan x target putar, toleransi tiga lagu
//! paling jarang, bonus 0-3 lagu, validasi keunikan durasi + susunan, lalu
//! pengacakan anti-berurutan yang mempertahankan lagu pertama di indeks 0.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use rand::rngs::StdRng;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{Rng, SeedableRng};

use crate::core::ffmpeg::metadata::MetadataReader;
use crate::core::ffmpeg::runner::{FFmpegRunner, LogCallback};
use crate::core::logging::{LogLevel, LogRecord};
use crate::error::AppError;
use crate::features::channel::detector::ChannelDetector;
use crate::features::channel::models::{
    Channel, ChannelAudioItem, ChannelPlan, ChannelPlanStatus, ChannelStatus,
};

/// Batas percobaan keseluruhan, dikalikan target (sama seperti prototipe).
const ATTEMPTS_PER_TARGET: usize = 500;
/// Batas suntikan toleransi saat kolam tidak sampai durasi minimum.
const TOLERANCE_PICKS: usize = 50;
/// Jumlah lagu bonus maksimum yang disuntikkan ke satu rencana.
const MAX_BONUS_SONGS: usize = 3;
/// Berapa lagu paling jarang yang dijadikan kandidat toleransi.
const TOLERANCE_CANDIDATES: usize = 3;

/// Satu playlist hasil perencanaan sebelum dipasangkan video latar.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaylistDraft {
    /// Id lagu dalam urutan main.
    pub audio_ids: Vec<String>,
    pub total_duration_seconds: f64,
    /// Pemakaian terbanyak dan tersedikit di dalam rencana ini.
    pub max_usage: i64,
    pub min_usage: i64,
}

/// Perencana rencana episode channel.
pub struct ChannelPlanner {
    detector: ChannelDetector,
    on_log: Option<LogCallback>,
}

impl ChannelPlanner {
    pub fn new(
        metadata: Arc<MetadataReader>,
        runner: Arc<FFmpegRunner>,
        on_log: Option<LogCallback>,
    ) -> Self {
        Self {
            detector: ChannelDetector::new(metadata, runner, on_log.clone()),
            on_log,
        }
    }

    /// Deteksi ulang aset channel lalu susun rencananya.
    pub async fn plan(&self, channel: &Channel) -> Result<Channel, AppError> {
        let detected = self.detector.detect(channel).await?;
        let target = detected.plan_target();
        let plans = Self::build_plans(&detected);

        if plans.is_empty() {
            return Err(AppError::new(
                "Tidak ada plan yang bisa dibuat dari lagu yang terdeteksi — tambah lagu atau turunkan durasi minimum",
            ));
        }
        if plans.len() < target {
            self.emit(
                format!(
                    "Hanya {} dari {} plan bisa dibuat dari {} lagu — sisanya menunggu variasi lagu tambahan",
                    plans.len(),
                    target,
                    detected.audio_items.len()
                ),
                LogLevel::Warning,
                1,
            );
        }
        let waiting = detected.waiting_for_video();
        if waiting > 0 {
            self.emit(
                format!(
                    "{waiting} plan menunggu video — tambahkan video ke folder lalu periksa ulang, atau pasang satu per satu"
                ),
                LogLevel::Warning,
                1,
            );
        }
        self.emit(
            format!(
                "{} plan siap · acuan awal loop {}x · rata-rata {:.0} menit · total {:.1} jam",
                plans.len(),
                detected.loop_cap(),
                plans
                    .iter()
                    .map(|plan| plan.total_duration_seconds)
                    .sum::<f64>()
                    / plans.len() as f64
                    / 60.0,
                plans
                    .iter()
                    .map(|plan| plan.total_duration_seconds)
                    .sum::<f64>()
                    / 3600.0
            ),
            LogLevel::Info,
            0,
        );

        let mut updated = detected;
        updated.plans = plans;
        updated.status = ChannelStatus::Ready;
        updated.touch();
        Ok(updated)
    }

    /// Susun rencana lengkap (durasi terpendek lebih dulu, video latar dipasangkan).
    pub fn build_plans(channel: &Channel) -> Vec<ChannelPlan> {
        let mut drafts = Self::generate_playlists(
            &channel.audio_items,
            channel.plan_target(),
            channel.min_duration_seconds(),
            channel.loop_cap(),
            channel.seed,
        );
        drafts.sort_by(|left, right| {
            left.total_duration_seconds
                .partial_cmp(&right.total_duration_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        drafts
            .into_iter()
            .enumerate()
            .map(|(position, draft)| {
                let background = background_for(&channel.video_items, position);
                let runnable = !background.is_empty();
                ChannelPlan {
                    id: format!("plan_{}", position + 1),
                    index: position as i64,
                    background_video_path: background,
                    audio_item_ids: draft.audio_ids,
                    total_duration_seconds: draft.total_duration_seconds,
                    max_usage: draft.max_usage,
                    min_usage: draft.min_usage,
                    // Rencana yang belum kebagian video belum bisa diproses, jadi
                    // tidak ikut terpilih dulu.
                    selected: runnable,
                    status: ChannelPlanStatus::NotProcessed,
                    intro_video_path: None,
                    output_path: None,
                    error: None,
                    processed_at: None,
                    finished_at: None,
                }
            })
            .collect()
    }

    /// Algoritma yang sama dengan `generate_unique_playlists` di `algo.py`.
    pub fn generate_playlists(
        items: &[ChannelAudioItem],
        target: usize,
        min_duration_seconds: f64,
        loop_cap: i64,
        seed: i64,
    ) -> Vec<PlaylistDraft> {
        if items.is_empty() || target == 0 {
            return Vec::new();
        }
        let cap = loop_cap.max(1);
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let mut drafts: Vec<PlaylistDraft> = Vec::new();
        let mut attempts = 0usize;
        let max_attempts = target * ATTEMPTS_PER_TARGET;

        let mut seen_durations: BTreeSet<i64> = BTreeSet::new();
        let mut seen_combinations: BTreeSet<Vec<&str>> = BTreeSet::new();

        // Antrean lagu pembuka: tiap rencana mengambil satu, dan antreannya
        // diisi ulang bila lagunya lebih sedikit daripada target output.
        let mut opening_pool: Vec<&ChannelAudioItem> = items.iter().collect();
        opening_pool.shuffle(&mut rng);

        while drafts.len() < target && attempts < max_attempts {
            attempts += 1;

            // Pemakaian lagu untuk rencana yang sedang disusun ini.
            let mut counter: BTreeMap<&str, i64> =
                items.iter().map(|item| (item.path.as_str(), 0)).collect();
            let mut playlist: Vec<&ChannelAudioItem> = Vec::new();
            let mut total = 0.0;

            // 1. Lagu pertama diambil dari antrean pembuka supaya tiap episode
            //    dibuka lagu yang berbeda.
            if opening_pool.is_empty() {
                opening_pool = items.iter().collect();
                opening_pool.shuffle(&mut rng);
            }
            if let Some(opener) = opening_pool.pop() {
                playlist.push(opener);
                total += opener.effective_duration();
                *counter.entry(opener.path.as_str()).or_insert(0) += 1;
            }

            // 2. Kolam berisi perpustakaan sebanyak target putar, diacak, lalu
            //    diambil sampai durasi minimum terpenuhi.
            let mut pool: Vec<&ChannelAudioItem> =
                Vec::with_capacity(items.len().saturating_mul(cap as usize));
            for _ in 0..cap {
                pool.extend(items.iter());
            }
            pool.shuffle(&mut rng);
            while !pool.is_empty() && total < min_duration_seconds {
                let item = pool.pop().expect("kolam tidak kosong");
                playlist.push(item);
                total += item.effective_duration();
                *counter.entry(item.path.as_str()).or_insert(0) += 1;
            }

            // 3. Toleransi: kolam sudah habis tapi durasi belum cukup, jadi
            //    lagu diambil acak dari tiga yang paling jarang dipakai.
            if total < min_duration_seconds {
                let mut tries = 0usize;
                while total < min_duration_seconds && tries < TOLERANCE_PICKS {
                    let candidates = least_used_top(items, &counter, TOLERANCE_CANDIDATES);
                    let Some(item) = candidates.choose(&mut rng).copied() else {
                        break;
                    };
                    playlist.push(item);
                    total += item.effective_duration();
                    *counter.entry(item.path.as_str()).or_insert(0) += 1;
                    tries += 1;
                }
            }

            // 4. Suntikan bonus 0-3 lagu, tetap dari yang paling jarang dipakai.
            let bonus = rng.random_range(0..=MAX_BONUS_SONGS);
            if bonus > 0 {
                for item in least_used_top(items, &counter, bonus) {
                    playlist.push(item);
                    total += item.effective_duration();
                    *counter.entry(item.path.as_str()).or_insert(0) += 1;
                }
            }

            // 5. Validasi keunikan durasi dan susunan lagu.
            if total < min_duration_seconds {
                continue;
            }
            let duration_key = (total * 10.0).round() as i64;
            let mut combination: Vec<&str> =
                playlist.iter().map(|item| item.path.as_str()).collect();
            combination.sort_unstable();
            if seen_durations.contains(&duration_key) || seen_combinations.contains(&combination) {
                continue;
            }
            seen_durations.insert(duration_key);
            seen_combinations.insert(combination);

            // 6. Acak anti-berurutan: lagu pertama tetap di indeks 0.
            let ordered = shuffle_anti_adjacent(playlist, &mut rng);
            let (max_usage, min_usage) = counter_span(&counter);
            drafts.push(PlaylistDraft {
                audio_ids: ordered.iter().map(|item| item.id.clone()).collect(),
                total_duration_seconds: total,
                max_usage,
                min_usage,
            });
        }

        drafts.sort_by(|left, right| {
            left.total_duration_seconds
                .partial_cmp(&right.total_duration_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        drafts
    }

    /// Beri video kepada rencana yang belum kebagian, memakai video yang belum
    /// dipakai rencana lain (urut folder). Dipakai saat pengguna menambah video
    /// ke folder lalu memeriksanya ulang.
    pub fn assign_waiting_videos(channel: &Channel) -> Channel {
        let mut used: BTreeSet<String> = channel
            .plans
            .iter()
            .map(|plan| plan.background_video_path.clone())
            .filter(|path| !path.is_empty())
            .collect();
        let mut updated = channel.clone();
        for plan in updated.plans.iter_mut() {
            if !plan.background_video_path.is_empty() {
                continue;
            }
            let Some(free) = channel
                .video_items
                .iter()
                .find(|path| !used.contains(path.as_str()))
            else {
                break;
            };
            plan.background_video_path = free.clone();
            // Rencana yang sudah punya video siap diproses.
            plan.selected = true;
            used.insert(free.clone());
        }
        updated.touch();
        updated
    }

    fn emit(&self, message: impl Into<String>, level: LogLevel, depth: usize) {
        if let Some(callback) = &self.on_log {
            callback(LogRecord::new(message, level, depth));
        }
    }
}

/// Video latar untuk rencana ke-`position`. Satu video per rencana dan tanpa
/// pengulangan: jumlah rencana sudah dibatasi jumlah video yang tersedia.
fn background_for(videos: &[String], position: usize) -> String {
    videos.get(position).cloned().unwrap_or_default()
}

/// `sorted(mp3_list, key=usage)[:count]` — urutan perpustakaan dipertahankan
/// bila pemakaiannya sama, seperti `sorted()` Python yang stabil.
fn least_used_top<'a>(
    items: &'a [ChannelAudioItem],
    counter: &BTreeMap<&str, i64>,
    count: usize,
) -> Vec<&'a ChannelAudioItem> {
    let mut ranked: Vec<&ChannelAudioItem> = items.iter().collect();
    ranked.sort_by_key(|item| counter.get(item.path.as_str()).copied().unwrap_or(0));
    ranked.truncate(count);
    ranked
}

/// `max(counter.values())` dan `min(counter.values())`: dihitung atas seluruh
/// lagu di perpustakaan, jadi lagu yang tidak terpakai membuat nilai
/// terkecilnya nol — persis seperti tampilan "Sebaran Loop" di prototipe.
fn counter_span(counter: &BTreeMap<&str, i64>) -> (i64, i64) {
    let values: Vec<i64> = counter.values().copied().collect();
    (
        values.iter().copied().max().unwrap_or(0),
        values.iter().copied().min().unwrap_or(0),
    )
}

/// `shuffle_anti_adjacent` dari prototipe: lagu pertama di indeks 0 dibiarkan,
/// sisanya diacak, lalu diatur agar berkas yang sama tidak berurutan.
///
/// Sifatnya best-effort seperti versi Python: aturan "ambil yang pertama
/// berbeda" bisa menyisakan pasangan kembar walau susunan bebas-kembar masih
/// mungkin. Itu dipertahankan apa adanya supaya portnya setia, dan rencana yang
/// dihasilkan nyaris selalu bebas-kembar karena lagunya banyak.
fn shuffle_anti_adjacent<'a>(
    playlist: Vec<&'a ChannelAudioItem>,
    rng: &mut StdRng,
) -> Vec<&'a ChannelAudioItem> {
    if playlist.is_empty() {
        return playlist;
    }
    let mut items = playlist;
    let first = items.remove(0);
    let mut pool = items;
    pool.shuffle(rng);

    let mut ordered: Vec<&ChannelAudioItem> = vec![first];
    while !pool.is_empty() {
        let last = ordered.last().map(|item| item.path.as_str());
        match pool
            .iter()
            .position(|item| Some(item.path.as_str()) != last)
        {
            Some(index) => ordered.push(pool.remove(index)),
            None => {
                // Sisa semuanya sama dengan yang terakhir: tidak ada pilihan lain.
                ordered.append(&mut pool);
                break;
            }
        }
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kumpulan lagu seperti contoh di `algo.py`: 16 lagu, durasi bervariasi.
    fn varied_songs() -> Vec<ChannelAudioItem> {
        let durations = [
            245.5, 310.2, 198.0, 285.4, 340.1, 215.3, 260.7, 305.0, 290.2, 275.5, 230.1, 315.4,
            240.8, 295.6, 350.0, 210.2,
        ];
        durations
            .iter()
            .enumerate()
            .map(|(index, duration)| ChannelAudioItem {
                id: format!("audio_{index}"),
                path: format!("/musik/lagu_{:02}.mp3", index + 1),
                duration_seconds: Some(*duration),
            })
            .collect()
    }

    fn songs(count: usize, duration: f64) -> Vec<ChannelAudioItem> {
        (0..count)
            .map(|index| ChannelAudioItem {
                id: format!("audio_{index}"),
                path: format!("/musik/equ_{:02}.mp3", index + 1),
                duration_seconds: Some(duration),
            })
            .collect()
    }

    /// Beberapa video supaya target rencana lebih dari satu.
    fn videos(count: usize) -> Vec<String> {
        (1..=count)
            .map(|index| format!("/video/bg_{index:02}.mp4"))
            .collect()
    }

    fn channel_with(items: Vec<ChannelAudioItem>, videos: Vec<String>) -> Channel {
        let mut channel = Channel::create(
            String::from("ch_1"),
            String::from("Lounge"),
            String::from("/p"),
            String::from("/p/outputs"),
            7,
        );
        channel.audio_items = items;
        channel.video_items = videos;
        channel
    }

    #[test]
    fn every_plan_meets_the_minimum_duration() {
        let drafts = ChannelPlanner::generate_playlists(&varied_songs(), 10, 120.0 * 60.0, 2, 7);
        assert!(!drafts.is_empty());
        for draft in &drafts {
            assert!(
                draft.total_duration_seconds >= 120.0 * 60.0,
                "durasi {} di bawah minimum",
                draft.total_duration_seconds
            );
        }
    }

    #[test]
    fn plans_keep_unique_duration_and_combination() {
        let drafts = ChannelPlanner::generate_playlists(&varied_songs(), 20, 120.0 * 60.0, 2, 7);
        assert!(
            drafts.len() >= 5,
            "harus ada beberapa rencana, dapat {}",
            drafts.len()
        );

        let mut durations: BTreeSet<i64> = BTreeSet::new();
        let mut combinations: BTreeSet<Vec<String>> = BTreeSet::new();
        for draft in &drafts {
            let key = (draft.total_duration_seconds * 10.0).round() as i64;
            assert!(durations.insert(key), "durasi {key} terpakai dua kali");
            let mut combination = draft.audio_ids.clone();
            combination.sort();
            assert!(
                combinations.insert(combination),
                "susunan lagu terpakai dua kali"
            );
        }
    }

    #[test]
    fn plan_set_is_deterministic_for_the_same_seed() {
        let first = ChannelPlanner::generate_playlists(&varied_songs(), 12, 120.0 * 60.0, 2, 99);
        let second = ChannelPlanner::generate_playlists(&varied_songs(), 12, 120.0 * 60.0, 2, 99);
        assert_eq!(first, second);

        let other = ChannelPlanner::generate_playlists(&varied_songs(), 12, 120.0 * 60.0, 2, 100);
        assert_ne!(first, other, "seed berbeda menghasilkan susunan berbeda");
    }

    #[test]
    fn plans_open_with_different_songs() {
        let drafts = ChannelPlanner::generate_playlists(&varied_songs(), 8, 60.0 * 60.0, 2, 4);
        assert_eq!(drafts.len(), 8, "delapan rencana dibuat");
        let openers = drafts
            .iter()
            .map(|draft| draft.audio_ids.first().cloned().unwrap_or_default())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            openers.len(),
            8,
            "lagu pembuka tiap rencana diambil dari antrean unik"
        );
    }

    #[test]
    fn opening_queue_is_refilled_when_the_library_is_smaller() {
        // Tiga lagu, enam rencana: antrean pembuka pasti diisi ulang, jadi ada
        // rencana yang dibuka lagu sama. Keunikan durasi tetap membatasi
        // jumlah rencananya, seperti prototipe.
        let drafts = ChannelPlanner::generate_playlists(&songs(3, 900.0), 6, 30.0 * 60.0, 2, 6);
        assert!(!drafts.is_empty());
        assert!(
            drafts.len() < 6,
            "perpustakaan tiga lagu tidak bisa memenuhi enam rencana unik, dapat {}",
            drafts.len()
        );
        let known: BTreeSet<String> = songs(3, 900.0).iter().map(|item| item.id.clone()).collect();
        for draft in &drafts {
            let opener = draft.audio_ids.first().cloned().unwrap_or_default();
            assert!(
                known.contains(&opener),
                "pembuka tetap lagu dari perpustakaan"
            );
        }
    }

    #[test]
    fn fills_from_the_pool_until_the_minimum_is_met() {
        // 16 lagu ~4,4 menit: satu putaran kolam ±70 menit, dan dengan target
        // putar 2x kolamnya ±140 menit — cukup untuk minimum 120 menit.
        let library = varied_songs();
        let drafts = ChannelPlanner::generate_playlists(&library, 1, 120.0 * 60.0, 2, 7);
        let draft = &drafts[0];
        assert!(
            draft.total_duration_seconds >= 120.0 * 60.0,
            "durasi {} belum mencapai minimum",
            draft.total_duration_seconds
        );
        assert!(
            draft.audio_ids.len() <= library.len() * 2 + MAX_BONUS_SONGS,
            "tidak pernah melebihi isi kolam, dapat {}",
            draft.audio_ids.len()
        );
    }

    #[test]
    fn tolerance_tops_up_when_the_pool_is_too_short() {
        // Target putar 1x hanya menyediakan ±70 menit sedangkan minimum 120
        // menit: sisanya harus datang dari suntikan toleransi.
        let library = varied_songs();
        let drafts = ChannelPlanner::generate_playlists(&library, 1, 120.0 * 60.0, 1, 7);
        let draft = &drafts[0];
        assert!(draft.total_duration_seconds >= 120.0 * 60.0);
        assert!(
            draft.audio_ids.len() > library.len(),
            "perlu lebih banyak lagu daripada satu putaran kolam, dapat {}",
            draft.audio_ids.len()
        );
    }

    #[test]
    fn a_tiny_pool_yields_fewer_plans() {
        // Dua lagu berdurasi sama: durasi totalnya selalu kelipatan yang sama,
        // jadi keunikan durasi membatasi jumlah rencananya.
        let drafts = ChannelPlanner::generate_playlists(&songs(2, 600.0), 6, 30.0 * 60.0, 2, 3);
        assert!(drafts.len() < 6, "tidak mungkin enam rencana unik");
        assert!(!drafts.is_empty(), "tetap hasilkan sebanyak mungkin");
    }

    #[test]
    fn no_file_plays_twice_in_a_row() {
        let drafts = ChannelPlanner::generate_playlists(&varied_songs(), 8, 60.0 * 60.0, 2, 3);
        for draft in &drafts {
            for pair in draft.audio_ids.windows(2) {
                assert_ne!(
                    pair[0], pair[1],
                    "berkas sama berurutan: {:?}",
                    draft.audio_ids
                );
            }
        }
    }

    #[test]
    fn anti_adjacent_keeps_the_first_song_at_index_zero() {
        // Empat lagu, dua salinan masing-masing: lagu pertama harus tetap di
        // indeks 0 dan tidak ada lagu yang hilang, apa pun pengacakannya.
        let items = songs(4, 60.0);
        let playlist: Vec<&ChannelAudioItem> = vec![
            &items[0], &items[1], &items[2], &items[3], &items[0], &items[1], &items[2], &items[3],
        ];

        for seed in 0..8 {
            let mut rng = StdRng::seed_from_u64(seed);
            let ordered = shuffle_anti_adjacent(playlist.clone(), &mut rng);
            assert_eq!(ordered.len(), 8, "tidak ada lagu yang hilang");
            assert_eq!(
                ordered[0].path, items[0].path,
                "lagu pertama harus tetap di indeks 0"
            );
            for item in &items {
                let before = playlist.iter().filter(|x| x.path == item.path).count();
                let after = ordered.iter().filter(|x| x.path == item.path).count();
                assert_eq!(before, after, "jumlah pemakaian tidak berubah");
            }
        }
    }

    #[test]
    fn anti_adjacent_keeps_every_item_when_it_is_impossible() {
        // Empat lagu sama dengan hanya dua pemisah: pasangan kembar tidak
        // terhindarkan, tapi tidak boleh ada lagu yang hilang.
        let items = songs(3, 60.0);
        let mut playlist: Vec<&ChannelAudioItem> = vec![&items[1], &items[0], &items[0]];
        playlist.push(&items[0]);
        playlist.push(&items[0]);
        playlist.push(&items[2]);

        let mut rng = StdRng::seed_from_u64(1);
        let ordered = shuffle_anti_adjacent(playlist, &mut rng);
        assert_eq!(ordered.len(), 6);
        assert_eq!(ordered[0].path, items[1].path);
        assert_eq!(
            ordered
                .iter()
                .filter(|item| item.path == items[0].path)
                .count(),
            4
        );
    }

    #[test]
    fn usage_span_covers_the_whole_library() {
        // "Sebaran Loop" di prototipe menghitung seluruh perpustakaan, jadi lagu
        // yang tidak terpakai membuat nilai terkecilnya nol.
        let drafts = ChannelPlanner::generate_playlists(&varied_songs(), 3, 60.0 * 60.0, 2, 5);
        for draft in &drafts {
            assert!(draft.max_usage >= 1, "ada lagu yang terpakai");
            assert_eq!(draft.min_usage, 0);
        }
    }

    #[test]
    fn plans_are_ordered_shortest_first() {
        let plans = ChannelPlanner::build_plans(&channel_with(varied_songs(), videos(6)));
        assert!(plans.len() >= 5, "dapat {} rencana", plans.len());
        for (position, plan) in plans.iter().enumerate() {
            assert_eq!(plan.id, format!("plan_{}", position + 1));
            assert_eq!(plan.index, position as i64);
        }
        for pair in plans.windows(2) {
            assert!(
                pair[0].total_duration_seconds <= pair[1].total_duration_seconds,
                "rencana harus urut dari yang terpendek"
            );
        }
    }

    #[test]
    fn backgrounds_pair_one_per_plan_and_leave_the_rest_waiting() {
        let mut channel = channel_with(varied_songs(), videos(3));
        channel.total_content = 10;
        let plans = ChannelPlanner::build_plans(&channel);

        assert_eq!(plans.len(), 10, "permintaan 10 konten dihormati");
        assert_eq!(plans[0].background_video_path, "/video/bg_01.mp4");
        assert_eq!(plans[1].background_video_path, "/video/bg_02.mp4");
        assert_eq!(plans[2].background_video_path, "/video/bg_03.mp4");

        for plan in &plans[..3] {
            assert!(plan.selected, "yang sudah punya video langsung terpilih");
        }
        for plan in &plans[3..] {
            assert!(
                plan.background_video_path.is_empty(),
                "belum kebagian video"
            );
            assert!(!plan.selected, "yang menunggu video belum terpilih");
        }
    }

    #[test]
    fn rescan_assigns_free_videos_to_waiting_plans() {
        let mut channel = channel_with(varied_songs(), vec![String::from("/video/a.mp4")]);
        channel.total_content = 3;
        channel.plans = ChannelPlanner::build_plans(&channel);
        assert_eq!(channel.plans[0].background_video_path, "/video/a.mp4");
        assert!(channel.plans[1].background_video_path.is_empty());

        channel.video_items = vec![
            String::from("/video/a.mp4"),
            String::from("/video/c.mp4"),
            String::from("/video/d.mp4"),
        ];
        let assigned = ChannelPlanner::assign_waiting_videos(&channel);

        assert_eq!(assigned.plans[0].background_video_path, "/video/a.mp4");
        assert_eq!(assigned.plans[1].background_video_path, "/video/c.mp4");
        assert_eq!(assigned.plans[2].background_video_path, "/video/d.mp4");
        assert!(assigned.plans.iter().all(|plan| plan.selected));
        assert_eq!(assigned.waiting_for_video(), 0);
    }

    #[test]
    fn fresh_plans_are_not_processed() {
        let plans = ChannelPlanner::build_plans(&channel_with(varied_songs(), videos(1)));
        assert!(!plans.is_empty());
        for plan in &plans {
            assert_eq!(plan.status, ChannelPlanStatus::NotProcessed);
            assert!(plan.output_path.is_none());
            assert!(plan.processed_at.is_none());
            assert!(plan.selected);
        }
    }

    #[test]
    fn empty_input_or_target_yields_nothing() {
        assert!(ChannelPlanner::generate_playlists(&[], 5, 60.0, 2, 1).is_empty());
        assert!(ChannelPlanner::generate_playlists(&varied_songs(), 0, 60.0, 2, 1).is_empty());
    }
}
