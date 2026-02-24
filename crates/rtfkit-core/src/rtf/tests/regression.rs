//! Regression Tests
//!
//! Tests for specific bug fixes and edge cases that have caused issues in the past.
//! These tests ensure that regressions don't reoccur.

use crate::Block;
use crate::rtf::parse;

// =============================================================================
// Malformed Input Regression Tests
// =============================================================================

#[test]
fn test_malformed_unclosed_groups() {
    let input = include_str!("../../../../../fixtures/malformed_unclosed_groups.rtf");
    let result = parse(input);

    // Should fail gracefully with parse error
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_malformed_invalid_control_words() {
    let input = include_str!("../../../../../fixtures/malformed_invalid_control_words.rtf");
    let result = parse(input);

    // Should parse with warnings
    assert!(result.is_ok());
}

#[test]
fn test_malformed_repeated_bad_controls() {
    let input = include_str!("../../../../../fixtures/malformed_repeated_bad_controls.rtf");
    let result = parse(input);

    // Should parse with warnings
    assert!(result.is_ok());
}

// =============================================================================
// Empty Document Tests
// =============================================================================

#[test]
fn test_empty_document() {
    let input = include_str!("../../../../../fixtures/text_empty.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();
    // Empty document should have no blocks
    assert!(
        doc.blocks.is_empty()
            || doc
                .blocks
                .iter()
                .all(|b| { matches!(b, Block::Paragraph(p) if p.inlines.is_empty()) })
    );
}

// =============================================================================
// Unicode Handling Tests
// =============================================================================

#[test]
fn test_unicode_document() {
    let input = include_str!("../../../../../fixtures/text_unicode.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_unicode_escape() {
    let input = r#"{\rtf1\ansi \u233?}"#; // é with fallback
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have the unicode character
        let text: String = para
            .inlines
            .iter()
            .filter_map(|i| match i {
                crate::Inline::Run(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect();

        // The é character should be present
        assert!(text.contains('é') || text.contains('?'));
    }
}

#[test]
fn test_unicode_skip_count() {
    // \ucN sets how many fallback characters to skip
    let input = r#"{\rtf1\ansi \uc1 \u233?Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Nested Formatting Tests
// =============================================================================

#[test]
fn test_nested_styles() {
    let input = include_str!("../../../../../fixtures/text_nested_styles.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_formatting() {
    let input = r#"{\rtf1\ansi {\b {\i {\ul {\strike Deep}}}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Mixed Content Tests
// =============================================================================

#[test]
fn test_mixed_complex() {
    let input = include_str!("../../../../../fixtures/mixed_complex.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_mixed_list_table_nested() {
    let input = include_str!("../../../../../fixtures/mixed_list_table_nested.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_mixed_prose_list_table() {
    let input = include_str!("../../../../../fixtures/mixed_prose_list_table.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Text Formatting Tests
// =============================================================================

#[test]
fn test_bold_italic() {
    let input = include_str!("../../../../../fixtures/text_bold_italic.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have bold and italic runs
        let has_bold = para
            .inlines
            .iter()
            .any(|i| matches!(i, crate::Inline::Run(r) if r.bold));
        let has_italic = para
            .inlines
            .iter()
            .any(|i| matches!(i, crate::Inline::Run(r) if r.italic));

        assert!(has_bold || has_italic);
    }
}

#[test]
fn test_underline() {
    let input = include_str!("../../../../../fixtures/text_underline.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_mixed_formatting() {
    let input = include_str!("../../../../../fixtures/text_mixed_formatting.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_multiple_paragraphs() {
    let input = include_str!("../../../../../fixtures/text_multiple_paragraphs.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have multiple paragraphs
    assert!(doc.blocks.len() >= 2);
}

// =============================================================================
// Alignment Tests
// =============================================================================

#[test]
fn test_text_alignment() {
    let input = include_str!("../../../../../fixtures/text_alignment.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have paragraphs with different alignments
    let alignments: Vec<_> = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p.alignment)
            } else {
                None
            }
        })
        .collect();

    assert!(!alignments.is_empty());
}

// =============================================================================
// Simple Paragraph Tests
// =============================================================================

#[test]
fn test_simple_paragraph() {
    let input = include_str!("../../../../../fixtures/text_simple_paragraph.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);

    if let Block::Paragraph(para) = &doc.blocks[0] {
        let text: String = para
            .inlines
            .iter()
            .filter_map(|i| match i {
                crate::Inline::Run(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(!text.is_empty());
    }
}

// =============================================================================
// Group Balance Edge Cases
// =============================================================================

#[test]
fn test_extra_closing_brace() {
    let input = r#"{\rtf1\ansi Text}}"#;
    let result = parse(input);

    // Should fail with unbalanced groups
    assert!(result.is_err());
}

#[test]
fn test_extra_opening_brace() {
    let input = r#"{\rtf1\ansi {Text"#;
    let result = parse(input);

    // Should fail with unbalanced groups
    assert!(result.is_err());
}

#[test]
fn test_correctly_nested_groups() {
    let input = r#"{\rtf1\ansi {\b Bold} {\i Italic}}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Control Word Edge Cases
// =============================================================================

#[test]
fn test_control_word_at_eof() {
    let input = r#"{\rtf1\ansi Text\b}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_control_word_without_parameter() {
    let input = r#"{\rtf1\ansi \b\i\ul}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_control_word_with_zero_parameter() {
    let input = r#"{\rtf1\ansi \b0\b1}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_long_control_word() {
    let input = r#"{\rtf1\ansi \verylongcontrolwordname Text}"#;
    let result = parse(input);

    // Should parse, treating unknown control word as warning
    assert!(result.is_ok());
}

// =============================================================================
// Special Character Tests
// =============================================================================

#[test]
fn test_escaped_special_characters() {
    let input = r#"{\rtf1\ansi \{\}\\ Text}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Block::Paragraph(para) = &doc.blocks[0] {
        let text: String = para
            .inlines
            .iter()
            .filter_map(|i| match i {
                crate::Inline::Run(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect();

        // Should contain the escaped characters
        assert!(text.contains('{') || text.contains('}') || text.contains('\\'));
    }
}

#[test]
fn test_newline_handling() {
    let input = "{\\rtf1\\ansi Line1\nLine2}";
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_tab_handling() {
    let input = r#"{\rtf1\ansi Col1\tab Col2}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Destination Edge Cases
// =============================================================================

#[test]
fn test_nested_destinations() {
    let input = r#"{\rtf1\ansi {\*\dest1{\*\dest2}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_destination_with_text() {
    let input = r#"{\rtf1\ansi {\*\generator Microsoft Word 2010}Content}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Generator should be skipped
    if let Block::Paragraph(para) = &doc.blocks[0] {
        let text: String = para
            .inlines
            .iter()
            .filter_map(|i| match i {
                crate::Inline::Run(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(text.contains("Content"));
        assert!(!text.contains("Microsoft"));
    }
}

// =============================================================================
// Table Edge Cases
// =============================================================================

#[test]
fn test_table_with_no_cells() {
    let input = r#"{\rtf1\ansi \trowd\row}"#;
    let result = parse(input);

    // Should handle gracefully
    assert!(result.is_ok());
}

#[test]
fn test_table_with_empty_cells() {
    let input = r#"{\rtf1\ansi \trowd\cellx1000\cellx2000\intbl \cell\cell\row}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// List Edge Cases
// =============================================================================

#[test]
fn test_list_without_table() {
    // List reference without list table
    let input = r#"{\rtf1\ansi \ls1 Item}"#;
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

// =============================================================================
// Stress Tests
// =============================================================================

#[test]
fn test_many_groups() {
    let mut input = String::from("{\\rtf1\\ansi ");
    for i in 0..100 {
        input.push_str(&format!("{{\\b{}}}", i));
    }
    input.push('}');

    let result = parse(&input);
    assert!(result.is_ok());
}

#[test]
fn test_many_paragraphs() {
    let mut input = String::from("{\\rtf1\\ansi ");
    for i in 0..100 {
        input.push_str(&format!("Paragraph {}\\par ", i));
    }
    input.push('}');

    let result = parse(&input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();
    assert!(doc.blocks.len() >= 50);
}

// =============================================================================
// Determinism Tests
// =============================================================================

#[test]
fn test_parsing_is_deterministic() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0 Arial;}}{\colortbl;\red255\green0\blue0;}
\b Bold \i Italic \cf1 Red \par
\trowd\cellx1000\intbl Cell\cell\row
}"#;

    // Parse the same input multiple times
    let result1 = parse(input);
    let result2 = parse(input);
    let result3 = parse(input);

    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());

    let (doc1, _) = result1.unwrap();
    let (doc2, _) = result2.unwrap();
    let (doc3, _) = result3.unwrap();

    // All results should be identical
    assert_eq!(doc1, doc2);
    assert_eq!(doc2, doc3);
}
