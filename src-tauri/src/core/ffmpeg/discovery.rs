//! Mencari executable FFmpeg dan FFprobe.

use std::path::{Path, PathBuf};

use crate::core::ffmpeg::runner::FFmpegRunner;

const WINDOWS_EXTRA_DIRS: [&str; 3] = [
    r"C:\ffmpeg\bin",
    r"C:\Program Files\ffmpeg\bin",
    r"C:\Program Files (x86)\ffmpeg\bin",
];
const UNIX_EXTRA_DIRS: [&str; 3] = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"];

/// Apakah aplikasi hanya boleh memakai FFmpeg bawaannya (mengabaikan biner
/// sistem). Aplikasi memasang FFmpeg-nya sendiri lewat layar setup di semua
/// platform, jadi biner sistem tidak dipakai supaya semua pengguna memakai
/// versi yang sama dan perilakunya tidak bergantung pada apa yang terpasang di
/// komputer mereka.
pub const BUNDLED_FFMPEG_ONLY: bool = true;

/// Resolusi path FFmpeg/FFprobe dan validasi biner pilihan.
///
/// Prioritas: path kustom pengguna, folder bawaan aplikasi, PATH, lalu folder
/// instalasi umum per platform.
#[derive(Clone)]
pub struct FFmpegLocator {
    custom_path: Option<String>,
    bundled_dirs: Vec<String>,
    system_fallback: bool,
    platform: String,
}

impl Default for FFmpegLocator {
    fn default() -> Self {
        Self::new(None)
    }
}

impl FFmpegLocator {
    pub fn new(custom_path: Option<String>) -> Self {
        Self::with_platform(custom_path, std::env::consts::OS)
    }

    /// Buat locator dengan nama platform eksplisit (dipakai pengujian).
    pub fn with_platform(custom_path: Option<String>, platform: impl Into<String>) -> Self {
        Self {
            custom_path: custom_path.filter(|value| !value.is_empty()),
            bundled_dirs: Vec::new(),
            system_fallback: true,
            platform: platform.into(),
        }
    }

    /// Folder bawaan aplikasi (folder exe hasil install atau resource bundle)
    /// yang diperiksa tepat setelah path pilihan pengguna, supaya FFmpeg yang
    /// dipasang bersama aplikasi selalu ditemukan tanpa mengubah PATH sistem.
    pub fn with_bundled_dirs(mut self, bundled_dirs: Vec<String>) -> Self {
        self.bundled_dirs = bundled_dirs;
        self
    }

    /// Jangan pakai FFmpeg milik sistem: hanya path pilihan pengguna dan folder
    /// bawaan aplikasi. Dipakai di platform yang binernya dipasang bersama
    /// aplikasi, supaya semua pengguna memakai versi yang sama dan perilakunya
    /// tidak bergantung pada apa yang terpasang di komputer mereka.
    pub fn without_system_fallback(mut self) -> Self {
        self.system_fallback = false;
        self
    }

    pub fn bundled_dirs(&self) -> &[String] {
        &self.bundled_dirs
    }

    pub fn is_windows(&self) -> bool {
        self.platform.starts_with("win")
    }

