//! Urutan playlist audio: loop pertama berurutan, loop berikutnya diacak.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

/// Menyusun urutan indeks untuk playlist audio yang diulang.
pub struct PlaylistBuilder {
    rng: StdRng,
}

impl Default for PlaylistBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistBuilder {
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_os_rng(),
        }
    }

    /// Versi deterministik untuk pengujian.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Kembalikan urutan indeks: urutan asli dulu, lalu diacak untuk loop berikutnya.
    pub fn build_order(&mut self, count: usize, loop_count: i64) -> Vec<usize> {
        let original: Vec<usize> = (0..count).collect();
        let mut order = original.clone();
        for _ in 1..std::cmp::max(1, loop_count) {
            let mut shuffled = original.clone();
            shuffled.shuffle(&mut self.rng);
            order.extend(shuffled);
        }
        order
    }

    /// Petakan urutan indeks ke daftar file.
    pub fn apply(files: &[String], order: &[usize]) -> Vec<String> {
        order
            .iter()
            .filter_map(|index| files.get(*index).cloned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_loop_keeps_order() {
        let mut builder = PlaylistBuilder::with_seed(1);
        assert_eq!(builder.build_order(3, 1), vec![0, 1, 2]);
    }

    #[test]
    fn multiple_loops_repeat_count() {
        let mut builder = PlaylistBuilder::with_seed(42);
        let order = builder.build_order(4, 3);
        assert_eq!(order.len(), 12);
        assert_eq!(&order[..4], &[0, 1, 2, 3]);
        // Setiap blok tetap berisi seluruh indeks.
        for chunk in order.chunks(4) {
            let mut sorted = chunk.to_vec();
            sorted.sort_unstable();
            assert_eq!(sorted, vec![0, 1, 2, 3]);
        }
    }

    #[test]
    fn applies_order_to_files() {
        let files = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(
            PlaylistBuilder::apply(&files, &[2, 0]),
            vec!["c".to_string(), "a".to_string()]
        );
    }
}
