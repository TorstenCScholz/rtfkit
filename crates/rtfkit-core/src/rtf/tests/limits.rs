//! Parser Limits Tests
//!
//! Tests for parser limits enforcement including:
//! - Input size limits
//! - Group depth limits
//! - Table row/cell limits
//! - Merge span limits

use crate::error::ParseError;
use crate::limits::ParserLimits;
use crate::rtf::{RtfParser, parse, parse_with_limits};

// =============================================================================
// Input Size Limit Tests
// =============================================================================

#[test]
fn test_input_size_under_limit_succeeds() {
    let input = r#"{\rtf1\ansi Hello World}"#;
    let limits = ParserLimits::default();

    let result = parse_with_limits(input, limits);
    assert!(result.is_ok());
}

#[test]
fn test_input_size_exceeds_limit_fails() {
    let input = r#"{\rtf1\ansi Hello World}"#;
    let limits = ParserLimits::new().with_max_input_bytes(5);

    let result = parse_with_limits(input, limits);
    assert!(matches!(
        result,
        Err(crate::error::ConversionError::Parse(
            ParseError::InputTooLarge { .. }
        ))
    ));
}

#[test]
fn test_input_size_exact_limit_succeeds() {
    let input = r#"{\rtf1\ansi Test}"#;
    let exact_size = input.len();
    let limits = ParserLimits::new().with_max_input_bytes(exact_size);

    let result = parse_with_limits(input, limits);
    assert!(result.is_ok());
}

// =============================================================================
// Group Depth Limit Tests
// =============================================================================

#[test]
fn test_group_depth_under_limit_succeeds() {
    // Create a document with 10 nested groups
    let mut input = String::from("{\\rtf1\\ansi ");
    for _ in 0..10 {
        input.push('{');
    }
    input.push_str("text");
    for _ in 0..10 {
        input.push('}');
    }
    input.push('}');

    let limits = ParserLimits::new().with_max_group_depth(20);
    let result = parse_with_limits(&input, limits);

    assert!(result.is_ok());
}

#[test]
fn test_group_depth_exceeds_limit_fails() {
    // Create a document with 100 nested groups
    let mut input = String::from("{\\rtf1\\ansi ");
    for _ in 0..100 {
        input.push('{');
    }
    input.push_str("text");
    for _ in 0..100 {
        input.push('}');
    }
    input.push('}');

    let limits = ParserLimits::new().with_max_group_depth(50);
    let result = parse_with_limits(&input, limits);

    assert!(matches!(
        result,
        Err(crate::error::ConversionError::Parse(
            ParseError::GroupDepthExceeded { .. }
        ))
    ));
}

#[test]
fn test_group_depth_at_default_limit() {
    // Default limit is 256
    let mut input = String::from("{\\rtf1\\ansi ");
    for _ in 0..255 {
        input.push('{');
    }
    input.push_str("text");
    for _ in 0..255 {
        input.push('}');
    }
    input.push('}');

    let limits = ParserLimits::default();
    let result = parse_with_limits(&input, limits);

    // Should succeed - we're at 256 depth (1 outer + 255 nested)
    assert!(result.is_ok());
}

// =============================================================================
// Table Row Limit Tests
// =============================================================================

#[test]
fn test_table_rows_under_limit_succeeds() {
    // Create a table with 10 rows
    let mut input = String::from("{\\rtf1\\ansi\n");
    for i in 1..=10 {
        input.push_str(&format!("\\trowd\\cellx1000\\intbl R{}\\cell\\row\n", i));
    }
    input.push('}');

    let limits = ParserLimits::new().with_max_rows_per_table(100);
    let result = parse_with_limits(&input, limits);

    assert!(result.is_ok());
}

#[test]
fn test_table_rows_exceeds_limit_fails() {
    // Create a table with 100 rows
    let mut input = String::from("{\\rtf1\\ansi\n");
    for i in 1..=100 {
        input.push_str(&format!("\\trowd\\cellx1000\\intbl R{}\\cell\\row\n", i));
    }
    input.push('}');

    let limits = ParserLimits::new().with_max_rows_per_table(50);
    let result = parse_with_limits(&input, limits);

    assert!(result.is_err());
}

// =============================================================================
// Table Cell Limit Tests
// =============================================================================

#[test]
fn test_table_cells_under_limit_succeeds() {
    // Create a table row with 10 cells
    let mut input = String::from("{\\rtf1\\ansi\n\\trowd");
    for i in 1..=10 {
        input.push_str(&format!("\\cellx{}", i * 1000));
    }
    input.push_str("\n\\intbl ");
    for i in 1..=10 {
        input.push_str(&format!("C{}\\cell ", i));
    }
    input.push_str("\\row\n}");

    let limits = ParserLimits::new().with_max_cells_per_row(100);
    let result = parse_with_limits(&input, limits);

    assert!(result.is_ok());
}

#[test]
fn test_table_cells_exceeds_limit_fails() {
    // Create a table row with 100 cells
    let mut input = String::from("{\\rtf1\\ansi\n\\trowd");
    for i in 1..=100 {
        input.push_str(&format!("\\cellx{}", i * 100));
    }
    input.push_str("\n\\intbl ");
    for i in 1..=100 {
        input.push_str(&format!("C{}\\cell ", i));
    }
    input.push_str("\\row\n}");

    let limits = ParserLimits::new().with_max_cells_per_row(50);
    let result = parse_with_limits(&input, limits);

    assert!(result.is_err());
}

// =============================================================================
// Merge Span Limit Tests
// =============================================================================

