# Spec Delta

## Purpose

The MCP (Model Context Protocol) surface: how it is turned on and reached, the tools it
exposes, how it is served inside the running app over loopback and reached by a stdio
bridge for clients that speak only stdio, how it shares the app's project and run state,
and the safety rules that keep a loopback listener from becoming an open door.

## ADDED Requirements

### Requirement: Embedded MCP server

The app SHALL embed an MCP server inside the running process, SHALL keep it disabled
until the user enables it in settings, SHALL bind it to the loopback address only, and
SHALL expose a Streamable HTTP endpoint an MCP client can connect to. The server SHALL
share the running application's state — the active project, the single-run guard, and
the telemetry that feeds the queue card — so AI- and user-initiated work are
indistinguishable in the interface. The server SHALL NOT start automatically on a later
launch even when it was enabled before.

#### Scenario: Disabled by default

- **WHEN** the app starts
- **THEN** no MCP endpoint is listening and no tool is reachable

#### Scenario: Enable from settings

- **WHEN** the user enables AI control in settings
- **THEN** the server starts on the configured loopback port and its endpoint and token are shown so they can be copied

#### Scenario: Loopback only

- **WHEN** the server is running
- **THEN** it accepts connections only on `127.0.0.1` and never binds a public interface

#### Scenario: Shared state

- **WHEN** the server is running and a project is open
- **THEN** MCP tools act on the same project and services the interface is using, and an AI-initiated run appears in the queue card with progress and logs like any other run

#### Scenario: Not restarted on launch

- **WHEN** the app is restarted after AI control was enabled
- **THEN** the server is off until the user enables it again

#### Scenario: Stop

- **WHEN** the user disables AI control
- **THEN** the listener stops, its connections are closed, and the app continues normally

### Requirement: Tool catalog

The server SHALL expose a documented set of tools, each with a JSON input schema and a
structured result, covering project control, the looping pipeline, EZ Loop channels,
media inspection, run control, and FFmpeg status. The same catalog SHALL be served by
both the in-app endpoint and the stdio bridge. Each tool SHALL be a thin adapter
over the same services the interface commands use, so there is one implementation of the
domain logic. The tools SHALL use the app's own vocabulary: the merge tool SHALL be
named `loop_create_content`, and no tool SHALL name the produced unit an "episode".

The catalog SHALL include at least: `project_open`, `project_create`, `project_status`,
`project_close`, `ffmpeg_status`, `loop_preflight`, `loop_metadata`,
`loop_create_content`, `channel_list`, `channel_load`, `channel_create`, `channel_save`,
`channel_plan`, `channel_rescan_videos`, `channel_preflight`, `channel_run`,
`channel_cancel`, `media_info`, `run_status`, and `run_cancel`.

#### Scenario: List tools

- **WHEN** a client lists tools
- **THEN** every tool is reported with its name, its description, and its input schema

#### Scenario: Content, not episode

- **WHEN** the merge tool is listed
- **THEN** its name is `loop_create_content` and no tool name or description calls the unit an "episode"

#### Scenario: Structured results

- **WHEN** a tool succeeds
- **THEN** it returns structured content matching the payloads the interface uses, such as the loop result's output path, timestamp path, and playlist

#### Scenario: One implementation

- **WHEN** a tool performs a merge or plans a channel
- **THEN** it calls the same service the corresponding command calls, so behavior cannot drift between the interface and the agent

### Requirement: Project-scoped session

Tools that operate on a project SHALL require an open project, SHALL fail with a
readable message when none is open, and SHALL activate a project for the running app
when one is opened — so the agent and the user share one workspace rather than separate
ones.

#### Scenario: No project open

- **WHEN** a project-scoped tool is called and no project is open
- **THEN** it fails with a readable "belum ada proyek yang terbuka" message and changes nothing

#### Scenario: Open a project

- **WHEN** `project_open` is called with a project folder
- **THEN** that project becomes the active project in the running app, exactly as opening it from the interface would

#### Scenario: Status

- **WHEN** `project_status` is called
- **THEN** it reports the project name and root, the output location, FFmpeg readiness, and whether a run is active

#### Scenario: Close a project

- **WHEN** `project_close` is called
- **THEN** the active run is cancelled, temporary files are cleaned, and the app returns to the no-project state

### Requirement: Asynchronous runs

Content creation and channel batches SHALL be asynchronous: the tool SHALL start the
work and return promptly with a run handle, the client SHALL follow progress through
`run_status`, and SHALL be able to stop the work through `run_cancel`. The app's
single-run guard SHALL apply, so an AI-initiated run and a user-initiated run cannot
overlap.

#### Scenario: Start content creation

- **WHEN** `loop_create_content` is called with a background video, audio files, an output location, and a loop count
- **THEN** a run starts, the call returns without waiting for the merge to finish, and the run appears in the queue card

#### Scenario: Preflight before creating

- **WHEN** `loop_preflight` is called for a background and an optional intro
- **THEN** it returns the codec-mismatch and warning report the interface shows before a merge

#### Scenario: Follow progress

- **WHEN** `run_status` is called while a run is active
- **THEN** it reports the running state, the latest progress fraction, and the recent log lines

#### Scenario: Cancel

- **WHEN** `run_cancel` is called
- **THEN** the running merge is stopped and the tool reports that it was cancelled

#### Scenario: Single run

- **WHEN** a tool starts a run while another run is active
- **THEN** it is rejected with the app's "Ada proses lain yang sedang berjalan" message

#### Scenario: Batch outcome

- **WHEN** `channel_run` is called with a channel and a selection
- **THEN** the selected plans are processed sequentially and `run_status` reports how many succeeded and how many failed

### Requirement: Access control and confinement

The server SHALL require a per-session token on every request, SHALL reject requests
without it, and SHALL confine file-writing tools to the open project's root, rejecting
any path that resolves outside it.

#### Scenario: Token required

- **WHEN** a request arrives without the current token
- **THEN** it is rejected and no tool runs

#### Scenario: Token rotation

- **WHEN** the server is stopped and started again
- **THEN** a new token is issued and the previous one no longer works

#### Scenario: Path outside the project

- **WHEN** a tool is given an output path outside the open project's root
- **THEN** the tool is rejected with an explanatory message and writes nothing

### Requirement: stdio bridge

For MCP clients that speak only stdio, the app SHALL provide a bridge: the binary SHALL
detect a stdio argument (for example `swaloka --mcp-stdio`) and, in that mode, serve the
same tool catalog over stdin and stdout by forwarding to the running app's loopback
endpoint. The bridge SHALL NOT build its own application state, so the agent and the
user share one process, one active project, and one single-run guard. When it cannot
reach a running app, the bridge SHALL exit with a readable message rather than start a
second process, so two processes never write to the same project at once. A launch with
no stdio argument SHALL leave the GUI path unchanged.

#### Scenario: Bridge to the running app

- **WHEN** the binary is started with the stdio argument while the app is running with AI control enabled
- **THEN** it serves MCP over stdin and stdout by forwarding to the app's loopback endpoint, and tools act on the app's active project

#### Scenario: Runs stay visible

- **WHEN** a tool served through the bridge starts a run
- **THEN** that run appears in the app's queue card with progress and logs, because the bridge uses the app's state rather than its own

#### Scenario: No app running

- **WHEN** the bridge cannot reach a running app
- **THEN** it exits with a readable message telling the user to open the app and enable AI control

#### Scenario: Never a second writer

- **WHEN** the app is not running
- **THEN** no path starts a second process that could write to the same project

#### Scenario: Normal launch

- **WHEN** the binary is started with no stdio argument
- **THEN** the GUI starts exactly as before
