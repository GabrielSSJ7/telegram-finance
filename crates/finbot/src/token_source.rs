use app::ports::{TokenSource, TokenSourceError};

/// Operating-system randomness (getrandom).
#[derive(Debug, Clone, Copy, Default)]
pub struct OsTokenSource;

impl TokenSource for OsTokenSource {
    fn secret_bytes(&self) -> Result<[u8; 32], TokenSourceError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| TokenSourceError(error.to_string()))?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_distinct_nonzero_tokens() {
        let (first, second) =
            (OsTokenSource.secret_bytes().unwrap(), OsTokenSource.secret_bytes().unwrap());
        assert_ne!(first, second);
        assert_ne!(first, [0u8; 32]);
    }
}
