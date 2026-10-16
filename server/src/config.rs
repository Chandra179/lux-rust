//! Application configuration, loaded from `server/config/config_<env>.yaml`.
//!
//! The database location can be overridden with the `SQLITE_DSN` environment
//! variable.

use std::{env, path::Path, time::Duration};

use ::config::{Config as Settings, File, FileFormat};
use anyhow::Context;
use serde::Deserialize;

/// Top-level server configuration. Unknown keys are rejected.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// HTTP listener, timeouts, and body limit.
    pub http: HttpConfig,
    /// SQLite database location.
    pub sqlite: SqliteConfig,
}

/// HTTP server settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    /// Port (`"8080"`), `":8080"`, or a full `host:port` address.
    pub port: String,
    /// How long a request may take before it is answered with 408.
    pub request_timeout_in_second: u64,
    /// How long in-flight requests get to finish after a shutdown signal.
    pub shutdown_timeout_in_second: u64,
    /// Largest accepted request body; bigger ones get 413.
    pub max_body_size_in_bytes: usize,
}

/// SQLite settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SqliteConfig {
    /// Path of the SQLite database file.
    pub dsn: String,
}

/// Every problem found by [`Config::validate`], joined into one message.
#[derive(Debug, thiserror::Error)]
#[error("{}", .0.join("; "))]
pub struct ValidationError(Vec<String>);

impl Config {
    /// Loads the config file at `path` and applies the `SQLITE_DSN`
    /// environment override.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        Self::load_with_overrides(path, env::var("SQLITE_DSN").ok())
    }

    /// Like [`Config::load`], but with an explicit override instead of reading
    /// the environment.
    pub fn load_with_overrides(path: &Path, sqlite_dsn: Option<String>) -> anyhow::Result<Self> {
        let settings = Settings::builder()
            .add_source(File::from(path).format(FileFormat::Yaml))
            .set_override_option("sqlite.dsn", sqlite_dsn)?
            .build()
            .with_context(|| format!("read {}", path.display()))?;

        let cfg: Self = settings.try_deserialize().context("parse config")?;
        cfg.validate().context("validate config")?;
        Ok(cfg)
    }

    /// Checks the values that are required for the server to start safely.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut problems = Vec::new();

        if self.http.port.trim().is_empty() {
            problems.push("http.port is required".to_owned());
        }
        if self.http.request_timeout_in_second == 0 {
            problems.push("http.request_timeout_in_second must be greater than zero".to_owned());
        }
        if self.http.shutdown_timeout_in_second == 0 {
            problems.push("http.shutdown_timeout_in_second must be greater than zero".to_owned());
        }
        if self.http.max_body_size_in_bytes == 0 {
            problems.push("http.max_body_size_in_bytes must be greater than zero".to_owned());
        }

        if self.sqlite.dsn.trim().is_empty() {
            problems.push("sqlite.dsn is required".to_owned());
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(ValidationError(problems))
        }
    }
}

impl HttpConfig {
    /// Socket address to bind. A bare port or `:port` listens on all
    /// interfaces.
    pub fn listen_address(&self) -> String {
        let port = self.port.trim();
        if let Some(port) = port.strip_prefix(':') {
            format!("0.0.0.0:{port}")
        } else if port.contains(':') {
            port.to_owned()
        } else {
            format!("0.0.0.0:{port}")
        }
    }

    /// [`HttpConfig::request_timeout_in_second`] as a [`Duration`].
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_in_second)
    }

    /// [`HttpConfig::shutdown_timeout_in_second`] as a [`Duration`].
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_in_second)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    fn config_file(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join(name)
    }

    fn dev_config() -> Config {
        Config::load_with_overrides(&config_file("config_dev.yaml"), None).unwrap()
    }

    #[test]
    fn dev_config_is_valid() {
        let cfg = dev_config();
        assert_eq!(cfg.http.port, "8080");
        assert_eq!(cfg.sqlite.dsn, "lux.db");
    }

    #[test]
    fn prd_config_requires_the_database_override() {
        let path = config_file("config_prd.yaml");

        let err = Config::load_with_overrides(&path, None).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("sqlite.dsn is required"), "{message}");

        let cfg = Config::load_with_overrides(&path, Some("/data/lux.db".to_owned())).unwrap();
        assert_eq!(cfg.sqlite.dsn, "/data/lux.db");
    }

    #[test]
    fn rejects_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let yaml = fs::read_to_string(config_file("config_dev.yaml")).unwrap();
        fs::write(&path, format!("{yaml}\nsurprise: true\n")).unwrap();

        assert!(Config::load_with_overrides(&path, None).is_err());
    }

    #[test]
    fn missing_file_is_an_error() {
        let path = config_file("does_not_exist.yaml");
        assert!(Config::load_with_overrides(&path, None).is_err());
    }

    #[test]
    fn validate_reports_every_problem() {
        let mut cfg = dev_config();
        cfg.http.port = "  ".to_owned();
        cfg.http.request_timeout_in_second = 0;
        cfg.http.shutdown_timeout_in_second = 0;
        cfg.http.max_body_size_in_bytes = 0;
        cfg.sqlite.dsn = String::new();

        let message = cfg.validate().unwrap_err().to_string();
        for expected in [
            "http.port is required",
            "http.request_timeout_in_second must be greater than zero",
            "http.shutdown_timeout_in_second must be greater than zero",
            "http.max_body_size_in_bytes must be greater than zero",
            "sqlite.dsn is required",
        ] {
            assert!(
                message.contains(expected),
                "missing {expected:?} in {message:?}"
            );
        }
    }

    #[test]
    fn listen_address_forms() {
        let mut http = dev_config().http;
        for (port, expected) in [
            ("8080", "0.0.0.0:8080"),
            (":9090", "0.0.0.0:9090"),
            ("127.0.0.1:7000", "127.0.0.1:7000"),
            (" 8081 ", "0.0.0.0:8081"),
        ] {
            http.port = port.to_owned();
            assert_eq!(http.listen_address(), expected);
        }
    }
}
