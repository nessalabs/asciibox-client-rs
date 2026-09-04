# Parity with `@asciidev/box-sdk`

## What the TypeScript package actually ships

Inspected npm `@asciidev/box-sdk@0.0.34`:

- OpenAPI Generator client (`BoxApi`, `Configuration`, `runtime.ts`)
- Hand helpers in [`box-helpers.ts`](https://www.npmjs.com/package/@asciidev/box-sdk) (`waitUntilReady`, `execCommand`, `readText`, `writeText`, …)
- **No unit/integration test suite** in the published tarball

So “match their tests” means **match their public contract** (helpers + generated methods we implement), then encode that contract as Rust tests under `tests/parity_ts.rs`.

## Shared surface (v0.1) — 1:1 matrix

| TypeScript | Rust (`box_client`) | Covered by |
| --- | --- | --- |
| `new BoxApi(new Configuration({ accessToken, basePath }))` | `BoxApi::new(Configuration::…)?` | mock + live |
| `me()` | `me()` | `parity_ts::ts_me` |
| `limits()` | `limits()` | `parity_ts::ts_limits` |
| `boxes({ state, limit, … })` | `boxes(Some(&BoxesQuery{…}))` | `ts_boxes_list_and_query` |
| `create({ createBoxRequest, idempotencyKey })` | `create` / `create_with_idempotency` | `ts_create_with_idempotency_key` |
| `get({ boxId })` | `get(box_id)` | `ts_get_update_stop_resume` |
| `update({ boxId, updateBoxRequest })` | `update` | same |
| `stop({ boxId })` | `stop` | same |
| `resume({ boxId, resumeRequest? })` | `resume` | same |
| `command({ boxId, commandRequest })` | `command` / `exec` | via helpers |
| `execCommand(…, timeoutSeconds=30)` | `exec_command(…)` | `ts_exec_command_*` |
| `readText` / `writeText` (utf8) | `read_text` / `write_text` | `ts_read_text_write_text_utf8` |
| `hostPort` / `sshKey` | `host_port` / `ssh_key` | `ts_host_port_and_ssh_key` |
| `waitUntilReady` defaults `timeoutMs=300000`, `intervalMs=2000` | `WaitOptions::default()` | wait tests |
| success states `ready\|idle\|running` | `BoxState::is_operable` | `ts_wait_until_ready_success_states` |
| terminal `archived\|archiving\|error` | `BoxTerminal` | `ts_wait_until_ready_terminal_states` |
| API error envelope | `Error::Api` | `ts_api_error_envelope` |

Not in Rust v0.1 (TS has them; defer): `prompt`, events/stream helpers, `waitForPrompt`, desktop, snapshots, environments, `deleteBox` / `stopAndRemove({ delete: true })`.

## Behavioral deltas we intentionally keep

| Topic | TypeScript | Rust |
| --- | --- | --- |
| HTTP timeouts | browser/`fetch` defaults (no built-in connect/request split) | connect 10s + request 60s; command extends for `timeoutSeconds` |
| GET retries | none | up to 3 on transient/idempotent failures |
| `boxId` validation | runtime `RequiredError` if null/undefined | strict `bx_…` alphabet check |
| Secrets in Debug | N/A | token redacted |

## Commands

```bash
# Contract tests (no network)
cargo test --test parity_ts

# All offline tests
cargo test

# Live smoke (ignored by default)
BOX_API_KEY=… cargo test --test live -- --ignored --nocapture
BOX_API_KEY=… BOX_ID=bx_… cargo test --test live -- --ignored
```

## Next: testing env / stress

See [testing.md](testing.md). First milestone was parity with the TS contract; stress/load against a dedicated Box environment comes after.
