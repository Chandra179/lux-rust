//! API integration tests. Each test spawns the app on a random local port
//! with a temporary SQLite file and key-value store.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod example;
mod health;
mod helpers;
mod middleware;
mod swagger;
