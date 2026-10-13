# media-tools Specification

## Purpose

The Media Tools page: audio assembly (concatenation, overlay mixing, and overlay preset export), video encoding to YouTube's live broadcast standard, and media information readout with YouTube standards validation.

## Requirements

### Requirement: Page structure

The Media Tools page SHALL present three tabs — "Audio", "Standar Live", and "Info Video" — and SHALL offer the tab's primary action in the page header, disabled while the tab's inputs are incomplete or a run is active.

#### Scenario: Tab switching

- **WHEN** the user switches tabs
- **THEN** the matching tab content is shown and any inline audio preview stops

#### Scenario: Action availability

- **WHEN** a tab has no inputs yet
- **THEN** its header action is disabled

### Requirement: Audio assembly

On the "Audio" tab the app SHALL accept one or more main audio tracks and zero or more overlay tracks, each overlay carrying a path and a volume between 0 and 1 defaulting to full, SHALL let overlays be played and their volumes adjusted while playing, SHALL let the user pick an output format and output name, and SHALL assemble an output file.

#### Scenario: Main tracks only

- **WHEN** the user assembles audio with main tracks and no overlays
- **THEN** the tracks are joined in order into one output file

#### Scenario: Duplicate main tracks

- **WHEN** the main list contains the same file more than once
- **THEN** each distinct file is encoded once and the joined result reuses it

#### Scenario: With overlays

- **WHEN** one or more overlays are present
- **THEN** the overlays are mixed over the main track for the main track's duration, each at its own volume

#### Scenario: Missing input

- **WHEN** no main track is present
- **THEN** the assembly is refused with an explanation

#### Scenario: Inline preview

- **WHEN** the user plays a main track or the mix
- **THEN** audio plays in place and a volume change on a playing overlay takes effect immediately

### Requirement: Overlay preset export

The app SHALL export the configured overlays as a standalone mixed preset file, independently of any main tracks, and SHALL refuse the export when no overlay is configured.

#### Scenario: Export a preset

- **WHEN** one or more overlays are configured and the preset action is used
- **THEN** a mixed preset file is written to the output folder

#### Scenario: Nothing to export

- **WHEN** no overlay is configured
- **THEN** the preset action is disabled and the app explains that an overlay is needed first

### Requirement: Output format

The audio output SHALL be written with an audio codec matching the chosen output extension — MP3 at 192 kbps, uncompressed WAV, FLAC, or Ogg Vorbis — and SHALL default to AAC at 384 kbps, 48 kHz, stereo for the m4a container and for any unrecognized extension.

#### Scenario: Format by extension

- **WHEN** the user picks an output format
- **THEN** the output is encoded with that format's codec

#### Scenario: Default format

- **WHEN** no format is chosen or the extension is unrecognized
- **THEN** AAC at 384 kbps, 48 kHz, stereo is used

### Requirement: Live broadcast standard encoding

On the "Standar Live" tab the app SHALL encode videos one at a time to YouTube's live broadcast standard, using constant-bitrate H.264, a keyframe every two seconds, a closed GOP, two B-frames, one reference frame, BT.709 colour metadata, and AAC audio at 128 kbps, 44.1 kHz, stereo, SHALL preserve each source's resolution and frame rate while normalizing to even dimensions, square pixels, and `yuv420p`, and SHALL write the output with the index at the front.

#### Scenario: Encode a video

- **WHEN** the user starts the encode for one or more videos
- **THEN** each video is encoded in turn and the resulting files are reported

#### Scenario: Resolution preserved

- **WHEN** a source video is encoded
- **THEN** its resolution and frame rate are preserved, with dimensions rounded to even numbers

#### Scenario: Bitrate bounds

- **WHEN** a bitrate is requested below 3 Mbps
- **THEN** the encode is refused because that is below the broadcast minimum

#### Scenario: Default bitrate

- **WHEN** no bitrate is requested
- **THEN** 8 Mbps is used

#### Scenario: Keyframe interval

- **WHEN** the encoder parameters are built
- **THEN** the keyframe interval is two seconds' worth of frames for the source frame rate, at least one frame

### Requirement: Software and hardware encoding

On the "Standar Live" tab the app SHALL use the software H.264 encoder for the guaranteed parameters, SHALL allow a hardware encoder from the settings, and SHALL state that hardware encoders are best-effort — they may not hold the constant bitrate or exactly meet the keyframe and colour-metadata requirements.

#### Scenario: Software mode

- **WHEN** the configured encoder is the software encoder
- **THEN** the page states that all broadcast parameters are guaranteed

#### Scenario: Hardware mode

- **WHEN** a hardware encoder is configured
- **THEN** the page warns that the parameters are best-effort and points the user at the Info Video tab to verify

#### Scenario: Bitrate below the minimum

- **WHEN** the entered bitrate is below the configured minimum
- **THEN** the page warns that the bitrate is below the allowed minimum

### Requirement: Media information

On the "Info Video" tab the app SHALL let the user pick one video and SHALL report its size, codec, resolution, frame rate, pixel format, duration, audio codec, and the YouTube broadcast-standard status with the specific issues found.

#### Scenario: Inspect a video

- **WHEN** a video is selected for inspection
- **THEN** its metadata and the YouTube status with issues are shown

#### Scenario: No selection

- **WHEN** no video is selected
- **THEN** the tab prompts the user to pick one

### Requirement: Scope

The video tools SHALL be limited to broadcast-standard encoding and inspection: the app SHALL NOT offer video concatenation, transitions between clips, video compression, or generic resolution conversion.

#### Scenario: Only the live encoder

- **WHEN** video work is offered in Media Tools
- **THEN** the only video operation is encoding to the live broadcast standard plus reading media information
