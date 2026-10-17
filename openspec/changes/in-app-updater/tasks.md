# Tasks

## 1. Backend and configuration

- [x] 1.1 Add `tauri-plugin-updater` and `tauri-plugin-process` to `src-tauri/Cargo.toml` and register both in `src-tauri/src/lib.rs`.
- [x] 1.2 Configure `tauri.conf.json`: `bundle.createUpdaterArtifacts: true`, `bundle.macOS.signingIdentity: "-"`, and `plugins.updater` with the minisign public key, the GitHub Releases `latest.json` endpoint, and `windows.installMode: "passive"`.
- [x] 1.3 Grant `updater:default` and `process:default` in `capabilities/default.json`.
- [x] 1.4 Generate the updater signing key (`pnpm tauri signer generate -w ~/.tauri/swaloka.key`) and put the public key in the config; the private key stays out of the repository.

## 2. Frontend

- [x] 2.1 Add the `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process` dependencies and install them.
- [x] 2.2 Extend `src/lib/updater.ts` so `checkUpdate()` returns the version, the release notes, and the date.
- [x] 2.3 Add `stores/update.svelte.ts` (check/install state, silent automatic vs reported manual errors) and `views/UpdateDialog.svelte` (version, notes, progress, "Nanti"/"Perbarui").
- [x] 2.4 Render the dialog in `App.svelte`, run the check after bootstrap, and add the update section with the manual check to `views/SettingsDialog.svelte`.

## 3. Release pipeline

- [x] 3.1 Give the build workflows the signing key secrets and upload the `.app.tar.gz`/`.sig`, `-setup.exe.sig`, and `.AppImage.sig` files.
- [x] 3.2 Rename the macOS updater tarball to include the architecture so arm64 and x64 do not overwrite each other on the release.
- [x] 3.3 In `release.yml`, pass secrets to the build workflows, name the Windows/Linux artifacts with the `-x64` slug, and make the publish job generate `latest.json` and upload every artifact plus the manifest.
- [x] 3.4 Add the `pack:unsigned` script and `src-tauri/tauri.unsigned.conf.json` so local builds work without the private key.

## 4. Verification

- [x] 4.1 `pnpm check` (svelte-check) reports 0 errors and 0 warnings, `pnpm build` succeeds, and `cargo check` in `src-tauri` is clean.
- [ ] 4.2 A local signed build (`TAURI_SIGNING_PRIVATE_KEY` set) produces `.app.tar.gz` and `.sig`, and `codesign -dv` reports an ad-hoc signature.
- [ ] 4.3 An end-to-end update (an older installed build sees a newer `latest.json`, installs, and relaunches on the new version) is exercised once the repository secrets and a published release exist.
