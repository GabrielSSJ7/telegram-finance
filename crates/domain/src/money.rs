//! Money as integer cents. Floats never touch an amount.

use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// Amount in BRL cents. Signed so balances can go negative.
///
/// ```
/// use domain::Cents;
/// assert_eq!(Cents::new(1050) + Cents::new(50), Cents::new(1100));
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cents(i64);

impl Cents {
    pub const ZERO: Cents = Cents(0);

    pub const fn new(cents: i64) -> Self {
        Self(cents)
    }

    pub const fn value(self) -> i64 {
        self.0
    }

    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    pub const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Multiplies by a small signed factor, used to apply +1/-1/0 effects.
    pub const fn times(self, factor: i64) -> Self {
        Self(self.0 * factor)
    }
}

impl Add for Cents {
    type Output = Cents;
    fn add(self, rhs: Cents) -> Cents {
        Cents(self.0 + rhs.0)
    }
}

impl AddAssign for Cents {
    fn add_assign(&mut self, rhs: Cents) {
        self.0 += rhs.0;
    }
}

impl Sub for Cents {
    type Output = Cents;
    fn sub(self, rhs: Cents) -> Cents {
        Cents(self.0 - rhs.0)
    }
}

impl SubAssign for Cents {
    fn sub_assign(&mut self, rhs: Cents) {
        self.0 -= rhs.0;
    }
}

impl Neg for Cents {
    type Output = Cents;
    fn neg(self) -> Cents {
        Cents(-self.0)
    }
}

impl Sum for Cents {
    fn sum<I: Iterator<Item = Cents>>(iter: I) -> Cents {
        iter.fold(Cents::ZERO, Add::add)
    }
}

impl<'a> Sum<&'a Cents> for Cents {
    fn sum<I: Iterator<Item = &'a Cents>>(iter: I) -> Cents {
        iter.copied().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_keeps_integer_cents() {
        let mut total = Cents::new(1000) - Cents::new(1);
        total += Cents::new(2);
        total -= Cents::new(1);
        assert_eq!(total, Cents::new(1000));
        assert_eq!(-total, Cents::new(-1000));
    }

    #[test]
    fn sum_over_owned_and_borrowed() {
        let items = [Cents::new(1), Cents::new(2), Cents::new(3)];
        assert_eq!(items.iter().sum::<Cents>(), Cents::new(6));
        assert_eq!(items.into_iter().sum::<Cents>(), Cents::new(6));
    }

    #[test]
    fn helpers() {
        assert!(Cents::new(1).is_positive());
        assert!(!Cents::ZERO.is_positive());
        assert_eq!(Cents::new(-5).abs(), Cents::new(5));
        assert_eq!(Cents::new(7).times(-1), Cents::new(-7));
    }
}
