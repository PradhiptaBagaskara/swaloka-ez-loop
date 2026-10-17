"""Tkinter sederhana untuk fitur Looping Swaloka Looping Tool.

Meniru alur halaman "Looping" pada aplikasi Flet: pilih video latar (wajib),
tambah playlist audio (bisa banyak dan diurutkan), opsional video pembuka,
atur jumlah loop + nama output, lalu render video dengan FFmpeg.

Pipeline (mengikuti swaloka.features.looping):
  1. Normalisasi setiap audio ke AAC (kalau belum AAC).
  2. Susun urutan playlist: loop pertama urut, loop berikutnya diacak.
  3. Gabung playlist audio dengan concat demuxer (-c copy).
  4. Loop video latar menempel ke panjang audio (stream_loop).
     Kalau ada video pembuka: pre-loop latar ke >= 10 menit, lalu concat
     [intro, latar diulang] dan tempel ke audio.
  5. Tulis sidecar timestamp (<nama>_timestamp.txt).

Butuh ffmpeg & ffprobe di PATH. Jalankan: python manual_loop.py
"""

from __future__ import annotations

import math
import os
import queue
import random
import re
import shlex
import shutil
import subprocess
import tempfile
import threading
import time
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk

VIDEO_TYPES = [("Video", "*.mp4 *.mov *.mkv *.avi *.webm"), ("Semua file", "*.*")]
AUDIO_TYPES = [("Audio", "*.mp3 *.wav *.m4a *.aac *.ogg *.flac"), ("Semua file", "*.*")]

AAC_EXTENSIONS = {".m4a", ".aac"}
PREPROCESS_MIN_SECONDS = 600.0
UNSAFE_NAME = re.compile(r"[^a-zA-Z0-9_\-]")

COLOR_METADATA_FLAGS = [
    "-colorspace", "bt709",
    "-color_trc", "bt709",
    "-color_primaries", "bt709",
    "-color_range", "tv",
]
AAC_AUDIO_ARGS = ["-vn", "-c:a", "aac", "-b:a", "192k", "-ar", "44100", "-ac", "2"]


# ------------------------------------------------------------------ helpers ---
def find_ffmpeg() -> str:
    found = shutil.which("ffmpeg")
    if found:
        return found
    for directory in ("/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"):
        candidate = Path(directory) / "ffmpeg"
        if candidate.is_file():
            return str(candidate)
    return "ffmpeg"


def find_ffprobe(ffmpeg: str) -> str:
    found = shutil.which("ffprobe")
    if found:
        return found
    sibling = Path(ffmpeg).with_name("ffprobe")
    if ffmpeg and sibling.is_file():
        return str(sibling)
    return "ffprobe"


def format_concat_path(path) -> str:
    text = str(Path(path)).replace("\\", "/")
    return text.replace("'", "'\\''")


def audio_concat_content(files) -> str:
    return "\n".join(f"file '{format_concat_path(p)}'" for p in files)


def video_concat_content(intro, background, background_loops: int) -> str:
    lines = [f"file '{format_concat_path(intro)}'"]
    lines.extend(f"file '{format_concat_path(background)}'" for _ in range(max(0, background_loops)))
    return "\n".join(lines)


def format_clock(seconds: float) -> str:
    total = int(seconds)
    hours, rem = divmod(total, 3600)
    minutes, secs = divmod(rem, 60)
    if hours > 0:
        return f"{hours}:{minutes:02d}:{secs:02d}"
    if minutes == 0:
        return f"0:{secs:02d}"
    return f"{minutes}:{secs:02d}"


def safe_base_name(name: str, max_length: int = 64) -> str:
    cleaned = UNSAFE_NAME.sub("_", name)
    return cleaned[:max_length] if cleaned else "video"


def unique_path(path: Path) -> Path:
    if not path.exists():
        return path
    counter = 1
    while True:
        candidate = path.parent / f"{path.stem}_{counter}{path.suffix}"
        if not candidate.exists():
            return candidate
        counter += 1


def build_order(count: int, loop_count: int, rng: random.Random | None = None) -> list[int]:
    rng = rng or random.Random()
    order = list(range(count))
    for _ in range(1, max(1, loop_count)):
        shuffled = list(range(count))
        rng.shuffle(shuffled)
        order.extend(shuffled)
    return order


