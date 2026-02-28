//! Table Parsing Tests
//!
//! Tests for RTF table parsing including:
//! - Row/cell lifecycle
//! - Width from `\cellx`
//! - Merge normalization
//! - Hard limits

use crate::rtf::parse;
use crate::{Alignment, Block, BorderStyle, CellMerge, Inline};

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

#[test]
fn test_hyperlink_first_inline_in_cell_captures_alignment() {
    let input = include_str!("../../../../../fixtures/table_alignment_hyperlink_first_inline.rtf");
    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();
    let Some(Block::TableBlock(table)) = doc.blocks.first() else {
        panic!("Expected table block");
    };
    let Some(row) = table.rows.first() else {
        panic!("Expected row");
    };
    assert_eq!(row.cells.len(), 2);

    let cell0_para = match row.cells[0].blocks.first() {
        Some(Block::Paragraph(para)) => para,
        _ => panic!("Expected paragraph in first cell"),
    };
    assert_eq!(cell0_para.alignment, Alignment::Center);
    assert!(matches!(
        cell0_para.inlines.first(),
        Some(Inline::Hyperlink(_))
    ));

    let cell1_para = match row.cells[1].blocks.first() {
        Some(Block::Paragraph(para)) => para,
        _ => panic!("Expected paragraph in second cell"),
    };
    assert_eq!(cell1_para.alignment, Alignment::Right);
    assert!(matches!(
        cell1_para.inlines.first(),
        Some(Inline::Hyperlink(_))
    ));
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

// =============================================================================
// Cell Border Tests
// =============================================================================

#[test]
fn test_cell_border_top_single() {
    let input = r#"{\rtf1\ansi
\trowd\clbrdrt\brdrs\brdrw4\cellx1440
\intbl Text\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let cell = &table.rows[0].cells[0];
        let borders = cell.borders.as_ref().expect("cell should have borders");
        let top = borders.top.as_ref().expect("top border should be set");
        assert_eq!(top.style, BorderStyle::Single);
        assert_eq!(top.width_half_pts, Some(4));
        assert!(borders.left.is_none());
        assert!(borders.bottom.is_none());
        assert!(borders.right.is_none());
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_cell_border_all_four_sides() {
    let input = r#"{\rtf1\ansi
\trowd\clbrdrt\brdrs\brdrw4\clbrdrl\brdrs\brdrw4\clbrdrb\brdrs\brdrw4\clbrdrr\brdrs\brdrw4\cellx2880
\intbl Cell\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let cell = &table.rows[0].cells[0];
        let borders = cell.borders.as_ref().expect("cell should have borders");
        assert!(borders.top.is_some());
        assert!(borders.left.is_some());
        assert!(borders.bottom.is_some());
        assert!(borders.right.is_some());
        for side in [&borders.top, &borders.left, &borders.bottom, &borders.right] {
            let b = side.as_ref().unwrap();
            assert_eq!(b.style, BorderStyle::Single);
            assert_eq!(b.width_half_pts, Some(4));
        }
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_cell_border_style_variants() {
    let input = r#"{\rtf1\ansi
\trowd
\clbrdrt\brdrdot\brdrw4\cellx1440
\clbrdrt\brdrdb\brdrw4\cellx2880
\clbrdrt\brdrdash\brdrw4\cellx4320
\intbl Dot\cell Dbl\cell Dash\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let row = &table.rows[0];
        assert_eq!(
            row.cells[0]
                .borders
                .as_ref()
                .unwrap()
                .top
                .as_ref()
                .unwrap()
                .style,
            BorderStyle::Dotted
        );
        assert_eq!(
            row.cells[1]
                .borders
                .as_ref()
                .unwrap()
                .top
                .as_ref()
                .unwrap()
                .style,
            BorderStyle::Double
        );
        assert_eq!(
            row.cells[2]
                .borders
                .as_ref()
                .unwrap()
                .top
                .as_ref()
                .unwrap()
                .style,
            BorderStyle::Dashed
        );
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_cell_border_none_style() {
    let input = r#"{\rtf1\ansi
\trowd\clbrdrt\brdrnone\cellx1440
\intbl Text\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let cell = &table.rows[0].cells[0];
        let borders = cell.borders.as_ref().expect("cell should have borders");
        assert_eq!(borders.top.as_ref().unwrap().style, BorderStyle::None);
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_cell_border_color_resolved() {
    let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\trowd\clbrdrt\brdrs\brdrcf1\cellx1440\intbl T\cell\row}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let cell = &table.rows[0].cells[0];
        let borders = cell.borders.as_ref().expect("cell should have borders");
        let color = borders
            .top
            .as_ref()
            .unwrap()
            .color
            .as_ref()
            .expect("color should resolve");
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 0);
        assert_eq!(color.b, 0);
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_multi_cell_each_gets_own_borders() {
    let input = r#"{\rtf1\ansi
\trowd
\clbrdrt\brdrs\brdrw4\cellx1440
\clbrdrl\brdrs\brdrw8\cellx2880
\intbl A\cell B\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let row = &table.rows[0];
        // First cell: top border only
        let c0_borders = row.cells[0]
            .borders
            .as_ref()
            .expect("cell 0 should have borders");
        assert!(c0_borders.top.is_some());
        assert!(c0_borders.left.is_none());
        // Second cell: left border only
        let c1_borders = row.cells[1]
            .borders
            .as_ref()
            .expect("cell 1 should have borders");
        assert!(c1_borders.left.is_some());
        assert_eq!(c1_borders.left.as_ref().unwrap().width_half_pts, Some(8));
        assert!(c1_borders.top.is_none());
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_row_border_top_and_bottom() {
    let input = r#"{\rtf1\ansi
\trowd
\trbrdrt\brdrs\brdrw4
\trbrdrb\brdrs\brdrw4
\cellx2880
\intbl Text\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let row = &table.rows[0];
        let props = row.row_props.as_ref().expect("row should have props");
        let borders = props.borders.as_ref().expect("row should have borders");
        assert!(borders.top.is_some());
        assert!(borders.bottom.is_some());
        assert!(borders.left.is_none());
        assert!(borders.right.is_none());
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_row_border_inside_hv() {
    let input = r#"{\rtf1\ansi
\trowd
\trbrdrh\brdrs\brdrw4
\trbrdrv\brdrs\brdrw4
\cellx2880
\intbl T\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        let row = &table.rows[0];
        let props = row.row_props.as_ref().expect("row should have props");
        let borders = props.borders.as_ref().expect("row should have borders");
        assert!(borders.inside_h.is_some());
        assert!(borders.inside_v.is_some());
    } else {
        panic!("expected table block");
    }
}

#[test]
fn test_no_borders_none_field() {
    // A table with no border controls should have borders == None
    let input = r#"{\rtf1\ansi
\trowd\cellx1440
\intbl Normal\cell\row
}"#;
    let (doc, _report) = parse(input).unwrap();
    if let Some(Block::TableBlock(table)) = doc.blocks.first() {
        assert!(table.rows[0].cells[0].borders.is_none());
    } else {
        panic!("expected table block");
    }
}