#[test]
fn test_merge_span_under_limit_succeeds() {
    // Create a table with a 5-cell horizontal merge
    let input = r#"{\rtf1\ansi
\trowd\clmgf\cellx1000\clmrg\cellx2000\clmrg\cellx3000\clmrg\cellx4000\clmrg\cellx5000
\intbl Merged\cell\cell\cell\cell\cell\row
}"#;

    let limits = ParserLimits::new().with_max_merge_span(10);
    let result = parse_with_limits(input, limits);

    assert!(result.is_ok());
}

// =============================================================================
// RtfParser Tests
// =============================================================================

#[test]
fn test_rtf_parser_new() {
    let limits = ParserLimits::new()
        .with_max_input_bytes(1024)
        .with_max_group_depth(50);

    let parser = RtfParser::new(limits.clone());
    assert_eq!(parser.limits().max_input_bytes, 1024);
    assert_eq!(parser.limits().max_group_depth, 50);
}

#[test]
fn test_rtf_parser_default_parser() {
    let parser = RtfParser::default_parser();
    assert_eq!(parser.limits(), &ParserLimits::default());
}

#[test]
fn test_rtf_parser_parse() {
    let mut parser = RtfParser::default_parser();
    let input = r#"{\rtf1\ansi Test}"#;

    let result = parser.parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_rtf_parser_set_limits() {
    let mut parser = RtfParser::default_parser();

    let new_limits = ParserLimits::new().with_max_input_bytes(2048);
    parser.set_limits(new_limits.clone());

    assert_eq!(parser.limits().max_input_bytes, 2048);
}

// =============================================================================
// Default Limits Verification Tests
// =============================================================================

#[test]
fn test_default_limits_values() {
    let limits = ParserLimits::default();

    assert_eq!(limits.max_input_bytes, 10 * 1024 * 1024); // 10 MB
    assert_eq!(limits.max_group_depth, 256);
    assert_eq!(limits.max_warning_count, 1000);
    assert_eq!(limits.max_rows_per_table, 10000);
    assert_eq!(limits.max_cells_per_row, 1000);
    assert_eq!(limits.max_merge_span, 1000);
}

#[test]
fn test_none_limits() {
    let limits = ParserLimits::none();

    assert_eq!(limits.max_input_bytes, usize::MAX);
    assert_eq!(limits.max_group_depth, usize::MAX);
    assert_eq!(limits.max_warning_count, usize::MAX);
    assert_eq!(limits.max_rows_per_table, usize::MAX);
    assert_eq!(limits.max_cells_per_row, usize::MAX);
    assert_eq!(limits.max_merge_span, u16::MAX);
}

// =============================================================================
// Builder Pattern Tests
// =============================================================================

#[test]
fn test_limits_builder_pattern() {
    let limits = ParserLimits::new()
        .with_max_input_bytes(5 * 1024 * 1024)
        .with_max_group_depth(128)
        .with_max_warning_count(500)
        .with_max_rows_per_table(5000)
        .with_max_cells_per_row(500)
        .with_max_merge_span(500);

    assert_eq!(limits.max_input_bytes, 5 * 1024 * 1024);
    assert_eq!(limits.max_group_depth, 128);
    assert_eq!(limits.max_warning_count, 500);
    assert_eq!(limits.max_rows_per_table, 5000);
    assert_eq!(limits.max_cells_per_row, 500);
    assert_eq!(limits.max_merge_span, 500);
}

// =============================================================================
// Error Message Quality Tests
// =============================================================================

#[test]
fn test_input_too_large_error_message() {
    let input = r#"{\rtf1\ansi Hello World}"#;
    let limits = ParserLimits::new().with_max_input_bytes(5);

    let result = parse_with_limits(input, limits);

    if let Err(crate::error::ConversionError::Parse(ParseError::InputTooLarge { size, limit })) =
        result
    {
        assert!(size > limit);
        assert_eq!(limit, 5);
    } else {
        panic!("Expected InputTooLarge error");
    }
}

#[test]
fn test_group_depth_exceeded_error_message() {
    let mut input = String::from("{\\rtf1\\ansi ");
    for _ in 0..100 {
        input.push('{');
    }
    input.push_str("text");
    for _ in 0..100 {
        input.push('}');
    }
    input.push('}');

    let limits = ParserLimits::new().with_max_group_depth(50);
    let result = parse_with_limits(&input, limits);

    if let Err(crate::error::ConversionError::Parse(ParseError::GroupDepthExceeded {
        depth,
        limit,
    })) = result
    {
        assert!(depth > limit);
        assert_eq!(limit, 50);
    } else {
        panic!("Expected GroupDepthExceeded error");
    }
}

// =============================================================================
// Integration Tests with parse()
// =============================================================================

#[test]
fn test_parse_with_default_limits() {
    let input = r#"{\rtf1\ansi Hello World}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, report) = result.unwrap();
    assert_eq!(doc.blocks.len(), 1);
    assert_eq!(report.stats.paragraph_count, 1);
}

#[test]
fn test_parse_rejects_non_rtf() {
    let input = "not rtf at all";
    let result = parse(input);

    assert!(matches!(
        result,
        Err(crate::error::ConversionError::Parse(
            ParseError::MissingRtfHeader
        ))
    ));
}

#[test]
fn test_parse_rejects_unbalanced() {
    let input = r#"{\rtf1\ansi missing_end"#;
    let result = parse(input);

    assert!(matches!(
        result,
        Err(crate::error::ConversionError::Parse(
            ParseError::UnbalancedGroups
        ))
    ));
}
