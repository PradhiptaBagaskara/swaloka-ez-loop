import random

# ==========================================
# 1. INPUT DATA MENGGUNAKAN PYTHON DICTIONARY
# ==========================================
data_input = {
    "parameters": {
        "min_duration_minutes": 120,
        "recommended_loop": 2,
        "target_outputs": 30,
    },
    "mp3_files": [
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge 10.mp3",
            "duration": 259.632,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge 7.mp3",
            "duration": 304.512,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-3.mp3",
            "duration": 369.0,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-2.mp3",
            "duration": 294.48,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-1.mp3",
            "duration": 177.504,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge-v3-4.mp3",
            "duration": 257.352,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-5.mp3",
            "duration": 169.752,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-4.mp3",
            "duration": 384.96,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge-v3-7.mp3",
            "duration": 308.664,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-6.mp3",
            "duration": 304.92,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-7.mp3",
            "duration": 309.984,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge-v3-4.m4a",
            "duration": 257.34321995464853,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-5.m4a",
            "duration": 169.74321995464854,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-4.m4a",
            "duration": 384.9432199546485,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge-v3-7.m4a",
            "duration": 308.6632199546485,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-6.m4a",
            "duration": 304.90321995464853,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-7.m4a",
            "duration": 309.9832199546485,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-3.m4a",
            "duration": 368.9832199546485,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-2.m4a",
            "duration": 294.46321995464854,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-1.m4a",
            "duration": 177.50321995464853,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge 7.m4a",
            "duration": 304.5032199546485,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge 10.m4a",
            "duration": 259.6232199546485,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge 7.aac",
            "duration": 306.52614796162794,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge-v3-7.aac",
            "duration": 317.4564190343231,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-6.aac",
            "duration": 315.54292719132644,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-7.aac",
            "duration": 315.43629883524494,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge-v3-4.aac",
            "duration": 253.25854330170398,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-5.aac",
            "duration": 173.41959462553285,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-4.aac",
            "duration": 387.8286669419551,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-1.aac",
            "duration": 184.04796826361647,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-3.aac",
            "duration": 380.6649560472948,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/luxury-lounge-2.aac",
            "duration": 317.28851500125194,
        },
        {
            "fname": "/Users/bagas/Desktop/calm-sky-lounge/test/Lounge 10.aac",
            "duration": 266.55509630988496,
        },
    ],
}


# ==========================================
# 2. FUNGSI PENGACAK ANTI-TUMPANG-TINDIH
# ==========================================
def shuffle_anti_adjacent(playlist_lagu):
    """Mengacak urutan lagu dengan mempertahankan elemen indeks 0 (lagu pertama)"""
    if not playlist_lagu:
        return []

    # Kunci lagu pertama di indeks 0 agar tidak bergeser posisinya
    lagu_pertama = playlist_lagu[0]
    pool = playlist_lagu[1:].copy()
    random.shuffle(pool)

    shuffled_result = [lagu_pertama]

    while pool:
        idx_terpilih = -1
        for i, lagu in enumerate(pool):
            if lagu["fname"] != shuffled_result[-1]["fname"]:
                idx_terpilih = i
                break

        if idx_terpilih == -1:
            shuffled_result.extend(pool)
            break
        else:
            shuffled_result.append(pool.pop(idx_terpilih))

    return shuffled_result


