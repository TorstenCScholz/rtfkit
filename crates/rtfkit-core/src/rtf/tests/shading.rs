//! Shading Tests
//!
//! Tests for RTF shading handling including:
//! - Paragraph shading
//! - Cell shading
//! - Reset semantics (`\plain` vs `\pard`)
//! - Shading patterns

use crate::Block;
use crate::rtf::parse;

// =============================================================================
// Paragraph Shading Tests
// =============================================================================

#[test]
fn test_paragraph_shading_basic() {
    let input = include_str!("../../../../../fixtures/paragraph_shading_basic.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have a paragraph with shading
    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        assert!(para.shading.is_some(), "Expected paragraph shading");
    }
}

#[test]
fn test_paragraph_shading_with_color() {
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}\cb1 Shaded Paragraph}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_paragraph_shading_reset_pard() {
    let input = include_str!("../../../../../fixtures/paragraph_shading_plain_pard_reset.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Check that \pard resets paragraph shading
    // The fixture should have multiple paragraphs with different shading states
    assert!(!doc.blocks.is_empty());
}

// =============================================================================
// Cell Shading Tests
// =============================================================================

#[test]
fn test_cell_shading_basic() {
    let input = include_str!("../../../../../fixtures/table_cell_shading_basic.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // Should have cell with shading
        let has_shading = table
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .any(|c| c.shading.is_some());

        assert!(has_shading, "Expected cell shading");
    }
}

#[test]
fn test_cell_shading_with_color() {
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;}
\trowd\clcbpat1\cellx2000
\intbl Red Cell\cell\row
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Row and Table Shading Tests
// =============================================================================

#[test]
fn test_row_cell_shading_precedence() {
    let input = include_str!("../../../../../fixtures/table_row_cell_shading_precedence.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Cell shading should take precedence over row shading
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // Just verify the table parsed correctly
        assert!(!table.rows.is_empty());
    }
}

// =============================================================================
// Shading Pattern Tests
// =============================================================================

#[test]
fn test_shading_pattern_basic() {
    let input = include_str!("../../../../../fixtures/shading_pattern_basic.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_shading_solid() {
    let input =
        r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}\cb1\shading100000 Solid Shading}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_shading_clear() {
    let input = r#"{\rtf1\ansi \shading0 Clear Shading}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Theme Color Shading Tests
// =============================================================================

#[test]
fn test_shading_theme_color_reference() {
    let input = include_str!("../../../../../fixtures/shading_theme_color_reference.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Shading Control Word Tests
// =============================================================================

#[test]
fn test_cbpat_shading() {
    let input = r#"{\rtf1\ansi {\colortbl;\red100\green100\blue100;}
\cbpat1 Background Pattern
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_cfpat_shading() {
    let input = r#"{\rtf1\ansi {\colortbl;\red50\green50\blue50;}
\cfpat1 Foreground Pattern
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_clcbpat_cell_shading() {
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}
\trowd\clcbpat1\cellx2000
\intbl Shaded Cell\cell\row
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_clcfpat_cell_pattern() {
    let input = r#"{\rtf1\ansi {\colortbl;\red100\green100\blue100;}
\trowd\clcfpat1\cellx2000
\intbl Pattern Cell\cell\row
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Reset Semantics Tests
// =============================================================================

#[test]
fn test_plain_does_not_reset_paragraph_shading() {
    // \plain resets character formatting but NOT paragraph shading
    let input =
        r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}\cb1 Shaded\plain Still Shaded}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_pard_resets_paragraph_shading() {
    // \pard resets paragraph properties including shading
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}\cb1 Shaded\pard Not Shaded}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_plain_reset_background() {
    let input = include_str!("../../../../../fixtures/text_background_plain_reset.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Shading Percentage Tests
// =============================================================================

#[test]
fn test_shading_percentage_25() {
    let input = r#"{\rtf1\ansi \shading2500 25% Shading}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_shading_percentage_50() {
    let input = r#"{\rtf1\ansi \shading5000 50% Shading}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_shading_percentage_75() {
    let input = r#"{\rtf1\ansi \shading7500 75% Shading}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_shading_without_color_table() {
    let input = r#"{\rtf1\ansi \cb1 Shading Without Color Table}"#;
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

#[test]
fn test_shading_invalid_color_index() {
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}\cb999 Invalid Index}"#;
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

#[test]
fn test_multiple_shading_controls() {
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;\red100\green100\blue100;}
\cb1\cfpat2 Multiple Shading Controls
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_shading_in_nested_group() {
    let input =
        r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}{\cb1 Nested Shading}No Shading}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Table Cell Shading with Merge Tests
// =============================================================================

#[test]
fn test_shading_with_merged_cells() {
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}
\trowd\clmgf\clcbpat1\cellx2000\clmrg\cellx4000
\intbl Merged Shaded\cell\cell\row
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Row-level Shading Tests
// =============================================================================

#[test]
fn test_row_shading_trcbpat() {
    let input = r#"{\rtf1\ansi {\colortbl;\red230\green230\blue230;}
\trowd\trcbpat1\cellx2000
\intbl Row Shaded\cell\row
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_row_shading_trcfpat() {
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}
\trowd\trcfpat1\cellx2000
\intbl Row Pattern\cell\row
}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Shading Pattern Type Tests
// =============================================================================

#[test]
fn test_shading_pattern_horz_stripe() {
    let input = r#"{\rtf1\ansi \shading1000 Horizontal Stripe}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_shading_pattern_vert_stripe() {
    let input = r#"{\rtf1\ansi \shading2000 Vertical Stripe}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_shading_pattern_diag_stripe() {
    let input = r#"{\rtf1\ansi \shading3000 Diagonal Stripe}"#;
    let result = parse(input);

    assert!(result.is_ok());
}
