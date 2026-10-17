# Proposal

## Why

Swaloka publishes installers to GitHub Releases, but there is no way to get a new version to
someone who already installed the app: the updater helper (`src/lib/updater.ts`) and the manifest
script (`scripts/latest-json.mjs`) exist but nothing wires them together. The updater plugin is
not installed, `latest.json` is never produced or uploaded, and macOS bundles are unsigned so
Apple Silicon refuses to launch a downloaded bundle. Every release has to be installed by hand.
Release Please already owns the version in `package.json`, `src-tauri/Cargo.toml`, and
`src-tauri/tauri.conf.json`, so the build workflows must not set it a second time.

## What Changes

- Enable Tauri's in-app updater on all three platforms, reading a static `latest.json` manifest
  published as a GitHub Release asset (`releases/latest/download/latest.json`).
- Add the updater and process plugins (Rust + JS), grant their capabilities, and configure
  `plugins.updater` with the minisign public key.
- Produce updater artifacts through `createUpdaterArtifacts`: `.app.tar.gz` + `.sig` on macOS,
  `-setup.exe.sig` on Windows, `.AppImage.sig` on Linux, signed with a private key kept only in
  CI secrets.
- Sign macOS bundles **ad-hoc** (`signingIdentity: "-"`), which satisfies Apple Silicon's
  requirement that a downloaded app be signed. Notarization is out of scope, so Gatekeeper still
  asks for confirmation once.
- Surface updates in the app: a check on startup that opens a dialog (version, release notes,
  progress, "Nanti"/"Perbarui"), plus a manual "Periksa pembaruan" action in settings.
- Build the manifest in the publish job from the per-platform artifacts, keeping each artifact in
  its own folder so the updater file pairs with its `.sig`, and rename the arch-less macOS tarball
  so arm64 and x64 do not collide on the release.
- Leave the version entirely to Release Please: the build workflows no longer set it before
  building, and the now-redundant `scripts/set-version.mjs` is removed.

## Capabilities

### Modified Capabilities

- `app-shell`: adds the in-app update flow — a startup check, the update dialog with version,
  notes, progress and install, and a manual check in settings.

## Impact

- **Code**: `src-tauri/Cargo.toml` and `src/lib.rs` (updater + process plugins); `tauri.conf.json`
  (`createUpdaterArtifacts`, `bundle.macOS.signingIdentity`, `plugins.updater`); `capabilities/default.json`
  (`updater:default`, `process:default`); `package.json` (two JS plugins and a `pack:unsigned`
  script); `src/lib/updater.ts` returns richer info; new `stores/update.svelte.ts` and
  `views/UpdateDialog.svelte`; wiring in `App.svelte` and `views/SettingsDialog.svelte`.
- **CI**: `build-{macos,windows,linux}.yml` sign the updater artifacts and upload the `.sig` files;
  `release.yml` passes secrets to the build workflows, names the Windows/Linux artifacts with the
  `-x64` slug, and generates and uploads `latest.json`. The manual version plumbing — the
  `Set version` step, the `version` inputs, and `scripts/set-version.mjs` — is removed, so Release
  Please is the single owner of the version.
- **Secrets**: `TAURI_SIGNING_PRIVATE_KEY` (and optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) must
  exist as repository secrets; the minisign private key is never committed.
- **Local builds**: with `createUpdaterArtifacts` enabled, `tauri build` fails without the private
  key, so `pnpm pack:unsigned` builds without updater artifacts for local use.
- **Users**: installed apps can update themselves; on macOS the first launch still needs a
  Gatekeeper confirmation because the build is not notarized.
