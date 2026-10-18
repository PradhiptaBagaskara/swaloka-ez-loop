# Tasks

## 1. Dependencies and server scaffold

- [ ] 1.1 Add `rmcp` (MCP Rust SDK) and an HTTP transport dependency (for example `axum`) to `src-tauri/Cargo.toml`, and confirm they coexist with the Tauri v2 runtime and tokio; verify with `cargo check`.
- [ ] 1.2 Add `src-tauri/src/mcp/mod.rs` with a `McpServer` type holding its config (enabled, port, per-session token), an async `start(app: AppHandle, port)`, a `stop()`, and a `status()`; bind strictly to `127.0.0.1`; verify with a unit test that the bind address is loopback.
- [ ] 1.3 Have the server capture an `AppHandle` so every tool reaches `handle.state::<AppState>()` — the same instance the interface uses, so project and run state are shared.
- [ ] 1.4 Keep the server off on launch even when it was enabled previously, and mint a fresh token on every start; verify with a test that a previous token is rejected after a restart.

## 2. Tool layer

- [ ] 2.1 Define the tool catalog with JSON schemas and structured results: `project_open`, `project_create`, `project_status`, `project_close`, `ffmpeg_status`, `loop_preflight`, `loop_metadata`, `loop_create_content`, `channel_list`, `channel_load`, `channel_create`, `channel_save`, `channel_plan`, `channel_rescan_videos`, `channel_preflight`, `channel_run`, `channel_cancel`, `media_info`, `run_status`, `run_cancel`.
- [ ] 2.2 Implement each tool as a thin adapter over the same services the interface commands call (`LoopMergeService`, `ChannelPlanner`, `ChannelExecutor`, `ChannelRepository`, `ProjectStore`, `MetadataReader`) rather than over the `#[tauri::command]` functions, since those take `State`/`AppHandle` and are not callable directly; verify with a test that a tool and its command yield the same result for the same input.
- [ ] 2.3 Name the merge tool `loop_create_content` and audit the catalog so no tool name or description says "episode"; verify with a test asserting the name and the absence of the word in the catalog.

## 3. Asynchronous runs and progress

- [ ] 3.1 Model content creation and channel batches as start-then-poll: the tool returns a run handle immediately and the work continues on the async runtime, guarded by `AppState::start_run`; verify with a test that a second start while a run is active is rejected with "Ada proses lain yang sedang berjalan".
- [ ] 3.2 Add `run_status` returning the running state, the latest progress fraction, and recent log records, read from the same `Telemetry` buffers the queue card uses; verify with a test that progress reported during a run is visible to `run_status`.
- [ ] 3.3 Wire `run_cancel` to the runner's cancellation (and the channel batch's cancellation path) so a long merge can be stopped; verify with a unit test and a manual cancel mid-merge.

## 4. Access control and confinement

- [ ] 4.1 Require the per-session bearer token on every request and reject requests without it before any tool runs; verify with a test that a missing or stale token is rejected.
- [ ] 4.2 Reject non-loopback origins/addresses; verify with a test.
- [ ] 4.3 Confine file-writing tools to the open project root: canonicalize any output path and reject anything that resolves outside it; verify with tests for a sibling directory, a `..` traversal, and a symlink escape.

## 5. Commands and settings UI

- [ ] 5.1 Add `mcp_status`, `mcp_start`, and `mcp_stop` commands, register them in `src-tauri/src/lib.rs`, and have `AppState` own the server handle; verify with `cargo check`.
- [ ] 5.2 Add `stores/mcp.svelte.ts` and a "Kontrol AI (MCP)" section in `views/SettingsDialog.svelte`: enable toggle, port, endpoint and token copy, and running state; verify with `pnpm check`.
- [ ] 5.3 Add the workspace indicator while AI control is active; verify that an AI-initiated run shows in the queue card with progress and logs.

## 6. stdio bridge

- [ ] 6.1 Dispatch on arguments in `main.rs` before building the Tauri app: when the stdio argument is present (for example `--mcp-stdio`), run the bridge and return; otherwise run the GUI unchanged. Handle the Windows `windows_subsystem = "windows"` attribute so stdio output is not swallowed in a release build.
- [ ] 6.2 Implement the bridge as a thin client of the loopback endpoint: forward MCP traffic between stdin/stdout and the HTTP endpoint, reading the port and token from the app's settings. It builds no `AppState` of its own, so a run it starts shares the app's queue card and single-run guard; verify with a test that the bridge process opens no project of its own.
- [ ] 6.3 When no running app can be reached, exit with a readable message telling the user to open the app and enable AI control; verify there is no path that starts a second process able to write to the same project.
- [ ] 6.4 Verify with a real client (Claude Desktop or Command Code) while the app is running: list tools, and confirm a content creation run started through the bridge appears in the queue card with progress and logs.

## 7. Verification

- [ ] 7.1 `cargo test --lib` passes, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean, and `pnpm check` reports 0 errors and 0 warnings.
- [ ] 7.2 Connect a real MCP client (Claude Desktop or Command Code), list the tools, and confirm the catalog and the `loop_create_content` naming.
- [ ] 7.3 End-to-end: open a project, plan a channel, run a small channel, watch the progress in the queue card, and cancel mid-run.
- [ ] 7.4 Negative cases: a request without the token is rejected; an output path outside the project is rejected; a second run while one is active is rejected.
