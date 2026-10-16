//! Lux server. For now it loads the configuration and opens the database;
//! there is no HTTP endpoint yet.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod config;
mod db;

use std::{env, error::Error, path::Path};

use crate::config::Config;

// Returning `Box<dyn Error>` lets `?` pass any error up; Rust prints it and
// exits with a failure code. `map_err` adds what we were doing at the time.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let environment = app_environment()?;
    let config_path = Path::new("server/config").join(format!("config_{environment}.yaml"));
    let config = Config::load(&config_path)
        .map_err(|e| format!("load config {}: {e}", config_path.display()))?;

    let pool = db::connect(&config.sqlite.dsn)
        .await
        .map_err(|e| format!("open SQLite database {}: {e}", config.sqlite.dsn))?;
    // The migrations are embedded in the binary, so a fresh checkout or
    // container needs no extra tooling.
    db::migrate(&pool)
        .await
        .map_err(|e| format!("apply database migrations: {e}"))?;

    println!(
        "loaded {environment} config; database ready at {}",
        config.sqlite.dsn
    );

    pool.close().await;
    Ok(())
}

/// `APP_ENVIRONMENT`, lower-cased; `dev` when unset or empty.
fn app_environment() -> Result<String, String> {
    let environment = env::var("APP_ENVIRONMENT")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "dev".to_owned());

    // The value becomes part of a file name, so keep it to a safe alphabet.
    if !environment
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(format!(
            "APP_ENVIRONMENT {environment:?} may only contain letters, digits, '-' and '_'"
        ));
    }
    Ok(environment)
}
