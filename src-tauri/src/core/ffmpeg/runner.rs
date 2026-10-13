//! Eksekusi proses FFmpeg/FFprobe asinkron.
//!
//! Output mentah FFmpeg **tidak** dialirkan ke UI: log aplikasi hanya berisi
//! pesan langkah yang bisa dibaca manusia. Saat sebuah perintah gagal, ekor
//! stderr-nya dilampirkan supaya penyebabnya tetap bisa diperiksa.
//!
//! `FFmpegError` sengaja memuat `ProcessResult` (payload besar) supaya pemanggil
//! bisa menginspeksi output lengkap; boxing tidak sebanding manfaatnya di sini.
#![allow(clippy::result_large_err)]

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::core::logging::{LogLevel, LogRecord};

/// Callback yang menerima pesan log selama proses berjalan.
pub type LogCallback = Arc<dyn Fn(LogRecord) + Send + Sync>;

/// Jumlah baris output terakhir yang dilampirkan ketika perintah gagal.
pub const ERROR_TAIL_LINES: usize = 15;

/// Hasil eksekusi sebuah proses.
#[derive(Clone, Debug)]
pub struct ProcessResult {
    pub executable: String,
    pub command: Vec<String>,
    pub returncode: i32,
    pub duration: Duration,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl ProcessResult {
    pub fn ok(&self) -> bool {
        self.returncode == 0
    }

    /// Stream yang paling relevan untuk melaporkan kegagalan.
    pub fn failure_output(&self) -> &[String] {
        if self.stderr.iter().any(|line| !line.trim().is_empty()) {
            &self.stderr
        } else {
            &self.stdout
        }
    }
}

/// Kesalahan saat sebuah perintah keluar dengan status non-nol.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct FFmpegError {
    pub message: String,
    pub result: ProcessResult,
}

/// Pesan kegagalan yang bisa dibaca, berisi ekor output FFmpeg sebagai bukti.
pub fn build_error_report(executable: &str, returncode: i32, output: &[String]) -> String {
    let mut report = format!("{} gagal (kode {returncode})", program_name(executable));
    let tail = error_tail(output);
    if !tail.is_empty() {
        report.push('\n');
        report.push_str(&tail.join("\n"));
    }
    report
}

/// Baris terakhir yang tidak kosong, dibatasi [`ERROR_TAIL_LINES`].
pub fn error_tail(output: &[String]) -> Vec<String> {
    let mut lines: Vec<String> = output
        .iter()
        .filter(|line| !line.trim().is_empty())
        .cloned()
        .collect();
    if lines.len() > ERROR_TAIL_LINES {
        let excess = lines.len() - ERROR_TAIL_LINES;
        lines.drain(0..excess);
    }
    lines
}

fn program_name(executable: &str) -> String {
    Path::new(executable)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| executable.to_string())
}

/// Menjalankan FFmpeg/FFprobe tanpa membanjiri log dengan output mentahnya.
pub struct FFmpegRunner {
    paths: std::sync::Mutex<(String, String)>,
    pub max_captured_lines: usize,
    pub cancel_grace_seconds: f64,
    pid: Arc<AtomicI32>,
    cancelled: Arc<AtomicBool>,
}

