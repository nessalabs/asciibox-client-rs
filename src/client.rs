mod administration;
mod transport;

use std::time::Duration;

use reqwest::{Client, Method};

use crate::config::BoxClientConfig;
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

/// Asynchronous client for the Ascii Box API.
///
/// Cheap to clone: wraps a pooled `reqwest::Client`.
#[derive(Clone, Debug)]
pub struct BoxApi {
    http: Client,
    config: BoxClientConfig,
    base: String,
}

impl BoxApi {
    pub fn new(config: BoxClientConfig) -> Result<Self> {
        config.retry.validate()?;
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
    pub fn config(&self) -> &BoxClientConfig {
        &self.config
    }

    // --- Account ---

    /// Read the authenticated account profile.
    pub async fn me(&self) -> Result<MeResponse> {
        self.request("me", Method::GET, "/me").send().await
    }

    /// Read account limits and balances using the configured organization.
    pub async fn limits(&self) -> Result<LimitsResponse> {
        self.limits_with(&LimitsOptions::default()).await
    }

    /// Read limits for an explicit organization or team scope.
    pub async fn limits_with(&self, options: &LimitsOptions) -> Result<LimitsResponse> {
        self.request_with_org(
            "limits",
            Method::GET,
            "/limits",
            options.x_box_org.as_deref(),
        )
        .query(Some(options))
        .send()
        .await
    }

    // --- Lifecycle ---

    /// List Boxes with optional state filters and pagination.
    pub async fn boxes(&self, query: Option<&BoxesQuery>) -> Result<BoxListResponse> {
        self.request("boxes", Method::GET, "/boxes")
            .query(query)
            .send()
            .await
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
        self.request_with_org(
            "create",
            Method::POST,
            "/boxes",
            options.x_box_org.as_deref(),
        )
        .json(request.as_ref())
        .query(options.org.as_ref().map(|org| [("org", org)]).as_ref())
        .header("Idempotency-Key", options.idempotency_key.as_deref())
        .send()
        .await
    }

    /// Read the current state and configuration of a Box.
    pub async fn get(&self, box_id: &str) -> Result<BoxInfoResponse> {
        self.request(
            "get",
            Method::GET,
            &format!("/boxes/{}", validate_box_id(box_id)?),
        )
        .send()
        .await
    }

    /// Update the name, subdomain, or lifetime of a Box.
    pub async fn update(&self, box_id: &str, request: UpdateBoxRequest) -> Result<BoxInfoResponse> {
        self.request(
            "update",
            Method::PATCH,
            &format!("/boxes/{}", validate_box_id(box_id)?),
        )
        .json(Some(&request))
        .send()
        .await
    }

    /// Stop and archive a Box, retaining snapshots for later resume.
    pub async fn stop(
        &self,
        box_id: &str,
        request: Option<StopRequest>,
    ) -> Result<BoxActionResponse> {
        self.request(
            "stop",
            Method::POST,
            &format!("/boxes/{}/stop", validate_box_id(box_id)?),
        )
        .json(request.as_ref())
        .send()
        .await
    }

    /// Resume an archived Box with optional settings.
    pub async fn resume(
        &self,
        box_id: &str,
        request: Option<ResumeRequest>,
    ) -> Result<BoxActionResponse> {
        self.request(
            "resume",
            Method::POST,
            &format!("/boxes/{}/resume", validate_box_id(box_id)?),
        )
        .json(request.as_ref())
        .send()
        .await
    }

    /// Permanently delete a box and its exclusively-owned snapshot chains.
    /// The API requires the box id again in the confirmation header.
    pub async fn delete_box(&self, box_id: &str) -> Result<DeletionOperationResponse> {
        self.request(
            "delete_box",
            Method::DELETE,
            &format!("/boxes/{}", validate_box_id(box_id)?),
        )
        .header("X-Ascii-Confirm-Delete", Some(box_id))
        .send()
        .await
    }

    /// Interrupt work running in a Box.
    pub async fn interrupt(&self, box_id: &str) -> Result<BoxActionResponse> {
        self.request(
            "interrupt",
            Method::POST,
            &format!("/boxes/{}/interrupt", validate_box_id(box_id)?),
        )
        .send()
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
        self.request(
            "command",
            Method::POST,
            &format!("/boxes/{}/commands", validate_box_id(box_id)?),
        )
        .timeout(
            self.config
                .http_timeout_for_command(request.timeout_seconds),
        )
        .json(Some(&request))
        .send()
        .await
    }

    /// Read a detached command status and optionally limit its log tail.
    pub async fn command_status(
        &self,
        box_id: &str,
        process_id: &str,
        query: Option<&CommandStatusQuery>,
    ) -> Result<CommandStatusResponse> {
        self.request(
            "command_status",
            Method::GET,
            &format!(
                "/boxes/{}/commands/{}",
                validate_box_id(box_id)?,
                segment(process_id)?
            ),
        )
        .query(query)
        .send()
        .await
    }

    /// Queue an agent prompt in a Box.
    pub async fn prompt(&self, box_id: &str, request: PromptRequest) -> Result<PromptResponse> {
        self.request(
            "prompt",
            Method::POST,
            &format!("/boxes/{}/prompt", validate_box_id(box_id)?),
        )
        .json(Some(&request))
        .send()
        .await
    }

    /// Read the state of a queued prompt.
    pub async fn prompt_run_status(
        &self,
        box_id: &str,
        prompt_id: &str,
    ) -> Result<PromptRunResponse> {
        self.request(
            "prompt_run_status",
            Method::GET,
            &format!(
                "/boxes/{}/prompts/{}",
                validate_box_id(box_id)?,
                segment(prompt_id)?
            ),
        )
        .send()
        .await
    }

    /// Read a page of Box events with optional cursor and type filters.
    pub async fn events(
        &self,
        box_id: &str,
        query: Option<&EventsQuery>,
    ) -> Result<EventsResponse> {
        self.request(
            "events",
            Method::GET,
            &format!("/boxes/{}/events", validate_box_id(box_id)?),
        )
        .query(query)
        .send()
        .await
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
        self.request(
            "desktop",
            Method::POST,
            &format!("/boxes/{}/desktop", validate_box_id(box_id)?),
        )
        .json(request.as_ref())
        .query(query)
        .send()
        .await
    }

    // --- Snapshots ---
    /// List snapshot history for one Box.
    pub async fn list_box_snapshots(
        &self,
        box_id: &str,
        query: Option<&SnapshotsQuery>,
    ) -> Result<SnapshotListResponse> {
        self.request(
            "list_box_snapshots",
            Method::GET,
            &format!("/boxes/{}/snapshots", validate_box_id(box_id)?),
        )
        .query(query)
        .send()
        .await
    }
    /// Read the most recent snapshot for a Box, if available.
    pub async fn latest_box_snapshot(&self, box_id: &str) -> Result<SnapshotLatestResponse> {
        self.request(
            "latest_box_snapshot",
            Method::GET,
            &format!("/boxes/{}/snapshots/latest", validate_box_id(box_id)?),
        )
        .send()
        .await
    }
    /// List snapshots across the account with optional pagination.
    pub async fn list_snapshots(
        &self,
        query: Option<&SnapshotsQuery>,
    ) -> Result<SnapshotListResponse> {
        self.request("list_snapshots", Method::GET, "/snapshots")
            .query(query)
            .send()
            .await
    }
    /// Read the file tree and availability metadata of a snapshot.
    pub async fn snapshot_tree(&self, snapshot_id: &str) -> Result<SnapshotTreeResponse> {
        self.request(
            "snapshot_tree",
            Method::GET,
            &format!("/snapshots/{}/tree", segment(snapshot_id)?),
        )
        .send()
        .await
    }

    /// Download a snapshot file as bytes. `path` is a query value, not a URL segment.
    pub async fn snapshot_file(&self, snapshot_id: &str, path: Option<&str>) -> Result<Vec<u8>> {
        self.request(
            "snapshot_file",
            Method::GET,
            &format!("/snapshots/{}/files", segment(snapshot_id)?),
        )
        .query(path.map(|path| [("path", path)]).as_ref())
        .bytes()
        .await
    }

    /// Read a snapshot download manifest and reconstruction metadata.
    pub async fn snapshot_download(&self, snapshot_id: &str) -> Result<SnapshotDownloadResponse> {
        self.request(
            "snapshot_download",
            Method::GET,
            &format!("/snapshots/{}/download", segment(snapshot_id)?),
        )
        .send()
        .await
    }
    /// Accept permanent deletion of a snapshot and return its operation.
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<DeletionOperationResponse> {
        self.request(
            "delete_snapshot",
            Method::DELETE,
            &format!("/snapshots/{}", segment(snapshot_id)?),
        )
        .header("X-Ascii-Confirm-Delete", Some(snapshot_id))
        .send()
        .await
    }

    /// Read the current state of an accepted deletion operation.
    pub async fn get_deletion_operation(
        &self,
        operation_id: &str,
    ) -> Result<DeletionOperationResponse> {
        self.request(
            "get_deletion_operation",
            Method::GET,
            &format!("/deletion-operations/{}", segment(operation_id)?),
        )
        .send()
        .await
    }

    // --- Environments ---
    /// List configured environments.
    pub async fn environments(&self) -> Result<BoxEnvironmentListResponse> {
        self.request("environments", Method::GET, "/environments")
            .send()
            .await
    }
    /// Create a named environment.
    pub async fn create_environment(
        &self,
        request: CreateBoxEnvironmentRequest,
    ) -> Result<BoxEnvironmentListResponse> {
        self.request("create_environment", Method::POST, "/environments")
            .json(Some(&request))
            .send()
            .await
    }
    /// Update environment contents, repository settings, or setup options.
    pub async fn update_environment(
        &self,
        id: &str,
        request: UpdateBoxEnvironmentRequest,
    ) -> Result<BoxEnvironmentResponse> {
        self.request(
            "update_environment",
            Method::PUT,
            &format!("/environments/{}", segment(id)?),
        )
        .json(Some(&request))
        .send()
        .await
    }
    /// Delete an environment.
    pub async fn delete_environment(&self, id: &str) -> Result<BoxEnvironmentResponse> {
        self.request(
            "delete_environment",
            Method::DELETE,
            &format!("/environments/{}", segment(id)?),
        )
        .send()
        .await
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
        self.request(
            "read_file",
            Method::GET,
            &format!("/boxes/{}/files", validate_box_id(box_id)?),
        )
        .query(Some(&FileReadQuery {
            path,
            encoding: encoding.map(str::to_owned),
        }))
        .send()
        .await
    }

    /// Write a file using the supplied path, content, and encoding.
    pub async fn write_file(
        &self,
        box_id: &str,
        request: FileWriteRequest,
    ) -> Result<FileWriteResponse> {
        validate_file_path(&request.path)?;
        self.request(
            "write_file",
            Method::PUT,
            &format!("/boxes/{}/files", validate_box_id(box_id)?),
        )
        .json(Some(&request))
        .send()
        .await
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
        self.request(
            "host_port",
            Method::POST,
            &format!("/boxes/{}/host", validate_box_id(box_id)?),
        )
        .json(Some(&request))
        .send()
        .await
    }

    /// Configure a public SSH key for a Box.
    pub async fn ssh_key(
        &self,
        box_id: &str,
        public_key: impl Into<String>,
    ) -> Result<SshKeyResponse> {
        self.request(
            "ssh_key",
            Method::POST,
            &format!("/boxes/{}/sshkey", validate_box_id(box_id)?),
        )
        .json(Some(&SshKeyRequest {
            public_key: public_key.into(),
        }))
        .send()
        .await
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
