# Rust SDK guide

`box_client` is an asynchronous client for the Ascii Box API. Use it to create and
operate cloud sandboxes, execute commands, read and write files, run agent prompts,
consume events, and manage snapshots and environments.

`BoxApi` is the entry point. Its methods return typed `Result<T>` values and use a
shared connection pool. Cloning the client is inexpensive.

## Install and configure

Requires Rust 1.89 or newer.

```toml
[dependencies]
box_client = { git = "https://github.com/nessalabs/asciibox-client-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3" # StreamExt for event streams
```

Configure an API key from the Box dashboard or CLI:

```bash
export BOX_API_KEY=box_…
# Optional: BOX_ORG, BOX_BASE_URL
```

```rust,no_run
use box_client::{BoxApi, Configuration, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let api = BoxApi::new(Configuration::from_env()?)?;
    let boxes = api.boxes(None).await?;
    for item in boxes.boxes {
        println!("{}: {}", item.id, item.state.as_str());
    }
    Ok(())
}
```

You can also construct `Configuration::new(api_key)` directly and use its builder
methods to set the organization, base URL, timeouts, and response-size limit.

## Create a Box, run commands, and use files

This example creates a Box with a 30-minute lifetime, waits for it to become
available, writes a file, runs a command, and reads the result. It requests
stop/archive even if one of those operations returns an error. Archive retains
snapshots for later resume.

```rust,no_run
use box_client::{
    read_text, wait_until_ready, write_text, BoxApi, CreateBoxRequest, Result,
};

async fn run_job(api: &BoxApi) -> Result<String> {
    let created = api.create(CreateBoxRequest::ttl(1800)).await?;
    let box_id = &created.box_.id;

    let result = async {
        wait_until_ready(api, box_id).await?;
        write_text(api, box_id, "/tmp/input.txt", "hello from Rust").await?;
        let command = api.exec(box_id, "cat /tmp/input.txt > /tmp/output.txt").await?;
        println!("exit code: {:?}", command.exit_code);
        read_text(api, box_id, "/tmp/output.txt").await
    }.await;

    let stopped = api.stop(box_id, None).await;
    stopped?;
    result
}
```

`exec` waits for a command result. For a custom directory or timeout, use
`CommandRequest::new(command).with_cwd(path).with_timeout_seconds(seconds)` with
`command`. For background execution, set `detached = Some(true)` and call
`command_raw`. It returns `CommandResult::Started` with a process ID; query its
logs and completion through `command_status`.

`read_file` and `write_file` expose file content and encoding directly.
`snapshot_file` and `artifact` return binary `Vec<u8>` values.

## Run a prompt and consume events

`prompt` queues work for a configured provider. `wait_for_prompt` returns a run
once it reaches `finished` or `failed`. Use `stream_prompt` to consume that
prompt's events as they arrive:

```rust,no_run
use box_client::{
    stream_prompt, BoxApi, CancellationToken, PromptProvider, PromptRequest,
    Result, StreamPromptOptions,
};
use futures_util::{pin_mut, StreamExt};

async fn run_prompt(api: &BoxApi, box_id: &str, cancellation: CancellationToken) -> Result<()> {
    let request = PromptRequest::new(PromptProvider::Codex, "Inspect the project and summarize it.");
    let events = stream_prompt(api, box_id, request, Some(StreamPromptOptions {
        cancellation: Some(cancellation),
        ..Default::default()
    }));
    pin_mut!(events);

    while let Some(event) = events.next().await {
        let event = event?;
        println!("{}", event.type_);
        // Inspect event.data deliberately when you need the event payload.
    }
    Ok(())
}
```

Call `cancel()` on a clone of the token to interrupt local requests, sleeps, or
consumption of buffered events. Cancellation returns `Error::Aborted` and cannot
undo a prompt already accepted by the server. Dropping the stream stops local
work too.

Streams start on their first poll and fetch one page at a time. `stream_events`
defaults to new events only; use `StreamEventsOptions { include_existing: true,
..Default::default() }` to include history. It supports event-type filters and a
page-size limit. `events` exposes an individual page and its cursor directly.

Prompt streams filter events by the queued prompt's task ID and drain pages
before completion. A terminal run status triggers one final refresh to collect
recently persisted events. Repeated or unusable cursors return an error before
replaying a page. Events persisted after the final refresh are outside that
stream's observation window.

## Snapshots and environments

List snapshots globally or for a Box. Follow `page_info.next_cursor` when
`page_info.has_more` is true:

