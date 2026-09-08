# Ascii Box Rust SDK

An asynchronous Rust client for the [Ascii Box API](https://docs.ascii.dev/box/api/v1).
Create cloud sandboxes, execute commands, read and write files, run agent prompts,
stream events, and manage snapshots, environments, and account settings.

The crate is `box_client` and requires Rust 1.89 or newer.

## Install

```toml
[dependencies]
box_client = { git = "https://github.com/nessalabs/asciibox-client-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick start

Set `BOX_API_KEY` to a key from the Box dashboard or CLI. `BOX_ORG` and
`BOX_BASE_URL` are optional.

```rust
use box_client::{BoxApi, BoxClientConfig, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let api = BoxApi::new(BoxClientConfig::from_env()?)?;
    let response = api.boxes(None).await?;

    for item in response.boxes {
        println!("{}: {}", item.id, item.state.as_str());
    }
    Ok(())
}
```

## Documentation

- [SDK guide](docs/guide.md): complete Rust examples for commands, files, prompts,
  streams, snapshots, environments, cleanup, and error handling.
- [API reference](docs/api.md): all 59 endpoint operations and convenience helpers.
- [Handoff receipts](docs/handoff.md): durable progress records, checkpoint references,
  application callbacks, and configurable storage.
- [Testing](docs/testing.md): local checks, fixture verification, and live tests.

Build searchable API documentation with method signatures and request/response
types:

```bash
cargo doc --no-deps --open
```

## Main APIs

| Area | Entry points |
| --- | --- |
| Boxes | `boxes`, `create`, `get`, `update`, `stop`, `resume`, `fork`, `delete_box` |
| Commands | `exec`, `command`, `command_raw`, `command_status`, `interrupt` |
| Files | `read_file`, `write_file`, `read_text`, `write_text`, `artifact` |
| Prompts and events | `prompt`, `prompt_run_status`, `events`, `stream_prompt`, `stream_events` |
| Desktop and access | `desktop`, `host_port`, `ssh_key` |
| Snapshots | `list_snapshots`, `list_box_snapshots`, `snapshot_tree`, `snapshot_file`, `snapshot_download` |
| Named snapshots | `save_named_snapshot`, `get_named_snapshot`, `list_named_snapshots`, `delete_named_snapshot` |
| Environments | Environment lifecycle, variables, secret files, repositories, and upgrades |
| Account | Limits, API-key usage, webhooks, repositories, secrets, and retention policy |
| Waiting | `wait_until_ready`, `wait_until_idle`, `wait_for_prompt`, `wait_for_desktop`, `wait_for_deletion` |

## Runtime behavior

The client shares a connection pool across clones. Streams are lazy and support
`CancellationToken`. Waits bound requests, retries, and sleeps; zero timeout means
unlimited observation.

GET requests retry transient failures up to three attempts by default and honor
`Retry-After`. `BoxClientConfig` exposes attempts, exponential backoff and jitter.
Diagnostics use application-filtered `tracing`.
Mutations are never automatically retried. Responses have a configurable 64 MiB
buffer limit. Error formatting and content-bearing model `Debug` output redact
sensitive data.

Permanent deletion returns an accepted operation. Only `completed` confirms
cleanup; a hidden Box or a `blocked` operation does not. The deletion wait timeout
is a caller budget, not a service completion guarantee.

## Examples and checks

```bash
cargo run --example list_boxes
RUST_LOG=box_client=debug cargo run --example logging
BOX_ID=bx_… cargo run --example exec_smoke
BOX_ID=bx_… BOX_PROMPT='Summarize the project' cargo run --example stream_prompt

cargo test
cargo clippy --all-targets -- -D warnings
```

## License

MIT
