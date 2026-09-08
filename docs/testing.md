# Testing

## Offline checks

```bash
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Unit tests cover configuration, validation, error formatting and core models.
`tests/parity_ts.rs` preserves the original API/helper contracts.
`tests/runtime_regressions.rs` adds targeted tests for the review findings, using
non-sensitive fixtures under `tests/fixtures/`. No credential is needed.

To independently validate those fixtures and the corrected request shapes against
the published SDK, unpack `@asciidev/box-sdk@0.0.34` and run:

```bash
node tests/verify_ts_contract.cjs /path/to/unpacked/package
```

The Node script uses the package's actual converters and request builders. It
checks nested wire values and request options without making API requests. The
package path is an argument; no machine-specific path or dependency is checked in.

## Live checks

Each live test is ignored by default. Supply a short-lived `BOX_API_KEY` through
the process environment; never save it in source, fixtures or output artifacts.
Run a named test so mutation requirements are explicit:

```bash
# Account and list smoke
cargo test --test live live_me_and_boxes -- --ignored
# Read-only environment, snapshot tree and download-manifest contracts
cargo test --test live live_runtime_read_contracts -- --ignored
# Existing operable box: requires BOX_ID; runs a harmless shell command
cargo test --test live live_get_and_exec_existing_box -- --ignored
# Creates, updates and deletes only its own uniquely named environment
BOX_TEST_MUTATIONS=1 cargo test --test live live_environment_round_trip -- --ignored
```

`live_runtime_read_contracts` reads at most one snapshot tree/download manifest and
prints counts only. Without a snapshot fixture those routes are skipped. The
environment mutation test cleans up its fixture even if the update/read checks
fail, then verifies that the environment is absent. None of these tests creates a
box or deletes an existing snapshot. For deletion lifecycle checks, a returned
operation must be observed as `completed`; box invisibility alone is insufficient.

## Remaining verification scope

The fixed encoding and typed models were checked against the live environment
update/delete lifecycle and a nonempty snapshot tree/download manifest. These are
smoke checks, not load benchmarks or a complete state-space test. Provider-backed
prompt completion, full desktop availability, large output recovery and actual
multi-page events need suitable live fixtures. Offline tests cover those modeled
contracts and the shared polling logic where noted in the parity matrix.

For load testing, use a dedicated organization and bounded disposable resources;
measure command-log transfer, response size, latency, cancellation and rate-limit
behavior. Check `limits()` before creating compute resources. No stress-create
loop is part of the normal test suite.
