use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config: {0}")]
    Config(String),

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("api {status}: {code} — {message} (request_id={request_id})")]
    Api {
        status: u16,
        code: String,
        message: String,
        request_id: String,
        details: Option<serde_json::Value>,
    },

    #[error("unexpected response: {0}")]
    Unexpected(String),

    #[error("invalid box id `{0}` (expected bx_ + 8 chars)")]
    InvalidBoxId(String),

    #[error("timeout waiting for box {box_id} (last state={last_state})")]
    WaitTimeout { box_id: String, last_state: String },

    /// TS `waitUntilReady`: `Box entered terminal state ${box.state}`.
    #[error("box {box_id} entered terminal state {state}")]
    BoxTerminal { box_id: String, state: String },

    #[error("box {box_id} entered error state")]
    BoxFailed { box_id: String },

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

impl Error {
    /// Whether a **safe automatic retry** of an idempotent request may help.
    ///
    /// Does **not** treat every 409 as retryable — only known transient codes
    /// (`box_starting`, `box_securing`). Never retry `command` via this alone;
    /// Ascii documents that command execution is not safely auto-retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Api { status, code, .. } => match *status {
                429 => true,
                502..=504 => true,
                409 => code == "box_starting" || code == "box_securing",
                _ => false,
            },
            Error::Http(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } => Some(*status),
            Error::Http(e) => e.status().map(|s| s.as_u16()),
            _ => None,
        }
    }
}