    pub fn executable_suffix(&self) -> &'static str {
        if self.is_windows() {
            ".exe"
        } else {
            ""
        }
    }

    pub fn has_custom_path(&self) -> bool {
        self.custom_path.is_some()
    }

    pub fn custom_path(&self) -> Option<&str> {
        self.custom_path.as_deref()
    }

    pub fn extra_dirs(&self) -> &'static [&'static str] {
        if self.is_windows() {
            &WINDOWS_EXTRA_DIRS
        } else {
            &UNIX_EXTRA_DIRS
        }
    }

    /// Kandidat FFmpeg menurut prioritas, tanpa duplikat.
    pub fn candidates(&self) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        if let Some(custom) = &self.custom_path {
            found.push(custom.clone());
        }
        for directory in &self.bundled_dirs {
            let candidate = join_path(directory, &format!("ffmpeg{}", self.executable_suffix()));
            if Path::new(&candidate).is_file() {
                found.push(candidate);
            }
        }
        if self.system_fallback {
            if let Some(located) = which("ffmpeg") {
                found.push(located);
            }
            for directory in self.extra_dirs() {
                let candidate =
                    join_path(directory, &format!("ffmpeg{}", self.executable_suffix()));
                if Path::new(&candidate).is_file() {
                    found.push(candidate);
                }
            }
        }
        let mut ordered: Vec<String> = Vec::new();
        for path in found {
            if !ordered.contains(&path) {
                ordered.push(path);
            }
        }
        ordered
    }

    pub fn resolve_ffmpeg(&self) -> String {
        self.candidates()
            .into_iter()
            .next()
            .unwrap_or_else(|| String::from("ffmpeg"))
    }

    pub fn resolve_ffprobe(&self, ffmpeg_path: Option<&str>) -> String {
        let executable = ffmpeg_path
            .map(str::to_string)
            .unwrap_or_else(|| self.resolve_ffmpeg());
        if let Some(parent) = parent_of(&executable) {
            let sibling = join_path(&parent, &format!("ffprobe{}", self.executable_suffix()));
            if Path::new(&sibling).is_file() {
                return sibling;
            }
        }
        which("ffprobe").unwrap_or_else(|| String::from("ffprobe"))
    }

    /// Environment dengan folder instalasi umum ditambahkan di depan PATH.
    pub fn extended_environment(&self) -> Vec<(String, String)> {
        let separator = if self.is_windows() { ";" } else { ":" };
        let current = std::env::var("PATH").unwrap_or_default();
        let mut entries: Vec<String> = self
            .extra_dirs()
            .iter()
            .map(|value| value.to_string())
            .collect();
        if !current.is_empty() {
            entries.push(current);
        }
        vec![(String::from("PATH"), entries.join(separator))]
    }

    /// Jalankan `<path> -version` dan laporkan apakah berhasil.
    pub async fn validate(&self, runner: &FFmpegRunner, path: Option<&str>) -> bool {
        let target = path
            .map(str::to_string)
            .unwrap_or_else(|| self.resolve_ffmpeg());
        let args: Vec<String> = vec![String::from("-version")];
        match runner
            .run_executable(&target, args, None, None, false, true)
            .await
        {
            Ok(result) => result.ok(),
            Err(error) => error.result.returncode == 0,
        }
    }

    /// Apakah ada kandidat FFmpeg yang tervalidasi.
    ///
    /// Kandidat diperiksa berurutan lewat [`Self::candidates`], jadi folder
    /// instalasi umum tetap dicoba ketika PATH belum memuatnya — mis. build
    /// release yang dibuka dari Finder hanya mewarisi PATH minimal.
    pub async fn is_available(&self, runner: &FFmpegRunner) -> bool {
        if let Some(custom) = &self.custom_path {
            // Path pilihan pengguna dipakai apa adanya: kalau salah, itu yang
            // ingin dilaporkan, bukan diam-diam diganti kandidat lain.
            return self.validate(runner, Some(custom)).await;
        }
        for candidate in self.candidates() {
            if self.validate(runner, Some(&candidate)).await {
                return true;
            }
        }
        if self.system_fallback {
            // Jaring terakhir: nama polos, biar tetap jalan kalau PATH sendiri
            // sudah benar.
            return self.validate(runner, Some("ffmpeg")).await;
        }
        false
    }
}

