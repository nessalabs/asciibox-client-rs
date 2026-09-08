# Migration guide

All repository examples and tests use the corrected signatures. For callers of
the earlier extended implementation:

- Construct prompts with `PromptRequest::new(PromptProvider::Codex, "...")`.
  Explicit model/reasoning strings now use `Some(Some(value))`; use `Some(None)`
  for an explicit null, or `None` for the server default.
- `command_status(box_id, process_id, query)` adds optional `CommandStatusQuery`.
- `list_box_snapshots(box_id, query)` adds pagination. `SnapshotsQuery` has `sort`
  instead of the server-ignored `box_id`; use the per-box route for box scope.
- `snapshot_tree(snapshot_id)` takes no path. Use
  `snapshot_file(snapshot_id, Some(path))` for bytes of a file.
- `stop_and_remove_with` returns `StopOrDeleteResponse`, preserving the real branch.
  Its payloads are boxed to keep the enum compact; field access dereferences them
  automatically, or use `*response` to take the owned payload.
- `LimitsResponse` and runtime models now expose typed fields. Read known values
  from those fields instead of `extra`. Environment contents use `env_contents`,
  snapshot chunks use `Vec<SnapshotChunk>`, deletion responses contain a
  `DeletionOperation`, and event ID/timestamp/data are optional.
- File writes now use the documented `PUT` route, SSH key bodies use `key`, and
  interrupt sends no invented JSON body. Absent optional stop/resume bodies remain
  absent instead of becoming `{}`. Caller signatures stay the same.
- The tested minimum compiler is Rust 1.89, matching current dependencies. The
  previous 1.74 declaration was not supported by the resolved dependency graph.

