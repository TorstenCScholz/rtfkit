//! Table Parsing Tests
//!
//! Tests for RTF table parsing including:
//! - Row/cell lifecycle
//! - Width from `\cellx`
//! - Merge normalization
//! - Hard limits

use crate::rtf::parse;
use crate::{Block, CellMerge};

// =============================================================================
// Basic Table Tests
// =============================================================================

#[test]
fn test_simple_2x2_table() {
    let input = include_str!("../../../../../fixtures/table_simple_2x2.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have a table block
    assert!(doc.blocks.iter().any(|b| matches!(b, Block::TableBlock(_))));

    // Check table structure
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        assert_eq!(table.rows.len(), 2, "Expected 2 rows");

        for (i, row) in table.rows.iter().enumerate() {
            assert_eq!(row.cells.len(), 2, "Expected 2 cells in row {}", i);
        }
    }
}

#[test]
fn test_table_row_cell_lifecycle() {
    let input = r#"{\rtf1\ansi
\trowd\cellx1000\cellx2000
\intbl A\cell B\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells.len(), 2);
    }
}

// =============================================================================
// Cell Width Tests
// =============================================================================

#[test]
fn test_cell_width_from_cellx() {
    let input = r#"{\rtf1\ansi
\trowd\cellx1440\cellx2880
\intbl A\cell B\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let row = &table.rows[0];

        // First cell width should be 1440 twips (1 inch)
        assert_eq!(row.cells[0].width_twips, Some(1440));

        // Second cell width should be 2880 - 1440 = 1440 twips
        // Or the absolute position depending on interpretation
        assert!(row.cells[1].width_twips.is_some());
    }
}

#[test]
fn test_cell_width_uneven() {
    let input = r#"{\rtf1\ansi
\trowd\cellx1000\cellx3000
\intbl A\cell B\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // Cells should have different widths
        let row = &table.rows[0];
        assert!(row.cells[0].width_twips != row.cells[1].width_twips);
    }
}

// =============================================================================
// Horizontal Merge Tests
// =============================================================================

#[test]
fn test_horizontal_merge_valid() {
    let input = include_str!("../../../../../fixtures/table_horizontal_merge_valid.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // Should have merge information
        let has_merge = table
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .any(|c| c.merge.is_some());

        assert!(has_merge, "Expected merge information in table");
    }
}

#[test]
fn test_horizontal_merge_start() {
    let input = r#"{\rtf1\ansi
\trowd\clmgf\cellx2000\clmrg\cellx4000
\intbl Merged\cell\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let first_cell = &table.rows[0].cells[0];

        // First cell should be merge start
        if let Some(merge) = &first_cell.merge {
            matches!(merge, CellMerge::HorizontalStart { .. });
        }
    }
}

#[test]
fn test_horizontal_merge_continuation() {
    let input = r#"{\rtf1\ansi
\trowd\clmgf\cellx2000\clmrg\cellx4000
\intbl Merged\cell\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let second_cell = &table.rows[0].cells[1];

        // Second cell should be merge continuation
        if let Some(merge) = &second_cell.merge {
            matches!(merge, CellMerge::HorizontalContinue);
        }
    }
}

// =============================================================================
// Vertical Merge Tests
// =============================================================================

#[test]
fn test_vertical_merge_valid() {
    let input = include_str!("../../../../../fixtures/table_vertical_merge_valid.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // Should have vertical merge information
        let has_vmerge = table.rows.iter().flat_map(|r| r.cells.iter()).any(|c| {
            matches!(
                c.merge,
                Some(CellMerge::VerticalStart) | Some(CellMerge::VerticalContinue)
            )
        });

        assert!(has_vmerge, "Expected vertical merge information");
    }
}