impl FFmpegRunner {
    /// Runner dengan path ffmpeg dan ffprobe.
    ///
    /// `ffprobe_path: None` berarti path ffmpeg yang dipakai untuk probe juga —
    /// probe-nya akan gagal, jadi pemanggil sebaiknya mengoper hasil
    /// [`FFmpegLocator::resolve_ffprobe`], seperti yang dilakukan aplikasi.
    pub fn new(ffmpeg_path: impl Into<String>, ffprobe_path: Option<String>) -> Self {
        let ffmpeg_path = ffmpeg_path.into();
        let ffprobe_path = ffprobe_path.unwrap_or_else(|| ffmpeg_path.clone());
        Self {
            paths: std::sync::Mutex::new((ffmpeg_path, ffprobe_path)),
            max_captured_lines: 4000,
            cancel_grace_seconds: 0.5,
            pid: Arc::new(AtomicI32::new(0)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.pid.load(Ordering::SeqCst) > 0
    }

    fn paths(&self) -> (String, String) {
        self.paths
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn ffmpeg_path(&self) -> String {
        self.paths().0
    }

    pub fn ffprobe_path(&self) -> String {
        self.paths().1
    }

    /// Arahkan runner ke pasangan FFmpeg/FFprobe hasil resolusi.
    pub fn configure(&self, ffmpeg_path: impl Into<String>, ffprobe_path: Option<String>) {
        let ffmpeg_path = ffmpeg_path.into();
        let ffprobe_path = ffprobe_path.unwrap_or_else(|| ffmpeg_path.clone());
        let mut paths = self.paths.lock().unwrap_or_else(|error| error.into_inner());
        *paths = (ffmpeg_path, ffprobe_path);
    }

    /// Jalankan perintah FFmpeg.
    pub async fn run(
        &self,
        args: Vec<String>,
        on_log: Option<LogCallback>,
        cwd: Option<&Path>,
        check: bool,
        quiet: bool,
    ) -> Result<ProcessResult, FFmpegError> {
        let executable = self.ffmpeg_path();
        self.run_executable(&executable, args, on_log, cwd, check, quiet)
            .await
    }

    /// Jalankan perintah FFprobe.
    pub async fn run_probe(
        &self,
        args: Vec<String>,
        on_log: Option<LogCallback>,
        cwd: Option<&Path>,
        check: bool,
        quiet: bool,
    ) -> Result<ProcessResult, FFmpegError> {
        let executable = self.ffprobe_path();
        self.run_executable(&executable, args, on_log, cwd, check, quiet)
            .await
    }

    /// Jalankan executable apa pun dengan argument vector (tanpa shell).
    pub async fn run_executable(
        &self,
        executable: &str,
        args: Vec<String>,
        on_log: Option<LogCallback>,
        cwd: Option<&Path>,
        check: bool,
        quiet: bool,
    ) -> Result<ProcessResult, FFmpegError> {
        self.cancelled.store(false, Ordering::SeqCst);
        let started = Instant::now();

        let mut command = Command::new(executable);
        command.args(&args);
        command.stdin(std::process::Stdio::null());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.kill_on_drop(true);
        if let Some(directory) = cwd {
            command.current_dir(directory);
        }
        // Grup proses sendiri supaya pembatalan bisa mematikan seluruh pohon.
        #[cfg(unix)]
        command.process_group(0);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000 | 0x0000_0200);

        let command_line: Vec<String> = std::iter::once(executable.to_string())
            .chain(args.iter().cloned())
            .collect();

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let result = ProcessResult {
                    executable: executable.to_string(),
                    command: command_line,
                    returncode: -1,
                    duration: started.elapsed(),
                    stdout: Vec::new(),
                    stderr: vec![error.to_string()],
                };
                if !quiet {
                    emit_failure(&on_log, executable, -1, result.failure_output());
                }
                return Err(FFmpegError {
                    message: format!("{error}"),
                    result,
                });
            }
        };

        let pid = child.id().unwrap_or(0) as i32;
        self.pid.store(pid, Ordering::SeqCst);

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let max_lines = self.max_captured_lines;

        let stdout_task = tokio::spawn(async move {
            let mut collected: Vec<String> = Vec::new();
            if let Some(stream) = stdout {
                let mut lines = BufReader::new(stream).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let line = line.trim_end().to_string();
                    if line.trim().is_empty() {
                        continue;
                    }
                    append_bounded(&mut collected, line, max_lines);
                }
            }
            collected
        });

        let stderr_task = tokio::spawn(async move {
            let mut collected: Vec<String> = Vec::new();
            if let Some(stream) = stderr {
                let mut lines = BufReader::new(stream).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let line = line.trim_end().to_string();
                    if line.trim().is_empty() {
                        continue;
                    }
                    append_bounded(&mut collected, line, max_lines);
                }
            }
            collected
        });

        let (stdout_lines, stderr_lines) = tokio::join!(stdout_task, stderr_task);

        let status = child.wait().await;
        self.pid.store(0, Ordering::SeqCst);
        let returncode = status.map(|value| value.code().unwrap_or(-1)).unwrap_or(-1);

        let result = ProcessResult {
            executable: executable.to_string(),
            command: command_line,
            returncode,
            duration: started.elapsed(),
            stdout: stdout_lines.unwrap_or_default(),
            stderr: stderr_lines.unwrap_or_default(),
        };

        if !result.ok() && !quiet {
            emit_failure(&on_log, executable, returncode, result.failure_output());
        }

        if check && !result.ok() {
            return Err(FFmpegError {
                message: format!("{} gagal (kode {returncode})", program_name(executable)),
                result,
            });
        }

        Ok(result)
    }

    /// Hentikan pohon proses yang sedang berjalan, naik ke SIGKILL bila perlu.
    pub fn cancel(&self) {
        let pid = self.pid.load(Ordering::SeqCst);
        if pid <= 0 {
            return;
        }
        self.cancelled.store(true, Ordering::SeqCst);
        signal_tree(pid, false);

        let pid_ref = Arc::clone(&self.pid);
        let grace = self.cancel_grace_seconds;
        let escalation = async move {
            tokio::time::sleep(Duration::from_secs_f64(grace.max(0.05))).await;
            if pid_ref.load(Ordering::SeqCst) == pid {
                signal_tree(pid, true);
            }
        };

        // Perintah Tauri yang sinkron — tutup proyek, tombol batal — berjalan
        // tanpa runtime Tokio. Tanpa runtime tidak ada tempat menunggu masa
        // tenggang, jadi pohon proses langsung dihentikan paksa.
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn(escalation);
            }
            Err(_) => signal_tree(pid, true),
        }
    }
}

