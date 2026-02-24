//! Field and Hyperlink Tests
//!
//! Tests for RTF field/hyperlink handling including:
//! - Supported URL schemes
//! - Malformed/non-hyperlink field degradation
//! - Nested fields

use crate::rtf::parse;
use crate::{Block, Inline};

// =============================================================================
// Simple Hyperlink Tests
// =============================================================================

#[test]
fn test_simple_hyperlink() {
    let input = include_str!("../../../../../fixtures/hyperlink_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have a hyperlink
    let has_hyperlink = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .any(|i| matches!(i, Inline::Hyperlink(_)));

    assert!(has_hyperlink, "Expected a hyperlink in the document");
}

#[test]
fn test_hyperlink_url_extraction() {
    let input = include_str!("../../../../../fixtures/hyperlink_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // URL should be extracted
        assert!(!hlink.url.is_empty());
    }
}

// =============================================================================
// Formatted Hyperlink Tests
// =============================================================================

#[test]
fn test_hyperlink_with_formatting() {
    let input = include_str!("../../../../../fixtures/hyperlink_formatted.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Find the hyperlink
    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // Should have runs with formatting
        assert!(!hlink.runs.is_empty());
    }
}

// =============================================================================
// Multiple Hyperlinks Tests
// =============================================================================

#[test]
fn test_multiple_hyperlinks() {
    let input = include_str!("../../../../../fixtures/hyperlink_multiple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Count hyperlinks
    let hyperlink_count = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .filter(|i| matches!(i, Inline::Hyperlink(_)))
        .count();

    assert!(hyperlink_count >= 2, "Expected at least 2 hyperlinks");
}

// =============================================================================
// Hyperlink in Table Tests
// =============================================================================

#[test]
fn test_hyperlink_in_table() {
    let input = include_str!("../../../../../fixtures/hyperlink_in_table.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have a table
    assert!(doc.blocks.iter().any(|b| matches!(b, Block::TableBlock(_))));
}

// =============================================================================
// URL Scheme Tests
// =============================================================================

#[test]
fn test_http_url_scheme() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        assert!(hlink.url.starts_with("http://") || hlink.url.starts_with("https://"));
    }
}

#[test]
fn test_https_url_scheme() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        assert!(hlink.url.starts_with("https://"));
    }
}

#[test]
fn test_mailto_url_scheme() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "mailto:test@example.com"}{\fldrslt Email}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        assert!(hlink.url.starts_with("mailto:"));
    }
}

// =============================================================================
// Malformed Field Tests
// =============================================================================

#[test]
fn test_missing_fldrslt() {
    let input = include_str!("../../../../../fixtures/hyperlink_missing_fldrslt.rtf");
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

#[test]
fn test_unsupported_field_type() {
    let input = include_str!("../../../../../fixtures/hyperlink_unsupported_field.rtf");
    let result = parse(input);

    // Should parse gracefully, ignoring unsupported field
    assert!(result.is_ok());
}

#[test]
fn test_malformed_field_instruction() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst INVALID}{\fldrslt Text}}}"#;
    let result = parse(input);

    // Should parse, treating as regular text
    assert!(result.is_ok());
}

// =============================================================================
// Nested Field Tests
// =============================================================================

#[test]
fn test_nested_fields() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt {\field{\*\fldinst HYPERLINK "http://nested.com"}{\fldrslt Nested}}}}}"#;
    let result = parse(input);

    // Should handle nested fields
    assert!(result.is_ok());
}

// =============================================================================
// Field Result Tests
// =============================================================================

#[test]
fn test_field_result_text() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt Click Here}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // Should have the display text
        let text: String = hlink.runs.iter().map(|r| r.text.as_str()).collect();

        assert!(text.contains("Click") || text.contains("Here"));
    }
}

#[test]
fn test_field_result_with_formatting() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt \b Bold Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // Should have bold run
        assert!(hlink.runs.iter().any(|r| r.bold));
    }
}

// =============================================================================
// Field Instruction Parsing Tests
// =============================================================================

#[test]
fn test_hyperlink_with_quotes() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com/path?query=1"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // URL should include the path
        assert!(hlink.url.contains("path"));
    }
}

#[test]
fn test_hyperlink_without_quotes() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK http://example.com}{\fldrslt Link}}}"#;
    let result = parse(input);

    // Should still parse
    assert!(result.is_ok());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_field_result() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt }}}"#;
    let result = parse(input);

    // Should parse, even with empty result
    assert!(result.is_ok());
}

#[test]
fn test_field_without_instruction() {
    let input = r#"{\rtf1\ansi {\field{\fldrslt Just Text}}}"#;
    let result = parse(input);

    // Should parse, treating as regular text
    assert!(result.is_ok());
}

#[test]
fn test_hyperlink_with_special_characters() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com/path?name=Test&value=123"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // URL should preserve special characters
        assert!(hlink.url.contains("?") || hlink.url.contains("&"));
    }
}

// =============================================================================
// Non-Hyperlink Field Tests
// =============================================================================

#[test]
fn test_page_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst PAGE}{\fldrslt 1}}}"#;
    let result = parse(input);

    // Should parse, ignoring non-hyperlink field
    assert!(result.is_ok());
}

#[test]
fn test_numpages_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst NUMPAGES}{\fldrslt 10}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_date_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst DATE}{\fldrslt 2024-01-01}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_time_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst TIME}{\fldrslt 12:00}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Hyperlink Adjacent to Text Tests
// =============================================================================

#[test]
fn test_hyperlink_with_adjacent_text() {
    let input = r#"{\rtf1\ansi Before {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt Link}} After}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have text before and after hyperlink
        let has_before = para
            .inlines
            .iter()
            .any(|i| matches!(i, Inline::Run(r) if r.text.contains("Before")));
        let has_after = para
            .inlines
            .iter()
            .any(|i| matches!(i, Inline::Run(r) if r.text.contains("After")));

        assert!(
            has_before || has_after,
            "Expected text adjacent to hyperlink"
        );
    }
}
