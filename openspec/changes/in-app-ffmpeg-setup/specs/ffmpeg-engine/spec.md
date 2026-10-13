# Spec Delta

## Purpose

The FFmpeg/FFprobe engine: how the app resolves its own FFmpeg build on every platform, how that build is provisioned and verified, plus the existing execution, metadata, validation, and estimation surfaces.

## MODIFIED Requirements

### Requirement: Executable discovery

The app SHALL resolve FFmpeg from the user's custom path first, then the folder the app installs its own copy into, then the running executable's own folder, then the bundle's resource folder, in that order and without duplicates. The app SHALL NOT consult the system PATH or platform install directories, so that every user runs the build the app provisioned rather than whatever happens to be installed on their machine. FFprobe SHALL be resolved from a sibling of the resolved FFmpeg when present, else by its bare name. The bare command name remains the runner's placeholder when nothing resolves, but availability is still reported as unavailable.

#### Scenario: Custom path wins

- **WHEN** a custom FFmpeg path is configured
- **THEN** it is used for the availability check and the runner, ahead of the app's own copy

#### Scenario: Sibling probe

- **WHEN** the resolved FFmpeg has a parent directory containing an FFprobe executable
- **THEN** that sibling is used as FFprobe

#### Scenario: Fallback

- **WHEN** no candidate exists
- **THEN** the bare command names `ffmpeg` and `ffprobe` are used

#### Scenario: The app's own copy is the default

- **WHEN** no custom path is set and the app's install folder contains FFmpeg
- **THEN** that copy and its sibling FFprobe are used, even when a different FFmpeg is reachable through the system PATH

#### Scenario: No app copy installed

- **WHEN** no custom path is set and the app's install folder has no FFmpeg
- **THEN** FFmpeg is reported unavailable and the setup screen is shown, rather than falling back to a system installation

## ADDED Requirements

### Requirement: Bundled FFmpeg provisioning

The app SHALL provision FFmpeg **8.1.2** into a per-user folder of its own (`ffmpeg/` inside the application config directory), choosing a source pinned per platform and architecture, and SHALL NOT depend on a system package manager, an installer hook, or an external extraction tool for this. The pinned version is part of the requirement: FFmpeg 9 breaks command forms the looping pipeline relies on.

#### Scenario: Per-platform sources

- **WHEN** provisioning starts
- **THEN** Windows x64 downloads the pinned gyan.dev `.7z` essentials archive, and macOS and Linux download the pinned shaka-project static executables matching the running architecture

#### Scenario: Integrity verification

- **WHEN** a downloaded file completes
- **THEN** its SHA-256 digest is compared against the pinned digest, and a mismatch discards the file and reports a retryable error

#### Scenario: Archive extraction

- **WHEN** the source is an archive
- **THEN** it is extracted in-process and the required executables are located by name at any depth, so the archive's internal folder layout is not assumed

#### Scenario: Version validation

- **WHEN** provisioning has written both binaries
- **THEN** the app runs `ffmpeg -version` and requires an `ffmpeg version` line reporting `8.1.x`, otherwise the installed files are removed and the failure is reported

#### Scenario: Unsupported target

- **WHEN** the running platform and architecture have no pinned source
- **THEN** provisioning is refused with an explanatory message and the manual path in settings remains the way forward

#### Scenario: Progress reporting

- **WHEN** provisioning runs
- **THEN** it reports phases (`download`, `extract`, `verify`, `done`) with a 0..1 fraction and the downloaded and total byte counts, so the interface can show MB figures

#### Scenario: Cancellation

- **WHEN** the user cancels provisioning
- **THEN** the download stops, partial files and temporary folders are removed, and the app stays usable with the setup screen showing its retry action

#### Scenario: Single install at a time

- **WHEN** a provisioning run is requested while another is still active
- **THEN** the second request is rejected with a readable message

#### Scenario: Idempotent result

- **WHEN** a complete, valid pair of binaries is already present in the app's install folder
- **THEN** a repeated provisioning run reuses them without downloading again
