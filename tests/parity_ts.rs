//! Parity tests against `@asciidev/box-sdk` behavior for the shared v0.1 surface.
//!
//! The published TypeScript package ships **no test suite** (OpenAPI Generator +
//! hand-written `box-helpers.ts`). These tests encode that helper/API contract
//! so Rust stays 1:1 with what TS callers actually rely on.
//!
//! See `docs/parity.md` for the method matrix.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use box_client::{
    exec_command, read_text, wait_until_ready, wait_until_ready_with, write_text, BoxApi,
    BoxesQuery, CommandRequest, Configuration, CreateBoxRequest, Error, FileWriteRequest,
    ResumeRequest, StopRequest, UpdateBoxRequest, WaitOptions,
};
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn box_json(id: &str, state: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": "demo",
        "state": state,
        "desktopAvailable": false,
        "snapshotAvailable": false,
        "type": "default",
        "vcpu": 4,
        "memoryGB": 8
    })
}

async fn api(server: &MockServer) -> BoxApi {
    BoxApi::new(
        Configuration::new("box_test_key")
            .unwrap()
            .with_base_path(server.uri())
            .unwrap(),
    )
    .unwrap()
}

// --- Account (TS: me / limits) ---

#[tokio::test]
async fn ts_me() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("Authorization", "Bearer box_test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "type": "account.me",
            "user": { "login": "dev@example.com", "email": "dev@example.com" }
        })))
        .mount(&server)
        .await;

    let me = api(&server).await.me().await.unwrap();
    assert_eq!(me.user.email.as_deref(), Some("dev@example.com"));
}

#[tokio::test]
async fn ts_limits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/limits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "canStart": true,
            "activeBoxes": 0
        })))
        .mount(&server)
        .await;

    let limits = api(&server).await.limits().await.unwrap();
    assert!(limits.ok);
    assert_eq!(limits.extra.get("canStart"), Some(&json!(true)));
}

// --- Lifecycle (TS: boxes / create / get / update / stop / resume) ---

#[tokio::test]
async fn ts_boxes_list_and_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/boxes"))
        .and(query_param("state", "idle"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "type": "box.list",
            "boxes": [box_json("bx_23456789", "idle")],
            "pageInfo": { "nextCursor": null, "hasMore": false, "limit": 50 }
        })))
        .mount(&server)
        .await;

    let q = BoxesQuery {
        state: Some("idle".into()),
        limit: Some(50),
        ..Default::default()
    };
    let list = api(&server).await.boxes(Some(&q)).await.unwrap();
    assert_eq!(list.boxes.len(), 1);
    assert_eq!(list.boxes[0].id, "bx_23456789");
}

#[tokio::test]
async fn ts_create_with_idempotency_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/boxes"))
        .and(header("Idempotency-Key", "job-42"))
        .and(body_partial_json(json!({ "ttlSeconds": 1800 })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "type": "box.created",
            "status": "provisioning",
            "ttlSeconds": 1800,
            "box": box_json("bx_23456789", "provisioning")
        })))
        .mount(&server)
        .await;

    let created = api(&server)
        .await
        .create_with_idempotency(CreateBoxRequest::ttl(1800), Some("job-42"))
        .await
        .unwrap();
    assert_eq!(created.box_.id, "bx_23456789");
}

