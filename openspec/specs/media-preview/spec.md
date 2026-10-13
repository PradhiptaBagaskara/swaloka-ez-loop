# media-preview Specification

## Purpose

In-app media preview: inline audio playback for lists, and a shared video/audio preview surface that plays embeddable formats in the app and falls back to the system player, and then to an explanation, when a file cannot be played in the webview.

## Requirements

### Requirement: Inline audio playback

The app SHALL play audio selections in place, without opening a modal, playing at most one item at a time, reporting progress while an item plays, and stopping playback when the item ends, the user stops it, or the page is left.

#### Scenario: Play one item

- **WHEN** an audio item's play control is activated
- **THEN** that item plays in place with a progress bar and the control becomes a stop control

#### Scenario: Exclusive playback

- **WHEN** another item is started while one is already playing
- **THEN** the first stops

#### Scenario: Playback failure

- **WHEN** an item cannot be played
- **THEN** the app reports which file failed without leaving the player in a playing state

### Requirement: Audio mix preview

The app SHALL support previewing the assembled audio mix: main tracks played in order with overlays looping over them, and SHALL apply an overlay's volume as it is adjusted while the mix is playing.

#### Scenario: Play the mix

- **WHEN** the mix preview is started
- **THEN** the main tracks play in sequence while the overlays play over them

#### Scenario: Adjust a playing overlay

- **WHEN** the user moves an overlay's volume slider while the mix is playing
- **THEN** that overlay's volume changes immediately

#### Scenario: Main tracks end

- **WHEN** the last main track finishes
- **THEN** the overlays stop as well

### Requirement: Preview surface

The app SHALL present a preview for a selected file that plays embeddable audio and video formats inside the app, and SHALL show the file's name in the preview.

#### Scenario: Embeddable audio

- **WHEN** the file is one of the embeddable audio formats
- **THEN** it plays inline in the preview

#### Scenario: Embeddable video

- **WHEN** the file is one of the embeddable video formats
- **THEN** it plays inline in the preview

#### Scenario: Filename

- **WHEN** the preview opens
- **THEN** the file's base name is displayed

### Requirement: Fallback to the system player

The app SHALL open a file with the system default application when it cannot be played in the webview — because its format is not embeddable, its playback failed, or it did not finish loading in time — and SHALL close the preview when the system player opens the file.

#### Scenario: Unsupported format

- **WHEN** the file's extension is not embeddable
- **THEN** the app asks the system to open the file with its default application

#### Scenario: Playback error

- **WHEN** the in-app player reports an error for the file
- **THEN** the app asks the system to open the file with its default application

#### Scenario: Slow load

- **WHEN** the video has not loaded within about eight seconds
- **THEN** the app treats it as unplayable in the app and asks the system to open the file

#### Scenario: Opened externally

- **WHEN** the system player accepts the file
- **THEN** the preview closes

### Requirement: No default player

When the system has no application that can open the file, the app SHALL keep the preview open and explain that the file cannot be previewed in the app and that no default player is available, offering an action that reveals the file in the system file manager.

#### Scenario: No player available

- **WHEN** opening the file with the system default application fails
- **THEN** the preview explains that the file cannot be previewed in the app and that no default player is available, and offers to reveal it in the file manager

#### Scenario: Reveal the file

- **WHEN** the user uses the reveal action
- **THEN** the file's folder opens with the file selected

### Requirement: Preview state resets

The app SHALL reset the preview's failure state whenever the previewed file or the preview's open state changes, so a later preview of another file starts fresh.

#### Scenario: Preview a second file

- **WHEN** a different file is previewed after a failed preview
- **THEN** the preview starts again from the beginning instead of showing the earlier failure