# ==========================================
# 3. LOGIKA ALGORITMA DENGAN RECOMMENDED LOOP
# ==========================================
def generate_unique_playlists(mp3_list, params):
    target_outputs = params.get("target_outputs", 30)
    min_duration_minutes = params.get("min_duration_minutes", 120)
    recommended_loop = params.get("recommended_loop", 2)

    target_duration_seconds = min_duration_minutes * 60
    all_playlists = []
    attempts = 0
    max_attempts = target_outputs * 500

    seen_durations = set()
    seen_combinations = set()

    # Kunci Fitur Baru: Membuat antrean acak untuk kandidat lagu pertama (pembuka)
    opening_pool = mp3_list.copy()
    random.shuffle(opening_pool)

    print("=== Menjalankan Algoritma Terpilih + Pembuka Unik ===")
    print(f"🎯 Target Output    : {target_outputs} playlist")
    print(
        f"⏱️ Min Durasi       : {min_duration_minutes} menit ({target_duration_seconds} detik)"
    )
    print(f"🔄 Acuan Awal Loop  : {recommended_loop}x\n")

    while len(all_playlists) < target_outputs and attempts < max_attempts:
        attempts += 1

        # Reset tracker pemakaian lagu untuk iterasi playlist saat ini
        song_usage_counter = {lagu["fname"]: 0 for lagu in mp3_list}
        current_playlist = []
        current_duration = 0.0

        # --- LOGIKA BARU: MENENTUKAN LAGU PERTAMA SEBISA MUNGKIN UNIK ---
        # Jika antrean pembuka habis (karena lagu < target output), isi kembali antrean
        if not opening_pool:
            opening_pool = mp3_list.copy()
            random.shuffle(opening_pool)

        lagu_pembuka = opening_pool.pop()
        current_playlist.append(lagu_pembuka)
        current_duration += lagu_pembuka["duration"]
        song_usage_counter[lagu_pembuka["fname"]] += 1
        # -----------------------------------------------------------------

        # Pengisian lanjutan dari pool awal kelipatan n loop
        pool_lagu = mp3_list * recommended_loop
        random.shuffle(pool_lagu)

        # Mengambil lagu sisa dari antrean acak hingga durasi terpenuhi
        while pool_lagu and current_duration < target_duration_seconds:
            lagu = pool_lagu.pop()
            current_playlist.append(lagu)
            current_duration += lagu["duration"]
            song_usage_counter[lagu["fname"]] += 1

        # Kondisi Toleransi
        if current_duration < target_duration_seconds:
            sisa_percobaan = 0
            while current_duration < target_duration_seconds and sisa_percobaan < 50:
                mp3_list_sorted = sorted(
                    mp3_list, key=lambda x: song_usage_counter[x["fname"]]
                )
                kandidat_toleransi = mp3_list_sorted[:3]
                lagu = random.choice(kandidat_toleransi)
                current_playlist.append(lagu)
                current_duration += lagu["duration"]
                song_usage_counter[lagu["fname"]] += 1
                sisa_percobaan += 1

        # Suntikan Lagu Bonus (0 sampai 3 lagu)
        mp3_list_sorted_for_bonus = sorted(
            mp3_list, key=lambda x: song_usage_counter[x["fname"]]
        )
        bonus_target_count = random.choice([0, 1, 2, 3])

        if bonus_target_count > 0:
            bonus_terpilih = mp3_list_sorted_for_bonus[:bonus_target_count]
            for lagu_bonus in bonus_terpilih:
                current_playlist.append(lagu_bonus)
                current_duration += lagu_bonus["duration"]
                song_usage_counter[lagu_bonus["fname"]] += 1

        # Validasi Akhir Keunikan Durasi & Struktur Isi
        if current_duration >= target_duration_seconds:
            duration_key = round(current_duration, 1)
            combination_key = tuple(
                sorted([lagu["fname"] for lagu in current_playlist])
            )

            if (
                duration_key not in seen_durations
                and combination_key not in seen_combinations
            ):
                seen_durations.add(duration_key)
                seen_combinations.add(combination_key)

                # Mengacak formasi urutan lagu tengah ke belakang tanpa menggeser lagu pertama
                ordered_playlist = shuffle_anti_adjacent(current_playlist)

                all_playlists.append(
                    {
                        "playlist": ordered_playlist,
                        "total_duration": current_duration,
                        "counter": song_usage_counter,
                    }
                )

    print(f"📊 Selesai dalam {attempts} iterasi.")
    print(
        f"✨ Berhasil mendapatkan {len(all_playlists)} dari {target_outputs} target playlist.\n"
    )

    all_playlists.sort(key=lambda x: x["total_duration"])
    return all_playlists


# ==========================================
# 4. PRINT OUT HASILNYA
# ==========================================
def print_result(mp3_files, parameters):
    hasil_playlist = generate_unique_playlists(mp3_files, parameters)

    for idx, p in enumerate(hasil_playlist, 1):
        menit = int(p["total_duration"] // 60)
        detik = int(p["total_duration"] % 60)
        daftar_nama = [lagu["fname"] for lagu in p["playlist"]]

        max_loop_count = max(p["counter"].values())
        min_loop_count = min(p["counter"].values())

        print(f"▶️ Output #{idx} ({len(daftar_nama)} file)")
        print(f"   🔝 Lagu Pertama   : ⭐ {daftar_nama[0]} ⭐")
        print(
            f"   ⏱️ Total Durasi   : {menit} menit {detik} detik ({p['total_duration']:.1f} detik)"
        )
        print(
            f"   ⚖️ Sebaran Loop   : Paling sering {max_loop_count}x, paling jarang {min_loop_count}x"
        )
        print(f"   🎵 Urutan Lagu    : {daftar_nama}\n")


if __name__ == "__main__":
    print_result(data_input["mp3_files"], data_input["parameters"])
