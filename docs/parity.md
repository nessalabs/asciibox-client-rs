# Compatibility with `@asciidev/box-sdk@0.0.34`

The Rust client covers all **59 generated BoxApi operations** and all published
`box-helpers.ts` helpers in SDK 0.0.34. The pinned published package is the
reference; it does not ship an upstream test suite. Our fixtures exercise its
actual request builders and model converters. This is API and helper coverage;
JavaScript runtime facilities such as custom fetch middleware are not Rust APIs.

## Endpoint mapping

All methods share the same pooled HTTP client, authentication, response cap,
retry policy and error mapping. Queries and request bodies expose every field in
the reference contract. Response models retain unknown fields where `extra` is
provided. See the public request/query types for their Rust field names.

| TypeScript operation | Rust method | HTTP route |
| --- | --- | --- |
| `addEnvironmentRepo` | `add_environment_repo` | `POST /environments/{environmentId}/repos` |
| `apiKeyUsage` | `api_key_usage` | `GET /api-keys/{apiKeyId}/usage` |
| `apiKeys` | `api_keys` | `GET /api-keys` |
| `artifact` | `artifact` | `GET /boxes/{boxId}/artifacts` |
| `boxes` | `boxes` | `GET /boxes` |
| `command` | `command_raw` | `POST /boxes/{boxId}/commands` |
| `commandStatus` | `command_status` | `GET /boxes/{boxId}/commands/{processId}` |
| `create` | `create_with_options` | `POST /boxes` |
| `createEnvironment` | `create_environment` | `POST /environments` |
| `createWebhook` | `create_webhook` | `POST /webhooks` |
| `deleteBox` | `delete_box` | `DELETE /boxes/{boxId}` |
| `deleteEnvironment` | `delete_environment` | `DELETE /environments/{environmentId}` |
| `deleteEnvironmentRepo` | `delete_environment_repo` | `DELETE /environments/{environmentId}/repos/{repositoryId}` |
| `deleteEnvironmentSecretFile` | `delete_environment_secret_file` | `DELETE /environments/{environmentId}/secret-files` |
| `deleteEnvironmentVar` | `delete_environment_var` | `DELETE /environments/{environmentId}/vars/{key}` |
| `deleteNamedSnapshot` | `delete_named_snapshot` | `DELETE /named-snapshots/{name}` |
| `deleteSnapshot` | `delete_snapshot` | `DELETE /snapshots/{snapshotId}` |
| `deleteWebhook` | `delete_webhook` | `DELETE /webhooks/{webhookId}` |
| `desktop` | `desktop_with` | `POST /boxes/{boxId}/desktop` |
| `environments` | `environments` | `GET /environments` |
| `events` | `events` | `GET /boxes/{boxId}/events` |
| `fork` | `fork` | `POST /boxes/{boxId}/fork` |
| `get` | `get` | `GET /boxes/{boxId}` |
| `getDataRetention` | `get_data_retention` | `GET /account/data-retention` |
| `getDeletionOperation` | `get_deletion_operation` | `GET /deletion-operations/{operationId}` |
| `getLatestBoxSnapshot` | `latest_box_snapshot` | `GET /boxes/{boxId}/snapshots/latest` |
| `getNamedSnapshot` | `get_named_snapshot` | `GET /named-snapshots/{name}` |
| `getSnapshotDownload` | `snapshot_download` | `GET /snapshots/{snapshotId}/download` |
| `getSnapshotFile` | `snapshot_file` | `GET /snapshots/{snapshotId}/files` |
| `getSnapshotTree` | `snapshot_tree` | `GET /snapshots/{snapshotId}/tree` |
| `getWebhook` | `get_webhook` | `GET /webhooks/{webhookId}` |
| `hostPort` | `host_port_with` | `POST /boxes/{boxId}/host` |
| `interrupt` | `interrupt` | `POST /boxes/{boxId}/interrupt` |
| `limits` | `limits_with` | `GET /limits` |
| `listBoxSnapshots` | `list_box_snapshots` | `GET /boxes/{boxId}/snapshots` |
| `listNamedSnapshots` | `list_named_snapshots` | `GET /named-snapshots` |
| `listSnapshots` | `list_snapshots` | `GET /snapshots` |
| `listWebhooks` | `list_webhooks` | `GET /webhooks` |
| `me` | `me` | `GET /me` |
| `prompt` | `prompt` | `POST /boxes/{boxId}/prompt` |
| `promptRunStatus` | `prompt_run_status` | `GET /boxes/{boxId}/prompts/{promptId}` |
| `readFile` | `read_file` | `GET /boxes/{boxId}/files` |
| `repos` | `repos` | `GET /repos` |
| `resume` | `resume` | `POST /boxes/{boxId}/resume` |
| `rotateWebhookSigningSecret` | `rotate_webhook_signing_secret` | `POST /webhooks/{webhookId}/rotate` |
| `saveNamedSnapshot` | `save_named_snapshot` | `POST /named-snapshots` |
| `secrets` | `secrets` | `GET /secrets` |
| `selectRepo` | `select_repo` | `POST /repos` |
| `setEnvironmentSecretFile` | `set_environment_secret_file` | `PUT /environments/{environmentId}/secret-files` |
| `setEnvironmentVar` | `set_environment_var` | `PUT /environments/{environmentId}/vars/{key}` |
| `sshKey` | `ssh_key` | `POST /boxes/{boxId}/sshkey` |
| `stop` | `stop` | `POST /boxes/{boxId}/stop` |
| `update` | `update` | `PATCH /boxes/{boxId}` |
| `updateDataRetention` | `update_data_retention` | `PATCH /account/data-retention` |
| `updateEnvironment` | `update_environment` | `PUT /environments/{environmentId}` |
| `updateSecrets` | `update_secrets` | `POST /secrets` |
| `updateWebhook` | `update_webhook` | `PATCH /webhooks/{webhookId}` |
| `upgradeEnvironment` | `upgrade_environment` | `POST /environments/{environmentId}/upgrade` |
| `writeFile` | `write_file` | `PUT /boxes/{boxId}/files` |

