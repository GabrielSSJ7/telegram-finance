//! Shared text validation for names and descriptions typed by people.

use crate::{AppError, AppResult};

pub const MAX_DESCRIPTION_CHARS: usize = 120;

/// Trims `raw` and requires 1..=`max_chars` characters.
///
/// ```
/// use app::services::text_rules::clean_name;
/// assert_eq!(clean_name("account name", "  Nubank ", 40).unwrap(), "Nubank");
/// ```
pub fn clean_name(field: &'static str, raw: &str, max_chars: usize) -> AppResult<String> {
    let trimmed = raw.trim();
    let length = trimmed.chars().count();
    if length == 0 || length > max_chars {
        return Err(AppError::invalid(field, raw, format!("1 to {max_chars} characters")));
    }
    Ok(trimmed.to_owned())
}

/// Trims `raw`; empty is allowed, longer than 120 characters is not.
pub fn clean_description(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > MAX_DESCRIPTION_CHARS {
        let expected = format!("at most {MAX_DESCRIPTION_CHARS} characters");
        return Err(AppError::invalid("description", raw, expected));
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_rejects_blank_and_too_long() {
        assert!(clean_name("account name", "   ", 40).is_err());
        assert!(clean_name("account name", &"x".repeat(41), 40).is_err());
        assert_eq!(clean_name("account name", &"é".repeat(40), 40).unwrap().chars().count(), 40);
    }

    #[test]
    fn description_allows_empty_but_caps_length() {
        assert_eq!(clean_description("  ").unwrap(), "");
        assert_eq!(clean_description(" mercado ").unwrap(), "mercado");
        let error = clean_description(&"a".repeat(121)).unwrap_err().to_string();
        assert!(error.contains("at most 120"), "{error}");
    }
}
