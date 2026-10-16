//! Application configuration, loaded from `server/config/config_<env>.yaml`.
//!
//! The database location can be overridden with the `SQLITE_DSN` environment
//! variable.

use std::{env, error::Error, fmt, path::Path};

use ::config::{Config as Settings, File, FileFormat};
use serde::Deserialize;

/// Top-level server configuration. Unknown keys are rejected.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// SQLite database location.
    pub sqlite: SqliteConfig,
}

/// SQLite settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SqliteConfig {
    /// Path of the SQLite database file.
    pub dsn: String,
}

/// A configuration problem, as a ready-to-print message.
#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ConfigError {}

impl Config {
    /// Loads the config file at `path` and applies the `SQLITE_DSN`
    /// environment override.
    ///
    /// # Errors
    ///
    /// If the file cannot be read or parsed, or a value is invalid.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        Self::load_with_overrides(path, env::var("SQLITE_DSN").ok())
    }

    /// Like [`Config::load`], but with an explicit override instead of reading
    /// the environment.
    ///
    /// # Errors
    ///
    /// If the file cannot be read or parsed, or a value is invalid.
    pub fn load_with_overrides(
        path: &Path,
        sqlite_dsn: Option<String>,
    ) -> Result<Self, ConfigError> {
        let settings = Settings::builder()
            .add_source(File::from(path).format(FileFormat::Yaml))
            .set_override_option("sqlite.dsn", sqlite_dsn)
            .and_then(|builder| builder.build())
            .map_err(|e| ConfigError(format!("read {}: {e}", path.display())))?;

        let cfg: Self = settings
            .try_deserialize()
            .map_err(|e| ConfigError(format!("parse config: {e}")))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Checks the values that are required for the server to start safely.
    ///
    /// # Errors
    ///
    /// One message listing every problem found.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut problems = Vec::new();

        if self.sqlite.dsn.trim().is_empty() {
            problems.push("sqlite.dsn is required".to_owned());
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(ConfigError(format!(
                "invalid config: {}",
                problems.join("; ")
            )))
        }
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

    #[test]
    fn dev_config_is_valid() {
        let cfg = Config::load_with_overrides(&config_file("config_dev.yaml"), None).unwrap();
        assert_eq!(cfg.sqlite.dsn, "lux.db");
    }

    #[test]
    fn prd_config_requires_the_database_override() {
        let path = config_file("config_prd.yaml");

        let err = Config::load_with_overrides(&path, None).unwrap_err();
        let message = err.to_string();
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
}
