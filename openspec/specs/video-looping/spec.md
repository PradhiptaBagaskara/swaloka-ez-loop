# video-looping Specification

## Purpose

The Looping tool: combining one background video that auto-loops to the length of a batch of audio tracks, optionally preceded by an intro video, into a single output video with a timestamp sidecar, progress, and log reporting.

## Requirements

### Requirement: Loop inputs

The Looping page SHALL accept one background video, one or more audio tracks, an optional intro video, a loop count of at least one, and an output name, and SHALL enable generation only when a background video and at least one audio track are present.

#### Scenario: Inputs complete

- **WHEN** a background video and at least one audio track are selected
- **THEN** the generate action becomes available

#### Scenario: Missing required input

- **WHEN** no background video or no audio track is selected
- **THEN** the generate action is unavailable and the page names what is missing

### Requirement: Video selection surface

The Looping page SHALL present the intro and the background as two preview slots side by side, with the intro on the left and the background on the right, each showing a seeked preview frame when it holds a video, a click-to-preview behaviour, and a remove control at the video's top-right corner, and each showing a click-to-add control when empty.

#### Scenario: Empty slot

- **WHEN** a slot holds no video
- **THEN** it shows an add control that opens the file picker

#### Scenario: Filled slot

- **WHEN** a slot holds a video
- **THEN** it shows a preview frame, opens the preview when clicked, and offers a remove control

#### Scenario: Remove a video

- **WHEN** the user removes a slot's video
- **THEN** that slot becomes empty and generation reflects the missing input

### Requirement: Audio playlist surface

The Looping page SHALL list the audio tracks below the video slots and SHALL play them inline without opening a modal, one track at a time, showing a progress bar for the playing track.

#### Scenario: Inline playback

- **WHEN** the user activates a track's play control
- **THEN** that track plays in place and the control becomes a stop control

#### Scenario: Exclusive playback

- **WHEN** another track is started while one is playing
- **THEN** the first stops and only one track plays at a time

#### Scenario: Leaves the page

- **WHEN** the user leaves the Looping page
- **THEN** any inline playback stops

### Requirement: Audio reordering

The Looping page SHALL let the user reorder the audio tracks by dragging a track's handle onto another position, SHALL reflect the new order immediately, and SHALL use that order for the next merge.

#### Scenario: Drag to reorder

- **WHEN** the user drags a track's handle and drops it onto another track
- **THEN** the dragged track moves to that position and the list renumbers

#### Scenario: Dropped on itself

- **WHEN** a track is dropped onto its own position
- **THEN** the order is unchanged

#### Scenario: Playback and reorder

- **WHEN** a reorder happens while a track is playing
- **THEN** playback stops so the playing indicator cannot point at the wrong row

### Requirement: Audio normalization and caching

The app SHALL normalize audio inputs to AAC, 44.1 kHz, stereo, in an m4a container before merging, SHALL pass through unchanged only inputs that already match those parameters, SHALL re-encode raw ADTS `.aac` inputs because ADTS carries no timestamps, SHALL re-encode inputs whose sample rate differs because a different timebase shifts the joined result, and SHALL reuse previously normalized results for identical source files.

#### Scenario: Mixed audio formats

- **WHEN** the audio tracks include non-AAC formats
- **THEN** they are normalized to AAC before merging

#### Scenario: Raw ADTS AAC input

- **WHEN** an audio track is a raw `.aac` (ADTS) file
- **THEN** it is re-encoded into the normalized AAC container instead of being passed through

#### Scenario: Already AAC but a different sample rate

- **WHEN** an audio track is already AAC but at another sample rate
- **THEN** it is normalized as well, because the join copies streams and a different timebase would shift the joined audio

#### Scenario: Repeat run with the same sources

- **WHEN** a merge reuses a source normalized in an earlier run
- **THEN** the cached normalized copy is reused without re-encoding

### Requirement: Playlist ordering

The app SHALL use the user's order for the first loop and SHALL shuffle the order for each subsequent loop, so repeated loops do not present the same sequence.

#### Scenario: Multiple loops

- **WHEN** the loop count is greater than one
- **THEN** the first loop follows the user's order and later loops are shuffled

### Requirement: Background looping and audio merge

The app SHALL join the normalized audio tracks into one continuous track by stream copy, so the audio is encoded once and the join adds neither loss nor time, SHALL loop the background video as needed so the video covers that audio, and SHALL produce an output whose duration matches the audio.