#[tokio::test]
async fn ts_get_update_stop_resume() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/boxes/bx_23456789"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "type": "box.info", "box": box_json("bx_23456789", "idle")
        })))
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/boxes/bx_23456789"))
        .and(body_partial_json(json!({ "name": "sdk-demo" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "type": "box.info",
            "box": { "id": "bx_23456789", "name": "sdk-demo", "state": "idle",
                      "desktopAvailable": false, "snapshotAvailable": false }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/stop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "type": "box.stopping", "id": "bx_23456789", "status": "archiving"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/resume"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "type": "box.resuming", "id": "bx_23456789", "status": "provisioning"
        })))
        .mount(&server)
        .await;

    let client = api(&server).await;
    assert_eq!(
        client.get("bx_23456789").await.unwrap().box_.state.as_str(),
        "idle"
    );
    assert_eq!(
        client
            .update(
                "bx_23456789",
                UpdateBoxRequest {
                    name: Some("sdk-demo".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .box_
            .name,
        "sdk-demo"
    );
    assert_eq!(
        client
            .stop("bx_23456789", Some(StopRequest::default()))
            .await
            .unwrap()
            .status,
        "archiving"
    );
    assert_eq!(
        client
            .resume("bx_23456789", Some(ResumeRequest::default()))
            .await
            .unwrap()
            .status,
        "provisioning"
    );
}

// --- Helpers (TS box-helpers.ts) ---

#[tokio::test]
async fn ts_wait_until_ready_success_states() {
    for state in ["ready", "idle", "running"] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/boxes/bx_23456789"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ok": true, "type": "box.info", "box": box_json("bx_23456789", state)
            })))
            .mount(&server)
            .await;
        let box_ = wait_until_ready(&api(&server).await, "bx_23456789")
            .await
            .unwrap();
        assert_eq!(box_.state.as_str(), state);
    }
}

#[tokio::test]
async fn ts_wait_until_ready_terminal_states() {
    for state in ["archived", "archiving", "error"] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/boxes/bx_23456789"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ok": true, "type": "box.info", "box": box_json("bx_23456789", state)
            })))
            .mount(&server)
            .await;
        let err = wait_until_ready(&api(&server).await, "bx_23456789")
            .await
            .unwrap_err();
        match err {
            Error::BoxTerminal { state: s, .. } => assert_eq!(s, state),
            other => panic!("expected BoxTerminal, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn ts_wait_until_ready_polls_then_succeeds() {
    let server = MockServer::start().await;
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = hits.clone();
    Mock::given(method("GET"))
        .and(path("/boxes/bx_23456789"))
        .respond_with(move |_req: &Request| {
            let n = hits2.fetch_add(1, Ordering::SeqCst);
            let state = if n == 0 { "provisioning" } else { "ready" };
            ResponseTemplate::new(200).set_body_json(json!({
                "ok": true, "type": "box.info", "box": box_json("bx_23456789", state)
            }))
        })
        .mount(&server)
        .await;

    let box_ = wait_until_ready_with(
        &api(&server).await,
        "bx_23456789",
        WaitOptions {
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(20),
        },
    )
    .await
    .unwrap();
    assert_eq!(box_.state.as_str(), "ready");
    assert!(hits.load(Ordering::SeqCst) >= 2);
}

#[tokio::test]
async fn ts_exec_command_default_timeout_30() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/commands"))
        .and(body_partial_json(json!({
            "command": "uname -a",
            "timeoutSeconds": 30
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "type": "command.finished",
            "success": true,
            "exitCode": 0,
            "stdout": "Linux\n",
            "stderr": "",
            "timedOut": false
        })))
        .mount(&server)
        .await;

    let out = exec_command(&api(&server).await, "bx_23456789", "uname -a", None, None)
        .await
        .unwrap();
    assert_eq!(out.stdout, "Linux\n");
    assert_eq!(out.exit_code, Some(0));
}

#[tokio::test]
async fn ts_exec_command_with_cwd_and_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/commands"))
        .and(body_partial_json(json!({
            "command": "pwd",
            "cwd": "project",
            "timeoutSeconds": 60
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "type": "command.finished", "success": true,
            "exitCode": 0, "stdout": "/home/user/project\n", "stderr": "", "timedOut": false
        })))
        .mount(&server)
        .await;

    let out = exec_command(
        &api(&server).await,
        "bx_23456789",
        "pwd",
        Some("project"),
        Some(60),
    )
    .await
    .unwrap();
    assert!(out.stdout.contains("project"));
}

