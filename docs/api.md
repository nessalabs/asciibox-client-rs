# API reference

`BoxApi` exposes 59 endpoint operations. Each call returns `Result<T>` with a typed
response. The client shares its connection pool when cloned.

Use `cargo doc --no-deps --open` for searchable method signatures, request types,
response types, and examples. See [the SDK guide](guide.md) for complete workflows.

## Endpoints

| Method | HTTP route |
| --- | --- |
| `add_environment_repo` | `POST /environments/{environmentId}/repos` |
| `api_key_usage` | `GET /api-keys/{apiKeyId}/usage` |
| `api_keys` | `GET /api-keys` |
| `artifact` | `GET /boxes/{boxId}/artifacts` |
| `boxes` | `GET /boxes` |
| `command_raw` | `POST /boxes/{boxId}/commands` |
| `command_status` | `GET /boxes/{boxId}/commands/{processId}` |
| `create_with_options` | `POST /boxes` |
| `create_environment` | `POST /environments` |
| `create_webhook` | `POST /webhooks` |
| `delete_box` | `DELETE /boxes/{boxId}` |
| `delete_environment` | `DELETE /environments/{environmentId}` |
| `delete_environment_repo` | `DELETE /environments/{environmentId}/repos/{repositoryId}` |
| `delete_environment_secret_file` | `DELETE /environments/{environmentId}/secret-files` |
| `delete_environment_var` | `DELETE /environments/{environmentId}/vars/{key}` |
| `delete_named_snapshot` | `DELETE /named-snapshots/{name}` |
| `delete_snapshot` | `DELETE /snapshots/{snapshotId}` |
| `delete_webhook` | `DELETE /webhooks/{webhookId}` |
| `desktop_with` | `POST /boxes/{boxId}/desktop` |
| `environments` | `GET /environments` |
| `events` | `GET /boxes/{boxId}/events` |
| `fork` | `POST /boxes/{boxId}/fork` |
| `get` | `GET /boxes/{boxId}` |
| `get_data_retention` | `GET /account/data-retention` |
| `get_deletion_operation` | `GET /deletion-operations/{operationId}` |
| `latest_box_snapshot` | `GET /boxes/{boxId}/snapshots/latest` |
| `get_named_snapshot` | `GET /named-snapshots/{name}` |
| `snapshot_download` | `GET /snapshots/{snapshotId}/download` |
| `snapshot_file` | `GET /snapshots/{snapshotId}/files` |
| `snapshot_tree` | `GET /snapshots/{snapshotId}/tree` |
| `get_webhook` | `GET /webhooks/{webhookId}` |
| `host_port_with` | `POST /boxes/{boxId}/host` |
| `interrupt` | `POST /boxes/{boxId}/interrupt` |
| `limits_with` | `GET /limits` |
| `list_box_snapshots` | `GET /boxes/{boxId}/snapshots` |
| `list_named_snapshots` | `GET /named-snapshots` |
| `list_snapshots` | `GET /snapshots` |
| `list_webhooks` | `GET /webhooks` |
| `me` | `GET /me` |
| `prompt` | `POST /boxes/{boxId}/prompt` |
| `prompt_run_status` | `GET /boxes/{boxId}/prompts/{promptId}` |
| `read_file` | `GET /boxes/{boxId}/files` |
| `repos` | `GET /repos` |
| `resume` | `POST /boxes/{boxId}/resume` |
| `rotate_webhook_signing_secret` | `POST /webhooks/{webhookId}/rotate` |
| `save_named_snapshot` | `POST /named-snapshots` |
| `secrets` | `GET /secrets` |
| `select_repo` | `POST /repos` |
| `set_environment_secret_file` | `PUT /environments/{environmentId}/secret-files` |
| `set_environment_var` | `PUT /environments/{environmentId}/vars/{key}` |
| `ssh_key` | `POST /boxes/{boxId}/sshkey` |
| `stop` | `POST /boxes/{boxId}/stop` |
| `update` | `PATCH /boxes/{boxId}` |
| `update_data_retention` | `PATCH /account/data-retention` |
| `update_environment` | `PUT /environments/{environmentId}` |
| `update_secrets` | `POST /secrets` |
| `update_webhook` | `PATCH /webhooks/{webhookId}` |
| `upgrade_environment` | `POST /environments/{environmentId}/upgrade` |
| `write_file` | `PUT /boxes/{boxId}/files` |

## Convenience methods

`create`, `create_with_idempotency`, `limits`, `desktop` and `host_port` provide
shorter calls for common options. Their full variants expose organization scope,
idempotency keys, desktop theme, and hosted-port visibility/title settings.

`command` returns a completed command result. `command_raw` supports both
completed and detached responses through `CommandResult`. Convert a detached
process ID to a string when calling `command_status`.

## Helpers

| Function | Purpose |
| --- | --- |
| `exec_command`, `BoxApi::exec` | Run a shell command and wait for its result |
| `read_text`, `write_text` | Read and write UTF-8 files |
| `stop_and_remove` | Stop/archive a Box and retain snapshots |
| `stop_and_remove_with` | Choose stop/archive or permanent deletion |
| `wait_until_ready`, `wait_until_ready_with` | Wait for a Box to accept work |
| `wait_until_idle`, `wait_until_idle_with` | Wait for a Box to become idle |
| `wait_for_prompt`, `wait_for_prompt_done` | Wait for a prompt to finish or fail |
| `wait_for_desktop`, `wait_for_desktop_with` | Wait for a desktop URL |
| `wait_for_deletion` | Observe deletion until completion or timeout |
| `stream_events`, `stream_prompt` | Consume event pages as cancellable streams |

Wait helpers accept `WaitOptions` through their option-taking variants. Streams
accept `StreamEventsOptions` or `StreamPromptOptions`. See [the SDK guide](guide.md)
for default budgets, cancellation, nullable fields, and error handling.