```rust,no_run
use box_client::{BoxApi, Result, SnapshotsQuery};

async fn inspect_snapshots(api: &BoxApi, box_id: &str) -> Result<()> {
    let first = api.list_box_snapshots(box_id, Some(&SnapshotsQuery {
        limit: Some(20),
        sort: Some("desc".into()),
        ..Default::default()
    })).await?;

    for snapshot in first.snapshots {
        println!("{}", snapshot.id);
    }
    if let Some(page) = first.page_info.filter(|page| page.has_more) {
        let next = api.list_box_snapshots(box_id, Some(&SnapshotsQuery {
            limit: Some(20),
            sort: Some("desc".into()),
            cursor: page.next_cursor,
        })).await?;
        println!("{} snapshots on the next page", next.snapshots.len());
    }
    Ok(())
}
```

`snapshot_tree` inspects a snapshot's files. `snapshot_file` downloads a file or
folder, and `snapshot_download` returns the reconstruction manifest. Named
snapshots have separate save, list, get, and delete methods.

Environment methods manage reusable variables, secret files, repositories, and
versions. For example:

```rust,no_run
use box_client::{BoxApi, Result, SetEnvironmentVarRequest};

async fn configure_environment(api: &BoxApi, environment_id: &str) -> Result<()> {
    api.set_environment_var(environment_id, "APP_MODE", SetEnvironmentVarRequest {
        value: "development".into(),
    }).await?;
    Ok(())
}
```

The client also provides webhook management, API-key usage, repository selection,
account limits, secrets, and data-retention policy methods. `update_secrets`
replaces the secret configuration, so include every value that should remain.
Changing data retention requires an interactive session token.

## Permanent deletion

`stop` archives a Box for later resume. `delete_box` permanently deletes it and
returns an accepted background operation. The client supplies the required
confirmation header. Observe that operation to confirm cleanup:

```rust,no_run
use box_client::{wait_for_deletion, BoxApi, Result, WaitOptions};

async fn delete_and_wait(api: &BoxApi, box_id: &str) -> Result<()> {
    let accepted = api.delete_box(box_id).await?;
    let operation = wait_for_deletion(api, &accepted.operation.id, Some(
        WaitOptions::from_millis(900_000, 5_000),
    )).await?;
    println!("deletion {} completed", operation.id);
    Ok(())
}
```

Only `completed` proves cleanup finished. A Box can disappear from normal reads
while its deletion remains `pending`, `processing`, or `blocked`. The default
five-minute wait is a caller budget; Box documents no completion deadline or
blocked-state retry schedule. Choose a longer timeout, zero for unlimited
observation, or check `get_deletion_operation` later. See the
[Box deletion documentation](https://docs.ascii.dev/box/data-retention).

## Options and errors

Request structs use `Option` for optional fields and snake_case Rust names for
JSON fields. Nullable settings such as TTL, webhook names, and prompt model or
reasoning options use nested `Option`:

| Rust value | Request behavior |
| --- | --- |
| `None` | Omit the field |
| `Some(None)` | Send JSON null |
| `Some(Some(value))` | Send the value |

Dates are ISO strings. Extensible response statuses and kinds remain strings;
models with `extra` retain unrecognized fields. Response `Option` values generally
combine absent and null. IDs, counts and byte sizes use integer types, while
fractional balances and usage use `f64`.

Every API call returns `box_client::Result<T>`. Inspect `Error::status()`,
`api_message()`, and `api_details()` for a server failure. Non-envelope HTTP errors
use `Error::HttpStatus`; decoding errors use `Error::Serde`. An HTTP 2xx response
still preserves the payload's `ok` field, including `false`.

Error formatting omits server messages, details, raw bodies, and decoding values.
Transport errors strip request URLs. Content-bearing model fields are redacted
from `Debug`; deliberate field access still exposes their values.

## Timeouts and retries

| Setting | Default |
| --- | --- |
| Connect timeout | 10 seconds |
| HTTP request timeout | 60 seconds |
| Command timeout through `exec` | 30 seconds |
| Command HTTP timeout | Command timeout + 15 seconds, at least the request timeout |
| Ready / desktop / deletion wait | 5 minutes |
| Idle wait | 10 minutes |
| Prompt wait / stream | 30 minutes |
| Event stream | Unlimited |
| Wait polling interval | 2 seconds |
| Stream polling interval / page size | 1 second / 100 events |
| Buffered response limit | 64 MiB |

Nonzero wait budgets include requests, retries, sleeps, and buffered stream
consumption. A zero timeout means unlimited observation. Cancellation is
cooperative; synchronous JSON parsing is not a real-time scheduling guarantee.

GET calls make at most three attempts for connection/timeouts, HTTP 429/502–504,
or 409 `box_starting` / `box_securing`. Backoff includes jitter and respects
`Retry-After`. A requested delay longer than the HTTP request timeout returns the
error without retrying early. Waits and streams propagate exhausted errors.

Mutations are never automatically retried. Use `create_with_idempotency` or the
full create/fork options when you need to supply an idempotency key for your own
retry handling. Responses above the configured buffer limit return
`Error::ResponseTooLarge`, including errors and chunked downloads.
