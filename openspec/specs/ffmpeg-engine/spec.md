# ffmpeg-engine Specification

## Purpose

The shared FFmpeg/FFprobe engine: executable discovery, hardware-encoder detection, process execution with streamed, human-readable logging and cancellation, media metadata reading, YouTube broadcast-standard validation, and disk-use estimation with free-space probing.

## Requirements

### Requirement: Executable discovery

The app SHALL resolve FFmpeg from the user's custom path first, then the system PATH, then platform-specific install directories, in that order, ignoring duplicate candidates, and SHALL resolve FFprobe from a sibling of the resolved FFmpeg when present, else from the PATH, else by its bare name.

#### Scenario: Custom path wins

- **WHEN** a custom FFmpeg path is configured
- **THEN** it is preferred over PATH and install-directory candidates

#### Scenario: Sibling probe

- **WHEN** the resolved FFmpeg has a parent directory containing an FFprobe executable
- **THEN** that sibling is used as FFprobe

#### Scenario: Fallback

- **WHEN** no candidate exists
- **THEN** the bare command names `ffmpeg` and `ffprobe` are used

### Requirement: Availability validation

The app SHALL consider an FFmpeg candidate usable only when running it with `-version` exits with code 0, and SHALL validate only the custom path when one is configured.

#### Scenario: Valid candidate

- **WHEN** `<ffmpeg> -version` exits with code 0
- **THEN** FFmpeg is reported available

#### Scenario: Invalid candidate

- **WHEN** the command exits non-zero or cannot be spawned
- **THEN** FFmpeg is reported unavailable

### Requirement: Hardware-encoder detection

The app SHALL offer the hardware encoders VideoToolbox, NVENC, AMF, QuickSync, Media Foundation, VA-API, and V4L2 M2M plus the software encoder `libx264`, SHALL verify a hardware encoder by a short trial encode and keep the first one that succeeds, and SHALL fall back to the software encoder.

#### Scenario: Trial encode proves an encoder

- **WHEN** an encoder completes the trial encode successfully
- **THEN** it is considered available

#### Scenario: Order by platform

- **WHEN** detection runs
- **THEN** candidates are tried in platform order (VideoToolbox then software on macOS; NVENC, AMF, QuickSync, Media Foundation, then software on Windows; NVENC, VA-API, V4L2 M2M, then software elsewhere)

#### Scenario: No hardware

- **WHEN** no hardware encoder passes its trial
- **THEN** the software encoder is reported

#### Scenario: Software is not a hardware encoder

- **WHEN** the selected encoder is `libx264`
- **THEN** it is reported as software rather than hardware

### Requirement: Process execution

The app SHALL run FFmpeg and FFprobe without a shell, passing an argument vector, reading stdout and stderr concurrently, keeping at most 4000 captured lines per stream, running each process in its own process group so it can be terminated as a tree, and SHALL NOT stream raw FFmpeg output to the interface.

#### Scenario: Argument vector, no shell

- **WHEN** a command is run
- **THEN** the executable receives its arguments directly, so shell metacharacters in paths are not interpreted

#### Scenario: Bounded capture

- **WHEN** a process emits more than 4000 lines on one stream
- **THEN** only the most recent 4000 lines are kept

#### Scenario: Quiet success

- **WHEN** a process succeeds
- **THEN** its raw output is not shown; only the application's own step messages are logged

#### Scenario: Failure evidence

- **WHEN** a process fails and is not quiet
- **THEN** the app logs a readable failure naming the program and exit code, followed by the last non-blank lines of its output

### Requirement: Cancellation

The app SHALL cancel the running process tree by sending a graceful termination signal to the process group, and SHALL escalate to a forced kill after a short grace period if the process is still alive, using the platform-appropriate mechanism.

#### Scenario: Graceful cancel

- **WHEN** cancellation is requested
- **THEN** the process group receives a graceful termination signal and the run is marked cancelled

#### Scenario: Escalation

- **WHEN** the process is still alive after the grace period (about half a second)
- **THEN** the process group is forcibly killed

### Requirement: Media metadata

The app SHALL read media metadata with an FFprobe JSON probe, SHALL cache results in memory for the session, and SHALL obtain a file's duration from a local fast reader first and only then from FFprobe.