# ---------------------------------------------------------------------- app ---
class LoopingToolApp:
    def __init__(self, root):
        self.root = root
        self.root.title("Swaloka Looping Tool — Looping")
        self.root.geometry("620x740")
        self.root.minsize(600, 700)

        self.background_path = ""
        self.intro_path = ""
        self.audio_paths = []
        self.output_dir = ""
        self.loop_count = tk.IntVar(value=1)
        self.output_name = tk.StringVar()
        self._running = False
        self._queue: "queue.Queue[tuple[str, object]]" = queue.Queue()

        self.ffmpeg = find_ffmpeg()
        self.ffprobe = find_ffprobe(self.ffmpeg)

        self._build_ui()
        self._log(f"ffmpeg : {self.ffmpeg}")
        self._log(f"ffprobe: {self.ffprobe}")
        if shutil.which("ffmpeg") is None and not Path(self.ffmpeg).is_file():
            self._log("PERINGATAN: ffmpeg tidak ditemukan di PATH. Pasang FFmpeg dulu.")
        self.root.after(100, self._drain_queue)

    # ------------------------------------------------------------------ UI ---
    def _build_ui(self):
        style = ttk.Style()
        style.configure("Header.TLabel", font=("Arial", 14, "bold"))
        style.configure("Sub.TLabel", foreground="#666666")

        header = ttk.Frame(self.root)
        header.pack(fill="x", padx=16, pady=(14, 6))
        ttk.Label(header, text="Looping", style="Header.TLabel").pack(anchor="w")
        ttk.Label(
            header,
            text="Putar video latar berulang di atas playlist audio.",
            style="Sub.TLabel",
        ).pack(anchor="w")

        self._build_background_section()
        self._build_audio_section()
        self._build_intro_section()
        self._build_settings_section()
        self._build_actions()

    def _build_background_section(self):
        frame = ttk.LabelFrame(self.root, text="Video latar (wajib)", padding=10)
        frame.pack(fill="x", padx=16, pady=6)

        self.lbl_bg = ttk.Label(frame, text="Belum ada file dipilih", foreground="#c0392b")
        self.lbl_bg.pack(anchor="w", pady=(0, 6))

        row = ttk.Frame(frame)
        row.pack(fill="x")
        ttk.Button(row, text="Pilih video latar", command=self.pick_background).pack(side="left")
        ttk.Button(row, text="Hapus", command=self.clear_background).pack(side="left", padx=6)

    def _build_audio_section(self):
        self.audio_frame = ttk.LabelFrame(self.root, text="Trek audio (0)", padding=10)
        self.audio_frame.pack(fill="both", expand=False, padx=16, pady=6)

        list_wrap = ttk.Frame(self.audio_frame)
        list_wrap.pack(fill="x")

        self.audio_listbox = tk.Listbox(list_wrap, height=6, activestyle="dotbox")
        self.audio_listbox.pack(side="left", fill="both", expand=True)

        scroll = ttk.Scrollbar(list_wrap, orient="vertical", command=self.audio_listbox.yview)
        scroll.pack(side="left", fill="y")
        self.audio_listbox.config(yscrollcommand=scroll.set)

        btns = ttk.Frame(self.audio_frame)
        btns.pack(fill="x", pady=(6, 0))
        ttk.Button(btns, text="Tambah audio", command=self.add_audio).pack(side="left")
        ttk.Button(btns, text="Naik", command=lambda: self.move_audio(-1)).pack(side="left", padx=6)
        ttk.Button(btns, text="Turun", command=lambda: self.move_audio(1)).pack(side="left")
        ttk.Button(btns, text="Hapus", command=self.remove_audio).pack(side="left", padx=6)
        ttk.Button(btns, text="Hapus semua", command=self.clear_audio).pack(side="left")

    def _build_intro_section(self):
        frame = ttk.LabelFrame(self.root, text="Video pembuka (opsional)", padding=10)
        frame.pack(fill="x", padx=16, pady=6)

        self.lbl_intro = ttk.Label(
            frame, text="Tidak ada (diputar sekali sebelum loop)", foreground="#888888"
        )
        self.lbl_intro.pack(anchor="w", pady=(0, 6))

        row = ttk.Frame(frame)
        row.pack(fill="x")
        ttk.Button(row, text="Pilih video pembuka", command=self.pick_intro).pack(side="left")
        ttk.Button(row, text="Hapus", command=self.clear_intro).pack(side="left", padx=6)

    def _build_settings_section(self):
        frame = ttk.LabelFrame(self.root, text="Pengaturan", padding=10)
        frame.pack(fill="x", padx=16, pady=6)

        row = ttk.Frame(frame)
        row.pack(fill="x")
        ttk.Label(row, text="Jumlah loop:").pack(side="left")
        ttk.Spinbox(row, from_=1, to=99, width=5, textvariable=self.loop_count).pack(
            side="left", padx=(6, 16)
        )
        ttk.Label(row, text="Nama output:").pack(side="left")
        ttk.Entry(row, textvariable=self.output_name).pack(
            side="left", fill="x", expand=True, padx=(6, 0)
        )

        row2 = ttk.Frame(frame)
        row2.pack(fill="x", pady=(8, 0))
        ttk.Label(row2, text="Folder output:").pack(side="left")
        self.lbl_output = ttk.Label(row2, text="Bawaan: folder skrip", foreground="#888888")
        self.lbl_output.pack(side="left", padx=6)
        ttk.Button(row2, text="Pilih", command=self.pick_output_dir).pack(side="right")

    def _build_actions(self):
        frame = ttk.Frame(self.root)
        frame.pack(fill="x", padx=16, pady=(6, 4))

        self.progress = ttk.Progressbar(frame, mode="determinate", maximum=100)
        self.progress.pack(fill="x", pady=(0, 6))

        self.btn_generate = ttk.Button(frame, text="Buat video", command=self.generate)
        self.btn_generate.pack(fill="x")

        log_frame = ttk.LabelFrame(self.root, text="Log FFmpeg", padding=6)
        log_frame.pack(fill="both", expand=True, padx=16, pady=(4, 14))
        self.log = tk.Text(log_frame, height=9, state="disabled", wrap="none")
        self.log.pack(fill="both", expand=True)

    # -------------------------------------------------------------- pickers ---
    def pick_background(self):
        path = filedialog.askopenfilename(title="Pilih video latar", filetypes=VIDEO_TYPES)
        if path:
            self.background_path = path
            self.lbl_bg.config(text=os.path.basename(path), foreground="#27ae60")
            if not self.output_name.get():
                self.output_name.set(os.path.splitext(os.path.basename(path))[0])

    def clear_background(self):
        self.background_path = ""
        self.lbl_bg.config(text="Belum ada file dipilih", foreground="#c0392b")

    def pick_intro(self):
        path = filedialog.askopenfilename(title="Pilih video pembuka", filetypes=VIDEO_TYPES)
        if path:
            self.intro_path = path
            self.lbl_intro.config(text=os.path.basename(path), foreground="#27ae60")

    def clear_intro(self):
        self.intro_path = ""
        self.lbl_intro.config(text="Tidak ada (diputar sekali sebelum loop)", foreground="#888888")

    def add_audio(self):
        files = filedialog.askopenfilenames(
            title="Pilih satu atau banyak audio", filetypes=AUDIO_TYPES
        )
        for path in files:
            if path not in self.audio_paths:
                self.audio_paths.append(path)
        self.refresh_audio()

    def remove_audio(self):
        selection = self.audio_listbox.curselection()
        if not selection:
            return
        for index in reversed(selection):
            del self.audio_paths[index]
        self.refresh_audio()

    def clear_audio(self):
        self.audio_paths = []
        self.refresh_audio()

    def move_audio(self, delta):
        selection = self.audio_listbox.curselection()
        if not selection:
            return
        index = selection[0]
        target = index + delta
        if 0 <= target < len(self.audio_paths):
            self.audio_paths[index], self.audio_paths[target] = (
                self.audio_paths[target],
                self.audio_paths[index],
            )
            self.refresh_audio()
            self.audio_listbox.selection_set(target)

    def refresh_audio(self):
        self.audio_listbox.delete(0, tk.END)
        for index, path in enumerate(self.audio_paths, start=1):
            self.audio_listbox.insert(tk.END, f"{index:02d}. {os.path.basename(path)}")
        self.audio_frame.config(text=f"Trek audio ({len(self.audio_paths)})")

    def pick_output_dir(self):
        path = filedialog.askdirectory(title="Pilih folder output")
        if path:
            self.output_dir = path
            self.lbl_output.config(text=path, foreground="#27ae60")

    # ------------------------------------------------------------ generation ---
    def generate(self):
        if self._running:
            return
        if not self.background_path:
            messagebox.showerror("Error", "Video latar wajib dipilih!")
            return
        if not self.audio_paths:
            messagebox.showerror("Error", "Tambahkan minimal satu trek audio!")
            return

        output_dir = Path(self.output_dir) if self.output_dir else Path.cwd()
        output_dir.mkdir(parents=True, exist_ok=True)

        name = self.output_name.get().strip()
        base = safe_base_name(name) if name else safe_base_name(Path(self.background_path).stem)
        suffix = f"{int(time.time() * 1000) % 10000:04d}"
        output_path = unique_path(output_dir / f"{base}_{suffix}.mp4")

        self._clear_log()
        self._log("=== Mulai render ===")
        self._log(f"Video latar   : {os.path.basename(self.background_path)}")
        self._log(
            "Video pembuka : "
            + (os.path.basename(self.intro_path) if self.intro_path else "Tidak ada")
        )
        self._log(f"Trek audio    : {len(self.audio_paths)} file")
        self._log(f"Jumlah loop   : {self.loop_count.get()}")
        self._log(f"Output        : {output_path}")
        self._log("")

        self._running = True
        self.btn_generate.config(state="disabled")
        self.progress["value"] = 0

        args = (
            self.background_path,
            self.intro_path or None,
            list(self.audio_paths),
            max(1, self.loop_count.get()),
            output_path,
        )
        threading.Thread(target=self._process, args=args, daemon=True).start()

    def _process(self, background, intro, audio_files, loop_count, output_path: Path):
        temp_dir = Path(tempfile.mkdtemp(prefix="swaloka_merger_"))
        try:
            self._emit("progress", 5)
            normalized = self._normalize_audio(audio_files, temp_dir)

            order = build_order(len(normalized), loop_count)
            playlist_files = [normalized[i] for i in order]
            timestamp_sources = [audio_files[i] for i in order]

            self._emit("progress", 35)
            merged_audio = self._merge_audio(temp_dir, playlist_files)

            self._emit("progress", 60)
            if intro:
                self._merge_with_intro(background, intro, merged_audio, output_path, temp_dir)
            else:
                self._merge_background(background, merged_audio, output_path)

            self._emit("progress", 95)
            self._write_timestamps(output_path, timestamp_sources)

            self._emit("progress", 100)
            self._emit("done", str(output_path))
        except Exception as exc:  # noqa: BLE001 - laporkan ke UI
            self._emit("error", str(exc))
        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)

    # ------------------------------------------------------------- pipeline ---
    def _normalize_audio(self, files, temp_dir: Path) -> list[str]:
        results: list[str] = []
        pending: list[tuple[str, str]] = []
        for index, path in enumerate(files):
            if Path(path).suffix.lower() in AAC_EXTENSIONS:
                results.append(path)
            else:
                out = str(temp_dir / f"normalized_{index}.m4a")
                pending.append((path, out))
                results.append(out)

        if pending:
            args = ["-y"]
            for source, _ in pending:
                args += ["-i", source]
            for index, (_, out) in enumerate(pending):
                args += ["-map", f"{index}:a", *AAC_AUDIO_ARGS, out]
            self._run_ffmpeg(args, "Menormalkan audio")
        else:
            self._emit("log", "Semua audio sudah AAC, normalisasi dilewati.")
        return results

    def _merge_audio(self, temp_dir: Path, playlist_files: list[str]) -> Path:
        concat_file = temp_dir / "audio_concat.txt"
        concat_file.write_text(audio_concat_content(playlist_files), encoding="utf-8")
        merged = temp_dir / "audio_merged.m4a"
        args = [
            "-y", "-f", "concat", "-safe", "0", "-i", str(concat_file),
            "-vn", "-c", "copy", str(merged),
        ]
        self._run_ffmpeg(args, "Menggabung audio playlist")
        return merged

    def _merge_background(self, background, audio: Path, output: Path) -> None:
        args = [
            "-y", "-hwaccel", "auto", "-stream_loop", "-1", "-i", background,
            "-i", str(audio), "-map", "0:v", "-map", "1:a",
            "-c:v", "copy", *COLOR_METADATA_FLAGS, "-c:a", "copy",
            "-shortest", "-movflags", "+faststart", str(output),
        ]
        self._run_ffmpeg(args, "Loop video latar + audio")

    def _merge_with_intro(self, background, intro, audio: Path, output: Path, temp_dir: Path) -> None:
        prelooped = self._preloop_background(background, temp_dir)

        intro_seconds = self._duration(intro)
        background_seconds = self._duration(prelooped)
        audio_seconds = self._duration(str(audio))
        if intro_seconds is None or background_seconds is None or audio_seconds is None:
            raise RuntimeError("gagal membaca durasi media untuk merge intro")

        remaining = audio_seconds - intro_seconds
        background_loops = max(1, math.ceil(remaining / background_seconds))

        concat_file = temp_dir / "video_concat.txt"
        concat_file.write_text(
            video_concat_content(intro, prelooped, background_loops), encoding="utf-8"
        )
        args = [
            "-y", "-hwaccel", "auto", "-f", "concat", "-safe", "0", "-i", str(concat_file),
            "-i", str(audio), "-map", "0:v", "-map", "1:a",
            "-c:v", "copy", *COLOR_METADATA_FLAGS, "-c:a", "copy",
            "-shortest", "-movflags", "+faststart", str(output),
        ]
        self._run_ffmpeg(args, "Susun intro + loop latar")

    def _preloop_background(self, background, temp_dir: Path) -> Path:
        duration = self._duration(background)
        if duration is None or duration <= 0 or duration >= PREPROCESS_MIN_SECONDS:
            return Path(background)

        repeat_count = math.ceil(PREPROCESS_MIN_SECONDS / duration)
        additional_loops = max(0, repeat_count - 1)
        prelooped = temp_dir / "background_preloop.mp4"
        args = [
            "-y", "-hwaccel", "auto", "-stream_loop", str(additional_loops), "-i", background,
            "-map", "0:v:0", "-c:v", "copy", "-an", str(prelooped),
        ]
        self._run_ffmpeg(args, f"Pre-loop latar {additional_loops}x agar mencapai 10 menit")
        return prelooped

    def _write_timestamps(self, output: Path, sources: list[str]) -> None:
        if not sources:
            return
        lines: list[str] = []
        accumulated = 0.0
        for source in sources:
            lines.append(f"{format_clock(accumulated)} - {Path(source).stem}")
            accumulated += self._duration(source) or 0.0
        path = output.parent / f"{output.stem}_timestamp.txt"
        path.write_text("\n".join(lines), encoding="utf-8")
        self._emit("log", f"Timestamp   : {path.name}")

    # ---------------------------------------------------------------- ffmpeg ---
    def _duration(self, path) -> float | None:
        try:
            result = subprocess.run(
                [
                    self.ffprobe, "-v", "error",
                    "-show_entries", "format=duration",
                    "-of", "default=nw=1:nk=1", str(path),
                ],
                capture_output=True, text=True,
            )
            return float(result.stdout.strip())
        except (ValueError, OSError):
            return None

    def _run_ffmpeg(self, args: list[str], label: str) -> None:
        self._emit("log", f"--- {label} ---")
        self._emit("log", "$ " + shlex.join([self.ffmpeg, *args]))
        proc = subprocess.Popen(
            [self.ffmpeg, *args],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        assert proc.stderr is not None
        for line in proc.stderr:
            line = line.rstrip()
            if line:
                self._emit("log", line)
        proc.wait()
        if proc.returncode != 0:
            raise RuntimeError(f"{label} gagal (ffmpeg exit {proc.returncode})")

    # ----------------------------------------------------- queue / ui bridge ---
    def _emit(self, kind: str, payload) -> None:
        self._queue.put((kind, payload))

    def _drain_queue(self) -> None:
        try:
            while True:
                kind, payload = self._queue.get_nowait()
                if kind == "log":
                    self._log(str(payload))
                elif kind == "progress":
                    self.progress["value"] = float(payload)
                elif kind == "done":
                    self._finish(str(payload))
                elif kind == "error":
                    self._fail(str(payload))
        except queue.Empty:
            pass
        self.root.after(100, self._drain_queue)

    def _finish(self, output_path: str) -> None:
        self._running = False
        self.btn_generate.config(state="normal")
        self._log("")
        self._log(f"=== Selesai: {output_path} ===")
        messagebox.showinfo("Sukses", f"Video selesai dibuat:\n\n{output_path}")

    def _fail(self, message: str) -> None:
        self._running = False
        self.btn_generate.config(state="normal")
        self.progress["value"] = 0
        self._log("")
        self._log(f"!!! GAGAL: {message}")
        messagebox.showerror("Gagal", message)

    # ------------------------------------------------------------------ log ---
    def _log(self, message: str) -> None:
        self.log.config(state="normal")
        self.log.insert(tk.END, message + "\n")
        self.log.see(tk.END)
        self.log.config(state="disabled")

    def _clear_log(self) -> None:
        self.log.config(state="normal")
        self.log.delete("1.0", tk.END)
        self.log.config(state="disabled")


if __name__ == "__main__":
    root = tk.Tk()
    LoopingToolApp(root)
    root.mainloop()
