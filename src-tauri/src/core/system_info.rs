//! Cek kemampuan mesin untuk menentukan ukuran pekerjaan paralel.

/// Jumlah core CPU (minimal 1).
pub fn cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
}

/// Rekomendasi kasar tingkat konkurensi.
pub fn concurrency_recommendation() -> &'static str {
    let cores = cpu_cores();
    if cores <= 2 {
        "light"
    } else if cores <= 6 {
        "balanced"
    } else {
        "heavy"
    }
}
