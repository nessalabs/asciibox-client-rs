use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// `ttlSeconds`: omit / `Some(secs)` / `null` (disable auto-stop).
/// Wire as `Option` where `None` skips the field and `Some(None)` sends JSON null.
pub type TtlSeconds = Option<u32>;

fn is_none_ttl(v: &Option<TtlSeconds>) -> bool {
    v.is_none()
}

// --- Account ---

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: String,
    pub user: BoxUser,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxUser {
    pub id: Option<String>,
    pub login: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// --- Box model ---

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BoxState {
    Init,
    Provisioning,
    Provisioned,
    Cloning,
    Ready,
    Idle,
    Running,
    Archiving,
    Archived,
    Error,
    #[serde(other)]
    Unknown,
}

impl BoxState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Provisioning => "provisioning",
            Self::Provisioned => "provisioned",
            Self::Cloning => "cloning",
            Self::Ready => "ready",
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Archiving => "archiving",
            Self::Archived => "archived",
            Self::Error => "error",
            Self::Unknown => "unknown",
        }
    }

    /// States where command / SSH / agent work can proceed.
    /// Matches TS `waitUntilReady` success set: `ready` | `idle` | `running`.
    pub fn is_operable(&self) -> bool {
        matches!(self, Self::Ready | Self::Idle | Self::Running)
    }

    /// TS `waitUntilReady` failure set: `archived` | `archiving` | `error`.
    pub fn is_terminal_failure(&self) -> bool {
        matches!(self, Self::Archived | Self::Archiving | Self::Error)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Box {
    pub id: String,
    pub name: String,
    pub state: BoxState,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub vcpu: Option<u32>,
    #[serde(rename = "memoryGB")]
    pub memory_gb: Option<u32>,
    pub billing_multiplier: Option<f64>,
    pub url: Option<String>,
    pub ip: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub archive_after: Option<String>,
    #[serde(default)]
    pub desktop_available: bool,
    pub desktop_url: Option<String>,
    #[serde(default)]
    pub snapshot_available: bool,
    pub subdomain: Option<String>,
    pub environment: Option<String>,
    pub environment_version: Option<u32>,
    pub setup_status: Option<String>,
    pub setup_error: Option<String>,
    pub error: Option<serde_json::Value>,
    pub ssh_endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub limit: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxListResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub boxes: Vec<Box>,
    pub page_info: Option<PageInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxInfoResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    #[serde(rename = "box")]
    pub box_: Box,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoxResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub status: Option<String>,
    pub ttl_seconds: Option<u32>,
    #[serde(rename = "box")]
    pub box_: Box,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxActionResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub id: String,
    pub status: String,
    #[serde(rename = "box")]
    pub box_: Option<Box>,
}

// --- Requests ---

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoxRequest {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// `None` = omit (server default). `Some(None)` = JSON null (no auto-stop).
    /// `Some(Some(n))` = n seconds.
    #[serde(skip_serializing_if = "is_none_ttl")]
    pub ttl_seconds: Option<TtlSeconds>,
    /// Often holds secrets — redacted in `Debug`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_env: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

impl fmt::Debug for CreateBoxRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateBoxRequest")
            .field("type_", &self.type_)
            .field("ttl_seconds", &self.ttl_seconds)
            .field("env", &self.env.as_ref().map(redact_env_map))
            .field("environment", &self.environment)
            .field("no_env", &self.no_env)
            .field(
                "setup_script",
                &self.setup_script.as_ref().map(|_| "<redacted>"),
            )
            .field("org", &self.org)
            .field("from", &self.from)
            .finish()
    }
}

impl CreateBoxRequest {
    pub fn ttl(seconds: u32) -> Self {
        Self {
            ttl_seconds: Some(Some(seconds)),
            ..Default::default()
        }
    }

    pub fn no_auto_stop() -> Self {
        Self {
            ttl_seconds: Some(None),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoxRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "is_none_ttl")]
    pub ttl_seconds: Option<TtlSeconds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdomain: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeRequest {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Often holds secrets — redacted in `Debug`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_env: Option<bool>,
    #[serde(skip_serializing_if = "is_none_ttl")]
    pub ttl_seconds: Option<TtlSeconds>,
}

impl fmt::Debug for ResumeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResumeRequest")
            .field("type_", &self.type_)
            .field("env", &self.env.as_ref().map(redact_env_map))
            .field("environment", &self.environment)
            .field("no_env", &self.no_env)
            .field("ttl_seconds", &self.ttl_seconds)
            .finish()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxesQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detached: Option<bool>,
}

