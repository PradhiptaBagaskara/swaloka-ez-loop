# Spec Delta

## Purpose

The application shell: window and lifecycle, the FFmpeg prerequisite gate, the project entry flow,
the sidebar/topbar workspace chrome, the dashboard home, settings, theme, notifications, the bottom
queue card, and how the interface stays in sync with changes made outside it (the AI/MCP path).

## MODIFIED Requirements

### Requirement: Queue card and progress reporting

The app SHALL show a persistent bottom card describing the current or last run — including a run
started outside the interface, such as one started through the MCP server — with a status
indicator, a label, a progress bar with percentage and elapsed time while relevant, cancellation
while running, a way to reveal the output when a run produced a file, and a togglable streaming log
panel. The card SHALL reflect run events regardless of which side started the run, and SHALL take
the label of an externally started run from the run itself.

#### Scenario: Idle

- **WHEN** no run has happened
- **THEN** the label reads "Belum ada proses" and the log toggle is disabled

#### Scenario: Running

- **WHEN** a run started from the interface is in progress
- **THEN** the card shows the current stage label, the progress bar, the percentage, the elapsed duration, and a "Batalkan proses" action

#### Scenario: External run

- **WHEN** a run is started outside the interface, for example through the MCP server
- **THEN** the card shows it as running with its label, progress, and streamed logs, without the user having started anything

#### Scenario: Cancel an external run

- **WHEN** the user cancels while a run started outside the interface is active
- **THEN** the run is stopped and the card reports it

#### Scenario: Staged labels

- **WHEN** a run reports progress values
- **THEN** the stage label is the label of the highest configured threshold at or below the reported value

#### Scenario: Finished

- **WHEN** a run finishes
- **THEN** the label summarizes the result and "Buka folder output" is offered when the run produced a file

#### Scenario: Logs

- **WHEN** the user opens the log panel
- **THEN** the streamed log lines are shown newest-last with per-level icons, oldest lines are dropped beyond the on-screen limit, and any dropped count is reported

## ADDED Requirements

### Requirement: Refresh on external change

The interface SHALL update the data it displays when that data changes outside the interface — a
channel created, changed, or deleted through the MCP server, or a project opened or closed — rather
than waiting for the user to navigate. The backend SHALL announce such changes with the scope they
affect, and the view SHALL reload the data for that scope when it receives an announcement.

#### Scenario: Channel changed elsewhere

- **WHEN** a channel is created, changed, or deleted through the MCP server while the EZ Loop view is open
- **THEN** the channel list reflects it without the user changing menus

#### Scenario: Project opened elsewhere

- **WHEN** a project is opened or closed through the MCP server
- **THEN** the shell shows the matching screen — the workspace or the landing screen — without a restart

#### Scenario: Scoped reload

- **WHEN** a change is announced
- **THEN** only the data for the announced scope is reloaded

#### Scenario: Interface-initiated change

- **WHEN** the change originates from the interface itself
- **THEN** the interface keeps its current behavior and does not flicker
