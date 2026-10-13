# Proposal

## Why

FFmpeg is currently an external prerequisite that each user has to arrange, and the Windows installer papered over that by running a PowerShell script after copying files. That produced three different experiences — a post-install download on Windows only, a system dependency on macOS and Linux — plus a release build that could not find FFmpeg when launched from Finder, because detection never consulted well-known install directories. It also left the FFmpeg version to chance, which this app cannot afford: FFmpeg 9 breaks command forms the looping pipeline relies on, while 8.1.2 is known good.

## What Changes

- The app provisions **its own FFmpeg 8.1.2**, on every platform, into a per-user application folder, driven from the setup screen with a byte-level progress bar so the wait is visible inside the app instead of in a console window.
- Downloads come from **pinned per-platform sources with pinned SHA-256 digests**: the gyan.dev `.7z` essentials build on Windows x64 (33.9 MB, extracted in pure Rust), and the shaka-project static binaries on macOS (arm64/x64) and Linux (x64/arm64) as raw executables (68–96 MB). Pinning 8.1.2 is deliberate and documented at the source table.
- A successful install is verified by running `ffmpeg -version` and requiring an `8.1.x` report, so a wrong or corrupted build is rejected and removed rather than used.
- The download can be **cancelled**, leaves no partial files behind, and reports a retryable error without stranding the app.
- FFmpeg resolution becomes **app-scoped on every platform**: the user's custom path first, then the app's own folder, then the executable's folder and the bundle's resource folder — the system PATH and platform install directories are no longer consulted, so every user runs the same build.
- The Windows installer path is removed: `install-ffmpeg.ps1`, `windows/hooks.nsh`, and `tauri.windows.conf.json` are deleted, and the release workflow needs no installer hook.

## Capabilities

### Modified Capabilities

- `ffmpeg-engine`: executable discovery becomes app-scoped (custom path, then the app's own install folder, then the executable and bundle folders; no PATH fallback), and adds provisioning — pinned per-platform sources, SHA-256 verification, in-Rust archive extraction, version validation, cancellation, and phase/byte progress reporting.
- `app-shell`: the FFmpeg prerequisite gate becomes an in-app setup screen with a download action, progress display, cancel and retry, and the manual install commands kept as a secondary path.

## Impact

- **Code**: new `core/ffmpeg/install.rs` (plan per platform, downloader, extractor, version check, single-run guard); new commands `ffmpeg_status`, `ffmpeg_install`, `ffmpeg_install_cancel`; new event `setup://progress`; `state.rs` puts the app's install folder first among bundled directories and owns the installer guard; new `stores/setup.svelte.ts`; `views/FfmpegGuide.svelte` becomes the setup screen; `api.ts` gains the status and install calls.
- **Runtime dependencies**: `ureq` (rustls + ring, so no OpenSSL and no extra C toolchain), `sha2`, and `sevenz-rust` (pure Rust 7z reader) are added.
- **Removed**: `src-tauri/install-ffmpeg.ps1`, `src-tauri/windows/hooks.nsh`, `src-tauri/tauri.windows.conf.json`.
- **Users**: FFmpeg no longer has to be installed by hand, and an app that already has FFmpeg on PATH now uses the app's own copy instead.
- **Disk**: each user gets a second FFmpeg copy (about 100–180 MB extracted) inside the application's config folder; it is removable by deleting that folder.
