# project-storage Specification

## Purpose

Project and preference persistence: the project file model and on-disk folder layout, the one-time legacy project import, recent projects, application preferences, the normalized-audio cache, temp and log housekeeping, and the dashboard statistics.

## Requirements

### Requirement: Project file

The app SHALL persist a project as `project.json` (format version 2) inside the project root, storing the project name, root path, optional custom output path, a parallel-processing flag, and the format version, and SHALL tolerate missing or empty values when reading.

#### Scenario: Round trip

- **WHEN** a project is saved and loaded again
- **THEN** the name, root path, output path, and format version are preserved

#### Scenario: Lenient read

- **WHEN** a stored project omits a field or stores an empty string
- **THEN** the app falls back to the root path, the folder's base name, "Project", or the documented defaults instead of failing

### Requirement: Project folder layout

Opening or creating a project SHALL ensure the standard folders exist: `outputs/`, `logs/`, `temp/`, `cache/`, `cache/audios/`, and `channels/`, and SHALL also ensure the custom output folder when one is configured.

#### Scenario: New project

- **WHEN** a project folder is created
- **THEN** the standard subfolders are created inside it

#### Scenario: Older project

- **WHEN** a project created before a folder existed is opened
- **THEN** the missing folder is created without touching existing files

#### Scenario: Custom output

- **WHEN** a custom output path is configured
- **THEN** outputs are written there and the catch-all `outputs/` folder still exists

### Requirement: Legacy project import

The app SHALL import an old `project.swaloka` file when no `project.json` exists, by reading its name, output path, and parallel-processing flag, writing a format-2 `project.json` next to it, and never modifying or deleting the legacy file.

#### Scenario: Import a legacy project

- **WHEN** a project folder contains only `project.swaloka`
- **THEN** a `project.json` is written and the legacy file is left untouched

#### Scenario: Invalid legacy file

- **WHEN** the legacy file is missing required fields or cannot be parsed
- **THEN** the import fails with an error and the original file is unchanged

### Requirement: Preferences

The app SHALL store user preferences in a per-user `preferences.json`, keeping the legacy snake_case keys — `theme`, `ffmpeg_path`, `hw_encoder`, `last_project_path`, `recent_projects` — and SHALL fall back to defaults when the file is missing or unreadable.

#### Scenario: Preference defaults

- **WHEN** no preferences file exists
- **THEN** the app uses theme "light", no FFmpeg path, no hardware encoder, no last project, and an empty recent list

#### Scenario: Unreadable preferences

- **WHEN** the preferences file cannot be parsed
- **THEN** defaults are used instead of failing to start

#### Scenario: Saving preferences

- **WHEN** preferences are saved
- **THEN** they are written to disk, kept in memory, and the FFmpeg runner is reconfigured from the saved FFmpeg path

### Requirement: Recent projects

The app SHALL keep at most five recent projects, most recent first, SHALL move an existing entry to the front instead of duplicating it, SHALL set the last project path when a project is opened or created, and SHALL support removing an entry.

#### Scenario: Add a recent project

- **WHEN** a project is opened or created
- **THEN** its path moves to the front of the recent list and the list is capped at five

#### Scenario: Remove a recent project

- **WHEN** a recent entry is removed
- **THEN** it no longer appears in the list

### Requirement: Normalized-audio cache

The app SHALL cache normalized audio inside `cache/audios/`, described by a `cache/audio_cache.json` manifest that maps a source identity to the cached file, SHALL identify a source by its resolved absolute path (case-insensitively on macOS and Windows), SHALL name cached files from a sanitized source stem plus a stable hash of that identity, and SHALL reuse a cached copy only when it still exists on disk.

#### Scenario: Reuse a cached file

- **WHEN** the same source is normalized again and its cached file still exists
- **THEN** the cached file is reused without re-encoding

#### Scenario: Missing cached file

- **WHEN** the manifest references a file that no longer exists
- **THEN** the source is normalized again

#### Scenario: Case-insensitive identity

- **WHEN** the same source path differs only in letter case on macOS or Windows
- **THEN** it maps to the same cache entry

### Requirement: Cache clearing

The app SHALL clear the cache by deleting the files inside `cache/audios/`, deleting the cache manifest, and deleting the files inside `logs/`, SHALL count how many audio and log files were removed, and SHALL leave the cache directory itself and the `outputs/` and `temp/` folders in place.

#### Scenario: Clear the cache

- **WHEN** the user runs "Bersihkan cache"
- **THEN** the cached audio and log files are deleted, the cache directory remains, and the count of removed files is reported

### Requirement: Temp and log housekeeping

The app SHALL create a uniquely named temp folder per working operation and remove it when the operation ends, SHALL remove stale temp entries when a project is closed, and SHALL write one log file per run named from the run's base name and the local start time, skipping the write when the run produced no log records.

#### Scenario: Unique temp folder

- **WHEN** a temp folder is requested with a prefix that already exists
- **THEN** a distinct folder name is produced

#### Scenario: Cleanup after an operation

- **WHEN** an operation that used a temp folder finishes
- **THEN** that temp folder is removed

#### Scenario: Stale temp cleanup

- **WHEN** a project is closed
- **THEN** whatever was left in `temp/` is removed

#### Scenario: Empty run

- **WHEN** a run ends without any log records
- **THEN** no log file is written

### Requirement: Project statistics

The app SHALL report the number of files directly in the output folder, the number of files directly in the audio cache folder, and whether FFmpeg is available, returning zeroes and unavailable when no project is open or a folder cannot be read.

#### Scenario: Stats with a project

- **WHEN** statistics are requested while a project is open
- **THEN** the output count, cache count, and FFmpeg readiness are returned

#### Scenario: Stats without a project

- **WHEN** statistics are requested with no project open
- **THEN** the counts are zero and FFmpeg readiness is false
