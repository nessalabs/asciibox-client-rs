//! BOX_API_KEY=… BOX_ID=bx_… BOX_PROMPT='…' cargo run --example stream_prompt
//! Queues one prompt and prints event IDs/types until that prompt finishes.
use box_client::{stream_prompt, BoxApi, BoxClientConfig, Error, PromptProvider, PromptRequest};
use futures_util::{pin_mut, StreamExt};

#[tokio::main]
async fn main() -> box_client::Result<()> {
    let api = BoxApi::new(BoxClientConfig::from_env()?)?;
    let box_id = std::env::var("BOX_ID").map_err(|_| Error::Config("set BOX_ID".into()))?;
    let prompt = std::env::var("BOX_PROMPT").map_err(|_| Error::Config("set BOX_PROMPT".into()))?;
    let stream = stream_prompt(
        &api,
        &box_id,
        PromptRequest::new(PromptProvider::Codex, prompt),
        None,
    );
    pin_mut!(stream);
    while let Some(event) = stream.next().await {
        let event = event?;
        println!(
            "{} {}",
            event.id.as_deref().unwrap_or("(no id)"),
            event.type_
        );
    }
    Ok(())
}