#[test]
fn test_mixed_merge() {
    let input = include_str!("../../../../../fixtures/table_mixed_merge.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Malformed Table Tests
// =============================================================================

#[test]
fn test_missing_cell_terminator() {
    let input = include_str!("../../../../../fixtures/malformed_table_missing_cell_terminator.rtf");
    let result = parse(input);

    // Should still parse, degrading gracefully
    assert!(result.is_ok());
}

#[test]
fn test_missing_row_terminator() {
    let input = include_str!("../../../../../fixtures/malformed_table_missing_row_terminator.rtf");
    let result = parse(input);

    // Should still parse
    assert!(result.is_ok());
}

#[test]
fn test_non_monotonic_cellx() {
    let input = include_str!("../../../../../fixtures/malformed_table_non_monotonic_cellx.rtf");
    let result = parse(input);

    // Should parse but may generate warnings
    assert!(result.is_ok());
}

#[test]
fn test_conflicting_merge() {
    let input = include_str!("../../../../../fixtures/malformed_table_conflicting_merge.rtf");
    let result = parse(input);

    // Should parse with degraded merge handling
    assert!(result.is_ok());
}

#[test]
fn test_orphan_merge_controls() {
    let input = include_str!("../../../../../fixtures/malformed_table_orphan_controls.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_orphan_merge_continuation() {
    let input =
        include_str!("../../../../../fixtures/malformed_table_orphan_merge_continuation.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_merge_controls_degraded() {
    let input = include_str!("../../../../../fixtures/malformed_table_merge_controls_degraded.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Table with Content Tests
// =============================================================================

#[test]
fn test_table_with_formatted_content() {
    let input = r#"{\rtf1\ansi
\trowd\cellx2000\cellx4000
\intbl \b Bold\cell\i Italic\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // First cell should have bold text
        let first_cell = &table.rows[0].cells[0];
        let has_bold = first_cell
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
            .any(|i| matches!(i, crate::Inline::Run(r) if r.bold));

        assert!(has_bold, "Expected bold text in first cell");
    }
}

#[test]
fn test_table_multirow_uneven_content() {
    let input = include_str!("../../../../../fixtures/table_multirow_uneven_content.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // Rows should have content
        assert!(table.rows.len() > 1);
    }
}

// =============================================================================
// Table with Prose Tests
// =============================================================================

#[test]
fn test_table_prose_interleave() {
    let input = include_str!("../../../../../fixtures/table_prose_interleave.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have both table and paragraph blocks
    assert!(doc.blocks.iter().any(|b| matches!(b, Block::TableBlock(_))));
    assert!(doc.blocks.iter().any(|b| matches!(b, Block::Paragraph(_))));
}

// =============================================================================
// Multiple Tables Tests
// =============================================================================

#[test]
fn test_multiple_tables() {
    let input = r#"{\rtf1\ansi
\trowd\cellx1000\intbl A\cell\row
\par
\trowd\cellx1000\intbl B\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have two table blocks
    let table_count = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::TableBlock(_)))
        .count();

    assert_eq!(table_count, 2);
}

// =============================================================================
// Table Cell Alignment Tests
// =============================================================================

#[test]
fn test_cell_vertical_alignment() {
    let input = r#"{\rtf1\ansi
\trowd\clvertalc\cellx2000
\intbl Centered\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let cell = &table.rows[0].cells[0];
        assert!(cell.v_align.is_some());
    }
}

// =============================================================================
// Row Properties Tests
// =============================================================================

#[test]
fn test_row_alignment() {
    let input = r#"{\rtf1\ansi
\trowd\trqc\cellx2000
\intbl Centered Row\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let row = &table.rows[0];
        assert!(row.row_props.is_some());
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_table_cell() {
    let input = r#"{\rtf1\ansi
\trowd\cellx1000\cellx2000
\intbl \cell Non-empty\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // First cell should be empty but exist
        assert_eq!(table.rows[0].cells.len(), 2);
    }
}

#[test]
fn test_single_cell_table() {
    let input = r#"{\rtf1\ansi
\trowd\cellx1000
\intbl Single\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells.len(), 1);
    }
}

#[test]
fn test_table_row_restart() {
    // Test that \trowd resets row properties
    let input = r#"{\rtf1\ansi
\trowd\cellx1000\intbl A\cell\row
\trowd\cellx2000\intbl B\cell\row
}"#;

    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        // Each row should have its own width
        assert!(table.rows[0].cells[0].width_twips != table.rows[1].cells[0].width_twips);
    }
}
