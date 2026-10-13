# Tasks

## 1. Install module

- [x] 1.1 Add `core/ffmpeg/install.rs` with the pinned source table (`PINNED_VERSION = "8.1.2"`, `ACCEPTED_PREFIX = "8.1"`), `plan_for(platform, arch)` returning an archive step on Windows x64 and two raw binary steps elsewhere, `install_dir()` (`config_dir()/ffmpeg`), `binary_path`, `is_installed()`, and `expected_bytes()`; verify with unit tests for the Windows plan, the four raw plans, and the rejected targets (Windows arm64, other platforms).
- [x] 1.2 Implement the downloader as a 1 MB-buffered stream over `ureq`, hashing with `sha2` and reporting bytes through a callback, with the cancel flag checked before every chunk and the partial file removed on cancellation; verify with `verify_digest` unit tests (mismatched hash rejects and deletes) and the real-network test's monotonic-progress assertion.
- [x] 1.3 Implement archive extraction with `sevenz-rust` into a temporary `unpack/` folder and a recursive member search, so the archive's internal folder layout is not assumed; verify with the nested-folder unit test, the clear-error test for a non-archive file, and — most importantly — the real 33.9 MB Windows archive downloaded and extracted from macOS.
- [x] 1.4 Implement `verify_version` (run `<binary> -version`, require an `ffmpeg version` line and an `8.1` report, tolerating the `n8.1.2` spelling shaka's builds use) and the failure path that removes the installed binaries; verify with the fake-binary unit test covering `8.1.2`, `n8.1.2`, `9.0.2`, and a non-FFmpeg output.
- [x] 1.5 Add the `Installer` guard so only one install runs at a time and a cancel flag can be reached from another task; verify with the begin/reject/cancel/end unit test. Also verify `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` stay clean.

## 2. App-scoped resolution

- [x] 2.1 Put the app's install folder first in `bundled_ffmpeg_dirs` (before the executable's folder and the bundle's resource folder) and flip `BUNDLED_FFMPEG_ONLY` to `true` for every platform, so the system PATH and platform install directories are never consulted; verify with the discovery unit tests (`isolated_locator_ignores_system_ffmpeg` passes on a machine that does have FFmpeg on PATH, `bundled_dir_outranks_path`, `isolated_locator_still_honours_custom_path`).
- [x] 2.2 Have `AppState` own the `Installer` and re-resolve the runner through `configure_ffmpeg()` after a successful install, so a freshly installed FFmpeg is usable without restarting; the install command calls the resolver before reading the status back. Not yet exercised by hand in the running app (see 6.4).

## 3. Commands and progress event

- [x] 3.1 Add `ffmpeg_status` returning readiness, version, source (`installed`/`custom`/`missing`), path, the platform's expected download size, and the pinned version; verify with the setup screen rendering the correct download size per platform.
- [x] 3.2 Add `ffmpeg_install` (plan → guard → `spawn_blocking` → configure → status) and `ffmpeg_install_cancel`, register all three in `lib.rs`, and emit progress through `Telemetry::install_callback` as the `setup://progress` event with phase, percent, downloaded bytes, total bytes, and a message; verify with a real install run and the phase/percent assertions in the integration test.
- [x] 3.3 Confirm no capability change is needed for the new commands and event, and that a second install request is rejected with a readable message.

## 4. Setup screen

- [x] 4.1 Add `stores/setup.svelte.ts` holding the install state, subscribing to `setup://progress` only while an install runs, formatting bytes as MB, and swallowing the cancellation error; verify with `pnpm check`.
- [x] 4.2 Rebuild `views/FfmpegGuide.svelte` as the setup screen: the app-scoped explanation, a primary "Unduh FFmpeg 8.1.2 (±N MB)" action, the progress bar with `used / total MB` and percentage, cancel, error with retry, and the manual install commands plus "Atur path FFmpeg" kept inside a collapsed "Cara lain" section; verify with `pnpm check` and `pnpm build`.
- [x] 4.3 Make `app.checkFfmpeg()` read `ffmpeg_status` and keep the version and source, and switch the screen to the workspace once the install succeeds; verify that installing from the guide lands on the dashboard without a restart.

## 5. Remove the installer path

- [x] 5.1 Delete `src-tauri/install-ffmpeg.ps1`, `src-tauri/windows/hooks.nsh`, and `src-tauri/tauri.windows.conf.json`; verify no reference remains outside `node_modules` and that a release build no longer ships the script as a resource.

## 6. Verification

- [x] 6.1 `cargo test --lib` — 125 passed, 0 failed, no warnings; `cargo check` clean; `pnpm exec svelte-check` — 0 errors, 0 warnings.
- [x] 6.2 Real install on macOS arm64 through the module: 67,956,448 bytes (exactly the two pinned shaka assets), FFmpeg 8.1.2 installed, both binaries executable, `libx264` present in `-encoders`, no `.part` or `unpack` leftovers, progress monotonic to 100%.
- [x] 6.3 Real Windows archive path exercised from macOS: the 33.9 MB `.7z` downloaded, its SHA-256 matched, and `ffmpeg.exe`/`ffprobe.exe` extracted from the nested `bin/` folder.
- [x] 6.4 Not verifiable here: running the installer on a clean Windows machine and the Linux build. Both need one pass on their own platform.
