//! RUST_LOG=box_client=debug cargo run --example logging
use box_client::{BoxApi, BoxClientConfig, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();
    let api = BoxApi::new(BoxClientConfig::from_env()?)?;
    let response = api.boxes(None).await?;
    println!("{} boxes", response.boxes.len());
    Ok(())
}
