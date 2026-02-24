//! RTF Tokenizer Module
//!
//! This module provides tokenization of RTF input using nom parsers.
//! It converts raw RTF text into a sequence of tokens for further processing.

use crate::error::ParseError;
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{take, take_while1},
    character::complete::{anychar, char, digit1},
    combinator::{map, opt, recognize, verify},
    sequence::{preceded, tuple},
};

// =============================================================================
// Token Types
// =============================================================================

/// A token representing a parsed RTF element.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Start of a group `{`
    GroupStart,
    /// End of a group `}`
    GroupEnd,
    /// A control word with optional parameter
    ControlWord {
        word: String,
        parameter: Option<i32>,
    },
    /// Text content
    Text(String),
    /// A control symbol (like `\*`, `\'`, etc.)
    ControlSymbol(char),
}

// =============================================================================
// Tokenization
// =============================================================================

/// Tokenizes RTF input into a vector of tokens.
pub fn tokenize(input: &str) -> Result<Vec<Token>, nom::Err<nom::error::Error<&str>>> {
    let mut tokens = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        // Skip source formatting whitespace. Spaces are meaningful and preserved.
        remaining = skip_ignorable_whitespace(remaining);

        // If only whitespace remained, we're done
        if remaining.is_empty() {
            break;
        }

        match parse_token(remaining) {
            Ok((new_remaining, token)) => {
                tokens.push(token);
                remaining = new_remaining;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(tokens)
}

/// Validate token sequence for structural correctness.
pub fn validate_tokens(tokens: &[Token]) -> Result<(), ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut depth = 0usize;
    for token in tokens {
        match token {
            Token::GroupStart => depth += 1,
            Token::GroupEnd => {
                if depth == 0 {
                    return Err(ParseError::UnmatchedGroupEnd);
                }
                depth -= 1;
            }
            Token::ControlWord { .. } | Token::Text(_) | Token::ControlSymbol(_) => {}
        }
    }
    if depth != 0 {
        return Err(ParseError::UnbalancedGroups);
    }

    // Basic format guard: RTF should begin with {\rtf...}
    let mut iter = tokens
        .iter()
        .filter(|t| !matches!(t, Token::Text(text) if text.trim().is_empty()));
    match (iter.next(), iter.next()) {
        (Some(Token::GroupStart), Some(Token::ControlWord { word, .. })) if word == "rtf" => Ok(()),
        _ => Err(ParseError::MissingRtfHeader),
    }
}

// =============================================================================
// Internal Parsing Functions
// =============================================================================

/// Decode a Windows-1252 codepoint to a Unicode character.
/// Windows-1252 is the default encoding for RTF documents with \ansi.
fn decode_windows1252(codepoint: u8) -> char {
    // Windows-1252 has some characters in the 0x80-0x9F range that differ from ISO-8859-1
    // See: https://en.wikipedia.org/wiki/Windows-1252
    match codepoint {
        0x80 => '\u{20AC}', // Euro sign
        0x82 => '\u{201A}', // Single low-9 quotation mark
        0x83 => '\u{0192}', // Latin small letter f with hook
        0x84 => '\u{201E}', // Double low-9 quotation mark
        0x85 => '\u{2026}', // Horizontal ellipsis
        0x86 => '\u{2020}', // Dagger
        0x87 => '\u{2021}', // Double dagger
        0x88 => '\u{02C6}', // Modifier letter circumflex accent
        0x89 => '\u{2030}', // Per mille sign
        0x8A => '\u{0160}', // Latin capital letter S with caron
        0x8B => '\u{2039}', // Single left-pointing angle quotation mark
        0x8C => '\u{0152}', // Latin capital ligature OE
        0x8E => '\u{017D}', // Latin capital letter Z with caron
        0x91 => '\u{2018}', // Left single quotation mark
        0x92 => '\u{2019}', // Right single quotation mark
        0x93 => '\u{201C}', // Left double quotation mark
        0x94 => '\u{201D}', // Right double quotation mark
        0x95 => '\u{2022}', // Bullet
        0x96 => '\u{2013}', // En dash
        0x97 => '\u{2014}', // Em dash
        0x98 => '\u{02DC}', // Small tilde
        0x99 => '\u{2122}', // Trade mark sign
        0x9A => '\u{0161}', // Latin small letter s with caron
        0x9B => '\u{203A}', // Single right-pointing angle quotation mark
        0x9C => '\u{0153}', // Latin small ligature oe
        0x9E => '\u{017E}', // Latin small letter z with caron
        0x9F => '\u{0178}', // Latin capital letter Y with diaeresis
        // For all other values (0x00-0x7F and 0xA0-0xFF), they match ISO-8859-1/Unicode
        byte => byte as char,
    }
}

/// Parse a single token from the input.
fn parse_token(input: &str) -> IResult<&str, Token> {
    alt((
        // Group start
        map(char('{'), |_| Token::GroupStart),
        // Group end
        map(char('}'), |_| Token::GroupEnd),
        // Control word or symbol
        preceded(
            char('\\'),
            alt((
                // Hex escape: \'hh (exactly two hex digits)
                map(
                    preceded(
                        char('\''),
                        verify(take(2usize), |hex: &&str| {
                            hex.chars().all(|c| c.is_ascii_hexdigit())
                        }),
                    ),
                    |hex: &str| {
                        if let Ok(byte) = u8::from_str_radix(hex, 16) {
                            Token::Text(decode_windows1252(byte).to_string())
                        } else {
                            Token::Text(String::new())
                        }
                    },
                ),
                // Control symbol (single non-letter character)
                map(
                    verify(anychar, |c| !c.is_ascii_alphabetic()),
                    Token::ControlSymbol,
                ),
                // Control word with optional parameter
                map(
                    tuple((
                        // Word: letters only
                        take_while1(|c: char| c.is_ascii_alphabetic()),
                        // Optional parameter: digits, possibly negative
                        opt(recognize(tuple((opt(char('-')), digit1)))),
                        // An optional single space delimiter is consumed by the RTF grammar.
                        opt(char(' ')),
                    )),
                    |(word, param, _): (&str, Option<&str>, Option<char>)| {
                        let parameter = param.and_then(|p| {
                            if p.is_empty() || p == "-" {
                                None
                            } else {
                                p.parse::<i32>().ok()
                            }
                        });
                        Token::ControlWord {
                            word: word.to_string(),
                            parameter,
                        }
                    },
                ),
            )),
        ),
        // Text content (until special character)
        map(parse_text, Token::Text),
    ))(input)
}

/// Parse text content until a special character.
fn parse_text(input: &str) -> IResult<&str, String> {
    let (remaining, text) =
        take_while1(|c: char| c != '\\' && c != '{' && c != '}' && !c.is_control())(input)?;

    // Decode RTF special characters in the text
    let decoded = decode_text(text);

    Ok((remaining, decoded))
}

/// Decode RTF special characters in text.
fn decode_text(text: &str) -> String {
    let mut result = String::new();

    for c in text.chars() {
        result.push(c);
    }

    result
}

/// Skip ignorable source formatting whitespace.
fn skip_ignorable_whitespace(input: &str) -> &str {
    let mut remaining = input;
    while let Some(c) = remaining.chars().next() {
        if c == '\n' || c == '\r' || c == '\t' {
            remaining = &remaining[c.len_utf8()..];
        } else {
            break;
        }
    }
    remaining
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let input = r#"{\rtf1 Hello}"#;
        let tokens = tokenize(input).unwrap();
        assert!(tokens.contains(&Token::GroupStart));
        assert!(tokens.contains(&Token::GroupEnd));
    }

    #[test]
    fn test_tokenize_control_word() {
        let input = r#"\b"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(
            tokens,
            vec![Token::ControlWord {
                word: "b".to_string(),
                parameter: None
            }]
        );
    }

    #[test]
    fn test_tokenize_control_word_with_param() {
        let input = r#"\b1"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(
            tokens,
            vec![Token::ControlWord {
                word: "b".to_string(),
                parameter: Some(1)
            }]
        );
    }

    #[test]
    fn test_tokenize_control_word_with_negative_param() {
        let input = r#"\fs-24"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(
            tokens,
            vec![Token::ControlWord {
                word: "fs".to_string(),
                parameter: Some(-24)
            }]
        );
    }

    #[test]
    fn test_tokenize_hex_escape() {
        let input = r#"{\rtf1 \'e9}"#; // é in Windows-1252
        let tokens = tokenize(input).unwrap();
        assert!(tokens.contains(&Token::Text("é".to_string())));
    }

    #[test]
    fn test_tokenize_control_symbol() {
        let input = r#"\*"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens, vec![Token::ControlSymbol('*')]);
    }

    #[test]
    fn test_tokenize_escaped_braces() {
        let input = r#"\{"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens, vec![Token::ControlSymbol('{')]);
    }

    #[test]
    fn test_validate_tokens_valid() {
        let tokens = vec![
            Token::GroupStart,
            Token::ControlWord {
                word: "rtf".to_string(),
                parameter: Some(1),
            },
            Token::GroupEnd,
        ];
        assert!(validate_tokens(&tokens).is_ok());
    }

    #[test]
    fn test_validate_tokens_empty() {
        let tokens: Vec<Token> = vec![];
        assert!(matches!(
            validate_tokens(&tokens),
            Err(ParseError::EmptyInput)
        ));
    }

    #[test]
    fn test_validate_tokens_unbalanced() {
        let tokens = vec![
            Token::GroupStart,
            Token::ControlWord {
                word: "rtf".to_string(),
                parameter: Some(1),
            },
            // Missing GroupEnd
        ];
        assert!(matches!(
            validate_tokens(&tokens),
            Err(ParseError::UnbalancedGroups)
        ));
    }

    #[test]
    fn test_validate_tokens_unmatched_end() {
        let tokens = vec![
            Token::GroupEnd, // Unmatched
        ];
        assert!(matches!(
            validate_tokens(&tokens),
            Err(ParseError::UnmatchedGroupEnd)
        ));
    }

    #[test]
    fn test_validate_tokens_missing_header() {
        let tokens = vec![
            Token::GroupStart,
            Token::Text("Hello".to_string()),
            Token::GroupEnd,
        ];
        assert!(matches!(
            validate_tokens(&tokens),
            Err(ParseError::MissingRtfHeader)
        ));
    }

    #[test]
    fn test_decode_windows1252() {
        // Euro sign
        assert_eq!(decode_windows1252(0x80), '\u{20AC}');
        // En dash
        assert_eq!(decode_windows1252(0x96), '\u{2013}');
        // Regular ASCII
        assert_eq!(decode_windows1252(0x41), 'A');
        // Regular byte in upper range
        assert_eq!(decode_windows1252(0xE9), 'é');
    }
}
