//! Verifikasi channel dari ujung ke ujung dengan media sungguhan: deteksi folder,
//! penyusunan rencana, cek kapasitas, lalu pemrosesan rencana terpilih.
//!
//! Bagian yang butuh FFmpeg dilewati (bukan gagal) bila FFmpeg tidak terpasang.
//! Verifikasi "volume kecil" hanya berjalan bila `SWALOKA_SMALL_VOLUME` diisi
//! path sebuah volume yang memang sempit (lihat tasks.md 9.2).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use swaloka_lib::core::ffmpeg::discovery::which;
use swaloka_lib::core::ffmpeg::{FFmpegLocator, FFmpegRunner, MetadataReader};
use swaloka_lib::features::channel::{
    Channel, ChannelCapacity, ChannelExecutor, ChannelPlan, ChannelPlanStatus, ChannelPlanner,
    ChannelRepository, ChannelStatus,
};
use swaloka_lib::features::looping::LoopMergeService;
use swaloka_lib::features::ProgressCallback;
use swaloka_lib::storage::cache::AudioCache;
use swaloka_lib::storage::project::Project;
use swaloka_lib::storage::temp::TempManager;

fn run_ffmpeg(ffmpeg: &str, args: Vec<String>) {
    let status = std::process::Command::new(ffmpeg)
        .args(&args)
        .status()
        .expect("ffmpeg bisa dijalankan");
    assert!(status.success(), "ffmpeg gagal: {args:?}");
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

/// Runner seperti di aplikasi: ffprobe dicari di sebelah ffmpeg, karena
/// `FFmpegRunner::new(path, None)` memakai ffmpeg juga sebagai ffprobe.
fn runner_like_the_app(ffmpeg: &str) -> Arc<FFmpegRunner> {
    let locator = FFmpegLocator::new(None);
    let ffprobe = locator.resolve_ffprobe(Some(ffmpeg));
    Arc::new(FFmpegRunner::new(ffmpeg, Some(ffprobe)))
}

/// Aset uji: lima lagu pendek dengan durasi berbeda dan dua video latar pendek.
fn build_assets(ffmpeg: &str, root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let audio = root.join("audio");
    let video = root.join("video");
    std::fs::create_dir_all(&audio).expect("folder audio dibuat");
    std::fs::create_dir_all(&video).expect("folder video dibuat");

    for (index, seconds) in [3, 4, 5, 6, 7].iter().enumerate() {
        let output = audio.join(format!("lagu_{:02}.mp3", index + 1));
        run_ffmpeg(
            ffmpeg,
            strings(&[
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency={}:duration={seconds}", 300 + index * 40),
                "-c:a",
                "libmp3lame",
                output.to_str().unwrap(),
            ]),
        );
    }

    for (index, colour) in ["navy", "darkgreen"].iter().enumerate() {
        let output = video.join(format!("bg_{:02}.mp4", index + 1));
        run_ffmpeg(
            ffmpeg,
            strings(&[
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={colour}:s=320x240:r=30:d=2"),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                output.to_str().unwrap(),
            ]),
        );
    }

    // Intro sengaja di luar folder video channel supaya tidak dihitung sebagai latar.
    let intro = root.join("intro.mp4");
    run_ffmpeg(
        ffmpeg,
        strings(&[
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x240:r=30:d=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            intro.to_str().unwrap(),
        ]),
    );
    (audio, video, intro)
}

#[tokio::test]
async fn channel_plans_reports_capacity_and_processes_the_selection() {
    let Some(ffmpeg) = which("ffmpeg") else {
        return;
    };

    let directory = tempfile::tempdir().unwrap();
    let project = Project::new(
        "Uji Channel",
        directory.path().to_string_lossy().to_string(),
    );
    project.ensure_structure().unwrap();
    let (audio_folder, video_folder, intro) = build_assets(&ffmpeg, directory.path());

    let runner = runner_like_the_app(&ffmpeg);
    let metadata = Arc::new(MetadataReader::new(true));
    let cache = Arc::new(AudioCache::new(project.clone()));
    let temp = Arc::new(TempManager::new(project.temp_dir()));
    let loop_service = Arc::new(LoopMergeService::new(
        runner.clone(),
        metadata.clone(),
        cache.clone(),
        temp.clone(),
        None,
    ));
    let repository = ChannelRepository::new(project.clone());

    let mut channel = Channel::create(
        String::from("ch_uji"),
        String::from("Uji Channel"),
        project.root_path.clone(),
        project.output_dir().to_string_lossy().to_string(),
        42,
    );
    channel.audio_folder_path = audio_folder.to_string_lossy().to_string();
    channel.video_folder_path = video_folder.to_string_lossy().to_string();
    // 15 detik per rencana supaya pengujian cepat.
    channel.min_duration_minutes = 0.25;
    // Minta lebih banyak daripada video yang ada: hasilnya dibatasi jumlah video.
    channel.total_content = 5;
    repository.save(&channel).unwrap();

    // 1. Deteksi + penyusunan rencana.
    let planner = ChannelPlanner::new(metadata.clone(), runner.clone(), None);
    let planned = planner.plan(&channel).await.expect("rencana bisa dibuat");
    repository.save(&planned).unwrap();

    assert_eq!(planned.audio_items.len(), 5, "lima lagu terdeteksi");
    assert_eq!(planned.video_items.len(), 2, "dua video latar terdeteksi");
    assert_eq!(planned.plans.len(), 5, "permintaan 5 konten dihormati");
    assert_eq!(
        planned.plans[0].background_video_path, planned.video_items[0],
        "rencana pertama memakai video pertama"
    );
    assert_eq!(
        planned.plans[1].background_video_path, planned.video_items[1],
        "rencana kedua memakai video kedua"
    );
    assert_eq!(
        planned.waiting_for_video(),
        3,
        "sisa rencana menunggu video, tanpa memakai video yang sama dua kali"
    );
    assert_eq!(
        planned.plans.iter().filter(|plan| plan.selected).count(),
        2,
        "hanya rencana yang punya video yang terpilih untuk diproses"
    );
    for plan in &planned.plans[2..] {
        assert!(!plan.selected);
        assert_eq!(plan.status, ChannelPlanStatus::NotProcessed);
    }
    assert!(
        planned
            .audio_items
            .iter()
            .all(|item| item.duration_seconds.unwrap_or(0.0) > 0.0),
        "durasi lagu sungguhan terbaca"
    );
    for plan in &planned.plans {
        assert!(
            plan.total_duration_seconds >= 15.0,
            "durasi minimum dipenuhi"
        );
        assert_eq!(plan.status, ChannelPlanStatus::NotProcessed);
    }
    for plan in &planned.plans[..2] {
        assert!(!plan.background_video_path.is_empty());
        assert!(plan.selected, "rencana yang punya video langsung terpilih");
    }

    // 2. Cek kapasitas di volume nyata.
    let capacity = ChannelCapacity::new(metadata.clone(), runner.clone());
    let report = capacity.report(&planned).await;
    assert_eq!(report.plan_count, 2);
    assert!(report.required_bytes > 0);
    assert!(report.available_bytes.is_some(), "ruang bebas terbaca");
    assert!(report.fits, "proyek di disk normal harus muat");
    assert!(!report.space_unknown);

    // 3. Proses: rencana pertama dengan intro, kedua tanpa.
    let mut to_run = planned.clone();
    to_run.plans[0].intro_video_path = Some(intro.to_string_lossy().to_string());

    let seen: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let progress: ProgressCallback = Arc::new(move |value| {
        sink.lock().expect("kumpulan progres terkunci").push(value);
    });

    let executor = ChannelExecutor::new(
        ChannelRepository::new(project.clone()),
        loop_service.clone(),
        None,
    );
    let finished = executor
        .run_selection(&to_run, Some(progress))
        .await
        .expect("batch berjalan");

    assert_eq!(
        finished.status,
        ChannelStatus::Completed,
        "status per rencana: {:?}",
        finished
            .plans
            .iter()
            .map(|plan| format!("#{} {:?} {:?}", plan.index + 1, plan.status, plan.error))
            .collect::<Vec<_>>()
    );
    assert_eq!(finished.done_count(), 2, "dua rencana selesai");

    for plan in finished
        .plans
        .iter()
        .filter(|plan| !plan.background_video_path.is_empty())
    {
        assert_eq!(
            plan.status,
            ChannelPlanStatus::Done,
            "rencana #{} gagal: {:?}",
            plan.index + 1,
            plan.error
        );
        let output = plan.output_path.clone().expect("path output tercatat");
        let path = Path::new(&output);
        assert!(path.exists(), "output {output} benar-benar ada");
        assert!(
            path.starts_with(project.output_dir()),
            "output ditulis di folder output proyek: {output}"
        );
        assert!(
            path.parent()
                .and_then(|parent| parent.file_name())
                .map(|name| name.to_string_lossy().contains("Uji Channel"))
                .unwrap_or(false),
            "output ada di folder per channel: {output}"
        );

        // Durasi hasil mengikuti durasi audionya (intro dan loop latar dipotong).
        let duration = metadata
            .read_duration(&runner, &output)
            .await
            .expect("durasi output terbaca");
        assert!(
            (duration - plan.total_duration_seconds).abs() < 1.5,
            "durasi {duration} tidak sesuai rencana {}",
            plan.total_duration_seconds
        );
    }

    // 4. Progres naik dan berakhir penuh.
    let values = seen.lock().expect("kumpulan progres terkunci").clone();
    assert!(values.len() >= 2, "progres dilaporkan per rencana");
    assert!(
        values.windows(2).all(|pair| pair[1] >= pair[0]),
        "progres tidak pernah mundur: {values:?}"
    );
    assert_eq!(values.last().copied(), Some(1.0));

    // 5. Rencana yang menunggu video tetap belum diproses.
    for plan in finished
        .plans
        .iter()
        .filter(|plan| plan.background_video_path.is_empty())
    {
        assert_eq!(plan.status, ChannelPlanStatus::NotProcessed);
        assert!(plan.output_path.is_none());
        assert!(!plan.selected);
    }

    // 6. State bertahan setelah channel dibuka ulang dari disk.
    let reloaded = ChannelRepository::new(project.clone())
        .load("ch_uji")
        .expect("channel termuat lagi");
    assert_eq!(reloaded.done_count(), 2);
    assert!(
        reloaded
            .plans
            .iter()
            .filter(|plan| !plan.background_video_path.is_empty())
            .all(|plan| plan.output_path.is_some()),
        "rencana yang diproses menyimpan path outputnya"
    );
    assert_eq!(
        reloaded.waiting_for_video(),
        3,
        "sisanya masih menunggu video"
    );
    assert!(reloaded.plans[0].intro_video_path.is_some());
    assert!(reloaded.plans[1].intro_video_path.is_none());
}

/// Verifikasi penolakan pada volume yang benar-benar sempit (tasks.md 9.2).
#[tokio::test]
async fn refuses_a_batch_that_does_not_fit_a_small_volume() {
    let Ok(volume) = std::env::var("SWALOKA_SMALL_VOLUME") else {
        return;
    };
    let Some(ffmpeg) = which("ffmpeg") else {
        return;
    };

    let directory = tempfile::tempdir().unwrap();
    let project = Project::new("Uji Sempit", directory.path().to_string_lossy().to_string());
    project.ensure_structure().unwrap();
    let metadata = Arc::new(MetadataReader::new(true));
    let runner = Arc::new(FFmpegRunner::new(ffmpeg, None));

    let mut channel = Channel::create(
        String::from("ch_sempit"),
        String::from("Uji Sempit"),
        project.root_path.clone(),
        volume.clone(),
        7,
    );
    // Dua rencana 2 jam: perkiraannya jauh melebihi volume percobaan.
    channel.plans = (0..2)
        .map(|index| {
            let mut plan = ChannelPlan::new(format!("plan_{}", index + 1), index);
            plan.total_duration_seconds = 7200.0;
            plan.background_video_path = format!("/video/tidak-ada-{index}.mp4");
            plan
        })
        .collect();

    let capacity = ChannelCapacity::new(metadata, runner);
    let report = capacity.report(&channel).await;

    assert_eq!(report.plan_count, 2);
    let available = report
        .available_bytes
        .expect("ruang bebas volume kecil harus terbaca");
    assert!(
        available < report.required_bytes,
        "volume uji harus lebih kecil dari kebutuhan: {available} < {}",
        report.required_bytes
    );
    assert!(!report.fits, "batch tidak boleh dinyatakan muat");
    assert!(!report.space_unknown);
    assert!(Path::new(&volume).exists());
}
