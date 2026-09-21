/// A configuration secret. `Debug` never shows the value, so logging the
/// configuration cannot leak the database password or the bot token.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Secret(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted_but_value_is_available() {
        let secret = Secret::new("postgres://finbot:hunter2@db/finbot");
        assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
        assert!(secret.expose().contains("hunter2"));
    }
}
