# Proposal

## Why

`mcp-server` lets an agent create channels, open projects, and start runs in the app, but the
interface learns about almost none of it. The Svelte views load their data when they mount, and
the only events the backend emits are `task://progress`, `task://log`, and `setup://progress`. So
a channel created through MCP is invisible until the user changes menus — and, worse, a run
started through MCP never appears in the queue card at all, because the task store subscribes to
run events only while the interface itself is running a job. The `mcp-server` change already
asserts that an AI-initiated run shows in the queue card; today that is not true. The gap is not
specific to MCP: any out-of-band viewer, or a future headless path, would have the same blind
spot.

## What Changes

- Announce domain changes from the backend: a `data://changed` event carrying a scope (`channels`,
  `project`, `prefs`), emitted whenever those change — from both the Tauri commands and the MCP
  tools — through one helper on `Telemetry`.
- Announce run lifecycle: `task://started` (with a label) from `AppState::start_run` and
  `task://finished` from `finish_run`, alongside the existing `task://progress` and `task://log`.
- Subscribe globally in the interface: install the run listeners once instead of per run, so the
  queue card reflects any run — including one started through MCP — and add a `data://changed`
  listener that reloads the affected view.
- Keep the interface-initiated flow working: `tasks.run()` still drives its own progress and logs,
  without double-subscribing or clearing the states a global listener owns.

## Capabilities

### Modified Capabilities

- `app-shell`: the queue card reports runs started outside the interface (label, progress, logs,
  cancel), and the workspace reloads data that changed elsewhere instead of waiting for
  navigation.

## Impact

- **Code**: `state.rs` gains `Telemetry::notify_data`, a labelled `start_run`, and lifecycle emits
  in `start_run`/`finish_run`; `commands/{looping,channel,media}.rs` and `mcp/tools.rs` pass a
  label to `start_run` and announce channel changes; `stores/task.svelte.ts` moves its
  subscription to a global install called from `App.svelte`; new handling for `data://changed`
  that reloads the channel list, project state, and preferences.
- **Behavior**: out-of-band changes (MCP) become visible without navigation, and AI-started runs
  appear in the queue card — closing the gap the `mcp-server` app-shell delta already promised.
- **Non-goals**: real-time collaboration between multiple app instances, and a general diff/patch
  stream — the reload is a refetch for the announced scope, not a merge.
