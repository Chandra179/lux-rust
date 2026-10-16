//! Logging setup (`tracing`).

use anyhow::Context;
use tracing_subscriber::EnvFilter;

/// Installs the global subscriber: human-readable output in development,
/// JSON lines in production. Dependencies log at `warn`; this crate logs at
/// `level`.
pub fn init(environment: &str, level: &str) -> anyhow::Result<()> {
    let filter = EnvFilter::try_new(format!(
        "warn,lux_server={}",
        level.trim().to_ascii_lowercase()
    ))
    .context("build log filter")?;
    let builder = tracing_subscriber::fmt().with_env_filter(filter);

    let result = if environment == "prd" {
        builder.json().try_init()
    } else {
        builder.compact().try_init()
    };
    result
        .map_err(anyhow::Error::msg)
        .context("install tracing subscriber")
}