`create`, `create_with_idempotency`, `limits`, `desktop` and `host_port` remain
convenient wrappers. The full variants expose per-call organization headers and
query scope, create/fork idempotency keys, desktop theme, and hosted port
public/title options. Per-call organization headers replace the configured
header rather than appending a duplicate. `command_raw` preserves both completed
and detached responses; `command`, `exec` and `exec_command` return synchronous
results. `command_status` accepts a process ID string so a returned integer can be
passed as `&process_id.to_string()`.

## Helpers and streams

| TypeScript helper | Rust helper |
| --- | --- |
| `execCommand` | `exec_command`, `BoxApi::exec` |
| `readText`, `writeText` | `read_text`, `write_text` |
| `stopAndRemove` | `stop_and_remove`, `stop_and_remove_with` |
| `waitUntilReady`, `waitUntilIdle` | `wait_until_ready[_with]`, `wait_until_idle[_with]` |
| `waitForPrompt`, `waitForPromptDone` | `wait_for_prompt`, `wait_for_prompt_done` |
| `waitForDesktop` | `wait_for_desktop`, `wait_for_desktop_with` |
| `streamEvents`, `streamPrompt` | `stream_events`, `stream_prompt` |
| Additional deletion helper | `wait_for_deletion` |

Streams implement `Stream<Item = Result<BoxEvent>>`. Pin them before calling
`StreamExt::next`, or consume with `TryStreamExt::try_collect` for finite prompt
streams. Event streams default to unlimited duration, one-second polling,
100 events per page, and excluding existing history. Prompt streams default to
30 minutes and queue one prompt on their first poll. They filter by that prompt's
task ID while advancing the cursor across every event in the page.

Each stream fetches one page at a time, without background prefetch. Dropping a
stream stops local work. An optional exported `CancellationToken` interrupts
requests, sleeps and consumption of buffered events with `Error::Aborted`;
cancellation cannot undo an accepted server operation. Failures are yielded once
and end the stream. See [the runnable example](../examples/stream_prompt.rs).