impl fmt::Debug for CommandRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandRequest")
            .field("command", &"<redacted>")
            .field("cwd", &self.cwd)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("detached", &self.detached)
            .finish()
    }
}

impl CommandRequest {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            cwd: None,
            timeout_seconds: None,
            detached: None,
        }
    }

    pub fn with_timeout_seconds(mut self, seconds: u32) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: Option<bool>,
    pub stderr_truncated: Option<bool>,
    pub timed_out: bool,
    pub cwd: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

impl fmt::Debug for CommandResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandResponse")
            .field("ok", &self.ok)
            .field("type_", &self.type_)
            .field("success", &self.success)
            .field("exit_code", &self.exit_code)
            .field("signal", &self.signal)
            .field("stdout_len", &self.stdout.len())
            .field("stderr_len", &self.stderr.len())
            .field("stdout_truncated", &self.stdout_truncated)
            .field("stderr_truncated", &self.stderr_truncated)
            .field("timed_out", &self.timed_out)
            .field("cwd", &self.cwd)
            .field("started_at", &self.started_at)
            .field("finished_at", &self.finished_at)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReadQuery {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReadResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub path: Option<String>,
    pub content: Option<String>,
    pub encoding: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl fmt::Debug for FileReadResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileReadResponse")
            .field("ok", &self.ok)
            .field("type_", &self.type_)
            .field("path", &self.path)
            .field("content_len", &self.content.as_ref().map(|c| c.len()))
            .field("encoding", &self.encoding)
            .field("extra_keys", &self.extra.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteRequest {
    pub path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

impl fmt::Debug for FileWriteRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileWriteRequest")
            .field("path", &self.path)
            .field("content_len", &self.content.len())
            .field("encoding", &self.encoding)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub success: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPortRequest {
    pub port: u16,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPortResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub success: Option<bool>,
    pub port: Option<u16>,
    /// May include a short-lived `_token` query param — treat as secret.
    pub url: Option<String>,
    pub is_protected: Option<bool>,
    pub access: Option<String>,
}

impl std::fmt::Debug for HostPortResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostPortResponse")
            .field("ok", &self.ok)
            .field("type_", &self.type_)
            .field("success", &self.success)
            .field("port", &self.port)
            .field("url", &self.url.as_ref().map(|_| "<redacted>"))
            .field("is_protected", &self.is_protected)
            .field("access", &self.access)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyRequest {
    pub public_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyResponse {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub success: Option<bool>,
    pub machine_ip: Option<String>,
    pub ssh_user: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorBody {
    pub ok: bool,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub status: Option<u16>,
    pub code: Option<String>,
    pub message: Option<String>,
    pub request_id: Option<String>,
    pub error: Option<ApiErrorDetail>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorDetail {
    pub code: Option<String>,
    pub message: Option<String>,
    pub status: Option<u16>,
    pub details: Option<serde_json::Value>,
}

fn redact_env_map(env: &HashMap<String, String>) -> HashMap<String, &'static str> {
    env.keys().map(|k| (k.clone(), "<redacted>")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_request_debug_redacts_env_and_setup() {
        let mut env = HashMap::new();
        env.insert("SECRET".into(), "super-secret-value".into());
        let req = CreateBoxRequest {
            env: Some(env),
            setup_script: Some("export TOKEN=abc".into()),
            ..Default::default()
        };
        let s = format!("{req:?}");
        assert!(s.contains("<redacted>"));
        assert!(!s.contains("super-secret-value"));
        assert!(!s.contains("export TOKEN"));
    }

    #[test]
    fn box_deserializes_without_desktop_flags() {
        let v = json!({
            "id": "bx_23456789",
            "name": "demo",
            "state": "ready"
        });
        let b: Box = serde_json::from_value(v).unwrap();
        assert!(!b.desktop_available);
        assert!(!b.snapshot_available);
    }

    #[test]
    fn command_response_debug_hides_streams() {
        let res = CommandResponse {
            ok: true,
            type_: None,
            success: true,
            exit_code: Some(0),
            signal: None,
            stdout: "secret-output".into(),
            stderr: "secret-err".into(),
            stdout_truncated: None,
            stderr_truncated: None,
            timed_out: false,
            cwd: None,
            started_at: None,
            finished_at: None,
        };
        let s = format!("{res:?}");
        assert!(s.contains("stdout_len"));
        assert!(!s.contains("secret-output"));
        assert!(!s.contains("secret-err"));
    }
}
