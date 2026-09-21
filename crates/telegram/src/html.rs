//! Escaping for Telegram's HTML parse mode. Every piece of user-typed text
//! (descriptions, names) goes through [`escape`] before reaching a message.

/// Escapes `&`, `<` and `>`, the characters Telegram's HTML mode parses.
///
/// ```
/// assert_eq!(telegram::html::escape("a<b & c>"), "a&lt;b &amp; c&gt;");
/// ```
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_plain_text_alone() {
        assert_eq!(escape("mercado 🛒 ção"), "mercado 🛒 ção");
    }

    #[test]
    fn escapes_tags_and_ampersands() {
        assert_eq!(escape("<b>R&D</b>"), "&lt;b&gt;R&amp;D&lt;/b&gt;");
    }
}
