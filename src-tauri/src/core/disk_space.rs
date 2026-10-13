//! Probe ruang bebas filesystem, dipakai untuk memastikan lokasi output
//! sanggup menampung seluruh rencana yang dipilih.

use std::path::Path;

/// Ruang bebas (byte) pada volume yang menampung `path`.
///
/// `None` berarti ruang tidak bisa ditentukan — path tidak ada, izin ditolak,
/// atau platform tidak didukung. Pemanggil harus memperlakukan ini sebagai
/// "cek tidak tersedia", bukan sebagai kegagalan.
pub fn available_bytes(path: &Path) -> Option<u64> {
    if !path.exists() {
        return None;
    }
    platform_available_bytes(path)
}

#[cfg(unix)]
fn platform_available_bytes(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;

    let raw = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: `statvfs` hanya mengisi struct yang kita berikan, dan `raw`
    // tetap hidup selama pemanggilan.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    let code = unsafe { libc::statvfs(raw.as_ptr(), &mut stats) };
    if code != 0 {
        return None;
    }
    // f_frsize adalah ukuran blok fragmentasi; f_bsize jadi cadangan pada
    // filesystem yang tidak mengisinya.
    let block_size = if stats.f_frsize > 0 {
        stats.f_frsize as u64
    } else {
        stats.f_bsize as u64
    };
    if block_size == 0 {
        return None;
    }
    // f_bavail = blok yang tersedia untuk pengguna biasa (bukan root).
    Some((stats.f_bavail as u64).saturating_mul(block_size))
}

#[cfg(windows)]
fn platform_available_bytes(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            directory: *const u16,
            free_bytes_available: *mut u64,
            total_bytes: *mut u64,
            total_free_bytes: *mut u64,
        ) -> i32;
    }

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let mut available: u64 = 0;
    // SAFETY: `wide` selalu NUL-terminated dan hidup selama panggilan; dua
    // parameter keluaran yang tidak dipakai memang boleh NULL.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        None
    } else {
        Some(available)
    }
}

#[cfg(not(any(unix, windows)))]
fn platform_available_bytes(_path: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_free_space_for_an_existing_path() {
        let bytes = available_bytes(Path::new(env!("CARGO_MANIFEST_DIR")));
        assert!(
            bytes.map(|value| value > 0).unwrap_or(false),
            "ruang bebas pada direktori proyek harus terbaca"
        );
    }

    #[test]
    fn returns_none_for_a_missing_path() {
        assert_eq!(available_bytes(Path::new("/berkas/tidak/ada/xyz")), None);
    }

    #[test]
    fn reads_free_space_for_a_relative_path() {
        let bytes = available_bytes(Path::new("."));
        assert!(bytes.is_some(), "path relatif yang ada tetap terbaca");
    }
}
