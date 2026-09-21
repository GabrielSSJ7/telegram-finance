//! Structured logs: JSON lines in production, readable text in development.

use tracing_subscriber::EnvFilter;

use crate::config::LogFormat;

const DEFAULT_FILTER: &str = "info,sqlx=warn,tower_http=info";

/// Installs the global subscriber; a second call is a no-op.
pub fn init(format: LogFormat) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    let installed = match format {
        LogFormat::Json => builder.json().flatten_event(true).try_init(),
        LogFormat::Pretty => builder.try_init(),
    };
    if installed.is_err() {
        tracing::debug!("log subscriber already installed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_twice_does_not_panic() {
        init(LogFormat::Json);
        init(LogFormat::Pretty);
    }
}
