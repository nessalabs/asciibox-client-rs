use std::env;
use std::fmt;
use std::time::Duration;

use url::Url;

use crate::error::{Error, Result};

/// Client configuration — mirrors TS `Configuration`.
///
/// Timeouts follow common Rust API-client practice (e.g. octocrab / reqwest):
/// a short **connect** budget and a separate **request** budget. Long-running
/// `command` calls extend the request timeout to cover `timeoutSeconds` plus slack.
#[derive(Clone)]
pub struct Configuration {
    pub base_path: String,
    pub access_token: String,
    /// Optional org / team wallet scope (`X-Box-Org`).
    pub org: Option<String>,
    /// TCP connect timeout (default 10s).
    pub connect_timeout: Duration,
    /// Default per-request timeout for API calls (default 60s).
    pub request_timeout: Duration,
    /// Added on top of a command's `timeout_seconds` for the HTTP layer (default 15s).
    pub command_timeout_slack: Duration,
    /// Sent as `User-Agent` (default `box_client/0.1`).
    pub user_agent: String,
}

impl fmt::Debug for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Configuration")
            .field("base_path", &self.base_path)
            .field("access_token", &redact_secret(&self.access_token))
            .field("org", &self.org)
            .field("connect_timeout", &self.connect_timeout)
            .field("request_timeout", &self.request_timeout)
            .field("command_timeout_slack", &self.command_timeout_slack)
            .field("user_agent", &self.user_agent)
            .finish()
    }
}

impl Configuration {
    pub const DEFAULT_BASE_PATH: &'static str = "https://ascii.dev/api/box/v1";
    pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
    pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
    pub const DEFAULT_COMMAND_TIMEOUT_SLACK: Duration = Duration::from_secs(15);

    pub fn new(access_token: impl Into<String>) -> Result<Self> {
        let access_token = access_token.into();
        validate_token(&access_token)?;
        let base_path = Self::DEFAULT_BASE_PATH.to_string();
        validate_base_path(&base_path)?;
        Ok(Self {
            base_path,
            access_token,
            org: None,
            connect_timeout: Self::DEFAULT_CONNECT_TIMEOUT,
            request_timeout: Self::DEFAULT_REQUEST_TIMEOUT,
            command_timeout_slack: Self::DEFAULT_COMMAND_TIMEOUT_SLACK,
            user_agent: format!("box_client/{}", env!("CARGO_PKG_VERSION")),
        })
    }

    /// Load from `BOX_API_KEY` (required) and optional `BOX_BASE_URL` / `BOX_ORG`.
    pub fn from_env() -> Result<Self> {
        let access_token = env::var("BOX_API_KEY").map_err(|_| {
            Error::Config("BOX_API_KEY is not set (create one with `box api-key create`)".into())
        })?;
        let mut cfg = Self::new(access_token)?;
        if let Ok(base) = env::var("BOX_BASE_URL") {
            if !base.is_empty() {
                cfg = cfg.with_base_path(base)?;
            }
        }
        if let Ok(org) = env::var("BOX_ORG") {
            if !org.is_empty() {
                cfg.org = Some(org);
            }
        }
        Ok(cfg)
    }

    pub fn with_base_path(mut self, base_path: impl Into<String>) -> Result<Self> {
        let base_path = base_path.into();
        validate_base_path(&base_path)?;
        self.base_path = base_path.trim_end_matches('/').to_string();
        Ok(self)
    }

    pub fn with_org(mut self, org: impl Into<String>) -> Self {
        self.org = Some(org.into());
        self
    }

    /// Sets the default per-request timeout (does not change `connect_timeout`).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn with_command_timeout_slack(mut self, slack: Duration) -> Self {
        self.command_timeout_slack = slack;
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// HTTP timeout for a `command` call given the in-box `timeout_seconds`.
    pub fn http_timeout_for_command(&self, timeout_seconds: Option<u32>) -> Duration {
        // Server default is 30s when omitted.
        let cmd = Duration::from_secs(u64::from(timeout_seconds.unwrap_or(30)));
        cmd.saturating_add(self.command_timeout_slack)
            .max(self.request_timeout)
    }
}

fn validate_token(token: &str) -> Result<()> {
    if token.is_empty() || token.chars().all(char::is_whitespace) {
        return Err(Error::Config("access token must not be empty".into()));
    }
    Ok(())
}

fn validate_base_path(base: &str) -> Result<()> {
    let parsed =
        Url::parse(base).map_err(|e| Error::Config(format!("invalid base_path `{base}`: {e}")))?;
    match parsed.scheme() {
        "https" => {}
        "http" => {
            let host = parsed.host_str().unwrap_or("");
            let loopback =
                matches!(host, "localhost" | "127.0.0.1" | "::1") || host.starts_with("127.");
            if !loopback {
                return Err(Error::Config(format!(
                    "base_path must use https (http only allowed for localhost), got `{base}`"
                )));
            }
        }
        other => {
            return Err(Error::Config(format!(
                "base_path must be http(s), got {other}"
            )));
        }
    }
    if parsed.host_str().is_none() {
        return Err(Error::Config("base_path must include a host".into()));
    }
    Ok(())
}

fn redact_secret(secret: &str) -> String {
    // Never echo key material in logs — length only.
    format!("<redacted len={}>", secret.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_token() {
        let cfg = Configuration::new("box_secret_value_here").unwrap();
        let s = format!("{cfg:?}");
        assert!(s.contains("<redacted len="));
        assert!(!s.contains("box_secret"));
        assert!(!s.contains("secret_value"));
    }

    #[test]
    fn rejects_empty_token() {
        assert!(Configuration::new("").is_err());
    }

    #[test]
    fn rejects_remote_http_base_path() {
        let cfg = Configuration::new("box_test_key").unwrap();
        assert!(cfg
            .clone()
            .with_base_path("http://evil.example/api")
            .is_err());
        assert!(cfg.with_base_path("http://127.0.0.1:8080").is_ok());
    }

    #[test]
    fn command_http_timeout_covers_command_budget() {
        let cfg = Configuration::new("box_test_key").unwrap();
        let t = cfg.http_timeout_for_command(Some(120));
        assert!(t >= Duration::from_secs(135));
    }
}
