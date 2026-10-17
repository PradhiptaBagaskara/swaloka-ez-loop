# Spec Delta

## Purpose

The application shell: window and lifecycle, the FFmpeg prerequisite gate, the project entry flow, the sidebar/topbar workspace chrome, the dashboard home, settings, theme, notifications, the bottom queue card, and in-app updates.

## ADDED Requirements

### Requirement: Application updates

The app SHALL check the configured update endpoint on startup and SHALL also let the user check manually from the settings dialog. When an update is announced the app SHALL show the new version and its release notes before installing, SHALL show download progress while installing, and SHALL run the new version afterwards — by relaunching, or by letting the Windows installer restart the app. A failed automatic check SHALL be silent; a failed manual check SHALL be reported. The settings dialog SHALL show the running version alongside the manual check.

#### Scenario: Update available on startup

- **WHEN** the startup check finds a version newer than the running one
- **THEN** a dialog opens naming the new version and its release notes, offering "Nanti" and "Perbarui"

#### Scenario: No update

- **WHEN** the user checks manually and the running version is current
- **THEN** the app reports "Sudah versi terbaru"

#### Scenario: Manual check from settings

- **WHEN** the user clicks "Periksa pembaruan"
- **THEN** the app checks the endpoint and either opens the update dialog or reports that the app is current, and the running version is shown next to the action

#### Scenario: Install

- **WHEN** the user chooses "Perbarui"
- **THEN** the update downloads with a percentage progress bar and, when it finishes, the app runs the new version

#### Scenario: Install failure

- **WHEN** the download or install fails
- **THEN** the failure is shown in the dialog and the install action becomes a retry

#### Scenario: Automatic check failure

- **WHEN** the startup check fails, for example when running a development build without an app bundle
- **THEN** nothing is reported and the app continues normally

#### Scenario: Dismiss

- **WHEN** the user chooses "Nanti" while an update is available
- **THEN** the dialog closes and the update can still be installed later from the settings dialog
