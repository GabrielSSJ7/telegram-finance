//! Monthly budgets per category and when to warn about them.

use crate::Cents;

/// Warn at 80% of the limit and again when it is reached.
pub const ALERT_THRESHOLDS: [u8; 2] = [80, 100];

/// `spent / limit` in basis points (100% = 10000); 0 for a zero limit.
///
/// ```
/// use domain::{Cents, budget::used_bp};
/// assert_eq!(used_bp(Cents::new(850), Cents::new(1000)), 8500);
/// ```
pub fn used_bp(spent: Cents, limit: Cents) -> i64 {
    if !limit.is_positive() {
        return 0;
    }
    spent.value().max(0).saturating_mul(10_000) / limit.value()
}

/// Thresholds already reached by `spent`, lowest first.
pub fn thresholds_reached(spent: Cents, limit: Cents) -> Vec<u8> {
    let used = used_bp(spent, limit);
    ALERT_THRESHOLDS.into_iter().filter(|threshold| used >= i64::from(*threshold) * 100).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_follow_spending() {
        let limit = Cents::new(1000);
        assert!(thresholds_reached(Cents::new(799), limit).is_empty());
        assert_eq!(thresholds_reached(Cents::new(800), limit), vec![80]);
        assert_eq!(thresholds_reached(Cents::new(1000), limit), vec![80, 100]);
        assert_eq!(thresholds_reached(Cents::new(5000), limit), vec![80, 100]);
    }

    #[test]
    fn zero_limit_and_refunds_are_safe() {
        assert_eq!(used_bp(Cents::new(10), Cents::ZERO), 0);
        assert_eq!(used_bp(Cents::new(-10), Cents::new(100)), 0);
        assert!(thresholds_reached(Cents::new(10), Cents::ZERO).is_empty());
    }
}
