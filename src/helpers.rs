//! Hand helpers mirroring `@asciidev/box-sdk` `box-helpers.ts` for the surface we ship.

use crate::client::BoxApi;
use crate::error::Result;
use crate::types::{CommandRequest, CommandResponse, FileWriteRequest, FileWriteResponse};

/// TS `execCommand(api, boxId, command, cwd?, timeoutSeconds = 30)`.
pub async fn exec_command(
    api: &BoxApi,
    box_id: &str,
    command: impl Into<String>,
    cwd: Option<&str>,
    timeout_seconds: Option<u32>,
) -> Result<CommandResponse> {
    let mut req = CommandRequest::new(command).with_timeout_seconds(timeout_seconds.unwrap_or(30));
    if let Some(cwd) = cwd {
        req = req.with_cwd(cwd);
    }
    api.command(box_id, req).await
}

/// TS `readText(api, boxId, path)` — `encoding: 'utf8'`.
pub async fn read_text(api: &BoxApi, box_id: &str, path: impl Into<String>) -> Result<String> {
    let res = api.read_file(box_id, path, Some("utf8")).await?;
    Ok(res.content.unwrap_or_default())
}

/// TS `writeText(api, boxId, path, content)` — `encoding: 'utf8'`.
pub async fn write_text(
    api: &BoxApi,
    box_id: &str,
    path: impl Into<String>,
    content: impl Into<String>,
) -> Result<FileWriteResponse> {
    api.write_file(
        box_id,
        FileWriteRequest {
            path: path.into(),
            content: content.into(),
            encoding: Some("utf8".into()),
        },
    )
    .await
}

/// TS `stopAndRemove(api, boxId)` without `{ delete: true }` — stop and keep snapshots.
pub async fn stop_and_remove(
    api: &BoxApi,
    box_id: &str,
) -> Result<crate::types::BoxActionResponse> {
    api.stop(box_id, None).await
}
