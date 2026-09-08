# Compatibility with `@asciidev/box-sdk@0.0.34`

This is a Rust client for the supported operations below, not a complete port of
the SDK. The reference is the published package's generated `BoxApi`, models, and
`box-helpers.ts`. Dates remain ISO strings in Rust; open response status/kind
fields remain strings so new server values survive. Known runtime fields have
typed models; unknown response fields are retained in `extra` where provided.
Those extension values and content-bearing fields are excluded from `Debug`.

## Supported methods and options

| TypeScript | Rust | Options and response coverage |
| --- | --- | --- |
| `Configuration` | `Configuration` | Bearer token, base URL, organization, transport budgets |
| `me`, `limits` | `me`, `limits` | Existing account models; limits retain extension fields |
| `boxes` | `boxes` | state, limit, cursor, sort; box and page info |
| `create` | `create`, `create_with_idempotency` | Existing request fields, explicit idempotency key |
| `get`, `update`, `stop`, `resume` | Same names | Existing box/action models |
| `command` | `command_raw` | Full synchronous/detached union; mutations are not retried |
| `command`, `execCommand` | `command`, `exec`, `exec_command` | Synchronous convenience; default command timeout 30 s |
| `commandStatus` | `command_status` | `tailBytes` through `CommandStatusQuery`; includes truncation, known/lost state, log paths, command and timing metadata |
| `prompt` | `prompt` | Required `PromptProvider`; model, reasoning effort, prompt; typed queued response and run |
| `promptRunStatus` | `prompt_run_status` | Prompt/run IDs, status, done, model and timing fields |
| `events` | `events` | limit, cursor, sort, type; optional event IDs/timestamps/data and arbitrary top-level extensions |
| `desktop` | `desktop` | vnc, public access; complete documented response fields |
| `readFile`, `writeFile`, `readText`, `writeText` | `read_file`, `write_file`, `read_text`, `write_text` | Existing file content/encoding models |
| `hostPort`, `sshKey` | `host_port`, `ssh_key` | Existing exposure models |
| `listSnapshots` | `list_snapshots` | limit, cursor, sort; no ineffective `boxId` filter |
| `listBoxSnapshots` | `list_box_snapshots` | Box-specific scope with limit, cursor and sort |
| `getLatestBoxSnapshot` | `latest_box_snapshot` | Typed nullable latest snapshot |
| `getSnapshotTree` | `snapshot_tree` | Snapshot ID only; typed entries, size/count, availability/truncation flags |
| `getSnapshotDownload` | `snapshot_download` | Typed inventory, chunks, generation, expiration and reconstruction metadata; signed URLs are redacted |
| `getSnapshotFile` | `snapshot_file` | Snapshot ID plus optional file path query; returns bytes |
| `deleteBox`, `deleteSnapshot` | `delete_box`, `delete_snapshot` | Matching confirmation header; typed accepted operation, never retried |
| `getDeletionOperation` | `get_deletion_operation` | Typed operation status, attempts and completion timestamp |
| — | `wait_for_deletion` | Waits for `completed`; `blocked` and a hidden box do not prove completion |
| `environments`, `createEnvironment` | `environments`, `create_environment` | Typed environments, secret files, selected repositories and version summaries |
| `updateEnvironment`, `deleteEnvironment` | `update_environment`, `delete_environment` | Typed update flags/content/repository requests and actual success response |
| `interrupt` | `interrupt` | Existing box action response |
| `stopAndRemove` | `stop_and_remove`, `stop_and_remove_with` | The latter preserves either the stop or deletion response through `StopOrDeleteResponse` |
| `waitUntilReady`, `waitUntilIdle` | `wait_until_ready[_with]`, `wait_until_idle[_with]` | Same success/terminal states and 5/10 minute defaults |
| `waitForPrompt`, `waitForPromptDone` | `wait_for_prompt`, `wait_for_prompt_done` | 30 minute default; both finished and failed return a terminal run |
| `waitForDesktop` | `wait_for_desktop`, `wait_for_desktop_with` | 5 minute default; `_with` exposes vnc/public-access options; empty URL is not ready |

