use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time::sleep;

use crate::config::Configuration;
use crate::error::{Error, Result};
use crate::types::*;

/// Ascii Box id pattern: `bx_` + 8 chars from a Crockford-like alphabet.
const BOX_ID_PREFIX: &str = "bx_";
const BOX_ID_BODY_LEN: usize = 8;
const BOX_ID_ALPHABET: &[u8] = b"23456789abcdefghjkmnpqrstuvwxyz";

const GET_MAX_RETRIES: u32 = 3;
const GET_RETRY_BASE: Duration = Duration::from_millis(200);
const ERROR_BODY_MAX: usize = 2_048;

/// Ascii Box API client — mirrors TypeScript `BoxApi`.
///
/// Cheap to clone: wraps a pooled `reqwest::Client`.
#[derive(Clone, Debug)]
pub struct BoxApi {
    http: Client,
    config: Configuration,
    base: String,
}

impl BoxApi {
    pub fn new(config: Configuration) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_nodelay(true)
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;
        let base = config.base_path.trim_end_matches('/').to_string();
        Ok(Self { http, config, base })
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    // --- Account ---

    pub async fn me(&self) -> Result<MeResponse> {
        self.get_json("/me").await
    }

    pub async fn limits(&self) -> Result<LimitsResponse> {
        self.get_json("/limits").await
    }

    // --- Lifecycle ---

    pub async fn boxes(&self, query: Option<&BoxesQuery>) -> Result<BoxListResponse> {
        match query {
            Some(q) => self.get_json_query("/boxes", q).await,
            None => self.get_json("/boxes").await,
        }
    }

    pub async fn create(&self, request: CreateBoxRequest) -> Result<CreateBoxResponse> {
        self.create_with_idempotency(request, None).await
    }

    /// Create with an `Idempotency-Key` so lost responses can be safely retried.
    pub async fn create_with_idempotency(
        &self,
        request: CreateBoxRequest,
        idempotency_key: Option<&str>,
    ) -> Result<CreateBoxResponse> {
        let url = format!("{}/boxes", self.base);
        let mut req = self.base_request(Method::POST, &url).json(&request);
        if let Some(key) = idempotency_key {
            if !key.is_empty() {
                req = req.header("Idempotency-Key", key);
            }
        }
        // Not auto-retried (billing); caller owns retries via the key.
        self.execute(req, false).await
    }

    pub async fn get(&self, box_id: &str) -> Result<BoxInfoResponse> {
        let path = format!("/boxes/{}", validate_box_id(box_id)?);
        self.get_json(&path).await
    }

    pub async fn update(&self, box_id: &str, request: UpdateBoxRequest) -> Result<BoxInfoResponse> {
        let path = format!("/boxes/{}", validate_box_id(box_id)?);
        self.send_body(Method::PATCH, &path, &request, false).await
    }

    pub async fn stop(
        &self,
        box_id: &str,
        request: Option<StopRequest>,
    ) -> Result<BoxActionResponse> {
        let path = format!("/boxes/{}/stop", validate_box_id(box_id)?);
        let body = request.unwrap_or_default();
        self.send_body(Method::POST, &path, &body, false).await
    }

    pub async fn resume(
        &self,
        box_id: &str,
        request: Option<ResumeRequest>,
    ) -> Result<BoxActionResponse> {
        let path = format!("/boxes/{}/resume", validate_box_id(box_id)?);
        let body = request.unwrap_or_default();
        self.send_body(Method::POST, &path, &body, false).await
    }

    // --- In-box ops ---

    /// HTTP timeout is raised to cover `timeout_seconds` + slack. Commands are
    /// never auto-retried (Ascii: execution may already be running).
    ///
    /// `detached: true` is not supported in v0.1 (response shape differs); use a
    /// sync command or wait for a later release.
    pub async fn command(&self, box_id: &str, request: CommandRequest) -> Result<CommandResponse> {
        if request.detached == Some(true) {
            return Err(Error::Config(
                "detached commands are not supported in box_client v0.1".into(),
            ));
        }
        let path = format!("/boxes/{}/commands", validate_box_id(box_id)?);
        let url = format!("{}{path}", self.base);
        let http_timeout = self
            .config
            .http_timeout_for_command(request.timeout_seconds);
        let req = self
            .base_request(Method::POST, &url)
            .timeout(http_timeout)
            .json(&request);
        self.execute(req, false).await
    }

    /// Convenience: sync shell string with the same default as TS `execCommand` (30s).
    pub async fn exec(&self, box_id: &str, command: impl Into<String>) -> Result<CommandResponse> {
        self.command(
            box_id,
            CommandRequest::new(command).with_timeout_seconds(30),
        )
        .await
    }

    pub async fn read_file(
        &self,
        box_id: &str,
        path: impl Into<String>,
        encoding: Option<&str>,
    ) -> Result<FileReadResponse> {
        let path = path.into();
        validate_file_path(&path)?;
        let api_path = format!("/boxes/{}/files", validate_box_id(box_id)?);
        let q = FileReadQuery {
            path,
            encoding: encoding.map(str::to_string),
        };
        self.get_json_query(&api_path, &q).await
    }

    pub async fn write_file(
        &self,
        box_id: &str,
        request: FileWriteRequest,
    ) -> Result<FileWriteResponse> {
        validate_file_path(&request.path)?;
        let path = format!("/boxes/{}/files", validate_box_id(box_id)?);
        self.send_body(Method::POST, &path, &request, false).await
    }

