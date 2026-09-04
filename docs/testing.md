# Testing

## Layers

1. **Unit / contract (`tests/parity_ts.rs`, `src/*/tests`)** — wiremock, no credentials. Encodes `@asciidev/box-sdk` helper + method contracts for the v0.1 surface. Run on every change: `cargo test`.
2. **Live smoke (`tests/live.rs`, `#[ignore]`)** — real `https://ascii.dev/api/box/v1` with `BOX_API_KEY` (+ `BOX_ID` for exec). Manual / CI optional.
3. **Stress env (planned)** — dedicated Box org/environment, scripted create→exec→stop loops, concurrency, timeout edges. Not started until parity stays green.

## Live smoke

```bash
export BOX_API_KEY=box_…
export BOX_ID=bx_…          # existing operable box
cargo test --test live -- --ignored --nocapture
```

Prefer a throwaway box / short TTL when you graduate to create/destroy in live tests. Today live tests **do not create** boxes (account `canStart` may be blocked).

## Stress env (next)

When ready:

- Named Box environment (e.g. `nessa-client-ci`) with `safeForThirdParties` / secrets policy you want
- Fixture prompts: `exec_command` matrix (fast / slow / timeout)
- Concurrent `get` / `command` against one warm box
- Compare latency/error rates; keep Ascii rate limits in mind (`maxCreationRequestsPerMinute`, etc.)

Do not stress-create boxes on a personal trial wallet without checking `limits()`.