## Intentional differences and transport limits

- A zero wait timeout means unlimited, matching TypeScript. Nonzero Rust wait
  budgets cover requests, transport retries and sleeps; the TypeScript helper has
  a soft deadline. Rust never returns a late success after the budget expires.
  Cancellation is cooperative: dropping the future stops local polling, not a
  previously accepted server operation. Synchronous deserialization still runs
  on the executor, so this is not a real-time scheduling guarantee.
- GET transport calls make at most **three attempts total** for known transient
  failures. Backoff includes jitter and honors `Retry-After` (seconds or HTTP date).
  If the server asks for a delay longer than `request_timeout`, the error is
  returned without retrying early. Wait helpers propagate exhausted transport
  errors; they do not start a second retry loop. TypeScript has no built-in retries.
- Connect/request timeouts default to 10/60 seconds. Command HTTP budgets include
  the command timeout plus 15 seconds of slack, with the normal request budget as
  the minimum. Mutations are never automatically retried.
- Responses, including errors and binary snapshot files, are buffered up to
  `Configuration::max_response_bytes` (64 MiB by default). Larger bodies fail with
  `Error::ResponseTooLarge`; raise the limit explicitly for larger artifacts.
  This bounds buffered input, not all allocations made while parsing JSON.
- Box IDs are validated more strictly than TypeScript's null check. URL segments
  preserve `encodeURIComponent`'s safe characters; empty and dot-only identifiers
  are rejected before sending a request.
- Optional JSON null and omission generally map to `Option`. The existing TTL
  request representation preserves its distinct omit/null/value semantics.

## Migration from the deferred implementation

These corrections change the uncommitted extended API surface:

- `PromptRequest::new(PromptProvider::Codex, "...")` now requires a provider.
- `command_status(box_id, process_id, query)` adds an optional `CommandStatusQuery`.
- `list_box_snapshots(box_id, query)` adds optional pagination. `SnapshotsQuery`
  has `sort` instead of the server-ignored `box_id`; use the per-box method to scope.
- `snapshot_tree(snapshot_id)` no longer takes a path. Use
  `snapshot_file(snapshot_id, Some(path))` for file access.
- `stop_and_remove_with` returns `StopOrDeleteResponse`, preserving the real branch.
- Runtime response fields are now typed according to the reference contract.
  For example, environment contents use `env_contents`, snapshot chunks use
  `Vec<SnapshotChunk>`, and deletion responses require a `DeletionOperation`.
  Event ID/timestamp/data are optional. Use the corresponding typed fields where
  data previously lived in `extra`.

## Explicitly unsupported SDK surface

`streamEvents` and `streamPrompt` async-generator helpers are not implemented;
call `events` with page cursors and the prompt status/wait methods. Named-snapshot
management, granular environment variable/secret-file/repository routes,
environment upgrades, and account administration (webhooks, API keys, retention
policy and repository selection) are also outside this supported matrix. No
claim of complete SDK or 1:1 whole-package parity is made.

## Verification

`tests/runtime_regressions.rs` covers the review findings, including realistic
hyphenated IDs, escaping/dot rejection, nested Debug redaction, required prompt
provider, multi-page snapshots, optional/extensible events, detached metadata,
confirmation headers/non-retry, deletion completion, timeout/zero semantics,
Retry-After, and response size limits.

`tests/fixtures/runtime.json` contains non-sensitive contract fixtures. To check
these fixtures and request options directly using an unpacked SDK 0.0.34 package:

```bash
node tests/verify_ts_contract.cjs /path/to/unpacked/package
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Live checks are separately ignored and credential-gated; see [testing.md](testing.md).
Passing fixtures and smoke tests does not certify sustained load, all API states,
or every optional field emitted by future server versions.
