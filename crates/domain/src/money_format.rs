//! pt-BR display of amounts: `R$ 1.234,56`.

use crate::Cents;

/// Formats cents as Brazilian currency.
///
/// ```
/// use domain::{Cents, money_format::format_brl};
/// assert_eq!(format_brl(Cents::new(-123_456)), "-R$ 1.234,56");
/// ```
pub fn format_brl(amount: Cents) -> String {
    let sign = if amount.value() < 0 { "-" } else { "" };
    let magnitude = amount.value().unsigned_abs();
    let reais = group_thousands(magnitude / 100);
    format!("{sign}R$ {reais},{:02}", magnitude % 100)
}

/// Decimal with comma and no currency symbol, for CSV opened in pt-BR Excel.
///
/// ```
/// use domain::{Cents, money_format::format_decimal_comma};
/// assert_eq!(format_decimal_comma(Cents::new(-5)), "-0,05");
/// ```
pub fn format_decimal_comma(amount: Cents) -> String {
    let sign = if amount.value() < 0 { "-" } else { "" };
    let magnitude = amount.value().unsigned_abs();
    format!("{sign}{},{:02}", magnitude / 100, magnitude % 100)
}

fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRL_CASES: &[(i64, &str)] = &[
        (0, "R$ 0,00"),
        (5, "R$ 0,05"),
        (1050, "R$ 10,50"),
        (100_000, "R$ 1.000,00"),
        (123_456_789, "R$ 1.234.567,89"),
        (-1000, "-R$ 10,00"),
    ];

    #[test]
    fn formats_brl() {
        for (cents, expected) in BRL_CASES {
            assert_eq!(format_brl(Cents::new(*cents)), *expected);
        }
    }

    #[test]
    fn formats_decimal_comma() {
        assert_eq!(format_decimal_comma(Cents::new(123_456)), "1234,56");
        assert_eq!(format_decimal_comma(Cents::new(-7)), "-0,07");
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(group_thousands(999), "999");
        assert_eq!(group_thousands(1000), "1.000");
        assert_eq!(group_thousands(1_000_000), "1.000.000");
    }
}
