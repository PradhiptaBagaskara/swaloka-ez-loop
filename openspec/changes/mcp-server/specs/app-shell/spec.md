# Spec Delta

## Purpose

The application shell: window and lifecycle, the FFmpeg prerequisite gate, the project
entry flow, the sidebar/topbar workspace chrome, the dashboard home, settings, theme,
notifications, the bottom queue card, and AI (MCP) control.

## MODIFIED Requirements

### Requirement: Settings dialog

The app SHALL provide a settings dialog with the theme, the FFmpeg path, the hardware
encoder (including an "Otomatis" option and a "Deteksi otomatis" action), and, when a
project is open, the project's output folder, and SHALL provide a "Kontrol AI (MCP)"
section with an enable toggle, a loopback port, and, while enabled, the endpoint and
token to copy.

#### Scenario: Save settings

- **WHEN** the user saves the dialog
- **THEN** the theme, FFmpeg path, hardware encoder, project output folder, and the AI control choice and port are persisted, preferences are reloaded, FFmpeg is re-checked, and the app reports "Pengaturan disimpan"

#### Scenario: Encoder survives save

- **WHEN** the user selects an encoder and saves
- **THEN** the saved value is the selected encoder, and reopening the dialog shows that same encoder

#### Scenario: Detect encoder

- **WHEN** the user runs "Deteksi otomatis"
- **THEN** the detected encoder value is selected in the dropdown

#### Scenario: Enable AI control

- **WHEN** the user enables "Kontrol AI (MCP)" and saves
- **THEN** the MCP server starts on the configured loopback port and the endpoint and token are shown for copying

#### Scenario: Disable AI control

- **WHEN** the user disables "Kontrol AI (MCP)" and saves
- **THEN** the MCP server stops and the endpoint and token are no longer shown

## ADDED Requirements

### Requirement: AI control indicator

The workspace SHALL indicate while AI (MCP) control is active, and the queue card SHALL
show AI-initiated runs identically to manual ones.

#### Scenario: Indicator while active

- **WHEN** AI control is enabled and the workspace is shown
- **THEN** an indicator states that AI control is active

#### Scenario: AI run in the queue card

- **WHEN** a run was started through the MCP server
- **THEN** the queue card shows its stage label, progress, elapsed time, and streamed logs exactly as for a manual run

#### Scenario: Single active run across both

- **WHEN** a run is active and the other side — agent or user — starts another
- **THEN** the second is rejected with "Ada proses lain yang sedang berjalan"
