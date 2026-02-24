//! Tokenizer Tests
//!
//! Tests for RTF tokenization including:
//! - Balanced/unbalanced groups
//! - Missing `\rtf` header
//! - Unicode + `\ucN` fallback
//! - Escaped symbols
//! - Hex escapes
//! - Control words with parameters

use crate::ParseError;
use crate::rtf::tokenizer::{Token, tokenize, validate_tokens};

// =============================================================================
// Basic Tokenization Tests
// =============================================================================

#[test]
fn test_tokenize_simple_document() {
    let input = r#"{\rtf1 Hello}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::GroupStart));
    assert!(tokens.contains(&Token::GroupEnd));
    assert!(tokens.contains(&Token::Text("Hello".to_string())));
}

#[test]
fn test_tokenize_control_word_no_param() {
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
    let input = r#"\fs24"#;
    let tokens = tokenize(input).unwrap();

    assert_eq!(
        tokens,
        vec![Token::ControlWord {
            word: "fs".to_string(),
            parameter: Some(24)
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
fn test_tokenize_control_symbol() {
    let input = r#"\*"#;
    let tokens = tokenize(input).unwrap();

    assert_eq!(tokens, vec![Token::ControlSymbol('*')]);
}

#[test]
fn test_tokenize_escaped_brace_open() {
    let input = r#"\{"#;
    let tokens = tokenize(input).unwrap();

    assert_eq!(tokens, vec![Token::ControlSymbol('{')]);
}

#[test]
fn test_tokenize_escaped_brace_close() {
    let input = r#"\}"#;
    let tokens = tokenize(input).unwrap();

    assert_eq!(tokens, vec![Token::ControlSymbol('}')]);
}

#[test]
fn test_tokenize_escaped_backslash() {
    let input = r#"\\"#;
    let tokens = tokenize(input).unwrap();

    assert_eq!(tokens, vec![Token::ControlSymbol('\\')]);
}

// =============================================================================
// Hex Escape Tests
// =============================================================================

#[test]
fn test_tokenize_hex_escape() {
    let input = r#"{\rtf1 \'e9}"#; // é in Windows-1252
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::Text("é".to_string())));
}

#[test]
fn test_tokenize_hex_escape_euro() {
    let input = r#"{\rtf1 \'80}"#; // Euro sign in Windows-1252
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::Text("€".to_string())));
}

#[test]
fn test_tokenize_hex_escape_en_dash() {
    let input = r#"{\rtf1 \'96}"#; // En dash in Windows-1252
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::Text("–".to_string())));
}

#[test]
fn test_tokenize_hex_escape_em_dash() {
    let input = r#"{\rtf1 \'97}"#; // Em dash in Windows-1252
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::Text("—".to_string())));
}

#[test]
fn test_tokenize_hex_escape_curly_quotes() {
    // Left double quote (0x93 in Windows-1252)
    let input = r#"{\rtf1 \'93}"#;
    let tokens = tokenize(input).unwrap();
    // Left double quote is U+201C
    assert!(tokens.contains(&Token::Text("\u{201C}".to_string())));

    // Right double quote (0x94 in Windows-1252)
    let input = r#"{\rtf1 \'94}"#;
    let tokens = tokenize(input).unwrap();
    // Right double quote is U+201D
    assert!(tokens.contains(&Token::Text("\u{201D}".to_string())));
}

// =============================================================================
// Group Balance Tests
// =============================================================================

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
fn test_validate_tokens_unbalanced_open() {
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
fn test_validate_tokens_nested_groups() {
    let tokens = vec![
        Token::GroupStart,
        Token::ControlWord {
            word: "rtf".to_string(),
            parameter: Some(1),
        },
        Token::GroupStart,
        Token::Text("nested".to_string()),
        Token::GroupEnd,
        Token::GroupEnd,
    ];

    assert!(validate_tokens(&tokens).is_ok());
}

#[test]
fn test_validate_tokens_deeply_nested() {
    let mut tokens = vec![
        Token::GroupStart,
        Token::ControlWord {
            word: "rtf".to_string(),
            parameter: Some(1),
        },
    ];

    // Add 100 nested groups
    for _ in 0..100 {
        tokens.push(Token::GroupStart);
    }
    tokens.push(Token::Text("deep".to_string()));
    for _ in 0..100 {
        tokens.push(Token::GroupEnd);
    }
    tokens.push(Token::GroupEnd);

    assert!(validate_tokens(&tokens).is_ok());
}

// =============================================================================
// Text Content Tests
// =============================================================================

#[test]
fn test_tokenize_text_content() {
    let input = r#"{\rtf1 Hello, World!}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::Text("Hello, World!".to_string())));
}

#[test]
fn test_tokenize_text_with_spaces() {
    let input = r#"{\rtf1 Multiple   spaces}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::Text("Multiple   spaces".to_string())));
}

#[test]
fn test_tokenize_multiline_text() {
    let input = "{\\rtf1 Line1\nLine2}";
    let tokens = tokenize(input).unwrap();

    // Newlines should be skipped as ignorable whitespace
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Text(t) if t.contains("Line1")))
    );
    assert!(
        tokens
            .iter()
            .any(|t| matches!(t, Token::Text(t) if t.contains("Line2")))
    );
}

