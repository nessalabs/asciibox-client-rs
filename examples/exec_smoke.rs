//! Smoke: get + exec against an existing box.
//!
//! ```bash
//! BOX_API_KEY=… BOX_ID=bx_… cargo run --example exec_smoke
//! ```
use box_client::{wait_until_ready, BoxApi, CommandRequest, Configuration, ResumeRequest};

#[tokio::main]
async fn main() -> box_client::Result<()> {
    let api = BoxApi::new(Configuration::from_env()?)?;
    let box_id = std::env::var("BOX_ID").map_err(|_| {
        box_client::Error::Config("set BOX_ID to a box id (bx_…); no default".into())
    })?;

    let info = api.get(&box_id).await?;
    println!("state={}", info.box_.state.as_str());
    if !info.box_.state.is_operable() {
        println!("resuming…");
        api.resume(&box_id, Some(ResumeRequest::default())).await?;
        wait_until_ready(&api, &box_id).await?;
    }

    let out = api
        .command(
            &box_id,
            CommandRequest::new("uname -a && command -v opencode").with_timeout_seconds(60),
        )
        .await?;
    println!("exit={:?} timed_out={}", out.exit_code, out.timed_out);
    print!("{}", out.stdout);
    if !out.stderr.is_empty() {
        eprint!("{}", out.stderr);
    }
    Ok(())
}
