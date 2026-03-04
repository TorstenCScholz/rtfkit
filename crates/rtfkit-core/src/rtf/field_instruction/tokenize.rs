//! Field instruction tokenizer.
//!
//! Splits a raw `\fldinst` string into tokens for further parsing.
//! Handles:
//! - Quoted strings with backslash escape sequences.
//! - Unquoted words.
//! - Switch tokens (starting with `\`).

/// Tokenize a field instruction string into individual tokens.
///
/// Quoted strings are unquoted (surrounding `"` removed, `\"` → `"`).
/// Backslash-prefixed tokens (switches like `\h`) are preserved as-is.
pub fn tokenize_field_words(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '"' {
            chars.next();
            let mut value = String::new();
            let mut escaped = false;
            for next in chars.by_ref() {
                if escaped {
                    value.push(next);
                    escaped = false;
                    continue;
                }
                if next == '\\' {
                    escaped = true;
                    continue;
                }
                if next == '"' {
                    break;
                }
                value.push(next);
            }
            tokens.push(value);
            continue;
        }

        let mut token = String::new();
        while let Some(next) = chars.peek().copied() {
            if next.is_whitespace() {
                break;
            }
            token.push(next);
            chars.next();
        }
        if !token.is_empty() {
            tokens.push(token);
        }
    }

    tokens
}

/// A token from a HYPERLINK field instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperlinkToken {
    /// A switch like `\l`, `\o`, `\t`, etc.
    Switch(String),
    /// A quoted or unquoted value.
    Value(String),
}

impl HyperlinkToken {
    /// Return the value string if this is a `Value` token.
    pub fn as_value(&self) -> Option<&str> {
        match self {
            HyperlinkToken::Value(v) => Some(v.as_str()),
            HyperlinkToken::Switch(_) => None,
        }
    }
}

/// Tokenize a HYPERLINK instruction (the part after the `HYPERLINK` keyword).
///
/// Switches (`\l`, `\o`, etc.) are distinguished from values.
pub fn tokenize_hyperlink_instruction(input: &str) -> Vec<HyperlinkToken> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '\\' {
            chars.next();
            let mut switch_name = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_alphabetic() {
                    switch_name.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if !switch_name.is_empty() {
                tokens.push(HyperlinkToken::Switch(switch_name));
                continue;
            }
        }

        if ch == '"' {
            chars.next();
            let mut value = String::new();
            let mut escaped = false;
            for next in chars.by_ref() {
                if escaped {
                    value.push(next);
                    escaped = false;
                    continue;
                }
                if next == '\\' {
                    escaped = true;
                    continue;
                }
                if next == '"' {
                    break;
                }
                value.push(next);
            }
            tokens.push(HyperlinkToken::Value(value));
            continue;
        }

        let mut value = String::new();
        while let Some(next) = chars.peek().copied() {
            if next.is_whitespace() {
                break;
            }
            value.push(next);
            chars.next();
        }
        if !value.is_empty() {
            tokens.push(HyperlinkToken::Value(value));
        }
    }

    tokens
}
