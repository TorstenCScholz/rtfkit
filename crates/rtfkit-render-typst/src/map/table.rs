//! Table mapping from IR to Typst source.
//!
//! This module provides functions to convert rtfkit-core `TableBlock` types
//! to Typst markup source code.
//!
//! ## Typst Table Mapping
//!
//! Tables in Typst use the `#table()` function with grid syntax:
//!
//! ```typst
//! #table(
//!   columns: (auto, auto),
//!   [Cell 1], [Cell 2],
//!   [Cell 3], [Cell 4],
//! )
//! ```
//!
//! ## Merge Support
//!
//! - **Colspan**: Uses `colspan` parameter on cells
//! - **Rowspan**: Uses `rowspan` parameter on cells
//! - Cells covered by merges are omitted from output
//!
//! ## Cell Shading
//!
//! - **Background color**: Uses `fill` parameter on `table.cell()`
//!
//! ## Column Width Calculation
//!
//! Column widths are computed deterministically from cell widths when available,
//! falling back to `auto` for unspecified columns.

use rtfkit_core::{
    CellMerge, Color, ShadingPattern, TableBlock as IrTableBlock, TableCell as IrTableCell,
    TableRow as IrTableRow,
};
use std::collections::{HashMap, HashSet};

use super::{MappingWarning, TypstAssetAllocator, map_block};

/// Result of mapping a table to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct TableOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated during mapping.
    pub warnings: Vec<MappingWarning>,
}

/// Result of mapping a table cell.
#[derive(Debug, Clone)]
struct CellInfo {
    /// The cell content as Typst source.
    content: String,
    /// Column span (1 = no span).
    colspan: u32,
    /// Row span (1 = no span).
    rowspan: u32,
    /// Cell width in twips (if specified).
    width_twips: Option<i32>,
    /// Cell fill color (from shading).
    fill_color: Option<Color>,
}

type RowspanMap = HashMap<(usize, usize), u32>;
type SkipCells = HashSet<(usize, usize)>;

/// Map a rtfkit-core TableBlock to Typst source code.
///
/// # Arguments
///
/// * `table` - The source table block from rtfkit-core.
///
/// # Returns
///
/// A `TableOutput` containing the Typst source and any warnings.
///
/// # Determinism
///
/// This function is deterministic: the same input always produces the same output.
pub fn map_table(table: &IrTableBlock) -> TableOutput {
    let mut assets = TypstAssetAllocator::new();
    map_table_with_assets(table, &mut assets)
}

pub(crate) fn map_table_with_assets(
    table: &IrTableBlock,
    assets: &mut TypstAssetAllocator,
) -> TableOutput {
    let mut warnings = Vec::new();

    if table.rows.is_empty() {
        return TableOutput {
            typst_source: String::new(),
            warnings,
        };
    }

    // Compute vertical merges
    let (rowspan_map, skip_cells) = compute_vertical_merges(table);

    // Map rows to cell info
    let rows = map_rows(
        &table.rows,
        &rowspan_map,
        &skip_cells,
        assets,
        &mut warnings,
    );

    if rows.is_empty() || rows.iter().all(|r| r.is_empty()) {
        return TableOutput {
            typst_source: String::new(),
            warnings,
        };
    }

    // Calculate column widths
    let column_count = calculate_column_count(&rows);
    let column_widths = calculate_column_widths(&rows, column_count, &mut warnings);

    // Generate Typst source
    let typst_source = generate_table_source(&rows, &column_widths, column_count);

    TableOutput {
        typst_source,
        warnings,
    }
}

