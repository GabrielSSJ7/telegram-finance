//! Progress of a savings goal ("meta").

use crate::Cents;

/// `saved / target` in basis points (100% = 10000), clamped to 0..=10000.
///
/// ```
/// use domain::{Cents, goal_progress::progress_bp};
/// assert_eq!(progress_bp(Cents::new(2_000_000), Cents::new(10_000_000)), 2000);
/// ```
pub fn progress_bp(saved: Cents, target: Cents) -> i64 {
    if !target.is_positive() {
        return 0;
    }
    (saved.value().max(0).saturating_mul(10_000) / target.value()).min(10_000)
}

/// What is still missing to reach the target, never negative.
pub fn remaining(saved: Cents, target: Cents) -> Cents {
    Cents::new((target.value() - saved.value()).max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_clamped() {
        assert_eq!(progress_bp(Cents::new(-5), Cents::new(100)), 0);
        assert_eq!(progress_bp(Cents::new(50), Cents::new(100)), 5000);
        assert_eq!(progress_bp(Cents::new(500), Cents::new(100)), 10_000);
        assert_eq!(progress_bp(Cents::new(500), Cents::ZERO), 0);
    }

    #[test]
    fn remaining_never_negative() {
        assert_eq!(remaining(Cents::new(30), Cents::new(100)), Cents::new(70));
        assert_eq!(remaining(Cents::new(300), Cents::new(100)), Cents::ZERO);
    }
}