#### Scenario: Joined audio is copied, not re-encoded

- **WHEN** the normalized tracks are joined
- **THEN** they are joined by stream copy, so no second generation of loss is introduced

#### Scenario: Joined audio keeps its length

- **WHEN** tracks whose combined length is known are joined
- **THEN** the joined audio keeps that combined length

#### Scenario: Short background, long audio

- **WHEN** the combined audio is longer than the background video
- **THEN** the background repeats until it covers the audio and the output ends with the audio

#### Scenario: Background longer than audio

- **WHEN** the background video is longer than the combined audio
- **THEN** the output is trimmed to the audio length

### Requirement: Intro video handling

When an intro is provided, the app SHALL place the intro first and follow it with the looped background, SHALL pre-loop a background shorter than ten minutes up to about ten minutes before the final merge, SHALL compute how many background repetitions cover the remaining audio, and SHALL warn when the intro's codec differs from the background's.

#### Scenario: Intro provided

- **WHEN** an intro video is selected and generation runs
- **THEN** the output begins with the intro followed by the background covering the remaining audio

#### Scenario: Short background with an intro

- **WHEN** the background is shorter than the ten-minute threshold and an intro is used
- **THEN** the background is extended before the final merge instead of looping on the fly

#### Scenario: Codec mismatch

- **WHEN** the intro and background videos use different codecs
- **THEN** the app warns before processing

### Requirement: Colour metadata and output container

The app SHALL tag merged video with BT.709 colour metadata (colour space, transfer, primaries, and TV range) and SHALL write the output with the file's index at the front so playback and preview can start without reading the whole file.

#### Scenario: Merge output flags

- **WHEN** either merge path runs
- **THEN** the output carries the BT.709 colour flags and the container is written for streaming playback

### Requirement: Output naming

The app SHALL write the output video with a name derived from the user's output name, else from the background video's file name, and SHALL avoid overwriting existing files by choosing a unique name.

#### Scenario: Named output

- **WHEN** the user provides an output name
- **THEN** the generated file uses that name

#### Scenario: Collision avoidance

- **WHEN** the derived name already exists in the output folder
- **THEN** a non-colliding name is used instead of overwriting

### Requirement: Timestamp sidecar

The app SHALL write a sidecar timestamp file beside the output listing each track's start offset and title, SHALL take the titles from the source tracks, SHALL measure the offsets from the normalized files that are actually joined, SHALL build the list as soon as the playlist order is known, and SHALL write the file only after the video is rendered successfully so a failed render leaves no orphan sidecar.

#### Scenario: Timestamp sidecar

- **WHEN** generation completes successfully
- **THEN** a sidecar file beside the output lists each track's start offset and title

#### Scenario: No orphan on failure

- **WHEN** the video render fails
- **THEN** no timestamp file is written

#### Scenario: Offsets follow the joined audio

- **WHEN** the sidecar offsets are computed
- **THEN** each offset is the sum of the durations of the earlier tracks as they are joined, not of the original source files

### Requirement: Pre-flight consistency checks

Before processing, the app SHALL check the background and intro videos for codec mismatches and for resolution, frame-rate, or pixel-format differences, and SHALL check the background against YouTube standards, presenting the findings as a warning the user must acknowledge before continuing.

#### Scenario: Property mismatch

- **WHEN** the intro and background differ in resolution, frame rate, or pixel format
- **THEN** the app warns about potential visual glitches before processing

#### Scenario: User proceeds past the warning

- **WHEN** the user confirms the warning
- **THEN** processing continues with the selected inputs

#### Scenario: User cancels at the warning

- **WHEN** the user cancels the warning
- **THEN** no processing starts and the inputs are unchanged

#### Scenario: Check unavailable

- **WHEN** the pre-flight check itself fails
- **THEN** the merge still proceeds, because the check is advisory

### Requirement: Progress and result reporting

The app SHALL report progress through the merge at staged checkpoints, SHALL stream hierarchical processing logs during the run, and SHALL present a result offering a way to reveal the output in the system file manager when finished.

#### Scenario: Watch a merge

- **WHEN** a merge is running
- **THEN** the queue card shows the current stage label, progress, and streaming logs, and offers cancellation

#### Scenario: Open output after completion

- **WHEN** a merge completes
- **THEN** the result offers an action that reveals the output file in the system file manager