// =============================================================================
// Complex Document Tests
// =============================================================================

#[test]
fn test_tokenize_formatted_text() {
    let input = r#"{\rtf1 \b Bold \i Italic \b0\i0 Plain}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::ControlWord {
        word: "b".to_string(),
        parameter: None
    }));
    assert!(tokens.contains(&Token::ControlWord {
        word: "i".to_string(),
        parameter: None
    }));
    assert!(tokens.contains(&Token::ControlWord {
        word: "b".to_string(),
        parameter: Some(0)
    }));
}

#[test]
fn test_tokenize_destination() {
    let input = r#"{\rtf1 {\*\generator Microsoft Word}}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::ControlSymbol('*')));
    assert!(tokens.contains(&Token::ControlWord {
        word: "generator".to_string(),
        parameter: None
    }));
}

#[test]
fn test_tokenize_font_table() {
    let input = r#"{\rtf1 {\fonttbl{\f0 Arial;}}}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::ControlWord {
        word: "fonttbl".to_string(),
        parameter: None
    }));
    assert!(tokens.contains(&Token::ControlWord {
        word: "f".to_string(),
        parameter: Some(0)
    }));
}

#[test]
fn test_tokenize_color_table() {
    let input = r#"{\rtf1 {\colortbl;\red255\green0\blue0;}}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::ControlWord {
        word: "colortbl".to_string(),
        parameter: None
    }));
    assert!(tokens.contains(&Token::ControlWord {
        word: "red".to_string(),
        parameter: Some(255)
    }));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_tokenize_empty_document() {
    let input = r#"{\rtf1}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::GroupStart));
    assert!(tokens.contains(&Token::GroupEnd));
}

#[test]
fn test_tokenize_only_header() {
    let input = r#"{\rtf1\ansi}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::ControlWord {
        word: "rtf".to_string(),
        parameter: Some(1)
    }));
    assert!(tokens.contains(&Token::ControlWord {
        word: "ansi".to_string(),
        parameter: None
    }));
}

#[test]
fn test_tokenize_control_word_space_delimiter() {
    // Space after control word is consumed as delimiter
    let input = r#"{\rtf1 \b text}"#;
    let tokens = tokenize(input).unwrap();

    // The space after \b should be consumed, not part of text
    assert!(tokens.contains(&Token::Text("text".to_string())));
}

#[test]
fn test_tokenize_multiple_control_words() {
    let input = r#"{\rtf1\b\i\ul}"#;
    let tokens = tokenize(input).unwrap();

    // GroupStart, rtf1, b, i, ul, GroupEnd = 6 tokens
    assert_eq!(tokens.len(), 6);
    assert!(tokens.contains(&Token::ControlWord {
        word: "b".to_string(),
        parameter: None
    }));
    assert!(tokens.contains(&Token::ControlWord {
        word: "i".to_string(),
        parameter: None
    }));
    assert!(tokens.contains(&Token::ControlWord {
        word: "ul".to_string(),
        parameter: None
    }));
}

#[test]
fn test_tokenize_unicode_escape() {
    // \uN followed by fallback character
    let input = r#"{\rtf1 \u233?}"#; // ? is fallback for é
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::ControlWord {
        word: "u".to_string(),
        parameter: Some(233)
    }));
}

#[test]
fn test_tokenize_unicode_skip_count() {
    // \ucN sets how many fallback chars to skip
    let input = r#"{\rtf1 \uc1 \u233?}"#;
    let tokens = tokenize(input).unwrap();

    assert!(tokens.contains(&Token::ControlWord {
        word: "uc".to_string(),
        parameter: Some(1)
    }));
}
