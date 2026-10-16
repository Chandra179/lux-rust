//! Binary entry point. All logic lives in the `lux_server` library.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    lux_server::run().await
}
