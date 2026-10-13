# channel-planner Specification

## Purpose

The EZ Loop feature: channels that turn an audio folder and a video folder into balanced, reviewable episode plans, and then process the selected plans sequentially through the loop merge pipeline. In the sidebar this feature is labelled "EZ Loop"; its interface speaks of "plan" and "konten".

## Requirements

### Requirement: Channel definition

A channel SHALL hold a name, an audio folder, a video folder, a minimum episode duration (default 120 minutes), a per-song loop target (default 2), a requested content count (0 meaning "follow the video count"), an output location, the detected assets, the plans, a seed, and its status, and SHALL be persisted as JSON inside the project's `channels/` folder.

#### Scenario: Persist a channel

- **WHEN** a channel is created or changed
- **THEN** it is written to the project and survives a reload

#### Scenario: Delete a channel

- **WHEN** the user deletes a channel
- **THEN** its definition, plans, and plan state are removed, and already-produced output files are left on disk

#### Scenario: List channels

- **WHEN** the channel list is shown
- **THEN** channels are listed most-recently-changed first with a name, asset counts, plan progress, and a short status

### Requirement: Creating a channel

The app SHALL create a channel through a dialog asking for the name, the requested content count, the minimum episode duration, and the per-song loop target, SHALL default the content count to 30, the minimum duration to 120 minutes, and the loop target to 2, and SHALL open the new channel's process page afterwards.

#### Scenario: Create with defaults

- **WHEN** the user opens the create dialog and submits without changing the numbers
- **THEN** the channel is created with a content count of 30, a minimum duration of 120 minutes, and a loop target of 2

#### Scenario: Empty name

- **WHEN** the name is left empty
- **THEN** the channel is created with a default name

#### Scenario: Follow the video count

- **WHEN** the content count is cleared or zero
- **THEN** the number of plans follows the number of videos in the video folder

### Requirement: Asset detection

Planning SHALL scan the audio folder for audio files and read each file's duration, SHALL scan the video folder for videos, and SHALL report what it found.

#### Scenario: Detect assets

- **WHEN** planning runs
- **THEN** the audio files with their durations and the video files are detected and stored on the channel

#### Scenario: Unreadable duration

- **WHEN** an audio file's duration cannot be read
- **THEN** a fallback duration is used so planning still completes

#### Scenario: Empty folders

- **WHEN** a folder yields no assets
- **THEN** planning reports what is missing instead of producing an empty plan set silently

### Requirement: Balanced plan generation

The app SHALL generate one plan per requested content item, each with an ordered playlist whose total duration reaches the minimum episode duration, SHALL ensure no two plans share the same total duration or the same set of songs, SHALL fill songs least-used-first so the library is spread across the plans, and MAY add a bonus of up to three extra songs per plan.

#### Scenario: Minimum duration met

- **WHEN** plans are generated
- **THEN** every plan's total duration is at least the configured minimum

#### Scenario: Unique durations and combinations

- **WHEN** plans are generated
- **THEN** no two plans share a total duration or a set of songs

#### Scenario: Even usage

- **WHEN** picks are chosen
- **THEN** the least-used songs are preferred so the library is spread across the plans

#### Scenario: Not enough variety

- **WHEN** the library cannot yield the requested number of unique plans
- **THEN** the plans it can yield are produced and the shortfall is reported

### Requirement: Playlist ordering rules

The app SHALL order each playlist so the same file never plays twice in a row, SHALL start each plan with a different song as far as the library allows, and SHALL list the plans shortest-first.

#### Scenario: No adjacent repeats

- **WHEN** a playlist is ordered
- **THEN** the same file does not play twice in a row

#### Scenario: Distinct openings

- **WHEN** plans are generated
- **THEN** their first songs come from a queue so they differ from one another as far as the library allows

#### Scenario: Shortest first

- **WHEN** the plans are listed
- **THEN** they are ordered from the shortest total duration to the longest

### Requirement: Deterministic planning

The app SHALL plan a channel reproducibly from its stored seed, so the same inputs and seed yield the same plan set.

#### Scenario: Same seed, same plans

- **WHEN** the same channel inputs and seed are planned twice
- **THEN** the same plan set results

### Requirement: Plan-to-video assignment

Each plan SHALL be paired with one video from the video folder, one plan per video and no reuse, and plans beyond the available videos SHALL be created without a background video and SHALL start unselected.

#### Scenario: Pair videos

- **WHEN** there are at least as many videos as plans
- **THEN** each plan gets its own background video

#### Scenario: More plans than videos

- **WHEN** more plans are requested than there are videos
- **THEN** the surplus plans are created without a background and start unselected

#### Scenario: Rescan for videos

- **WHEN** videos are added to the folder and the user runs "Periksa ulang folder"
- **THEN** free videos are assigned to the plans that were waiting for one

### Requirement: Source stage

The process page SHALL present a first stage where the user sets the channel name, the minimum episode duration, the audio and video folders, the requested content count, the loop target, and the output folder, and SHALL offer resetting the output folder to the project default.

#### Scenario: Pick folders

- **WHEN** the user picks an audio folder and a video folder
- **THEN** the channel can be planned

#### Scenario: Change settings after planning

- **WHEN** a source setting changes after plans exist
- **THEN** the page indicates that the plans no longer match the current settings and should be re-created

#### Scenario: Output location

- **WHEN** the user changes the output folder
- **THEN** later outputs are written there

### Requirement: Plan stage summary

