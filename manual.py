import os
import tkinter as tk
from tkinter import filedialog, messagebox


class VideoAudioApp:
    def __init__(self, root):
        self.root = root
        self.root.title("Video & Audio Selector")
        self.root.geometry("500x420")

        # Variabel untuk menyimpan path file
        self.video_bg_path = ""
        self.video_intro_path = ""
        self.audio_paths = []

        # --- TAMPILAN UI ---

        # 1. Bagian Video Background (Wajib)
        self.lbl_bg = tk.Label(
            root, text="1. Video Background (Wajib):", font=("Arial", 10, "bold")
        )
        self.lbl_bg.pack(anchor="w", padx=20, pady=(15, 2))

        self.btn_bg = tk.Button(
            root,
            text="Pilih Video Background (.mp4)",
            command=self.pilih_video_bg,
            bg="#ffcccb",
        )
        self.btn_bg.pack(fill="x", padx=20)

        self.txt_bg = tk.Label(
            root,
            text="Belum ada file yang dipilih",
            fg="red",
            wraplength=450,
            justify="left",
        )
        self.txt_bg.pack(anchor="w", padx=20, pady=(2, 10))

        # 2. Bagian Video Intro (Opsional)
        self.lbl_intro = tk.Label(
            root, text="2. Video Intro (Opsional):", font=("Arial", 10, "bold")
        )
        self.lbl_intro.pack(anchor="w", padx=20, pady=(5, 2))

        self.btn_intro = tk.Button(
            root, text="Pilih Video Intro (.mp4)", command=self.pilih_video_intro
        )
        self.btn_intro.pack(fill="x", padx=20)

        self.txt_intro = tk.Label(
            root,
            text="Tidak ada (Akan dilewati)",
            fg="gray",
            wraplength=450,
            justify="left",
        )
        self.txt_intro.pack(anchor="w", padx=20, pady=(2, 10))

        # 3. Bagian Multiselect Audio
        self.lbl_audio = tk.Label(
            root, text="3. Audio (Bisa Pilih Banyak):", font=("Arial", 10, "bold")
        )
        self.lbl_audio.pack(anchor="w", padx=20, pady=(5, 2))

        self.btn_audio = tk.Button(
            root, text="Pilih File Audio (.mp3 / .wav)", command=self.pilih_audio
        )
        self.btn_audio.pack(fill="x", padx=20)

        # Menggunakan Listbox untuk menampilkan banyak audio yang dipilih
        self.listbox_audio = tk.Listbox(root, height=4)
        self.listbox_audio.pack(fill="x", padx=20, pady=(5, 10))

        # 4. Tombol Proses Akhir
        self.btn_proses = tk.Button(
            root,
            text="PROSES SEKARANG",
            command=self.proses_data,
            font=("Arial", 11, "bold"),
            bg="#4CAF50",
            fg="white",
        )
        self.btn_proses.pack(fill="x", padx=20, py=15)

    # --- FUNGSI / LOGIKA UI ---

    def pilih_video_bg(self):
        file = filedialog.askopenfilename(
            title="Pilih Video Background (Wajib)", filetypes=[("Video MP4", "*.mp4")]
        )
        if file:
            self.video_bg_path = file
            self.txt_bg.config(text=os.path.basename(file), fg="green")

    def pilih_video_intro(self):
        file = filedialog.askopenfilename(
            title="Pilih Video Intro (Opsional)", filetypes=[("Video MP4", "*.mp4")]
        )
        if file:
            self.video_intro_path = file
            self.txt_intro.config(text=os.path.basename(file), fg="green")
        else:
            self.video_intro_path = ""
            self.txt_intro.config(text="Tidak ada (Akan dilewati)", fg="gray")

    def pilih_audio(self):
        files = filedialog.askopenfilenames(
            title="Pilih Satu atau Banyak Audio",
            filetypes=[("File Audio", "*.mp3 *.wav")],
        )
        if files:
            self.audio_paths = list(files)
            self.listbox_audio.delete(0, tk.END)  # Bersihkan list lama
            for f in self.audio_paths:
                self.listbox_audio.insert(
                    tk.END, os.path.basename(f)
                )  # Tampilkan nama file saja di listbox

    def proses_data(self):
        if not self.video_bg_path:
            messagebox.showerror("Error", "Video Background wajib dipilih!")
            return

        intro_status = (
            self.video_intro_path if self.video_intro_path else "Tidak digunakan"
        )
        jumlah_audio = len(self.audio_paths)

        hasil_pesan = (
            f"Data Siap Diproses:\n\n"
            f"• Background: {os.path.basename(self.video_bg_path)}\n"
            f"• Intro: {os.path.basename(self.video_intro_path) if self.video_intro_path else 'Tidak Ada'}\n"
            f"• Jumlah Audio: {jumlah_audio} file"
        )

        messagebox.showinfo("Sukses", hasil_pesan)

        print("Path Background:", self.video_bg_path)
        print("Path Intro:", self.video_intro_path)
        print("Path Audio:", self.audio_paths)


if __name__ == "__main__":
    root = tk.Tk()
    app = VideoAudioApp(root)
    root.mainloop()
