/// State pembaruan aplikasi: cek endpoint dan pasang versi baru.

import { errorMessage } from "../api";
import { checkUpdate, installUpdate, type UpdateInfo } from "../updater";
import { app } from "./app.svelte";

class UpdateStore {
  /** Pembaruan yang ditemukan dan menunggu dipasang. */
  available = $state<UpdateInfo | null>(null);
  /** Apakah dialog pembaruan sedang terbuka. */
  open = $state(false);
  checking = $state(false);
  installing = $state(false);
  percent = $state(0);
  error = $state<string | null>(null);

  /**
   * Periksa endpoint pembaruan. Pemeriksaan otomatis (`manual = false`)
   * menyembunyikan error karena di mode dev tak ada bundle sehingga updater
   * memang gagal; pemeriksaan manual selalu melaporkan hasilnya lewat toast.
   */
  async check(manual = false): Promise<boolean> {
    if (this.checking || this.installing) return false;

    this.checking = true;
    this.error = null;
    try {
      this.available = await checkUpdate();
      if (this.available) {
        this.open = true;
      } else if (manual) {
        app.showToast("Sudah versi terbaru");
      }
      return this.available != null;
    } catch (error) {
      if (manual) app.showToast(`Gagal memeriksa pembaruan: ${errorMessage(error)}`, "danger");
      return false;
    } finally {
      this.checking = false;
    }
  }

  /** Unduh lalu pasang pembaruan tertunda; aplikasi dijalankan ulang setelahnya. */
  async install() {
    if (!this.available || this.installing) return;

    this.installing = true;
    this.error = null;
    this.percent = 0;
    try {
      await installUpdate((percent) => {
        this.percent = percent;
      });
    } catch (error) {
      this.error = errorMessage(error);
      this.installing = false;
    }
  }
}

export const update = new UpdateStore();