The process page SHALL present a second stage summarizing the plans — the plan count, the average duration, and the total duration — plus the shortfall when fewer plans than requested could be built, and a one-line note stating how many videos from the folder are in use.

#### Scenario: Summary

- **WHEN** plans exist
- **THEN** the plan count, average duration, and total duration are shown

#### Scenario: Videos in use

- **WHEN** plans exist
- **THEN** the page states how many videos from the folder are used, without listing the individual videos

#### Scenario: Shortfall

- **WHEN** fewer plans were built than requested
- **THEN** the page explains how many are missing and why

### Requirement: Plan review surface

The app SHALL present each plan as a card showing, top to bottom, its status and selection, two video slots side by side (intro on the left, background on the right), the plan's summary, and its actions, and SHALL show the status explicitly during processing.

#### Scenario: Status is shown

- **WHEN** a plan's status is not processed, running, done, or failed
- **THEN** the card shows "Belum diproses", "Sedang diproses", "Selesai", or "Gagal" respectively

#### Scenario: Missing background

- **WHEN** a plan has no background video
- **THEN** the card marks it as needing a video

### Requirement: Per-plan video slots

Each plan card SHALL show the intro and background videos as preview slots where a present video can be previewed and removed and an absent video can be added, and SHALL let the plan's background be replaced by removing and adding one.

#### Scenario: Preview a slot

- **WHEN** the user activates a slot that holds a video
- **THEN** the video is previewed

#### Scenario: Remove the background

- **WHEN** the user removes a plan's background video
- **THEN** the plan no longer has a background and is no longer selected, because it cannot be processed without one

#### Scenario: Add a video

- **WHEN** the user activates an empty slot
- **THEN** a file picker opens and the chosen video is placed in that slot

### Requirement: Per-plan song list

The plan card SHALL summarize the plan with its total duration and loop spread and SHALL offer its song list behind a clearly-labelled, collapsed-by-default control on the right, so the card stays compact; the songs SHALL be playable inline without a modal.

#### Scenario: Collapsed by default

- **WHEN** a plan card is shown
- **THEN** the song list is collapsed and the control states how many songs the plan has

#### Scenario: Expand the songs

- **WHEN** the user activates the song control
- **THEN** the ordered songs are listed and can be played inline

#### Scenario: Repeated songs

- **WHEN** a plan uses the same song more than once
- **THEN** each occurrence is listed separately

### Requirement: Plan result and reset

When a plan has produced output, the card SHALL show the result file, SHALL offer an action that reveals it in the system file manager, and SHALL offer previewing it, and it SHALL allow marking a processed plan as not processed again so it can be re-run, changing only its status.

#### Scenario: Reveal the result

- **WHEN** the user activates the result's reveal action
- **THEN** the file's folder opens with the output file selected

#### Scenario: Preview the result

- **WHEN** the user previews a result
- **THEN** the app tries the system player first and only then the in-app preview

#### Scenario: Reset a processed plan

- **WHEN** the user marks a done or failed plan as not processed
- **THEN** only its status changes, its previous output path is kept, and it can be processed again

#### Scenario: Remove a plan

- **WHEN** the user removes a plan
- **THEN** it is deleted, the remaining plans are renumbered so their labels stay contiguous, and the requested content count becomes the number of remaining plans

### Requirement: Selection

Each plan SHALL carry a selection control, plans without a background video SHALL not be selectable, and the page SHALL offer selecting all selectable plans and clearing the selection.

#### Scenario: Select all

- **WHEN** the user selects all
- **THEN** every plan with a background is selected and plans without one stay unselected

#### Scenario: Clear

- **WHEN** the user clears the selection
- **THEN** no plan is selected

### Requirement: Capacity pre-check

Before processing, the app SHALL estimate the disk space required by the selected plans and compare it with the free space at the output location, SHALL show both numbers with the plan count and total duration, SHALL block processing when the space is insufficient, and SHALL allow processing with a note when the free space cannot be read.

#### Scenario: Enough space

- **WHEN** the estimate fits the free space
- **THEN** processing is allowed and both numbers are shown

#### Scenario: Not enough space

- **WHEN** the estimate exceeds the free space
- **THEN** processing is blocked and the page explains that the output location cannot hold the batch

#### Scenario: Space unknown

- **WHEN** the free space cannot be read
- **THEN** the page says so and still allows processing

### Requirement: Sequential processing

The app SHALL process the selected plans sequentially through the loop merge pipeline, one at a time, skipping plans already processed, recording each plan's outcome and output path as it goes, and SHALL allow cancelling the batch.

#### Scenario: Process the selection

- **WHEN** the user processes the selected plans
- **THEN** each selected plan that is not yet processed is merged in order and its status and output path are stored

#### Scenario: Already processed plans

- **WHEN** a selected plan has already been processed
- **THEN** it is skipped until the user marks it as not processed again

#### Scenario: Cancel

- **WHEN** the user cancels a batch
- **THEN** the running merge is stopped and the remaining plans are left untouched

#### Scenario: Open during a run

- **WHEN** a channel is opened while another run is active
- **THEN** the page still loads the channel instead of staying on its loading state

### Requirement: Processing progress

The batch SHALL report progress across the selected plans, SHALL stream hierarchical logs during the run, and SHALL report a summary of how many plans succeeded and failed.

#### Scenario: Watch a batch

- **WHEN** a batch is running
- **THEN** the queue card shows the batch progress and streaming logs, and offers cancellation

#### Scenario: Batch summary

- **WHEN** a batch finishes
- **THEN** the app reports how many plans succeeded and how many failed
