//! Configuration from environment variables. Any secret `X` may instead be
//! given as `X_FILE` (a path), which is how Docker secrets are mounted.

use std::net::SocketAddr;

use app::services::AllowedUsers;
use chrono_tz::Tz;
use thiserror::Error;

/// Where configuration values come from; tests pass a map.
pub trait ConfigSource {
    fn var(&self, key: &str) -> Option<String>;
    fn read_file(&self, path: &str) -> std::io::Result<String>;
}

/// The real process environment and filesystem.
#[derive(Debug, Clone, Copy)]
pub struct ProcessEnvironment;

impl ConfigSource for ProcessEnvironment {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|value| !value.is_empty())
    }

    fn read_file(&self, path: &str) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid configuration {key}={value:?}: expected {expected}")]
pub struct ConfigError {
    pub key: &'static str,
    pub value: String,
    pub expected: &'static str,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub http_bind: SocketAddr,
    pub timezone: Tz,
    pub allowed_users: AllowedUsers,
    pub swagger: bool,
    pub log_format: LogFormat,
}

impl AppConfig {
    /// Reads every setting; only malformed values fail here. Commands that
    /// need the database check `database_url` themselves.
    pub fn load(source: &dyn ConfigSource) -> Result<Self, ConfigError> {
        Ok(Self {
            database_url: secret(source, "DATABASE_URL")?,
            http_bind: parse_or(
                source,
                "HTTP_BIND",
                "0.0.0.0:8080",
                "a socket address like 0.0.0.0:8080",
            )?,
            timezone: parse_or(
                source,
                "HOUSEHOLD_TIMEZONE",
                "America/Sao_Paulo",
                "an IANA timezone",
            )?,
            allowed_users: allowed_users(source)?,
            swagger: flag(source, "SWAGGER_ENABLED")?,
            log_format: log_format(source)?,
        })
    }

    pub fn require_database_url(&self) -> Result<&str, ConfigError> {
        let expected = "a postgres:// URL in DATABASE_URL or a file path in DATABASE_URL_FILE";
        self.database_url.as_deref().ok_or(ConfigError {
            key: "DATABASE_URL",
            value: String::new(),
            expected,
        })
    }
}

/// `KEY_FILE` wins over `KEY`; file contents are trimmed.
fn secret(source: &dyn ConfigSource, key: &'static str) -> Result<Option<String>, ConfigError> {
    let file_key = format!("{key}_FILE");
    let Some(path) = source.var(&file_key) else {
        return Ok(source.var(key));
    };
    let contents = source.read_file(&path).map_err(|_| ConfigError {
        key,
        value: path,
        expected: "a readable file",
    })?;
    Ok(Some(contents.trim().to_owned()))
}

fn parse_or<T: std::str::FromStr>(
    source: &dyn ConfigSource,
    key: &'static str,
    default: &str,
    expected: &'static str,
) -> Result<T, ConfigError> {
    let raw = source.var(key).unwrap_or_else(|| default.to_owned());
    raw.parse().map_err(|_| ConfigError { key, value: raw, expected })
}

fn flag(source: &dyn ConfigSource, key: &'static str) -> Result<bool, ConfigError> {
    match source.var(key).as_deref() {
        None | Some("false" | "0") => Ok(false),
        Some("true" | "1") => Ok(true),
        Some(other) => {
            Err(ConfigError { key, value: other.to_owned(), expected: "true, false, 1 or 0" })
        }
    }
}

fn allowed_users(source: &dyn ConfigSource) -> Result<AllowedUsers, ConfigError> {
    let key = "ALLOWED_TELEGRAM_USER_IDS";
    let raw = source.var(key).unwrap_or_default();
    let ids: Result<Vec<i64>, _> =
        raw.split(',').map(str::trim).filter(|part| !part.is_empty()).map(str::parse).collect();
    let expected = "comma-separated Telegram user ids like 123,456";
    ids.map(AllowedUsers::new).map_err(|_| ConfigError { key, value: raw, expected })
}

fn log_format(source: &dyn ConfigSource) -> Result<LogFormat, ConfigError> {
    match source.var("LOG_FORMAT").as_deref() {
        None | Some("json") => Ok(LogFormat::Json),
        Some("pretty") => Ok(LogFormat::Pretty),
        Some(other) => Err(ConfigError {
            key: "LOG_FORMAT",
            value: other.to_owned(),
            expected: "json or pretty",
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Environment and files held in maps.
    #[derive(Default)]
    struct MapConfigSource {
        vars: HashMap<&'static str, &'static str>,
        files: HashMap<&'static str, &'static str>,
    }

    impl ConfigSource for MapConfigSource {
        fn var(&self, key: &str) -> Option<String> {
            self.vars.get(key).map(|value| (*value).to_owned())
        }

        fn read_file(&self, path: &str) -> std::io::Result<String> {
            self.files
                .get(path)
                .map(|value| (*value).to_owned())
                .ok_or_else(|| std::io::ErrorKind::NotFound.into())
        }
    }

    fn source(vars: &[(&'static str, &'static str)]) -> MapConfigSource {
        MapConfigSource { vars: vars.iter().copied().collect(), files: HashMap::new() }
    }

    #[test]
    fn defaults_when_unset() {
        let config = AppConfig::load(&source(&[])).unwrap();
        assert_eq!(config.http_bind.to_string(), "0.0.0.0:8080");
        assert_eq!(config.timezone, chrono_tz::America::Sao_Paulo);
        assert_eq!((config.swagger, config.log_format), (false, LogFormat::Json));
        assert_eq!(config.database_url, None);
        assert!(
            config.require_database_url().unwrap_err().to_string().contains("DATABASE_URL_FILE")
        );
    }

    #[test]
    fn reads_values_and_allowed_users() {
        let vars = [
            ("ALLOWED_TELEGRAM_USER_IDS", " 12, 34 ,"),
            ("SWAGGER_ENABLED", "1"),
            ("LOG_FORMAT", "pretty"),
        ];
        let config = AppConfig::load(&source(&vars)).unwrap();
        assert!(config.allowed_users.contains(12) && config.allowed_users.contains(34));
        assert_eq!((config.swagger, config.log_format), (true, LogFormat::Pretty));
    }

    #[test]
    fn file_secret_wins_and_is_trimmed() {
        let mut map =
            source(&[("DATABASE_URL", "postgres://env"), ("DATABASE_URL_FILE", "/run/secrets/db")]);
        map.files.insert("/run/secrets/db", "postgres://file\n");
        let config = AppConfig::load(&map).unwrap();
        assert_eq!(config.require_database_url().unwrap(), "postgres://file");
    }

    #[test]
    fn malformed_values_name_key_and_expectation() {
        let cases = [
            ("HTTP_BIND", "localhost"),
            ("HOUSEHOLD_TIMEZONE", "Mars/Olympus"),
            ("ALLOWED_TELEGRAM_USER_IDS", "12,abc"),
            ("SWAGGER_ENABLED", "yes"),
            ("LOG_FORMAT", "xml"),
            ("DATABASE_URL_FILE", "/missing"),
        ];
        for (key, value) in cases {
            let error = AppConfig::load(&source(&[(key, value)])).unwrap_err();
            assert_eq!(
                (error.key, error.value.as_str()),
                (key.trim_end_matches("_FILE"), value),
                "{error}"
            );
        }
    }
}