/// Compute rowspan values for vertical merges.
fn compute_vertical_merges(table: &IrTableBlock) -> (RowspanMap, SkipCells) {
    let mut rowspan_map: RowspanMap = HashMap::new();
    let mut skip_cells: SkipCells = HashSet::new();

    // Track active vertical merges: col_index -> (start_row, remaining_span)
    let mut active_merges: HashMap<usize, (usize, usize)> = HashMap::new();

    for (row_idx, row) in table.rows.iter().enumerate() {
        for (col_idx, cell) in row.cells.iter().enumerate() {
            match &cell.merge {
                Some(CellMerge::VerticalStart) => {
                    // Count how many continuations follow
                    let mut span: u32 = 1;
                    for scan_row_idx in (row_idx + 1)..table.rows.len() {
                        let scan_row = &table.rows[scan_row_idx];
                        if col_idx < scan_row.cells.len() {
                            let scan_cell = &scan_row.cells[col_idx];
                            if matches!(scan_cell.merge, Some(CellMerge::VerticalContinue)) {
                                span += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    rowspan_map.insert((row_idx, col_idx), span);
                    if span > 1 {
                        active_merges.insert(col_idx, (row_idx, (span - 1) as usize));
                    }
                }
                Some(CellMerge::VerticalContinue) => {
                    skip_cells.insert((row_idx, col_idx));

                    // Decrement active merge counter
                    if let Some(&(start_row, remaining)) = active_merges.get(&col_idx) {
                        if remaining > 0 {
                            active_merges.insert(col_idx, (start_row, remaining - 1));
                        }
                        if remaining <= 1 {
                            active_merges.remove(&col_idx);
                        }
                    }
                }
                _ => {
                    active_merges.remove(&col_idx);
                }
            }
        }
    }

    (rowspan_map, skip_cells)
}

/// Map IR rows to cell info structures.
fn map_rows(
    rows: &[IrTableRow],
    rowspan_map: &RowspanMap,
    skip_cells: &SkipCells,
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> Vec<Vec<CellInfo>> {
    rows.iter()
        .enumerate()
        .map(|(row_idx, row)| map_row(row, row_idx, rowspan_map, skip_cells, assets, warnings))
        .collect()
}

/// Map a single IR row to cell info structures.
fn map_row(
    row: &IrTableRow,
    row_idx: usize,
    rowspan_map: &RowspanMap,
    skip_cells: &SkipCells,
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> Vec<CellInfo> {
    let mut cells = Vec::new();
    let mut expected_h_continuations = 0usize;

    for (col_idx, cell) in row.cells.iter().enumerate() {
        // Check if this cell should be skipped due to vertical merge
        if skip_cells.contains(&(row_idx, col_idx)) {
            continue;
        }

        match &cell.merge {
            Some(CellMerge::HorizontalStart { span }) if *span > 1 => {
                // Start of horizontal merge
                let rowspan = rowspan_map
                    .get(&(row_idx, col_idx))
                    .copied()
                    .unwrap_or(1)
                    .max(1);

                let cell_info = map_cell(cell, *span as u32, rowspan, assets, warnings);
                cells.push(cell_info);
                expected_h_continuations = (*span as usize).saturating_sub(1);
            }
            Some(CellMerge::HorizontalStart { .. }) => {
                // span=0/1 is not a real merge
                expected_h_continuations = 0;
                cells.push(map_cell(cell, 1, 1, assets, warnings));
            }
            Some(CellMerge::HorizontalContinue) if expected_h_continuations > 0 => {
                // Valid continuation - skip this cell
                expected_h_continuations -= 1;
            }
            Some(CellMerge::HorizontalContinue) => {
                // Orphan continuation - preserve content
                warnings.push(MappingWarning::OrphanHorizontalContinue);
                cells.push(map_cell(cell, 1, 1, assets, warnings));
            }
            Some(CellMerge::VerticalStart) | Some(CellMerge::VerticalContinue) => {
                // Vertical merge - emit with computed rowspan
                let rowspan = rowspan_map.get(&(row_idx, col_idx)).copied().unwrap_or(1);
                cells.push(map_cell(cell, 1, rowspan, assets, warnings));
            }
            _ => {
                // No merge
                expected_h_continuations = 0;
                cells.push(map_cell(cell, 1, 1, assets, warnings));
            }
        }
    }

    cells
}

/// Map a single IR cell to cell info.
fn map_cell(
    cell: &IrTableCell,
    colspan: u32,
    rowspan: u32,
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> CellInfo {
    // Map cell content
    let mut content_parts = Vec::new();
    for block in &cell.blocks {
        let block_output = map_block(block, assets);
        if !block_output.typst_source.is_empty() {
            content_parts.push(block_output.typst_source);
            warnings.extend(block_output.warnings);
        }
    }

    // Track dropped features
    if cell.v_align.is_some() {
        warnings.push(MappingWarning::CellVerticalAlignDropped);
    }

    // Extract fill color from shading (Slice A: flat fill only)
    // Slice B: Emit warning for pattern degradation
    let fill_color = cell.shading.as_ref().and_then(|s| {
        // Check if pattern is something other than Solid or Clear
        if let Some(ref pattern) = s.pattern
            && !matches!(pattern, ShadingPattern::Solid | ShadingPattern::Clear)
        {
            warnings.push(MappingWarning::PatternDegraded {
                context: "cell shading".to_string(),
                pattern: format!("{:?}", pattern),
            });
        }
        s.fill_color.clone()
    });

    // Format content for table cell
    let content = if content_parts.is_empty() {
        String::new()
    } else if content_parts.len() == 1 {
        content_parts.remove(0)
    } else {
        content_parts.join(" \\\n")
    };

    CellInfo {
        content,
        colspan,
        rowspan,
        width_twips: cell.width_twips,
        fill_color,
    }
}

/// Calculate the total number of columns in the table.
fn calculate_column_count(rows: &[Vec<CellInfo>]) -> usize {
    rows.iter()
        .map(|row| row.iter().map(|cell| cell.colspan as usize).sum())
        .max()
        .unwrap_or(0)
}

/// Calculate column widths from cell widths.
fn calculate_column_widths(
    rows: &[Vec<CellInfo>],
    column_count: usize,
    _warnings: &mut Vec<MappingWarning>,
) -> Vec<Option<f32>> {
    if column_count == 0 {
        return Vec::new();
    }

    // Collect widths for each column
    let mut column_widths: Vec<Option<i32>> = vec![None; column_count];

    for row in rows {
        let mut col_idx = 0;
        for cell in row {
            if let Some(width) = cell.width_twips {
                // Distribute width across spanned columns
                if cell.colspan == 1 && column_widths[col_idx].is_none() {
                    column_widths[col_idx] = Some(width);
                }
            }
            col_idx += cell.colspan as usize;
        }
    }

    // Check if any widths were specified
    let has_widths = column_widths.iter().any(|w| w.is_some());
    if !has_widths {
        return vec![None; column_count];
    }

    // Convert twips to relative widths (twips = 1/20 point)
    // For Typst, we'll use relative fractions
    column_widths
        .into_iter()
        .map(|w| w.map(|twips| twips as f32 / 1440.0)) // Convert to inches
        .collect()
}

/// Generate the Typst table source code.
fn generate_table_source(
    rows: &[Vec<CellInfo>],
    column_widths: &[Option<f32>],
    column_count: usize,
) -> String {
    if column_count == 0 {
        return String::new();
    }

    let mut lines = Vec::new();

    // Generate column specification
    let columns_spec = if column_widths.iter().all(|w| w.is_none()) {
        // All auto
        format!(
            "columns: ({})",
            (0..column_count)
                .map(|_| "auto")
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        // Mixed widths
        let specs: Vec<String> = column_widths
            .iter()
            .map(|w| match w {
                Some(inches) => format!("{}in", inches),
                None => "auto".to_string(),
            })
            .collect();
        format!("columns: ({})", specs.join(", "))
    };

    lines.push("#table(".to_string());
    lines.push(format!("  {},", columns_spec));
    lines.push("  rows: auto,".to_string());

    // Generate cells
    let mut cell_entries = Vec::new();
    for row in rows {
        for cell in row {
            // Add parameters if needed
            let mut params = Vec::new();
            if cell.colspan > 1 {
                params.push(format!("colspan: {}", cell.colspan));
            }
            if cell.rowspan > 1 {
                params.push(format!("rowspan: {}", cell.rowspan));
            }
            // Add fill parameter for cell shading
            if let Some(ref color) = cell.fill_color {
                params.push(format!("fill: rgb({}, {}, {})", color.r, color.g, color.b));
            }

            let cell_spec = if !params.is_empty() {
                format!("table.cell({}, [{}])", params.join(", "), cell.content)
            } else {
                format!("[{}]", cell.content)
            };

            cell_entries.push(cell_spec);
        }
    }

    // Add cells to output
    for (i, entry) in cell_entries.iter().enumerate() {
        if i == cell_entries.len() - 1 {
            lines.push(format!("  {}", entry));
        } else {
            lines.push(format!("  {},", entry));
        }
    }

    lines.push(")".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{
        Block as IrBlock, Paragraph, Run, Shading, TableCell as IrTableCell, TableRow as IrTableRow,
    };

    fn make_simple_table() -> IrTableBlock {
        IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
        ])])
    }

    #[test]
    fn test_map_empty_table() {
        let table = IrTableBlock::new();
        let output = map_table(&table);

        assert!(output.typst_source.is_empty());
    }

    #[test]
    fn test_map_simple_table() {
        let table = make_simple_table();
        let output = map_table(&table);

        assert!(output.typst_source.contains("#table("));
        assert!(output.typst_source.contains("[A]"));
        assert!(output.typst_source.contains("[B]"));
        assert!(output.typst_source.contains("columns:"));
    }

    #[test]
    fn test_map_table_multiple_rows() {
        let table = IrTableBlock::from_rows(vec![
            IrTableRow::from_cells(vec![
                IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A1")])),
                IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B1")])),
            ]),
            IrTableRow::from_cells(vec![
                IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A2")])),
                IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B2")])),
            ]),
        ]);

        let output = map_table(&table);

        assert!(output.typst_source.contains("[A1]"));
        assert!(output.typst_source.contains("[B1]"));
        assert!(output.typst_source.contains("[A2]"));
        assert!(output.typst_source.contains("[B2]"));
    }

    #[test]
    fn test_map_table_horizontal_merge() {
        let mut start_cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        start_cell.merge = Some(CellMerge::HorizontalStart { span: 2 });

        let mut continue_cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Hidden")]));
        continue_cell.merge = Some(CellMerge::HorizontalContinue);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![
            start_cell,
            continue_cell,
        ])]);

        let output = map_table(&table);

        // Should have colspan parameter
        assert!(output.typst_source.contains("colspan: 2"));
        // Hidden cell should not appear
        assert!(!output.typst_source.contains("Hidden"));
    }

    #[test]
    fn test_map_table_vertical_merge() {
        let mut top_cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top")]));
        top_cell.merge = Some(CellMerge::VerticalStart);

        let mut bottom_cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bottom")]));
        bottom_cell.merge = Some(CellMerge::VerticalContinue);

        let table = IrTableBlock::from_rows(vec![
            IrTableRow::from_cells(vec![top_cell]),
            IrTableRow::from_cells(vec![bottom_cell]),
        ]);

        let output = map_table(&table);

        // Should have rowspan parameter
        assert!(output.typst_source.contains("rowspan: 2"));
        // Bottom cell content should not appear
        assert!(!output.typst_source.contains("Bottom"));
    }

    #[test]
    fn test_map_table_vertical_merge_three_rows() {
        let mut top_cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        top_cell.merge = Some(CellMerge::VerticalStart);

        let mut mid_cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Mid")]));
        mid_cell.merge = Some(CellMerge::VerticalContinue);

        let mut bottom_cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bottom")]));
        bottom_cell.merge = Some(CellMerge::VerticalContinue);

        let right_cell1 = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R1")]));
        let right_cell2 = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R2")]));
        let right_cell3 = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R3")]));

        let table = IrTableBlock::from_rows(vec![
            IrTableRow::from_cells(vec![top_cell, right_cell1]),
            IrTableRow::from_cells(vec![mid_cell, right_cell2]),
            IrTableRow::from_cells(vec![bottom_cell, right_cell3]),
        ]);

        let output = map_table(&table);

        // Should have rowspan=3
        assert!(output.typst_source.contains("rowspan: 3"));
        // All right cells should appear
        assert!(output.typst_source.contains("[R1]"));
        assert!(output.typst_source.contains("[R2]"));
        assert!(output.typst_source.contains("[R3]"));
    }

    #[test]
    fn test_map_table_combined_merge() {
        // Cell with both horizontal and vertical merge
        let mut h_start =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("H-Merge")]));
        h_start.merge = Some(CellMerge::HorizontalStart { span: 3 });

        let mut h_cont1 =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cont1")]));
        h_cont1.merge = Some(CellMerge::HorizontalContinue);

        let mut h_cont2 =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cont2")]));
        h_cont2.merge = Some(CellMerge::HorizontalContinue);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![
            h_start, h_cont1, h_cont2,
        ])]);

        let output = map_table(&table);

        assert!(output.typst_source.contains("colspan: 3"));
    }

    #[test]
    fn test_map_table_orphan_horizontal_continue() {
        let start = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Alpha")]));

        let mut orphan = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bravo")]));
        orphan.merge = Some(CellMerge::HorizontalContinue);

        let end = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Charlie")]));

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![start, orphan, end])]);

        let output = map_table(&table);

        // All three cells should be present
        assert!(output.typst_source.contains("[Alpha]"));
        assert!(output.typst_source.contains("[Bravo]"));
        assert!(output.typst_source.contains("[Charlie]"));

        // Should warn about orphan
        assert!(
            output
                .warnings
                .iter()
                .any(|w| matches!(w, crate::map::MappingWarning::OrphanHorizontalContinue))
        );
    }

    #[test]
    fn test_map_table_with_cell_width() {
        let cell = IrTableCell::from_paragraph_with_width(
            Paragraph::from_runs(vec![Run::new("Cell")]),
            2880, // 2 inches
        );

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Should include width specification
        assert!(output.typst_source.contains("2in") || output.typst_source.contains("auto"));
    }

    #[test]
    fn test_map_table_empty_cell() {
        let cell = IrTableCell::new();

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Empty cells should still produce a cell entry
        assert!(output.typst_source.contains("[]"));
    }

    #[test]
    fn test_map_table_with_nested_content() {
        use rtfkit_core::{ListBlock as IrListBlock, ListItem, ListKind as IrListKind};

        let mut list = IrListBlock::new(1, IrListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item")]),
        ));

        let cell = IrTableCell::from_blocks(vec![IrBlock::ListBlock(list)], None);
        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Should contain the list content
        assert!(output.typst_source.contains("- Item"));
    }

    #[test]
    fn test_determinism() {
        let table = make_simple_table();

        // Run multiple times to verify determinism
        let output1 = map_table(&table);
        let output2 = map_table(&table);
        let output3 = map_table(&table);

        assert_eq!(output1.typst_source, output2.typst_source);
        assert_eq!(output2.typst_source, output3.typst_source);
    }

    #[test]
    fn test_complex_table_determinism() {
        let mut start_cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        start_cell.merge = Some(CellMerge::HorizontalStart { span: 2 });

        let mut continue_cell = IrTableCell::new();
        continue_cell.merge = Some(CellMerge::HorizontalContinue);

        let mut v_start = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("V")]));
        v_start.merge = Some(CellMerge::VerticalStart);

        let mut v_cont = IrTableCell::new();
        v_cont.merge = Some(CellMerge::VerticalContinue);

        let table = IrTableBlock::from_rows(vec![
            IrTableRow::from_cells(vec![start_cell, continue_cell, v_start]),
            IrTableRow::from_cells(vec![
                IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
                IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
                v_cont,
            ]),
        ]);

        let output1 = map_table(&table);
        let output2 = map_table(&table);

        assert_eq!(output1.typst_source, output2.typst_source);
    }

    // =============================================================================
    // Cell Shading Tests
    // =============================================================================

    #[test]
    fn test_map_table_cell_shading() {
        let mut cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Shaded")]));
        cell.shading = Some(Shading::solid(Color::new(255, 255, 0))); // Yellow

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Should have fill parameter
        assert!(output.typst_source.contains("fill: rgb(255, 255, 0)"));
        assert!(output.typst_source.contains("[Shaded]"));
    }

    #[test]
    fn test_map_table_cell_shading_with_merge() {
        let mut cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 2 });
        cell.shading = Some(Shading::solid(Color::new(0, 128, 255))); // Blue

        let mut cont_cell = IrTableCell::new();
        cont_cell.merge = Some(CellMerge::HorizontalContinue);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell, cont_cell])]);

        let output = map_table(&table);

        // Should have both colspan and fill parameters
        assert!(output.typst_source.contains("colspan: 2"));
        assert!(output.typst_source.contains("fill: rgb(0, 128, 255)"));
    }

    #[test]
    fn test_map_table_cell_without_shading() {
        let cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal")]));

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Should NOT have fill parameter
        assert!(!output.typst_source.contains("fill:"));
        assert!(output.typst_source.contains("[Normal]"));
    }

    #[test]
    fn test_map_table_cell_shading_empty_fill_color() {
        let mut cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Text")]));
        cell.shading = Some(Shading::new()); // Empty shading

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Should NOT have fill parameter
        assert!(!output.typst_source.contains("fill:"));
        assert!(output.typst_source.contains("[Text]"));
    }

    #[test]
    fn test_map_table_cell_shading_deterministic() {
        let mut cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Test")]));
        cell.shading = Some(Shading::solid(Color::new(128, 64, 32)));

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        // Run multiple times to verify determinism
        let output1 = map_table(&table);
        let output2 = map_table(&table);
        let output3 = map_table(&table);

        assert_eq!(output1.typst_source, output2.typst_source);
        assert_eq!(output2.typst_source, output3.typst_source);
        assert!(output1.typst_source.contains("fill: rgb(128, 64, 32)"));
    }

    // =============================================================================
    // Pattern Degradation Tests (Slice B)
    // =============================================================================

    #[test]
    fn test_map_table_cell_with_patterned_shading_degrades() {
        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 255)); // White background
        shading.pattern_color = Some(Color::new(0, 0, 0)); // Black foreground
        shading.pattern = Some(ShadingPattern::Percent25);

        let mut cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Patterned")]));
        cell.shading = Some(shading);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Pattern should be degraded - only fill_color emitted
        assert!(output.typst_source.contains("fill: rgb(255, 255, 255)"));
        assert!(!output.typst_source.contains("0, 0, 0")); // Pattern color not emitted

        // Should have a warning about pattern degradation
        assert_eq!(output.warnings.len(), 1);
        assert!(matches!(
            &output.warnings[0],
            MappingWarning::PatternDegraded { context, .. } if context == "cell shading"
        ));
    }

    #[test]
    fn test_map_table_cell_with_horz_stripe_pattern_degrades() {
        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(200, 200, 200)); // Light gray
        shading.pattern_color = Some(Color::new(100, 100, 100)); // Dark gray
        shading.pattern = Some(ShadingPattern::HorzStripe);

        let mut cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Striped")]));
        cell.shading = Some(shading);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Only fill_color should be emitted
        assert!(output.typst_source.contains("fill: rgb(200, 200, 200)"));

        // Should have a warning
        assert_eq!(output.warnings.len(), 1);
        if let MappingWarning::PatternDegraded { pattern, .. } = &output.warnings[0] {
            assert!(pattern.contains("HorzStripe"));
        } else {
            panic!("Expected PatternDegraded warning");
        }
    }

    #[test]
    fn test_map_table_cell_with_solid_pattern_no_warning() {
        // Solid pattern should not emit a warning
        let mut cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Solid")]));
        cell.shading = Some(Shading::solid(Color::new(255, 255, 0)));

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Should emit fill color
        assert!(output.typst_source.contains("fill: rgb(255, 255, 0)"));

        // Should NOT have a warning - Solid is supported
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_table_cell_with_clear_pattern_no_warning() {
        // Clear pattern should not emit a warning
        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(200, 200, 255));
        shading.pattern = Some(ShadingPattern::Clear);

        let mut cell = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Clear")]));
        cell.shading = Some(shading);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Should emit fill color
        assert!(output.typst_source.contains("fill: rgb(200, 200, 255)"));

        // Should NOT have a warning - Clear is supported
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_table_cell_with_diag_cross_pattern_degrades() {
        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 0)); // Yellow
        shading.pattern_color = Some(Color::new(255, 0, 0)); // Red
        shading.pattern = Some(ShadingPattern::DiagCross);

        let mut cell =
            IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Crosshatch")]));
        cell.shading = Some(shading);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell])]);

        let output = map_table(&table);

        // Only fill_color should be emitted, pattern ignored
        assert!(output.typst_source.contains("fill: rgb(255, 255, 0)"));
        assert!(!output.typst_source.contains("255, 0, 0")); // Pattern color not emitted

        // Should have a warning
        assert_eq!(output.warnings.len(), 1);
        if let MappingWarning::PatternDegraded { pattern, .. } = &output.warnings[0] {
            assert!(pattern.contains("DiagCross"));
        } else {
            panic!("Expected PatternDegraded warning");
        }
    }

    #[test]
    fn test_map_table_multiple_cells_with_patterns() {
        let mut shading1 = Shading::new();
        shading1.fill_color = Some(Color::new(255, 0, 0));
        shading1.pattern = Some(ShadingPattern::Percent10);

        let mut shading2 = Shading::new();
        shading2.fill_color = Some(Color::new(0, 255, 0));
        shading2.pattern = Some(ShadingPattern::Percent20);

        let mut cell1 = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")]));
        cell1.shading = Some(shading1);

        let mut cell2 = IrTableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")]));
        cell2.shading = Some(shading2);

        let table = IrTableBlock::from_rows(vec![IrTableRow::from_cells(vec![cell1, cell2])]);

        let output = map_table(&table);

        // Both fill colors should be emitted
        assert!(output.typst_source.contains("fill: rgb(255, 0, 0)"));
        assert!(output.typst_source.contains("fill: rgb(0, 255, 0)"));

        // Should have two warnings (one per cell with pattern)
        assert_eq!(output.warnings.len(), 2);
    }
}
