# asciibox-client-rs

Rust client for the [Ascii Box Public API v1](https://docs.ascii.dev/box/api/v1).

Shaped like TypeScript [`@asciidev/box-sdk`](https://www.npmjs.com/package/@asciidev/box-sdk) `BoxApi`, including lifecycle, command exec (synchronous and detached), prompts/events, files, desktop/host/SSH, snapshots, environments, and polling helpers.

Crate name on Cargo: `box_client`.

## Install

```toml
box_client = { git = "https://github.com/nessalabs/asciibox-client-rs" }
# or path = "…"
```

## Configure

```bash
export BOX_API_KEY=box_…   # from `box api-key create` or the dashboard
# optional: BOX_BASE_URL, BOX_ORG
```

```rust,no_run
use box_client::{BoxApi, Configuration};

# async fn demo() -> box_client::Result<()> {
let api = BoxApi::new(Configuration::from_env()?)?;
let list = api.boxes(None).await?;
println!("{} boxes", list.boxes.len());
# Ok(())
# }
```

## Client surface

| Method | Purpose |
| --- | --- |
| `me` / `limits` | Account |
| `boxes` / `create` / `get` / `update` / `stop` / `resume` | Lifecycle |
| `command` / `exec` / `exec_command` | Run shell in box |
| `read_file` / `write_file` / `read_text` / `write_text` | File IO |
| `host_port` / `ssh_key` | Exposure / SSH |
| `wait_until_ready` | Poll until operable |
| `prompt` / `prompt_run_status` / `events` | Agent prompts and event pages |
| `desktop` / `wait_for_desktop` | Desktop provisioning |
| `command_raw` / `command_status` | Detached command lifecycle |
| `list_snapshots` / `list_box_snapshots` / `latest_box_snapshot` | Snapshot discovery |
| `snapshot_tree` / `snapshot_download` / `snapshot_file` / `delete_snapshot` | Snapshot access and deletion |
| `environments` / `create_environment` / `update_environment` / `delete_environment` | Environment lifecycle |
| `delete_box` / `interrupt` | Destructive cleanup and interruption |
| `wait_until_idle` / `wait_for_prompt` | Bounded polling with TypeScript defaults |
| `get_deletion_operation` / `wait_for_deletion` | Observe background deletion completion |

Destructive delete methods send the API's matching confirmation header and are never retried. They return an accepted operation; use `wait_for_deletion` to verify completion. A missing box is not proof that backend deletion finished.

This is the supported runtime subset, not a full SDK port. See [the exact compatibility matrix and migration notes](docs/parity.md).

## Defaults (timeouts)

| Knob | Default | Notes |
| --- | --- | --- |
| `connect_timeout` | 10s | TCP connect |
| `request_timeout` | 60s | Most API calls |
| `command` HTTP timeout | `timeout_seconds` (default 30) **+ 15s slack**, at least `request_timeout` | Avoids cutting off long in-box commands |
| GET attempts | up to 3 total | Connect/timeout/429/502–504/`box_starting`/`box_securing` only |

GET backoff includes jitter and respects `Retry-After`; exhausted transport failures propagate through waits. Nonzero wait budgets include HTTP requests/retries and sleeps. Zero means unlimited. Responses are buffered up to a configurable 64 MiB limit (`with_max_response_bytes`).

`Configuration` redacts the access token in `Debug` (length only). `CreateBoxRequest` / `ResumeRequest` redact `env` and setup scripts; `CommandRequest` / `CommandResponse` / file IO types hide command text and stream bodies in `Debug`. Hosted port URLs are redacted in `HostPortResponse`. `Error::Api` / `Unexpected` `Display`/`Debug` omit server message/details/bodies (use `api_message()` / `api_details()` / `unexpected_body()` when you need them). Runtime environment/event/prompt payloads, repository setup scripts, and snapshot signed URLs are redacted in `Debug`, including nested wrappers. Commands are **never** auto-retried. Pass `Idempotency-Key` via `create_with_idempotency`. Non-localhost `http://` base URLs are rejected.

## Tests

```bash
cargo test
cargo clippy --all-targets -- -D warnings

# live smoke (ignored by default)
BOX_API_KEY=… cargo test --test live live_me_and_boxes -- --ignored
BOX_API_KEY=… BOX_ID=bx_… cargo test --test live live_get_and_exec_existing_box -- --ignored
```

Contract parity with the TS SDK helpers: [docs/parity.md](docs/parity.md). Testing layers: [docs/testing.md](docs/testing.md).

## License

MIT
