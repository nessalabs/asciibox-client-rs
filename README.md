# asciibox-client-rs

Rust client for the [Ascii Box Public API v1](https://docs.ascii.dev/box/api/v1).

Shaped like TypeScript [`@asciidev/box-sdk`](https://www.npmjs.com/package/@asciidev/box-sdk) `BoxApi` for the subset we need first: lifecycle, command exec, files, host/SSH, and `wait_until_ready`.

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

## v0.1 surface

| Method | Purpose |
| --- | --- |
| `me` / `limits` | Account |
| `boxes` / `create` / `get` / `update` / `stop` / `resume` | Lifecycle |
| `command` / `exec` / `exec_command` | Run shell in box |
| `read_file` / `write_file` / `read_text` / `write_text` | File IO |
| `host_port` / `ssh_key` | Exposure / SSH |
| `wait_until_ready` | Poll until operable |

Not in v0.1: `prompt`, events stream, snapshots, environments, detached commands, `deleteBox`.

## Defaults (timeouts)

| Knob | Default | Notes |
| --- | --- | --- |
| `connect_timeout` | 10s | TCP connect |
| `request_timeout` | 60s | Most API calls |
| `command` HTTP timeout | `timeout_seconds` (default 30) **+ 15s slack**, at least `request_timeout` | Avoids cutting off long in-box commands |
| GET retries | up to 3 | Connect/timeout/429/502–504/`box_starting`/`box_securing` only |

`Configuration` redacts the access token in `Debug` (length only). `CreateBoxRequest` / `ResumeRequest` redact `env` and setup scripts; `CommandRequest` / `CommandResponse` / file IO types hide command text and stream bodies in `Debug`. Hosted port URLs are redacted in `HostPortResponse`. `Error::Api` / `Unexpected` `Display`/`Debug` omit server message/details/bodies (use `api_message()` / `api_details()` / `unexpected_body()` when you need them). Commands are **never** auto-retried. Pass `Idempotency-Key` via `create_with_idempotency`. Non-localhost `http://` base URLs are rejected.

## Tests

```bash
cargo test
cargo clippy --all-targets -- -D warnings

# live smoke (ignored by default)
BOX_API_KEY=… cargo test --test live -- --ignored
BOX_API_KEY=… BOX_ID=bx_… cargo test --test live -- --ignored
```

Contract parity with the TS SDK helpers: [docs/parity.md](docs/parity.md). Testing layers: [docs/testing.md](docs/testing.md).

## License

MIT
