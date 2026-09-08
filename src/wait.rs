use std::future::Future;
use std::time::Duration;

use tokio::time::{sleep, timeout_at, Instant};

use crate::client::BoxApi;
use crate::error::{Error, Result};
use crate::types::*;

/// Polling options. A zero timeout means unlimited.
/// Nonzero timeouts bound the entire wait, including HTTP retries and sleeps.
/// Dropping a wait future cancels local polling, not server-side work.
#[derive(Debug, Clone)]
pub struct WaitOptions {
    pub timeout: Duration,
    pub poll_interval: Duration,
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self::from_millis(300_000, 2_000)
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

/// Shared cooperative budget for waits and event streams.
pub(crate) struct PollBudget {
    deadline: Option<Instant>,
}
impl PollBudget {
    pub(crate) fn new(timeout: Duration) -> Result<Self> {
        let deadline = if timeout.is_zero() {
            None
        } else {
            Some(
                Instant::now()
                    .checked_add(timeout)
                    .ok_or_else(|| Error::Config("wait timeout is too large".into()))?,
            )
        };
        Ok(Self { deadline })
    }

    pub(crate) async fn run<T>(
        &self,
        resource_id: &str,
        state: &str,
        future: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        let expired = || Error::WaitTimeout {
            box_id: resource_id.into(),
            last_state: state.into(),
        };
        match self.deadline {
            None => future.await,
            Some(end) => {
                if Instant::now() >= end {
                    return Err(expired());
                }
                let result = timeout_at(end, future).await.map_err(|_| expired())?;
                if Instant::now() >= end {
                    return Err(expired());
                }
                result
            }
        }
    }
}

enum Poll<T> {
    Ready(T),
    Pending(String),
}

async fn poll<T, F, Fut>(resource_id: &str, opts: WaitOptions, mut fetch: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Poll<T>>>,
{
    let budget = PollBudget::new(opts.timeout)?;
    let mut last_state = "not yet observed".to_string();
    loop {
        let result = budget.run(resource_id, &last_state, fetch()).await;
        match result {
            Ok(Poll::Ready(value)) => return Ok(value),
            Ok(Poll::Pending(state)) => last_state = state,
            Err(error) => return Err(error),
        }
        budget
            .run(resource_id, &last_state, async {
                sleep(opts.poll_interval).await;
                Ok(())
            })
            .await?;
    }
}

pub async fn wait_until_ready(api: &BoxApi, box_id: &str) -> Result<Box> {
    wait_until_ready_with(api, box_id, WaitOptions::default()).await
}

pub async fn wait_until_ready_with(api: &BoxApi, box_id: &str, opts: WaitOptions) -> Result<Box> {
    wait_for_box(api, box_id, opts, BoxState::is_operable).await
}

pub async fn wait_until_idle(api: &BoxApi, box_id: &str) -> Result<Box> {
    wait_until_idle_with(api, box_id, WaitOptions::from_millis(600_000, 2_000)).await
}

pub async fn wait_until_idle_with(api: &BoxApi, box_id: &str, opts: WaitOptions) -> Result<Box> {
    wait_for_box(api, box_id, opts, |state| *state == BoxState::Idle).await
}

async fn wait_for_box(
    api: &BoxApi,
    box_id: &str,
    opts: WaitOptions,
    ready: impl Fn(&BoxState) -> bool,
) -> Result<Box> {
    poll(box_id, opts, || async {
        let info = api.get(box_id).await?;
        if ready(&info.box_.state) {
            return Ok(Poll::Ready(info.box_));
        }
        let state = info.box_.state.as_str().to_owned();
        if info.box_.state.is_terminal_failure() {
            return Err(Error::BoxTerminal {
                box_id: box_id.into(),
                state,
            });
        }
        Ok(Poll::Pending(state))
    })
    .await
}

/// Wait until a prompt finishes or fails, returning its terminal run.
pub async fn wait_for_prompt(
    api: &BoxApi,
    box_id: &str,
    prompt_id: &str,
    opts: Option<WaitOptions>,
) -> Result<PromptRun> {
    poll(
        box_id,
        opts.unwrap_or_else(|| WaitOptions::from_millis(1_800_000, 2_000)),
        || async {
            let run = api.prompt_run_status(box_id, prompt_id).await?.prompt_run;
            if matches!(run.status.as_str(), "finished" | "failed") {
                Ok(Poll::Ready(run))
            } else {
                Ok(Poll::Pending(format!("prompt:{}", run.status)))
            }
        },
    )
    .await
}

/// Desktop connection options and the budget for waiting until it is available.
#[derive(Debug, Clone)]
pub struct DesktopWaitOptions {
    pub wait: WaitOptions,
    pub vnc: Option<u8>,
    pub public_access: bool,
}
impl Default for DesktopWaitOptions {
    fn default() -> Self {
        Self {
            wait: WaitOptions::default(),
            vnc: Some(1),
            public_access: false,
        }
    }
}

pub async fn wait_for_desktop(
    api: &BoxApi,
    box_id: &str,
    public_access: bool,
    opts: Option<WaitOptions>,
) -> Result<DesktopResponse> {
    wait_for_desktop_with(
        api,
        box_id,
        DesktopWaitOptions {
            public_access,
            wait: opts.unwrap_or_default(),
            ..Default::default()
        },
    )
    .await
}

pub async fn wait_for_desktop_with(
    api: &BoxApi,
    box_id: &str,
    opts: DesktopWaitOptions,
) -> Result<DesktopResponse> {
    poll(box_id, opts.wait, || async {
        let desktop = api
            .desktop(
                box_id,
                opts.vnc,
                DesktopRequest {
                    public_access: opts.public_access.then_some(true),
                },
            )
            .await?;
        if desktop
            .desktop_url
            .as_ref()
            .is_some_and(|url| !url.is_empty())
            && desktop.provisioning != Some(true)
        {
            Ok(Poll::Ready(desktop))
        } else {
            Ok(Poll::Pending("desktop provisioning".into()))
        }
    })
    .await
}

/// Wait for backend deletion completion. A hidden box or `blocked` operation is
/// not completion; those operations remain pending until completed or timed out.
/// The five-minute default is a caller budget, not a service completion guarantee.
/// Pass a longer `WaitOptions` timeout, or zero for unlimited observation.
pub async fn wait_for_deletion(
    api: &BoxApi,
    operation_id: &str,
    opts: Option<WaitOptions>,
) -> Result<DeletionOperation> {
    poll(operation_id, opts.unwrap_or_default(), || async {
        let operation = api.get_deletion_operation(operation_id).await?.operation;
        if operation.status == "completed" {
            Ok(Poll::Ready(operation))
        } else {
            Ok(Poll::Pending(format!("deletion:{}", operation.status)))
        }
    })
    .await
}
