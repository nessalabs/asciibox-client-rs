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

## Configuration and durable receipts

Prefer `BoxClientConfig` for new code; `Configuration` remains an alias. Existing
constructor/builder calls still work. Struct literals need the new `retry` field; using the constructor with builders
avoids this issue.
Defaults preserve the previous three-attempt GET retry behavior.

`Error::Api` and `Error::HttpStatus` now include `retry_after: Option<Duration>`.
Use `..` when matching only selected fields. Code constructing these variants
must supply the new field. `error.retry_after()` avoids depending on the variant.

Handoff receipts are an opt-in module with an explicit storage backend. Adding
the SDK does not create files or automatically run transfers.

`Error::Api::details` is now boxed to keep errors small after adding retry metadata.
Use `api_details()` for borrowed access, or `.map(Box::new)` when constructing the field.
Logging is controlled by the application's `tracing` subscriber, not client settings.
