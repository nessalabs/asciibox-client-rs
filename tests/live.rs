//! Live API smoke — gated behind `#[ignore]` + `BOX_API_KEY`.
//!
//! ```bash
//! BOX_API_KEY=… cargo test --test live -- --ignored --nocapture
//! BOX_API_KEY=… BOX_ID=bx_… cargo test --test live -- --ignored
//! ```

use box_client::{exec_command, BoxApi, Configuration};

fn client() -> BoxApi {
    let key = std::env::var("BOX_API_KEY").expect("BOX_API_KEY required for live tests");
    BoxApi::new(Configuration::new(key).expect("config")).expect("client")
}

#[tokio::test]
#[ignore = "requires BOX_API_KEY; live network"]
async fn live_me_and_boxes() {
    let api = client();
    let me = api.me().await.expect("me");
    assert!(me.ok);
    let list = api.boxes(None).await.expect("boxes");
    assert!(list.ok);
    eprintln!("user={:?} boxes={}", me.user.email, list.boxes.len());
}

#[tokio::test]
#[ignore = "requires BOX_API_KEY and BOX_ID; live network"]
async fn live_get_and_exec_existing_box() {
    let api = client();
    let box_id = std::env::var("BOX_ID").expect("BOX_ID required");
    let info = api.get(&box_id).await.expect("get");
    eprintln!("box {} state={}", info.box_.id, info.box_.state.as_str());
    assert!(
        info.box_.state.is_operable(),
        "box must be ready/idle/running (got {})",
        info.box_.state.as_str()
    );
    let out = exec_command(&api, &box_id, "echo parity-ok && uname -s", None, Some(30))
        .await
        .expect("exec");
    assert!(out.stdout.contains("parity-ok"), "stdout={}", out.stdout);
    assert!(!out.timed_out);
}

/// Read-only coverage for the response models and routes corrected in the review.
#[tokio::test]
#[ignore = "requires BOX_API_KEY; live network, read-only"]
async fn live_runtime_read_contracts() {
    let api = client();
    let environments = api.environments().await.expect("environments");
    // Do not print environment contents, snapshot paths, or signed URLs.
    eprintln!("environment count={}", environments.environments.len());
    let snapshots = api
        .list_snapshots(Some(&box_client::SnapshotsQuery {
            limit: Some(1),
            sort: Some("desc".into()),
            ..Default::default()
        }))
        .await
        .expect("snapshots");
    if let Some(snapshot) = snapshots.snapshots.first() {
        let tree = api
            .snapshot_tree(&snapshot.id)
            .await
            .expect("snapshot tree");
        let download = api
            .snapshot_download(&snapshot.id)
            .await
            .expect("snapshot download");
        assert_eq!(tree.snapshot_id, snapshot.id);
        assert_eq!(download.snapshot_id, snapshot.id);
        eprintln!(
            "snapshot entries={} chunks={}",
            tree.entries.len(),
            download.chunks.len()
        );
    }
}

/// Mutations are limited to a newly created fixture environment, which is cleaned
/// up even when assertions fail. This never updates an existing environment.
#[tokio::test]
#[ignore = "requires BOX_API_KEY and BOX_TEST_MUTATIONS=1; creates a temporary environment"]
async fn live_environment_round_trip() {
    assert_eq!(std::env::var("BOX_TEST_MUTATIONS").as_deref(), Ok("1"));
    let api = client();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!("rust-contract-{unique}");
    let created = api
        .create_environment(box_client::CreateBoxEnvironmentRequest { name: name.clone() })
        .await
        .expect("create environment");
    let env = created
        .environments
        .iter()
        .find(|env| env.name == name)
        .expect("created fixture missing");
    let result = async {
        let updated = api
            .update_environment(
                &env.id,
                box_client::UpdateBoxEnvironmentRequest {
                    env_contents: Some("CONTRACT_MARKER=fixture-value".into()),
                    ..Default::default()
                },
            )
            .await?;
        if !updated.success {
            return Err(box_client::Error::Unexpected("update unsuccessful".into()));
        }
        let listed = api.environments().await?;
        let fixture = listed
            .environments
            .iter()
            .find(|value| value.id == env.id)
            .ok_or_else(|| box_client::Error::Unexpected("fixture missing after update".into()))?;
        if !fixture
            .env_contents
            .contains("CONTRACT_MARKER=fixture-value")
        {
            return Err(box_client::Error::Unexpected(
                "environment marker mismatch".into(),
            ));
        }
        if format!("{fixture:?}").contains("fixture-value")
            || format!("{listed:?}").contains("fixture-value")
        {
            return Err(box_client::Error::Unexpected(
                "Debug exposed environment marker".into(),
            ));
        }
        Ok::<_, box_client::Error>(())
    }
    .await;
    let cleanup = api.delete_environment(&env.id).await;
    cleanup.expect("delete fixture environment");
    assert!(!api
        .environments()
        .await
        .expect("verify cleanup")
        .environments
        .iter()
        .any(|value| value.id == env.id));
    result.expect("environment round trip");
}
