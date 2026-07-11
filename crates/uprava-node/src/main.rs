#[tokio::main]
async fn main() -> anyhow::Result<()> {
    uprava_node::run().await
}