#[tokio::test]
async fn ts_read_text_write_text_utf8() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/boxes/bx_23456789/files"))
        .and(query_param("path", "notes/result.txt"))
        .and(query_param("encoding", "utf8"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "path": "notes/result.txt",
            "content": "done\n",
            "encoding": "utf8"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/files"))
        .and(body_partial_json(json!({
            "path": "notes/result.txt",
            "content": "done\n",
            "encoding": "utf8"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "success": true
        })))
        .mount(&server)
        .await;

    let client = api(&server).await;
    write_text(&client, "bx_23456789", "notes/result.txt", "done\n")
        .await
        .unwrap();
    let text = read_text(&client, "bx_23456789", "notes/result.txt")
        .await
        .unwrap();
    assert_eq!(text, "done\n");
}

#[tokio::test]
async fn ts_host_port_and_ssh_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/host"))
        .and(body_partial_json(json!({ "port": 3000 })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "type": "port.hosted", "success": true, "port": 3000,
            "url": "https://example.on.ascii.dev?_token=redacted"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/sshkey"))
        .and(body_partial_json(
            json!({ "publicKey": "ssh-ed25519 AAAA" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true, "type": "ssh_key.configured", "success": true,
            "machineIp": "203.0.113.10", "sshUser": "user"
        })))
        .mount(&server)
        .await;

    let client = api(&server).await;
    let host = client.host_port("bx_23456789", 3000).await.unwrap();
    assert_eq!(host.port, Some(3000));
    let ssh = client
        .ssh_key("bx_23456789", "ssh-ed25519 AAAA")
        .await
        .unwrap();
    assert_eq!(ssh.ssh_user.as_deref(), Some("user"));
}

#[tokio::test]
async fn ts_required_box_id_rejected_like_required_error() {
    // TS throws RequiredError when boxId is null/undefined; we validate format.
    let client = BoxApi::new(Configuration::new("box_test_key").unwrap()).unwrap();
    let err = client
        .command("bad", CommandRequest::new("true"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidBoxId(_)));
}

#[tokio::test]
async fn ts_api_error_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/boxes/bx_23456789"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "ok": false,
            "type": "box.error",
            "status": 404,
            "code": "not_found",
            "message": "Box not found",
            "requestId": "req_abc",
            "error": { "code": "not_found", "message": "Box not found", "status": 404 }
        })))
        .mount(&server)
        .await;

    let err = api(&server).await.get("bx_23456789").await.unwrap_err();
    match err {
        Error::Api {
            status,
            code,
            request_id,
            ..
        } => {
            assert_eq!(status, 404);
            assert_eq!(code, "not_found");
            assert_eq!(request_id, "req_abc");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn rejects_detached_command() {
    let client = BoxApi::new(Configuration::new("box_test_key").unwrap()).unwrap();
    let err = client
        .command(
            "bx_23456789",
            CommandRequest {
                command: "sleep 1".into(),
                cwd: None,
                timeout_seconds: Some(30),
                detached: Some(true),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Config(_)));
}

#[tokio::test]
async fn create_request_ttl_serializes_like_ts() {
    assert_eq!(
        serde_json::to_value(CreateBoxRequest::ttl(1800)).unwrap(),
        json!({ "ttlSeconds": 1800 })
    );
    assert_eq!(
        serde_json::to_value(CreateBoxRequest::no_auto_stop()).unwrap(),
        json!({ "ttlSeconds": null })
    );
}

#[tokio::test]
async fn write_file_raw_matches_ts_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/boxes/bx_23456789/files"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "ok": true, "success": true })),
        )
        .mount(&server)
        .await;
    api(&server)
        .await
        .write_file(
            "bx_23456789",
            FileWriteRequest {
                path: "a.txt".into(),
                content: "x".into(),
                encoding: Some("utf8".into()),
            },
        )
        .await
        .unwrap();
}
