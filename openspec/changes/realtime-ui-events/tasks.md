# Tasks

## 1. Backend events

- [x] 1.1 Add `Telemetry::notify_data(scope)` emitting `data://changed` with `{ scope }`, and call it after channel changes in `commands/channel.rs` and `mcp/tools.rs`, in `AppState::activate_project` / `close_project` (scope `project`), and in `prefs_set` (scope `prefs`).
- [x] 1.2 Give `AppState::start_run` a human label and emit `task://started` with `{ label }`; emit `task://finished` in `finish_run`; update every `start_run` call site (looping, channel, media, and the MCP tools) with a label.
- [x] 1.3 Verify `cargo check`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` are clean.

## 2. Frontend subscriptions

- [x] 2.1 Move the run listeners out of `tasks.run()` into one global subscription (`install()`, called once from `App.svelte` on mount) covering `task://started`, `task://progress`, `task://log`, and `task://finished`, so an externally started run shows in the queue card.
- [x] 2.2 Make the card handle an external run: take its label from `task://started`, engage only when no interface run owns the card, and clear on `task://finished` without breaking the interface-initiated flow.
- [x] 2.3 Add a `data://changed` listener that reloads the announced scope — the channel list, the project state (so the shell screen follows), and preferences.
- [x] 2.4 Verify `pnpm check` and `pnpm build`.

## 3. Verification

- [x] 3.1 `cargo test --lib` passes, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean, and `pnpm check` / `pnpm build` succeed.
- [ ] 3.2 Live, with AI control on: create a channel through MCP while the EZ Loop view is open and confirm the list refreshes without navigation; start a run through MCP and confirm the queue card shows it with progress and logs; cancel it from the interface.
