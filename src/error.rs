use std::fmt;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Client errors.
///
/// `Display` / `Debug` are log-safe for `Api` and `Unexpected`: they omit server
/// `message` / `details` / raw bodies (which may echo secrets). Full text remains
/// on the struct fields / accessors for deliberate handling.
#[derive(Error)]
pub enum Error {
    #[error("config: {0}")]
    Config(String),

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    /// Server API failure. `Display` shows status/code/request_id only.
    #[error("api {status}: {code} (request_id={request_id})")]
    Api {
        status: u16,
        code: String,
        message: String,
        request_id: String,
        details: Option<serde_json::Value>,
    },

    #[error("unexpected response (body redacted)")]
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

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => f.debug_tuple("Config").field(msg).finish(),
            Self::Http(e) => f.debug_tuple("Http").field(e).finish(),
            Self::Api {
                status,
                code,
                message,
                request_id,
                details,
            } => f
                .debug_struct("Api")
                .field("status", status)
                .field("code", code)
                .field("message_len", &message.len())
                .field("request_id", request_id)
                .field("has_details", &details.is_some())
                .finish(),
            Self::Unexpected(body) => f
                .debug_struct("Unexpected")
                .field("body_len", &body.len())
                .finish(),
            Self::InvalidBoxId(id) => f.debug_tuple("InvalidBoxId").field(id).finish(),
            Self::WaitTimeout { box_id, last_state } => f
                .debug_struct("WaitTimeout")
                .field("box_id", box_id)
                .field("last_state", last_state)
                .finish(),
            Self::BoxTerminal { box_id, state } => f
                .debug_struct("BoxTerminal")
                .field("box_id", box_id)
                .field("state", state)
                .finish(),
            Self::BoxFailed { box_id } => {
                f.debug_struct("BoxFailed").field("box_id", box_id).finish()
            }
            Self::Serde(e) => f.debug_tuple("Serde").field(e).finish(),
        }
    }
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

    /// Server-provided API message, if this is an `Api` error.
    /// Prefer this over relying on `Display` (which omits the message).
    pub fn api_message(&self) -> Option<&str> {
        match self {
            Error::Api { message, .. } => Some(message.as_str()),
            _ => None,
        }
    }

    /// Server-provided API details JSON, if present.
    pub fn api_details(&self) -> Option<&serde_json::Value> {
        match self {
            Error::Api { details, .. } => details.as_ref(),
            _ => None,
        }
    }

    /// Raw unexpected response body (truncated), if this is `Unexpected`.
    pub fn unexpected_body(&self) -> Option<&str> {
        match self {
            Error::Unexpected(body) => Some(body.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn api_display_and_debug_omit_message_and_details() {
        let err = Error::Api {
            status: 400,
            code: "bad_request".into(),
            message: "leaked-secret-token-xyz".into(),
            request_id: "req_1".into(),
            details: Some(json!({"token": "also-secret"})),
        };
        let display = err.to_string();
        assert!(display.contains("bad_request"));
        assert!(display.contains("req_1"));
        assert!(!display.contains("leaked-secret"));
        assert!(!display.contains("also-secret"));

        let debug = format!("{err:?}");
        assert!(debug.contains("message_len"));
        assert!(debug.contains("has_details"));
        assert!(!debug.contains("leaked-secret"));
        assert!(!debug.contains("also-secret"));

        assert_eq!(err.api_message(), Some("leaked-secret-token-xyz"));
        assert!(err.api_details().is_some());
    }

    #[test]
    fn unexpected_display_and_debug_omit_body() {
        let err = Error::Unexpected("HTTP 500 — secret-in-body".into());
        assert!(!err.to_string().contains("secret-in-body"));
        assert!(!format!("{err:?}").contains("secret-in-body"));
        assert_eq!(err.unexpected_body(), Some("HTTP 500 — secret-in-body"));
    }
}
