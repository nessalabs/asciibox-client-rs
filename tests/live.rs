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