    pub async fn host_port(&self, box_id: &str, port: u16) -> Result<HostPortResponse> {
        let path = format!("/boxes/{}/host", validate_box_id(box_id)?);
        self.send_body(Method::POST, &path, &HostPortRequest { port }, false)
            .await
    }

    pub async fn ssh_key(
        &self,
        box_id: &str,
        public_key: impl Into<String>,
    ) -> Result<SshKeyResponse> {
        let path = format!("/boxes/{}/sshkey", validate_box_id(box_id)?);
        self.send_body(
            Method::POST,
            &path,
            &SshKeyRequest {
                public_key: public_key.into(),
            },
            false,
        )
        .await
    }

    // --- HTTP core ---

    fn base_request(&self, method: Method, url: &str) -> RequestBuilder {
        let mut req = self
            .http
            .request(method, url)
            .bearer_auth(&self.config.access_token)
            .header("Accept", "application/json");
        if let Some(org) = &self.config.org {
            req = req.header("X-Box-Org", org);
        }
        req
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    async fn get_json_query<Q: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let req = self.base_request(Method::GET, &url).query(query);
        self.execute(req, true).await
    }

    async fn send_body<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        retry_idempotent: bool,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let req = self.base_request(method, &url).json(body);
        self.execute(req, retry_idempotent).await
    }

    async fn execute<T: DeserializeOwned>(
        &self,
        req: RequestBuilder,
        retry_idempotent: bool,
    ) -> Result<T> {
        let max_attempts = if retry_idempotent { GET_MAX_RETRIES } else { 1 };
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let pending = req
                .try_clone()
                .ok_or_else(|| Error::Unexpected("request body is not retryable".into()))?;
            match send_once(pending).await {
                Ok(v) => return Ok(v),
                Err(e) if retry_idempotent && e.is_retryable() && attempt < max_attempts => {
                    let backoff =
                        GET_RETRY_BASE.saturating_mul(2u32.pow(attempt.saturating_sub(1)));
                    sleep(backoff).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

async fn send_once<T: DeserializeOwned>(req: RequestBuilder) -> Result<T> {
    let res = req.send().await?;
    parse(res).await
}

async fn parse<T: DeserializeOwned>(res: reqwest::Response) -> Result<T> {
    let status = res.status();
    let bytes = res.bytes().await?;
    if status.is_success() {
        return serde_json::from_slice(&bytes).map_err(Error::from);
    }
    Err(map_error(status, &bytes))
}

fn map_error(status: StatusCode, bytes: &[u8]) -> Error {
    if let Ok(body) = serde_json::from_slice::<ApiErrorBody>(bytes) {
        let code = body
            .error
            .as_ref()
            .and_then(|e| e.code.clone())
            .or(body.code)
            .unwrap_or_else(|| "unknown".into());
        let message = body
            .error
            .as_ref()
            .and_then(|e| e.message.clone())
            .or(body.message)
            .unwrap_or_else(|| truncate_lossy(bytes));
        let details = body.error.and_then(|e| e.details);
        return Error::Api {
            status: status.as_u16(),
            code,
            message,
            request_id: body.request_id.unwrap_or_default(),
            details,
        };
    }
    Error::Unexpected(format!("HTTP {status} — {}", truncate_lossy(bytes)))
}

fn truncate_lossy(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.len() <= ERROR_BODY_MAX {
        return s.into_owned();
    }
    let mut end = ERROR_BODY_MAX;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod truncate_tests {
    #[test]
    fn truncate_respects_utf8_boundaries() {
        // "é" is 2 bytes in UTF-8; force a mid-character cut point.
        let mut bytes = Vec::new();
        while bytes.len() < super::ERROR_BODY_MAX - 1 {
            bytes.extend_from_slice("a".as_bytes());
        }
        bytes.extend_from_slice("é".as_bytes());
        let out = super::truncate_lossy(&bytes);
        assert!(out.ends_with('…'));
        assert!(out.is_char_boundary(out.len() - '…'.len_utf8()));
    }
}

pub(crate) fn validate_box_id(box_id: &str) -> Result<&str> {
    let rest = match box_id.strip_prefix(BOX_ID_PREFIX) {
        Some(r) if r.len() == BOX_ID_BODY_LEN => r,
        _ => return Err(Error::InvalidBoxId(box_id.to_string())),
    };
    if !rest.bytes().all(|b| BOX_ID_ALPHABET.contains(&b)) {
        return Err(Error::InvalidBoxId(box_id.to_string()));
    }
    Ok(box_id)
}

fn validate_file_path(path: &str) -> Result<()> {
    if path.is_empty() || path.chars().all(char::is_whitespace) {
        return Err(Error::Config("file path must not be empty".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_box_id() {
        assert_eq!(validate_box_id("bx_23456789").unwrap(), "bx_23456789");
        assert_eq!(validate_box_id("bx_abcdefgh").unwrap(), "bx_abcdefgh");
    }

    #[test]
    fn rejects_bad_box_id() {
        assert!(validate_box_id("bx_SHORT").is_err());
        assert!(validate_box_id("../etc/passwd").is_err());
        assert!(validate_box_id("bx_AAAAAAAA").is_err());
        assert!(validate_box_id("bx_23456789/../x").is_err());
    }
}
