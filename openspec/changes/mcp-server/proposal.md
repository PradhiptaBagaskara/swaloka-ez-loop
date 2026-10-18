# Proposal

## Why

Swaloka's value is its FFmpeg pipeline and EZ Loop planner, but the only way to drive
them today is by hand through the Svelte interface. Every capability is already a
well-typed Rust command over `AppState` (`loop_run`, `channel_plan`, `channel_run`, …)
with serde payloads, so an agent could do the same work — plan a channel, create
content, batch-process plans, read metadata — if it had a standard way in. The Model
Context Protocol (MCP) is that standard: MCP-capable clients (Claude Desktop, Command
Code, IDEs) speak it natively, so embedding an MCP server turns the app from a manual
tool into something an agent can operate. Because the domain logic already lives behind
`AppState` and is independent of the webview, the server can run inside the app and
share the exact workspace the user is looking at, instead of re-implementing the
pipeline beside it. Some MCP clients speak only stdio, so the surface must also be
reachable from a shell — but it must be that same surface, not a second copy of it.

## What Changes

- Embed an MCP server in the Tauri process, disabled until enabled from settings,
  reachable only on `127.0.0.1` over Streamable HTTP, sharing the running app's
  `AppState`, active project, single-run guard, and telemetry — so an AI-initiated run
  appears in the queue card exactly like a manual one.
- Reach clients that speak only stdio through a **stdio bridge**: the binary inspects
  its arguments and, when asked (for example `swaloka --mcp-stdio`), serves MCP over
  stdin and stdout by forwarding to the running app's loopback endpoint. The bridge
  builds no application state of its own, so the agent and the user keep one process,
  one active project, and one single-run guard — the AI's work still appears in the same
  queue card, which is the whole point.
- Require the app to be running with AI control enabled: when the bridge cannot reach
  it, it exits with a readable message instead of starting a second process, so two
  processes can never write to the same project at once.
- Expose an action-oriented tool catalog built on the MCP Rust SDK (`rmcp`): project
  open/create/status/close, `ffmpeg_status`, `loop_preflight`/`loop_metadata`/
  `loop_create_content`, the channel set (`channel_list` → `channel_run`/
  `channel_cancel`), `media_info`, and `run_status`/`run_cancel`.
- Use the domain's own vocabulary: the merge tool is `loop_create_content`. The app
  speaks of "konten" — never "episode" — and the tool surface must match.
- Model long-running work as start-then-poll: content creation and channel batches
  return promptly with a run handle, and the client follows `run_status` for progress
  and logs and stops work with `run_cancel`, because a merge can run for minutes and
  MCP calls are request/response.
- Scope writes to the open project: tools refuse paths that resolve outside the project
  root, and tools that need a project fail with a readable message until one is open.
- Require a per-session bearer token, shown in settings, and reject requests without it,
  so a loopback listener is not an open door for any local process or page.
- Surface control in the UI: a "Kontrol AI (MCP)" settings section to enable/disable,
  set the port, and copy the endpoint and token, plus a workspace indicator while AI
  control is on.

## Capabilities

### Added Capabilities

- `mcp-server`: the MCP surface — the in-app loopback endpoint and the stdio bridge to
  it, the shared tool catalog, the project-scoped session, asynchronous runs with
  progress and cancellation, and the loopback/token safety rules.

### Modified Capabilities

- `app-shell`: adds the "Kontrol AI (MCP)" settings section (enable, port, endpoint and
  token, running status) and a workspace indicator while AI control is active.

## Impact

- **Code**: `src-tauri/Cargo.toml` (add `rmcp` and an HTTP transport such as `axum`);
  new `src-tauri/src/mcp/` module (server bootstrap, tool definitions, and a thin
  adapter that calls the same `AppState` services the commands use); `state.rs` gains an
  MCP server handle plus enable/disable; new commands `mcp_status`/`mcp_start`/
  `mcp_stop` registered in `lib.rs`; new `stores/mcp.svelte.ts` and a settings section;
  a workspace indicator. The queue card already renders telemetry, so it needs no change.
- **stdio bridge**: `main.rs` dispatches on its arguments before building the Tauri app,
  and the bridge is a thin client of the loopback endpoint — no second `AppState`, no
  second server, no core refactor. The one shared thing it must read is the endpoint port
  and token, taken from the app's settings.
- **Behavior**: AI runs and user runs share the single-run guard, so a merge started by
  the agent blocks a manual one and vice versa.
- **Security**: the listener is loopback-only and token-gated; file-writing tools are
  confined to the open project; the server stays off until the user turns it on and is
  not restarted automatically on a later launch.
- **Non-goals**: remote (non-loopback) access, a server that runs without the GUI, a
  standalone CLI that is not the MCP surface, and any tool that bypasses the project root
  are out of scope for this change.
