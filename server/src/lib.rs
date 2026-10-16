//! Lux API server.
//!
//! `main.rs` only calls [`run`]; everything else lives here so integration
//! tests can build the same app with [`build_router`].

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod config;
mod error;
mod middleware;
mod modules;
mod router;
mod state;
pub mod store;
mod swagger;
mod telemetry;

use std::{env, future::IntoFuture, path::Path, time::Duration};

use anyhow::{Context, anyhow, bail};
use axum::Router;
use tokio::{net::TcpListener, sync::oneshot};

use crate::config::Config;

pub use router::build_router;

/// Loads configuration, opens the datastores, and serves HTTP until SIGINT or
/// SIGTERM, then shuts down gracefully within the configured timeout.
pub async fn run() -> anyhow::Result<()> {
    let environment = app_environment()?;
    let config_path = Path::new("server/config").join(format!("config_{environment}.yaml"));
    let config = Config::load(&config_path)
        .with_context(|| format!("load config {}", config_path.display()))?;

    telemetry::init(&environment, &config.logger.level)?;

    let db = store::sqlite::connect(&config.sqlite.dsn)
        .await
        .with_context(|| format!("open SQLite database {}", config.sqlite.dsn))?;
    // The migrations are embedded in the binary, so a fresh checkout or
    // container starts without any extra tooling. `make migrate-up` applies
    // the same files explicitly.
    store::sqlite::migrate(&db)
        .await
        .context("apply database migrations")?;
    let kv = store::kv::KvStore::open(&config.kv.path)
        .await
        .with_context(|| format!("open key-value database {}", config.kv.path))?;

    let app = build_router(db.clone(), kv, &config);

    let address = config.http.listen_address();
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("listen on {address}"))?;
    tracing::info!(%address, environment = %environment, "server listening");

    serve(listener, app, config.http.shutdown_timeout()).await?;

    db.close().await;
    tracing::info!("server stopped");
    Ok(())
}

/// `APP_ENVIRONMENT`, lower-cased; `dev` when unset or empty.
fn app_environment() -> anyhow::Result<String> {
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
        bail!("APP_ENVIRONMENT {environment:?} may only contain letters, digits, '-' and '_'");
    }
    Ok(environment)
}

/// Serves `app` until a shutdown signal arrives, then waits at most
/// `shutdown_timeout` for in-flight requests to finish.
async fn serve(
    listener: TcpListener,
    app: Router,
    shutdown_timeout: Duration,
) -> anyhow::Result<()> {
    let (signalled_tx, signalled_rx) = oneshot::channel::<()>();

    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        shutdown_signal().await;
        tracing::info!("shutdown signal received");
        // The receiver only goes away once the server has already finished.
        let _ = signalled_tx.send(());
    });

    tokio::select! {
        result = server.into_future() => result.context("serve HTTP"),
        () = shutdown_deadline(signalled_rx, shutdown_timeout) => {
            Err(anyhow!("graceful shutdown did not finish within {shutdown_timeout:?}"))
        }
    }
}

/// Completes `timeout` after the shutdown signal; never completes if the
/// signal never arrives.
async fn shutdown_deadline(signalled: oneshot::Receiver<()>, timeout: Duration) {
    if signalled.await.is_ok() {
        tokio::time::sleep(timeout).await;
    } else {
        std::future::pending::<()>().await;
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
