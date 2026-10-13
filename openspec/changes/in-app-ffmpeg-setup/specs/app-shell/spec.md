# Spec Delta

## Purpose

The application shell: window and lifecycle, the FFmpeg setup gate, project entry, workspace chrome, dashboard, settings, theme, notifications, and the bottom queue card.

## MODIFIED Requirements

### Requirement: FFmpeg prerequisite gate

The app SHALL verify that its own FFmpeg and FFprobe are usable before letting the user work, SHALL resolve the executable from the user's custom path first and then from the app's own install folder, SHALL validate a candidate by running it with `-version` and requiring exit code 0, and SHALL block the workspace with a setup screen until FFmpeg is found. The setup screen SHALL offer to download the app's own FFmpeg build (naming the pinned version and the size for the running platform), SHALL show download progress in megabytes, SHALL allow the download to be cancelled, SHALL offer a retry after a failure, and SHALL keep the platform-appropriate manual install commands and the custom-path action available as a secondary path. The app SHALL NOT fall back to a system FFmpeg installation.

#### Scenario: FFmpeg present

- **WHEN** the app's own FFmpeg and FFprobe resolve and validate
- **THEN** the workspace is shown and the version and source are kept for display

#### Scenario: FFmpeg missing

- **WHEN** the app's own copy is absent and no custom path is set
- **THEN** the setup screen explains that the app uses its own FFmpeg build and offers the download action with the expected size

#### Scenario: Still missing after re-check

- **WHEN** the user re-checks and FFmpeg is still unavailable
- **THEN** the app reports "FFmpeg masih belum terdeteksi."

#### Scenario: Custom path

- **WHEN** the user sets a custom FFmpeg path in settings
- **THEN** only that path is used for the availability check and the runner is reconfigured to it

#### Scenario: Download progress

- **WHEN** a download is running
- **THEN** the screen shows the current phase message, a progress bar, the transferred and total megabytes, the percentage, and a cancel action

#### Scenario: Download completes

- **WHEN** the download and verification succeed
- **THEN** FFmpeg is re-checked, the app reports the installed version, and the workspace opens without restarting the app

#### Scenario: Download fails

- **WHEN** the download fails
- **THEN** the failure is shown inline and the action becomes a retry, with the manual install commands still reachable

#### Scenario: Download cancelled

- **WHEN** the user cancels the download
- **THEN** the cancellation is not reported as a failure and the download action is available again

#### Scenario: Unsupported platform

- **WHEN** no pinned source exists for the running platform
- **THEN** the screen states that automatic download is unavailable and opens the manual section by default

#### Scenario: Configured path unusable

- **WHEN** a custom FFmpeg path is configured but does not validate
- **THEN** the setup screen says the configured path cannot be used
