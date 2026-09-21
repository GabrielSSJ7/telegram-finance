use std::sync::atomic::{AtomicU8, Ordering};

use crate::ports::{TokenSource, TokenSourceError};

/// Predictable "random" bytes: the n-th call returns 32 bytes equal to n.
#[derive(Debug, Default)]
pub struct SequentialTokenSource {
    calls: AtomicU8,
}

impl TokenSource for SequentialTokenSource {
    fn secret_bytes(&self) -> Result<[u8; 32], TokenSourceError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
        Ok([call; 32])
    }
}