#### Scenario: Probe fields

- **WHEN** a media file is probed
- **THEN** the app reports the first video and first audio stream's codec, dimensions, frame rate, pixel format, duration, sample rate, channel count, known tags, color properties, profile, level, B-frame and reference-frame counts, GOP size, CABAC, interlacing, and bitrate

#### Scenario: No media streams

- **WHEN** the probe finds neither a video nor an audio stream
- **THEN** the file is reported as unreadable with an explanatory error

#### Scenario: Unreadable file

- **WHEN** the probe fails or returns invalid output
- **THEN** the file is reported as unreadable with the reason, and the media info is not fabricated

#### Scenario: Cached metadata

- **WHEN** the same path is probed again during the session
- **THEN** the cached metadata is returned without another probe

#### Scenario: Fast duration reader

- **WHEN** a duration is requested and the local fast reader can determine it
- **THEN** FFprobe is not invoked for that file

### Requirement: Frame rate default

The app SHALL treat a missing or unparsable frame rate as 30 fps when a concrete value is required.

#### Scenario: Unparsable frame rate

- **WHEN** metadata has no usable frame rate
- **THEN** calculations that need one use 30 fps

### Requirement: YouTube broadcast-standard validation

The app SHALL validate video metadata against the live-broadcast standard: H.264 video, AAC audio, `yuv420p` pixel format, BT.709 color space, progressive scan, H.264 High profile, two B-frames, one reference frame, a GOP no longer than four seconds (recommended two), and a bitrate at least the minimum for the resolution and frame rate, and SHALL report a status of passed, warning, or failed together with the specific issues and the observed details.

#### Scenario: Compliant file

- **WHEN** every checked property matches the standard
- **THEN** the status is passed

#### Scenario: A few issues

- **WHEN** between one and three issues are found
- **THEN** the status is warning

#### Scenario: Many issues

- **WHEN** four or more issues are found
- **THEN** the status is failed

#### Scenario: Insufficient evidence

- **WHEN** no issues are found but several properties are missing
- **THEN** the status is warning rather than passed

#### Scenario: Bitrate below the broadcast minimum

- **WHEN** the bitrate is below the minimum for the resolution and frame rate
- **THEN** that shortfall is reported as an issue

### Requirement: Disk-use estimation

The app SHALL estimate the bytes a merge will need before running it, from the output duration and the video and audio bitrates, adding the temp space for normalization and for pre-looping a short background used with an intro, and SHALL apply a safety multiplier.

#### Scenario: Output estimate

- **WHEN** an estimate is requested for a duration and bitrates
- **THEN** the required bytes are the output bytes plus temp peak bytes, multiplied by the safety margin and rounded up

#### Scenario: Unknown video bitrate

- **WHEN** the background's bitrate cannot be read
- **THEN** a default bitrate is assumed

#### Scenario: Intro with a short background

- **WHEN** an intro is used with a background shorter than the pre-loop threshold
- **THEN** the temp estimate includes room for pre-looping

### Requirement: Free-space probe

The app SHALL probe the free bytes available at a path, using `statvfs` on Unix and the Windows disk-free API elsewhere, SHALL report the space as unknown rather than failing when the path does not exist or the probe is unsupported, and SHALL introduce no external dependency for this.

#### Scenario: Space known

- **WHEN** the path exists and the platform probe succeeds
- **THEN** the available bytes are reported

#### Scenario: Space unknown

- **WHEN** the path does not exist or the platform probe is unsupported
- **THEN** the free space is reported as unknown, and callers must treat unknown as "cannot be verified" rather than "no space"

### Requirement: Helpers

The app SHALL provide shared formatting and naming helpers used across the engine — byte, duration, and clock formatting, a stable non-cryptographic hash used for cache identities, a sanitized file-stem helper, and a non-colliding path helper.

#### Scenario: Clock format

- **WHEN** a time in seconds is formatted for a timestamp sidecar
- **THEN** it is written as `H:MM:SS` when at least an hour long, `M:SS` otherwise with whole seconds

#### Scenario: Non-colliding path

- **WHEN** a path already exists
- **THEN** a suffixed variant is returned instead of overwriting
