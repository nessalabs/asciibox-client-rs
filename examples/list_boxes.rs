use box_client::{BoxApi, BoxClientConfig};

#[tokio::main]
async fn main() -> box_client::Result<()> {
    let api = BoxApi::new(BoxClientConfig::from_env()?)?;
    let list = api.boxes(None).await?;
    for b in &list.boxes {
        println!("{}  state={:<12}  name={}", b.id, b.state.as_str(), b.name);
    }
    println!("{} box(es)", list.boxes.len());
    Ok(())
}
