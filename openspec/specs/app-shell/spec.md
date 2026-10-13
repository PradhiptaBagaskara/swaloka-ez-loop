# app-shell Specification

## Purpose

The desktop application shell: window and lifecycle, the FFmpeg prerequisite gate, the project entry flow, the sidebar/topbar workspace chrome, the dashboard home, settings, theme, and the bottom queue card that streams run progress and logs.

## Requirements

### Requirement: Window and startup

The app SHALL open a single resizable window labelled `main`, titled "Swaloka Looping Tool", sized 1440×900 with a minimum of 1100×700, and SHALL disable the OS-level drag-and-drop handler so in-webview HTML5 drag and drop works.

#### Scenario: First paint

- **WHEN** the app starts
- **THEN** it reads preferences, the app version, the current project, and the FFmpeg status before deciding which root screen to show

#### Scenario: Loading state

- **WHEN** the FFmpeg check has not finished yet
- **THEN** the shell shows the centered text "Menyiapkan Swaloka..."

### Requirement: FFmpeg prerequisite gate

The app SHALL verify FFmpeg before letting the user work, SHALL resolve the executable from the user's custom path first, then the system PATH, then well-known install directories, SHALL validate a candidate by running it with `-version` and requiring exit code 0, and SHALL block the workspace with an install guide until FFmpeg is found.

#### Scenario: FFmpeg present

- **WHEN** a valid FFmpeg executable is resolved
- **THEN** the workspace is shown

#### Scenario: FFmpeg missing

- **WHEN** no valid FFmpeg executable can be resolved
- **THEN** the install guide is shown with the platform-appropriate install commands and a "Periksa lagi" action

#### Scenario: Still missing after re-check

- **WHEN** the user re-checks and FFmpeg is still not found
- **THEN** the app reports "FFmpeg masih tak ditemukan. Periksa PATH lalu coba lagi."

#### Scenario: Custom path

- **WHEN** the user sets a custom FFmpeg path in settings
- **THEN** only that path is used for the availability check and the runner is reconfigured to it

### Requirement: Project entry flow

The app SHALL present a landing screen while no project is open, offering "Buat proyek" (pick a folder, then create) and "Buka proyek" (pick an existing project folder, then open), and SHALL remember recently opened projects.

#### Scenario: Create a project

- **WHEN** the user picks a folder for a new project
- **THEN** the project is created with the folder's base name, the dashboard opens, and the app reports "Proyek dibuat"

#### Scenario: Open a project

- **WHEN** the user opens an existing project folder
- **THEN** the project loads, the dashboard opens, and the app reports "Proyek dibuka"

#### Scenario: Recent project row

- **WHEN** the user clicks anywhere on a row in "Proyek terkini"
- **THEN** that project opens; there is no separate "Buka" button

#### Scenario: Remove a recent project

- **WHEN** the user clicks the row's remove control
- **THEN** the entry is removed from the recent list without opening the project

#### Scenario: Recent list limit

- **WHEN** more than five projects have been opened
- **THEN** only the five most recent remain, most recent first

### Requirement: Workspace chrome

The workspace SHALL show a sidebar, a topbar, a scrollable main area, and a queue card pinned at the bottom.

#### Scenario: Sidebar navigation

- **WHEN** the sidebar is rendered
- **THEN** it lists the "Ruang Kerja" entries Dasbor, EZ Loop, Looping, and Media Tools, and pins the "Sistem" actions Pengaturan, Bersihkan cache, and Dukungan at the bottom, separated from the workspace entries

#### Scenario: Active entry

- **WHEN** a workspace entry matches the current view
- **THEN** that entry is highlighted

#### Scenario: Topbar context

- **WHEN** a project is open
- **THEN** the topbar shows the project name, a "Buka folder" action for the project root, a theme toggle, and a "Tutup proyek" action

#### Scenario: Clean the cache

- **WHEN** the user runs "Bersihkan cache"
- **THEN** the app clears cached audio and log files and reports how many files were removed

### Requirement: Dashboard home

The dashboard SHALL summarize the project with exactly three stat cards — "Output selesai", "File cache", and "FFmpeg" (value "Siap" or "Bermasalah") — and SHALL offer quick-start cards for Looping and Media Tools.

#### Scenario: Stats refresh

- **WHEN** the dashboard is opened, a run finishes, or the project's output location changes
- **THEN** the statistics are reloaded

#### Scenario: No project

- **WHEN** no project is open
- **THEN** the statistics report zero outputs, zero cache files, and FFmpeg unavailable without touching disk

### Requirement: Settings dialog

The app SHALL provide a settings dialog with the theme, the FFmpeg path, the hardware encoder (including an "Otomatis" option and a "Deteksi otomatis" action), and, when a project is open, the project's output folder.

#### Scenario: Save settings

- **WHEN** the user saves the dialog
- **THEN** the theme, FFmpeg path, hardware encoder, and project output folder are persisted, preferences are reloaded, FFmpeg is re-checked, and the app reports "Pengaturan disimpan"

#### Scenario: Encoder survives save

- **WHEN** the user selects an encoder and saves
- **THEN** the saved value is the selected encoder, and reopening the dialog shows that same encoder

#### Scenario: Detect encoder

- **WHEN** the user runs "Deteksi otomatis"
- **THEN** the detected encoder value is selected in the dropdown

### Requirement: Theme

The app SHALL support a light and a dark theme, selected in settings, persisted with the preferences, and applied by toggling the `dark` class on the document root.

#### Scenario: Toggle theme

- **WHEN** the user toggles the theme from the topbar
- **THEN** the interface switches immediately and the choice is persisted

### Requirement: Notifications

The app SHALL show transient notifications as a single banner centered at the top of the window, styled success or danger, that disappears on its own.

#### Scenario: Notification appears

- **WHEN** an action reports success or failure
- **THEN** the banner appears at the top center of the window

#### Scenario: Notification clears

- **WHEN** about three seconds pass
- **THEN** the banner disappears without user action

### Requirement: Queue card and progress reporting

The app SHALL show a persistent bottom card describing the current or last run: a status indicator, a label, a progress bar with percentage and elapsed time while relevant, cancellation while running, a way to reveal the output when a run produced a file, and a togglable streaming log panel.

#### Scenario: Idle

- **WHEN** no run has happened
- **THEN** the label reads "Belum ada proses" and the log toggle is disabled

#### Scenario: Running

- **WHEN** a run is in progress
- **THEN** the card shows the current stage label, the progress bar, the percentage, the elapsed duration, and a "Batalkan proses" action

#### Scenario: Staged labels

- **WHEN** a run reports progress values
- **THEN** the stage label is the label of the highest configured threshold at or below the reported value

#### Scenario: Finished

- **WHEN** a run finishes
- **THEN** the label summarizes the result and "Buka folder output" is offered when the run produced a file

#### Scenario: Logs

- **WHEN** the user opens the log panel
- **THEN** the streamed log lines are shown newest-last with per-level icons, oldest lines are dropped beyond the on-screen limit, and any dropped count is reported

### Requirement: Single active run

The app SHALL allow at most one run at a time and SHALL reject a second run with "Ada proses lain yang sedang berjalan".

#### Scenario: Reject concurrent run

- **WHEN** a run is started while another run is still active
- **THEN** the second run is rejected and the first continues unaffected