/// Lampirkan ringkasan kegagalan beserta ekor output FFmpeg ke log UI.
fn emit_failure(
    on_log: &Option<LogCallback>,
    executable: &str,
    returncode: i32,
    output: &[String],
) {
    if let Some(callback) = on_log {
        callback(LogRecord::new(
            build_error_report(executable, returncode, output),
            LogLevel::Error,
            1,
        ));
    }
}

/// Kirim sinyal ke seluruh grup proses.
fn signal_tree(pid: i32, force: bool) {
    #[cfg(unix)]
    {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        // PID negatif berarti "seluruh grup proses".
        unsafe {
            libc::kill(-pid, signal);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if force {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(0x0800_0000)
                .output();
        } else {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T"])
                .creation_flags(0x0800_0000)
                .output();
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (pid, force);
    }
}

fn append_bounded(target: &mut Vec<String>, line: String, limit: usize) {
    target.push(line);
    if target.len() > limit {
        let excess = target.len() - limit;
        target.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(range: std::ops::Range<usize>) -> Vec<String> {
        range.map(|index| format!("baris {index}")).collect()
    }

    #[test]
    fn error_tail_keeps_only_the_last_lines() {
        let tail = error_tail(&lines(0..25));
        assert_eq!(tail.len(), ERROR_TAIL_LINES);
        assert_eq!(tail.first().unwrap(), "baris 10");
        assert_eq!(tail.last().unwrap(), "baris 24");
    }

    #[test]
    fn error_tail_skips_blank_lines() {
        let mut output = lines(0..3);
        output.push(String::new());
        output.push("   ".into());
        output.push("pesan terakhir".into());
        assert_eq!(error_tail(&output).last().unwrap(), "pesan terakhir");
    }

    #[test]
    fn report_names_program_and_includes_evidence() {
        let report = build_error_report(
            "/opt/homebrew/bin/ffmpeg",
            1,
            &["Invalid data found".into(), "Conversion failed!".into()],
        );
        assert!(report.starts_with("ffmpeg gagal (kode 1)"));
        assert!(report.contains("Invalid data found"));
        assert!(report.contains("Conversion failed!"));
    }

    #[test]
    fn report_without_output_stays_single_line() {
        let report = build_error_report("ffmpeg", -1, &[]);
        assert_eq!(report, "ffmpeg gagal (kode -1)");
    }

    #[test]
    fn failure_output_prefers_stderr() {
        let mut result = ProcessResult {
            executable: "ffmpeg".into(),
            command: vec!["ffmpeg".into()],
            returncode: 1,
            duration: Duration::from_millis(5),
            stdout: vec!["dari stdout".into()],
            stderr: Vec::new(),
        };
        assert_eq!(result.failure_output(), ["dari stdout"]);
        result.stderr = vec!["dari stderr".into()];
        assert_eq!(result.failure_output(), ["dari stderr"]);
    }

    /// Perintah Tauri yang sinkron (tutup proyek, tombol batal) memanggil
    /// `cancel` tanpa runtime Tokio; itu tidak boleh membuat aplikasi panik.
    #[test]
    fn cancel_without_a_tokio_runtime_does_not_panic() {
        let runner = FFmpegRunner::new("ffmpeg", None);
        assert!(
            tokio::runtime::Handle::try_current().is_err(),
            "tes sinkron tanpa runtime"
        );

        // PID yang tidak mungkin hidup: cukup membuktikan jalur ini tidak panik.
        runner.pid.store(999_999, Ordering::SeqCst);
        runner.cancel();

        assert!(
            runner.cancelled.load(Ordering::SeqCst),
            "run ditandai dibatalkan"
        );
    }

    /// Tanpa PID aktif, `cancel` tidak melakukan apa-apa.
    #[test]
    fn cancel_without_a_process_is_a_no_op() {
        let runner = FFmpegRunner::new("ffmpeg", None);
        runner.cancel();
        assert!(
            !runner.cancelled.load(Ordering::SeqCst),
            "tidak ada proses, jadi tidak ada yang dibatalkan"
        );
    }
}
