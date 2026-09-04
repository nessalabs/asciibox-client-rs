use std::time::Duration;

use tokio::time::{sleep, Instant};

use crate::client::BoxApi;
use crate::error::{Error, Result};

/// Mirrors TS `WaitOptions` in `@asciidev/box-sdk` `box-helpers.ts`.
#[derive(Debug, Clone)]
pub struct WaitOptions {
    /// TS: `timeoutMs` (default 300_000 for `waitUntilReady`).
    pub timeout: Duration,
    /// TS: `intervalMs` (default 2_000).
    pub poll_interval: Duration,
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(300_000),
            poll_interval: Duration::from_millis(2_000),
        }
    }
}

impl WaitOptions {
    pub fn from_millis(timeout_ms: u64, interval_ms: u64) -> Self {
        Self {
            timeout: Duration::from_millis(timeout_ms),
            poll_interval: Duration::from_millis(interval_ms),
        }
    }
}

/// TS `waitUntilReady`: poll until `ready` | `idle` | `running`.
///
/// Terminal failure states match the TypeScript helper: `archived`, `archiving`, `error`.
pub async fn wait_until_ready(api: &BoxApi, box_id: &str) -> Result<crate::types::Box> {
    wait_until_ready_with(api, box_id, WaitOptions::default()).await
}

pub async fn wait_until_ready_with(
    api: &BoxApi,
    box_id: &str,
    opts: WaitOptions,
) -> Result<crate::types::Box> {
    let deadline = Instant::now() + opts.timeout;

    loop {
        match api.get(box_id).await {
            Ok(info) => {
                let state = info.box_.state.clone();
                if state.is_operable() {
                    return Ok(info.box_);
                }
                if state.is_terminal_failure() {
                    return Err(Error::BoxTerminal {
                        box_id: box_id.to_string(),
                        state: state.as_str().to_string(),
                    });
                }
                if Instant::now() >= deadline {
                    return Err(Error::WaitTimeout {
                        box_id: box_id.to_string(),
                        last_state: state.as_str().to_string(),
                    });
                }
            }
            Err(e) if e.is_retryable() => {
                if Instant::now() >= deadline {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        }
        sleep(opts.poll_interval).await;
    }
}
