//! Destination Handling Tests
//!
//! Tests for RTF destination handling including:
//! - Metadata skip
//! - Unknown destination warning
//! - List/font/color destination parsing

use crate::Block;
use crate::rtf::parse;

// =============================================================================
// Metadata Destination Tests
// =============================================================================

#[test]
fn test_skip_generator_destination() {
    let input = r#"{\rtf1\ansi {\*\generator Microsoft Word}Hello}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    // Generator should be skipped, only "Hello" should remain
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
        assert!(text.contains("Hello"));
        assert!(!text.contains("Microsoft"));
    } else {
        panic!("Expected Paragraph block");
    }
}

#[test]
fn test_skip_info_destination() {
    let input = r#"{\rtf1\ansi {\info{\title Test Title}{\author Test Author}}Content}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    // Info should be skipped, only "Content" should remain
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_skip_stylesheet_destination() {
    let input = r#"{\rtf1\ansi {\stylesheet{\s1 Normal;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}

// =============================================================================
// Font Table Destination Tests
// =============================================================================

#[test]
fn test_font_table_parsing() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0 Arial;}{\f1 Times New Roman;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    // Font table should be parsed but not appear as content
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_font_table_with_font_family() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0\fswiss Arial;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Color Table Destination Tests
// =============================================================================

#[test]
fn test_color_table_parsing() {
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;\red0\green255\blue0;}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    // Color table should be parsed but not appear as content
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_color_table_empty_first_entry() {
    // First entry in color table is auto (empty)
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// List Table Destination Tests
// =============================================================================

#[test]
fn test_list_table_parsing() {
    let input = r#"{\rtf1\ansi {\listtable{\list\listtemplateid1{\listlevel\levelnfc23{\leveltext\'01\u-3913 ?;}}}}{\listoverridetable}{\listtext\pard\plain\s1\ql{\listtext\par}\ls1\ilvl0\cf0\highlight0\b0\i0\ulnone\strike0\outl0\shad0\caps0\scaps0\expndtw0\kerning0\fs24\lang1033\f0\cbpat0 Test}}"#;
    let result = parse(input);

    // Should parse without error
    assert!(result.is_ok());
}

// =============================================================================
// Unknown Destination Tests
// =============================================================================

#[test]
fn test_unknown_destination_skipped() {
    let input = r#"{\rtf1\ansi {\*\unknowndest some content}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, report) = result.unwrap();

    // Unknown destination should be skipped
    assert_eq!(doc.blocks.len(), 1);

    // May generate a warning
    // The report should indicate the unknown destination was handled
    let _ = report; // Just verify it exists
}

#[test]
fn test_nested_destination_skipped() {
    let input = r#"{\rtf1\ansi {\*\dest1{nested content}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}

// =============================================================================
// Destination with Ignorable Content Tests
// =============================================================================

#[test]
fn test_destination_with_control_words() {
    let input = r#"{\rtf1\ansi {\*\generator\b Microsoft\b0 Word}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    // Control words in destination should be skipped
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_destination_with_groups() {
    let input = r#"{\rtf1\ansi {\*\dest{group1}{group2}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}

// =============================================================================
// Non-Destination Group Tests
// =============================================================================

#[test]
fn test_formatting_group_not_skipped() {
    let input = r#"{\rtf1\ansi {\b Bold}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    // Formatting group should produce content
    assert_eq!(doc.blocks.len(), 1);

    if let Block::Paragraph(para) = &doc.blocks[0] {
        // Should have bold run
        assert!(
            para.inlines
                .iter()
                .any(|i| { matches!(i, crate::Inline::Run(r) if r.bold) })
        );
    }
}

#[test]
fn test_nested_formatting_groups() {
    let input = r#"{\rtf1\ansi {\b {\i Bold Italic}}Plain}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_destination() {
    let input = r#"{\rtf1\ansi {\*\emptydest}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_destination_at_end() {
    let input = r#"{\rtf1\ansi Text{\*\generator Word}}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_multiple_destinations() {
    let input = r#"{\rtf1\ansi {\*\dest1}Text{\*\dest2}More}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_destination_before_and_after_content() {
    let input = r#"{\rtf1\ansi {\*\header}Content{\*\footer}}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    assert_eq!(doc.blocks.len(), 1);
}
