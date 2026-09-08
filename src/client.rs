mod administration;

use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time::sleep;

use crate::config::Configuration;
use crate::error::{Error, Result};
use crate::types::*;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

// Characters allowed unescaped inside an API path segment.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~')
    .remove(b'!')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'*');

/// Ascii Box id pattern: `bx_` + 8 chars from a Crockford-like alphabet.
const BOX_ID_PREFIX: &str = "bx_";
const BOX_ID_BODY_LEN: usize = 8;
const BOX_ID_ALPHABET: &[u8] = b"23456789abcdefghjkmnpqrstuvwxyz";

const GET_MAX_RETRIES: u32 = 3;
const GET_RETRY_BASE: Duration = Duration::from_millis(200);
const ERROR_BODY_MAX: usize = 2_048;

/// Asynchronous client for the Ascii Box API.
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

    /// Read the configuration used by this client.
    pub fn config(&self) -> &Configuration {
        &self.config
    }

    // --- Account ---

    /// Read the authenticated account profile.
    pub async fn me(&self) -> Result<MeResponse> {
        self.get_json("/me").await
    }

    /// Read account limits and balances using the configured organization.
    pub async fn limits(&self) -> Result<LimitsResponse> {
        self.limits_with(&LimitsOptions::default()).await
    }

    /// Read limits for an explicit organization or team scope.
    pub async fn limits_with(&self, options: &LimitsOptions) -> Result<LimitsResponse> {
        let url = format!("{}/limits", self.base);
        let req = self
            .base_request_with_org(Method::GET, &url, options.x_box_org.as_deref())
            .query(options);
        self.execute(req, true).await
    }

    // --- Lifecycle ---

    /// List Boxes with optional state filters and pagination.
    pub async fn boxes(&self, query: Option<&BoxesQuery>) -> Result<BoxListResponse> {
        match query {
            Some(q) => self.get_json_query("/boxes", q).await,
            None => self.get_json("/boxes").await,
        }
    }

    /// Create a Box with the supplied settings.
    pub async fn create(&self, request: CreateBoxRequest) -> Result<CreateBoxResponse> {
        self.create_with_idempotency(request, None).await
    }

    /// Create with an `Idempotency-Key` so lost responses can be safely retried.
    pub async fn create_with_idempotency(
        &self,
        request: CreateBoxRequest,
        idempotency_key: Option<&str>,
    ) -> Result<CreateBoxResponse> {
        self.create_with_options(
            Some(request),
            &CreateOptions {
                idempotency_key: idempotency_key.map(str::to_owned),
                ..Default::default()
            },
        )
        .await
    }

    /// Create a Box with optional body, organization scope, and idempotency key.
    pub async fn create_with_options(
        &self,
        request: Option<CreateBoxRequest>,
        options: &CreateOptions,
    ) -> Result<CreateBoxResponse> {
        let url = format!("{}/boxes", self.base);
        let req = self.base_request_with_org(Method::POST, &url, options.x_box_org.as_deref());
        let req = json_body(req, request.as_ref());
        let req = match &options.org {
            Some(org) => req.query(&[("org", org)]),
            None => req,
        };
        let req = match &options.idempotency_key {
            Some(key) => req.header("Idempotency-Key", key),
            None => req,
        };
        self.execute(req, false).await
    }

    /// Read the current state and configuration of a Box.
    pub async fn get(&self, box_id: &str) -> Result<BoxInfoResponse> {
        let path = format!("/boxes/{}", validate_box_id(box_id)?);
        self.get_json(&path).await
    }

    /// Update the name, subdomain, or lifetime of a Box.
    pub async fn update(&self, box_id: &str, request: UpdateBoxRequest) -> Result<BoxInfoResponse> {
        let path = format!("/boxes/{}", validate_box_id(box_id)?);
        self.send_body(Method::PATCH, &path, &request, false).await
    }

    /// Stop and archive a Box, retaining snapshots for later resume.
    pub async fn stop(
        &self,
        box_id: &str,
        request: Option<StopRequest>,
    ) -> Result<BoxActionResponse> {
        let path = format!("/boxes/{}/stop", validate_box_id(box_id)?);
        let req = self.base_request(Method::POST, &format!("{}{path}", self.base));
        self.execute(json_body(req, request.as_ref()), false).await
    }

    /// Resume an archived Box with optional settings.
    pub async fn resume(
        &self,
        box_id: &str,
        request: Option<ResumeRequest>,
    ) -> Result<BoxActionResponse> {
        let path = format!("/boxes/{}/resume", validate_box_id(box_id)?);
        let req = self.base_request(Method::POST, &format!("{}{path}", self.base));
        self.execute(json_body(req, request.as_ref()), false).await
    }

    /// Permanently delete a box and its exclusively-owned snapshot chains.
    /// The API requires the box id again in the confirmation header.
    pub async fn delete_box(&self, box_id: &str) -> Result<DeletionOperationResponse> {
        let id = validate_box_id(box_id)?;
        let url = format!("{}/boxes/{id}", self.base);
        let req = self
            .base_request(Method::DELETE, &url)
            .header("X-Ascii-Confirm-Delete", id);
        self.execute(req, false).await
    }

    /// Interrupt work running in a Box.
    pub async fn interrupt(&self, box_id: &str) -> Result<BoxActionResponse> {
        let path = format!("/boxes/{}/interrupt", validate_box_id(box_id)?);
        self.execute(
            self.base_request(Method::POST, &format!("{}{path}", self.base)),
            false,
        )
        .await
    }

    // --- In-box ops ---

    /// HTTP timeout is raised to cover `timeout_seconds` + slack. Commands are
    /// never auto-retried (Ascii: execution may already be running).
    ///
    pub async fn command(&self, box_id: &str, request: CommandRequest) -> Result<CommandResponse> {
        if request.detached == Some(true) {
            return Err(Error::Config(
                "use command_raw for detached commands".into(),
            ));
        }
        match self.command_raw(box_id, request).await? {
            CommandResult::Completed(response) => Ok(response),
            CommandResult::Started(_) => Err(Error::Unexpected(
                "detached response for synchronous command".into(),
            )),
        }
    }

    /// Start either a synchronous or detached command and preserve the union response.
    pub async fn command_raw(
        &self,
        box_id: &str,
        request: CommandRequest,
    ) -> Result<CommandResult> {
        let path = format!("/boxes/{}/commands", validate_box_id(box_id)?);
        let url = format!("{}{path}", self.base);
        let req = self
            .base_request(Method::POST, &url)
            .timeout(
                self.config
                    .http_timeout_for_command(request.timeout_seconds),
            )
            .json(&request);
        self.execute(req, false).await
    }

    /// Read a detached command status and optionally limit its log tail.
    pub async fn command_status(
        &self,
        box_id: &str,
        process_id: &str,
        query: Option<&CommandStatusQuery>,
    ) -> Result<CommandStatusResponse> {
        let path = format!(
            "/boxes/{}/commands/{}",
            validate_box_id(box_id)?,
            segment(process_id)?
        );
        match query {
            Some(q) => self.get_json_query(&path, q).await,
            None => self.get_json(&path).await,
        }
    }

    /// Queue an agent prompt in a Box.
    pub async fn prompt(&self, box_id: &str, request: PromptRequest) -> Result<PromptResponse> {
        let path = format!("/boxes/{}/prompt", validate_box_id(box_id)?);
        self.send_body(Method::POST, &path, &request, false).await
    }

    /// Read the state of a queued prompt.
    pub async fn prompt_run_status(
        &self,
        box_id: &str,
        prompt_id: &str,
    ) -> Result<PromptRunResponse> {
        let path = format!(
            "/boxes/{}/prompts/{}",
            validate_box_id(box_id)?,
            segment(prompt_id)?
        );
        self.get_json(&path).await
    }

    /// Read a page of Box events with optional cursor and type filters.
    pub async fn events(
        &self,
        box_id: &str,
        query: Option<&EventsQuery>,
    ) -> Result<EventsResponse> {
        let path = format!("/boxes/{}/events", validate_box_id(box_id)?);
        match query {
            Some(q) => self.get_json_query(&path, q).await,
            None => self.get_json(&path).await,
        }
    }

    /// Request desktop access with optional VNC settings.
    pub async fn desktop(
        &self,
        box_id: &str,
        vnc: Option<u8>,
        request: DesktopRequest,
    ) -> Result<DesktopResponse> {
        self.desktop_with(
            box_id,
            Some(&DesktopQuery {
                vnc,
                ..Default::default()
            }),
            Some(request),
        )
        .await
    }

    /// Request desktop access with connection and theme options.
    pub async fn desktop_with(
        &self,
        box_id: &str,
        query: Option<&DesktopQuery>,
        request: Option<DesktopRequest>,
    ) -> Result<DesktopResponse> {
        let url = format!("{}/boxes/{}/desktop", self.base, validate_box_id(box_id)?);
        let req = self.base_request(Method::POST, &url);
        let req = json_body(req, request.as_ref());
        let req = match query {
            Some(q) => req.query(q),
            None => req,
        };
        self.execute(req, false).await
    }

    // --- Snapshots ---
    /// List snapshot history for one Box.
    pub async fn list_box_snapshots(
        &self,
        box_id: &str,
        query: Option<&SnapshotsQuery>,
    ) -> Result<SnapshotListResponse> {
        let path = format!("/boxes/{}/snapshots", validate_box_id(box_id)?);
        match query {
            Some(q) => self.get_json_query(&path, q).await,
            None => self.get_json(&path).await,
        }
    }
    /// Read the most recent snapshot for a Box, if available.
    pub async fn latest_box_snapshot(&self, box_id: &str) -> Result<SnapshotLatestResponse> {
        self.get_json(&format!(
            "/boxes/{}/snapshots/latest",
            validate_box_id(box_id)?
        ))
        .await
    }
    /// List snapshots across the account with optional pagination.
    pub async fn list_snapshots(
        &self,
        query: Option<&SnapshotsQuery>,
    ) -> Result<SnapshotListResponse> {
        match query {
            Some(q) => self.get_json_query("/snapshots", q).await,
            None => self.get_json("/snapshots").await,
        }
    }
    /// Read the file tree and availability metadata of a snapshot.
    pub async fn snapshot_tree(&self, snapshot_id: &str) -> Result<SnapshotTreeResponse> {
        self.get_json(&format!("/snapshots/{}/tree", segment(snapshot_id)?))
            .await
    }

    /// Download a snapshot file as bytes. `path` is a query value, not a URL segment.
    pub async fn snapshot_file(&self, snapshot_id: &str, path: Option<&str>) -> Result<Vec<u8>> {
        let url = format!("{}/snapshots/{}/files", self.base, segment(snapshot_id)?);
        let mut req = self.base_request(Method::GET, &url).header("Accept", "*/*");
        if let Some(path) = path {
            req = req.query(&[("path", path)]);
        }
        self.execute_bytes(req, true).await
    }

    /// Read a snapshot download manifest and reconstruction metadata.
    pub async fn snapshot_download(&self, snapshot_id: &str) -> Result<SnapshotDownloadResponse> {
        self.get_json(&format!("/snapshots/{}/download", segment(snapshot_id)?))
            .await
    }
    /// Accept permanent deletion of a snapshot and return its operation.
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<DeletionOperationResponse> {
        let id = segment(snapshot_id)?;
        let req = self
            .base_request(Method::DELETE, &format!("{}/snapshots/{id}", self.base))
            .header("X-Ascii-Confirm-Delete", snapshot_id);
        self.execute(req, false).await
    }

    /// Read the current state of an accepted deletion operation.
    pub async fn get_deletion_operation(
        &self,
        operation_id: &str,
    ) -> Result<DeletionOperationResponse> {
        self.get_json(&format!("/deletion-operations/{}", segment(operation_id)?))
            .await
    }

    // --- Environments ---
    /// List configured environments.
    pub async fn environments(&self) -> Result<BoxEnvironmentListResponse> {
        self.get_json("/environments").await
    }
    /// Create a named environment.
    pub async fn create_environment(
        &self,
        request: CreateBoxEnvironmentRequest,
    ) -> Result<BoxEnvironmentListResponse> {
        self.send_body(Method::POST, "/environments", &request, false)
            .await
    }
    /// Update environment contents, repository settings, or setup options.
    pub async fn update_environment(
        &self,
        id: &str,
        request: UpdateBoxEnvironmentRequest,
    ) -> Result<BoxEnvironmentResponse> {
        self.send_body(
            Method::PUT,
            &format!("/environments/{}", segment(id)?),
            &request,
            false,
        )
        .await
    }
    /// Delete an environment.
    pub async fn delete_environment(&self, id: &str) -> Result<BoxEnvironmentResponse> {
        let req = self.base_request(
            Method::DELETE,
            &format!("{}/environments/{}", self.base, segment(id)?),
        );
        self.execute(req, false).await
    }

    /// Run a shell command and wait for its result, with a 30-second command timeout.
    pub async fn exec(&self, box_id: &str, command: impl Into<String>) -> Result<CommandResponse> {
        self.command(
            box_id,
            CommandRequest::new(command).with_timeout_seconds(30),
        )
        .await
    }

    /// Read a file with an optional encoding.
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

    /// Write a file using the supplied path, content, and encoding.
    pub async fn write_file(
        &self,
        box_id: &str,
        request: FileWriteRequest,
    ) -> Result<FileWriteResponse> {
        validate_file_path(&request.path)?;
        let path = format!("/boxes/{}/files", validate_box_id(box_id)?);
        self.send_body(Method::PUT, &path, &request, false).await
    }

    /// Expose a port with default access settings.
    pub async fn host_port(&self, box_id: &str, port: u16) -> Result<HostPortResponse> {
        self.host_port_with(
            box_id,
            HostPortRequest {
                port,
                ..Default::default()
            },
        )
        .await
    }

    /// Expose a port with explicit visibility and title options.
    pub async fn host_port_with(
        &self,
        box_id: &str,
        request: HostPortRequest,
    ) -> Result<HostPortResponse> {
        let path = format!("/boxes/{}/host", validate_box_id(box_id)?);
        self.send_body(Method::POST, &path, &request, false).await
    }

    /// Configure a public SSH key for a Box.
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
        self.base_request_with_org(method, url, None)
    }

    fn base_request_with_org(
        &self,
        method: Method,
        url: &str,
        org: Option<&str>,
    ) -> RequestBuilder {
        let mut req = self
            .http
            .request(method, url)
            .bearer_auth(&self.config.access_token)
            .header("Accept", "application/json");
        if let Some(org) = org.or(self.config.org.as_deref()) {
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
        let bytes = self.execute_bytes(req, retry_idempotent).await?;
        serde_json::from_slice(&bytes).map_err(Error::from)
    }

    async fn execute_bytes(&self, req: RequestBuilder, retry_idempotent: bool) -> Result<Vec<u8>> {
        let max_attempts = if retry_idempotent { GET_MAX_RETRIES } else { 1 };
        for attempt in 0..max_attempts {
            let pending = req
                .try_clone()
                .ok_or_else(|| Error::Unexpected("request body is not retryable".into()))?;
            let (result, retry_after) = send_once(pending, self.config.max_response_bytes).await;
            match result {
                Ok(bytes) => return Ok(bytes),
                Err(e) if retry_idempotent && e.is_retryable() && attempt + 1 < max_attempts => {
                    let base = GET_RETRY_BASE.saturating_mul(2u32.pow(attempt));
                    let backoff =
                        base + Duration::from_millis(fastrand::u64(0..=base.as_millis() as u64));
                    let delay = retry_after.map(|d| d.max(backoff)).unwrap_or(backoff);
                    // Never retry earlier than Retry-After or hold a caller for an
                    // arbitrarily long server delay. Let the caller schedule later.
                    if delay > self.config.request_timeout {
                        return Err(e);
                    }
                    sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!("at least one request attempt")
    }
}

fn json_body<B: Serialize>(request: RequestBuilder, body: Option<&B>) -> RequestBuilder {
    match body {
        Some(body) => request.json(body),
        None => request.header("Content-Type", "application/json"),
    }
}

async fn send_once(req: RequestBuilder, limit: usize) -> (Result<Vec<u8>>, Option<Duration>) {
    let mut response = match req.send().await {
        Ok(res) => res,
        Err(e) => return (Err(e.into()), None),
    };
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_retry_after);
    let result = async {
        if response.content_length().is_some_and(|n| n > limit as u64) {
            return Err(Error::ResponseTooLarge { limit });
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if chunk.len() > limit.saturating_sub(bytes.len()) {
                return Err(Error::ResponseTooLarge { limit });
            }
            bytes.extend_from_slice(&chunk);
        }
        if status.is_success() {
            Ok(bytes)
        } else {
            Err(map_error(status, &bytes))
        }
    }
    .await;
    (result, retry_after)
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    value
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
        .or_else(|| {
            httpdate::parse_http_date(value).ok().map(|date| {
                date.duration_since(std::time::SystemTime::now())
                    .unwrap_or_default()
            })
        })
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
        let details = body.error.and_then(|e| e.details).or(body.details);
        return Error::Api {
            status: status.as_u16(),
            code,
            message: truncate_str(&message),
            request_id: body.request_id.unwrap_or_default(),
            details,
        };
    }
    Error::HttpStatus {
        status: status.as_u16(),
        body: truncate_lossy(bytes),
    }
}

fn truncate_lossy(bytes: &[u8]) -> String {
    truncate_str(&String::from_utf8_lossy(bytes))
}

fn truncate_str(s: &str) -> String {
    if s.len() <= ERROR_BODY_MAX {
        return s.to_string();
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

fn segment(value: &str) -> Result<String> {
    if value.is_empty() || value.chars().all(char::is_whitespace) || matches!(value, "." | "..") {
        return Err(Error::Config(
            "path identifier must not be empty or a dot segment".into(),
        ));
    }
    Ok(utf8_percent_encode(value, PATH_SEGMENT).to_string())
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
