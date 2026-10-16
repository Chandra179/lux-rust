//! Lux API server.
//!
//! `main.rs` only calls [`run`]; everything else lives here.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod config;
pub mod db;
mod error;
mod routes;
mod startup;

pub use startup::run;
