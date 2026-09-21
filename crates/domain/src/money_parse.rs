//! Parses amounts the way Brazilians type them in chat.
//!
//! Comma is the decimal separator and dot groups thousands, but people also
//! type `12.50` on phones with a US keyboard. Rule: a single dot followed by
//! one or two digits is a decimal point; any other dot groups thousands.

use thiserror::Error;

use crate::Cents;

/// Largest amount accepted from a human: R$ 100.000.000,00.
pub const MAX_TYPED_AMOUNT: Cents = Cents::new(10_000_000_000);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "invalid amount {input:?}: {reason}; expected a positive value like 10, 10,50, 1.234,56 or R$ 10"
)]
pub struct MoneyParseError {
    pub input: String,
    pub reason: &'static str,
}

/// Parses a typed BRL amount into cents. Only positive amounts are valid.
///
/// ```
/// use domain::{Cents, money_parse::parse_brl};
/// assert_eq!(parse_brl("R$ 1.234,56"), Ok(Cents::new(123_456)));
/// ```
pub fn parse_brl(input: &str) -> Result<Cents, MoneyParseError> {
    let fail = |reason: &'static str| MoneyParseError { input: input.to_owned(), reason };
    let body = strip_currency(input);
    if body.is_empty() {
        return Err(fail("no digits"));
    }
    let (integer, fraction) = split_decimal(body).ok_or_else(|| fail("malformed decimal part"))?;
    let reais = parse_integer_part(integer).ok_or_else(|| fail("malformed integer part"))?;
    let cents = parse_fraction(fraction).ok_or_else(|| fail("malformed decimal part"))?;
    let total = reais.checked_mul(100).and_then(|r| r.checked_add(cents));
    let total = Cents::new(total.ok_or_else(|| fail("too large"))?);
    validate_range(total).map_err(fail)
}

fn strip_currency(input: &str) -> &str {
    let trimmed = input.trim();
    let without =
        trimmed.strip_prefix("R$").or_else(|| trimmed.strip_prefix("r$")).unwrap_or(trimmed);
    without.trim()
}

/// Splits `body` into (integer part, decimal digits). `None` when the
/// decimal part is present but not 1-2 digits.
fn split_decimal(body: &str) -> Option<(&str, &str)> {
    if let Some(pos) = body.rfind(',') {
        let fraction = &body[pos + 1..];
        return is_short_fraction(fraction).then_some((&body[..pos], fraction));
    }
    let single_dot = body.matches('.').count() == 1;
    match body.split_once('.') {
        Some((integer, fraction)) if single_dot && is_short_fraction(fraction) => {
            Some((integer, fraction))
        }
        _ => Some((body, "")),
    }
}

fn is_short_fraction(fraction: &str) -> bool {
    (1..=2).contains(&fraction.len()) && all_digits(fraction)
}

/// Accepts `1234`, `1.234`, `1.234.567` and the empty string (for `,50`).
fn parse_integer_part(integer: &str) -> Option<i64> {
    if integer.is_empty() {
        return Some(0);
    }
    let mut groups = integer.split('.');
    let head = groups.next()?;
    let grouped = integer.contains('.');
    let head_ok = all_digits(head) && !head.is_empty() && (!grouped || head.len() <= 3);
    let tail_ok = groups.all(|group| group.len() == 3 && all_digits(group));
    if !(head_ok && tail_ok) {
        return None;
    }
    integer.replace('.', "").parse().ok()
}

fn parse_fraction(fraction: &str) -> Option<i64> {
    match fraction.len() {
        0 => Some(0),
        1 => fraction.parse::<i64>().ok().map(|tenths| tenths * 10),
        _ => fraction.parse().ok(),
    }
}

fn all_digits(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_range(total: Cents) -> Result<Cents, &'static str> {
    if !total.is_positive() {
        return Err("must be greater than zero");
    }
    if total > MAX_TYPED_AMOUNT {
        return Err("above R$ 100.000.000,00");
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &[(&str, i64)] = &[
        ("10", 1000),
        ("10,50", 1050),
        ("10,5", 1050),
        ("1.234,56", 123_456),
        ("1234,56", 123_456),
        ("R$ 10", 1000),
        ("r$10,00", 1000),
        ("12.50", 1250),
        ("12.5", 1250),
        ("1.234", 123_400),
        ("1.234.567", 123_456_700),
        ("0,01", 1),
        (",99", 99),
        ("  7 ", 700),
        ("100000000", 10_000_000_000),
    ];

    const INVALID: &[&str] = &[
        "",
        "R$",
        "abc",
        "10,505",
        "1.23.4",
        "-10",
        "0",
        "0,00",
        "10,",
        "1,234,56",
        "12.345.6",
        "10.",
        "1.2345",
        "1234.567.890",
        "10 50",
        "100000000,01",
        "999999999999999999999",
    ];

    #[test]
    fn parses_typed_amounts() {
        for (input, expected) in VALID {
            assert_eq!(parse_brl(input), Ok(Cents::new(*expected)), "input {input:?}");
        }
    }

    #[test]
    fn rejects_malformed_amounts() {
        for input in INVALID {
            assert!(parse_brl(input).is_err(), "input {input:?} should fail");
        }
    }

    #[test]
    fn error_names_input_and_expected_shape() {
        let message = parse_brl("abc").unwrap_err().to_string();
        assert!(message.contains("\"abc\""), "{message}");
        assert!(message.contains("1.234,56"), "{message}");
    }
}