/// Cari executable di PATH tanpa bergantung pada crate eksternal.
pub fn which(name: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    let suffixes: Vec<&str> = if cfg!(windows) {
        vec!["", ".exe", ".bat", ".cmd"]
    } else {
        vec![""]
    };
    for directory in std::env::split_paths(&path_var) {
        for suffix in &suffixes {
            let candidate: PathBuf = directory.join(format!("{name}{suffix}"));
            if is_executable(&candidate) {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn join_path(directory: &str, filename: &str) -> String {
    let trimmed = directory.trim_end_matches(['/', '\\']);
    let separator = if directory.contains('\\') && !directory.contains('/') {
        "\\"
    } else {
        "/"
    };
    format!("{trimmed}{separator}{filename}")
}

fn parent_of(path: &str) -> Option<String> {
    let index = path.rfind(['/', '\\'])?;
    if index == 0 {
        return None;
    }
    Some(path[..index].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_dirs_follow_platform() {
        let unix = FFmpegLocator::with_platform(None, "darwin");
        assert_eq!(unix.extra_dirs(), &UNIX_EXTRA_DIRS);
        let windows = FFmpegLocator::with_platform(None, "win32");
        assert_eq!(windows.extra_dirs(), &WINDOWS_EXTRA_DIRS);
        assert_eq!(windows.executable_suffix(), ".exe");
    }

    #[test]
    fn custom_path_is_first_candidate() {
        let locator = FFmpegLocator::with_platform(Some("/custom/ffmpeg".into()), "linux");
        assert_eq!(locator.resolve_ffmpeg(), "/custom/ffmpeg");
        assert!(locator.has_custom_path());
    }

    #[test]
    fn joins_and_splits_paths() {
        assert_eq!(
            join_path("/opt/homebrew/bin", "ffmpeg"),
            "/opt/homebrew/bin/ffmpeg"
        );
        assert_eq!(
            join_path(r"C:\ffmpeg\bin", "ffmpeg.exe"),
            r"C:\ffmpeg\bin\ffmpeg.exe"
        );
        assert_eq!(
            parent_of("/usr/local/bin/ffmpeg").as_deref(),
            Some("/usr/local/bin")
        );
    }

    /// Folder instalasi umum harus ikut menjadi kandidat walau PATH memuatnya
    /// tidak — inilah yang membuat deteksi tetap jalan dari build release yang
    /// dibuka lewat Finder (PATH-nya minimal).
    #[test]
    fn well_known_dirs_are_candidates() {
        let locator = FFmpegLocator::with_platform(None, "darwin");
        for directory in UNIX_EXTRA_DIRS {
            let candidate = join_path(directory, "ffmpeg");
            if Path::new(&candidate).is_file() {
                assert!(
                    locator.candidates().contains(&candidate),
                    "{candidate} harus muncul sebagai kandidat"
                );
            }
        }
    }

    /// FFmpeg yang dipasang bersama aplikasi diutamakan atas yang ada di PATH.
    #[test]
    fn bundled_dir_outranks_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ffmpeg"), b"stub").unwrap();
        let bundled = dir.path().to_string_lossy().to_string();

        let locator =
            FFmpegLocator::with_platform(None, "linux").with_bundled_dirs(vec![bundled.clone()]);
        assert_eq!(
            locator.candidates().first(),
            Some(&join_path(&bundled, "ffmpeg"))
        );

        // Path pilihan pengguna tetap yang paling diutamakan.
        let with_custom = FFmpegLocator::with_platform(Some("/custom/ffmpeg".into()), "linux")
            .with_bundled_dirs(vec![bundled]);
        assert_eq!(
            with_custom.candidates().first().map(String::as_str),
            Some("/custom/ffmpeg")
        );
    }

    /// FFmpeg di folder aplikasi harus cukup: app hasil install dianggap siap
    /// walau PATH tidak memuat ffmpeg.
    #[cfg(unix)]
    #[tokio::test]
    async fn bundled_dir_is_enough() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("ffmpeg");
        std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let runner = FFmpegRunner::new("ffmpeg", None);
        let bundled = FFmpegLocator::with_platform(None, "linux")
            .with_bundled_dirs(vec![dir.path().to_string_lossy().to_string()]);
        assert!(
            bundled.is_available(&runner).await,
            "FFmpeg di folder aplikasi harus dianggap tersedia"
        );
    }

    /// Mode terisolasi: biner milik sistem tidak dipakai sama sekali, jadi
    /// hasilnya seragam untuk semua pengguna.
    #[tokio::test]
    async fn isolated_locator_ignores_system_ffmpeg() {
        let runner = FFmpegRunner::new("ffmpeg", None);
        let isolated = FFmpegLocator::with_platform(None, "darwin").without_system_fallback();

        assert!(isolated.candidates().is_empty());
        assert!(!isolated.is_available(&runner).await);
    }

    /// Path pilihan pengguna tetap berlaku walau mode terisolasi aktif.
    #[tokio::test]
    async fn isolated_locator_still_honours_custom_path() {
        let runner = FFmpegRunner::new("ffmpeg", None);
        let locator = FFmpegLocator::with_platform(Some("/custom/ffmpeg".into()), "darwin")
            .without_system_fallback();

        assert_eq!(
            locator.candidates().first().map(String::as_str),
            Some("/custom/ffmpeg")
        );
        assert!(
            !locator.is_available(&runner).await,
            "path salah tetap dilaporkan"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn is_available_honours_an_explicit_path() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("ffmpeg");
        std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let runner = FFmpegRunner::new("ffmpeg", None);
        let good =
            FFmpegLocator::with_platform(Some(script.to_string_lossy().to_string()), "linux");
        assert!(
            good.is_available(&runner).await,
            "path kustom valid diterima"
        );

        let bad = FFmpegLocator::with_platform(Some("/nonexistent/ffmpeg".into()), "linux");
        assert!(
            !bad.is_available(&runner).await,
            "path kustom salah tidak diganti kandidat lain"
        );
    }
}