The Rust stream deliberately fixes two SDK helper edge cases: its descending
one-event bootstrap excludes history larger than the SDK's first 200 ascending
events, and it drains all pages before ending a prompt. A terminal status triggers
one final refresh for events persisted between the last poll and status check.
It does not guarantee delivery of events persisted after that refresh. Pages must
advance their cursor; malformed or repeated pages fail before yielding duplicates.

## Language and transport differences

- Dates remain ISO strings. Open response statuses/kinds remain strings so future
  server values survive. Integer IDs, counts, ports and sizes use Rust integer
  types; fractional balances/usage use `f64`. The SDK uses JavaScript numbers.
- Nullable request fields preserve omit/null/value using nested `Option`:
  `None` omits, `Some(None)` sends null, `Some(Some(value))` sends a value. This
  applies to TTL, webhook names and prompt model/reasoning settings. Response
  `Option` generally combines absent and null; serialization may add nulls for
  absent response fields. Rust emits the canonical `private` repository field;
  the SDK converter also leaks an internal `_private` alias in one nested model.
- A zero wait timeout means unlimited. Nonzero budgets cover HTTP requests,
  retries, sleeps and buffered stream consumption. The SDK has a soft deadline;
  Rust checks again before returning a result. Cancellation is cooperative, and
  synchronous JSON decoding is not a real-time scheduling guarantee.
- GET requests make at most **three attempts total** for connect/timeout errors,
  HTTP 429/502–504, or 409 `box_starting`/`box_securing`. Backoff has jitter and
  honors `Retry-After` seconds or HTTP dates. A delay above `request_timeout`
  returns the error without retrying early. Waits and streams propagate exhausted
  transport errors. The SDK does not add these retries.
- Mutations are never automatically retried, even with an idempotency key.
  Connect/request timeouts default to 10/60 seconds. Command HTTP budgets include
  the command timeout plus 15 seconds, with the request timeout as the minimum.
- Responses, including errors and binary downloads, are buffered up to
  `max_response_bytes` (64 MiB by default), including chunked transfers without
  a Content-Length. Larger bodies return `Error::ResponseTooLarge`. This limits
  buffered input, not all allocations during JSON parsing.
- Box IDs are validated more strictly than the SDK. URL path segments preserve
  `encodeURIComponent` safe characters; empty/dot-only identifiers are rejected.
  Query paths are encoded as query values. File reads/writes reject empty paths.
- Destructive delete methods automatically send the matching confirmation header.
  They return a typed accepted operation. Use `wait_for_deletion` to verify
  completion; a hidden box and a `blocked` deletion are not proof of completion.
  The five-minute default is a caller budget. Box documents no deletion SLA or
  blocked-state retry schedule; choose a longer or unlimited wait as needed.
  [Deletion documentation](https://docs.ascii.dev/box/data-retention).
- API errors preserve status, code, request ID, message and details. Non-envelope
  HTTP errors preserve their status and a bounded raw body as `Error::HttpStatus`.
  Error formatting hides messages, bodies and decoding values; transport errors
  strip request URLs. Inspect the accessors deliberately when full data is needed.
  HTTP 2xx responses preserve `ok: false`, matching SDK transport behavior.

## Migration

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

## Verification

See [testing.md](testing.md) for the full commands. `tests/sdk_operations.rs`
checks all 59 operations with complete and minimal SDK request/response fixtures,
HTTP 400 errors, and non-retry of mutation HTTP 503 responses.
`tests/streams.rs`, `tests/error_contracts.rs` and `tests/runtime_regressions.rs`
cover behavior that one request/response fixture cannot establish.

```bash
node scripts/sdk_fixtures.cjs /path/to/unpacked/package
node tests/verify_ts_contract.cjs /path/to/unpacked/package
cargo test
cargo clippy --all-targets -- -D warnings
```

These local mock-server checks require no credentials. Live tests are separately
ignored and credential-gated. Fixture and smoke checks do not certify sustained
load, every server state, or compatibility with future SDK releases.
