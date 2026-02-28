//! Table block HTML emission.
//!
//! This module handles converting IR tables to HTML following the spec in PHASE_HTML.md Section 7.4.
//!
//! ## Mapping Summary
//!
//! | IR | HTML |
//! |---|---|
//! | `TableBlock` | `<table class="rtf-table">` |
//! | row | `<tr>` |
//! | cell | `<td>` |
//! | `width_twips` | inline style `width: Npt` |
//! | horizontal merge | `colspan` |
//! | vertical merge | `rowspan` |
//! | `CellVerticalAlign` | class `rtf-valign-top|middle|bottom` |
//! | `shading` | inline style `background-color: #rrggbb` |

use super::paragraph::{build_paragraph_style, paragraph_to_html};
use crate::serialize::HtmlBuffer;
use rtfkit_core::{
    Block, Border, BorderStyle, CellMerge, CellVerticalAlign, TableBlock, TableCell, TableRow,
    resolve_effective_cell_borders,
};
use std::collections::HashSet;

/// Converts twips to points.
/// 1 point = 20 twips.
fn twips_to_points(twips: i32) -> f32 {
    twips as f32 / 20.0
}

/// Computes rowspan values for vertical merges by scanning the table structure.
///
/// Returns a map from (row_index, col_index) to the computed rowspan value,
/// and a set of (row_index, col_index) positions that should be skipped
/// because they are covered by a vertical merge from above.
fn compute_vertical_merges(table: &TableBlock) -> (Vec<Vec<usize>>, HashSet<(usize, usize)>) {
    let mut rowspan_map: Vec<Vec<usize>> = Vec::new();
    let mut skip_cells: HashSet<(usize, usize)> = HashSet::new();

    // Track active vertical merges: (col_index, start_row, remaining_span)
    // We scan row by row to count continuations
    let mut active_merges: Vec<Option<(usize, usize)>> = Vec::new(); // (start_row, current_span_count)

    for (row_idx, row) in table.rows.iter().enumerate() {
        let mut row_rowspan: Vec<usize> = Vec::new();

        // Ensure active_merges has enough slots
        if active_merges.len() < row.cells.len() {
            active_merges.resize(row.cells.len(), None);
        }

        for (col_idx, cell) in row.cells.iter().enumerate() {
            match cell.merge {
                Some(CellMerge::VerticalStart) => {
                    // Start of a vertical merge - count how many continuations follow
                    let mut span = 1;
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
                    row_rowspan.push(span);
                    if span > 1 {
                        active_merges[col_idx] = Some((row_idx, span - 1));
                    }
                }
                Some(CellMerge::VerticalContinue) => {
                    // This cell is covered by a vertical merge from above
                    skip_cells.insert((row_idx, col_idx));
                    row_rowspan.push(0); // Placeholder, won't be used

                    // Decrement the active merge counter
                    if let Some((start_row, remaining)) = active_merges[col_idx] {
                        if remaining > 0 {
                            active_merges[col_idx] = Some((start_row, remaining - 1));
                        }
                        if remaining <= 1 {
                            active_merges[col_idx] = None;
                        }
                    }
                }
                _ => {
                    // No vertical merge
                    row_rowspan.push(0);
                    active_merges[col_idx] = None;
                }
            }
        }

        rowspan_map.push(row_rowspan);
    }

    (rowspan_map, skip_cells)
}

/// Converts a table block to HTML.
///
/// Emits a `<table class="rtf-table">` element with `<tr>` and `<td>` children.
/// Handles colspan, rowspan, width styling, and vertical alignment classes.
pub fn table_to_html(table: &TableBlock, buf: &mut HtmlBuffer) {
    let mut dropped_reasons = Vec::new();
    table_to_html_with_warnings(table, buf, &mut dropped_reasons);
}

/// Converts a table block to HTML while recording semantic degradations.
pub fn table_to_html_with_warnings(
    table: &TableBlock,
    buf: &mut HtmlBuffer,
    dropped_reasons: &mut Vec<String>,
) {
    buf.push_open_tag("table", &[("class", "rtf-table")]);

    // Pre-compute vertical merge information
    let (rowspan_map, skip_cells) = compute_vertical_merges(table);

    // Emit rows with optional thead/tbody wrappers.
    // Splitting only applies when the first row has no rowspan that would cross section boundaries.
    let can_split_header_body = table.rows.len() > 1
        && rowspan_map
            .first()
            .is_some_and(|row| row.iter().all(|&span| span <= 1));

    if can_split_header_body {
        buf.push_raw("<thead>");
        row_to_html(
            table,
            &table.rows[0],
            0,
            &rowspan_map,
            &skip_cells,
            buf,
            dropped_reasons,
        );
        buf.push_raw("</thead>");

        buf.push_raw("<tbody>");
        for (row_idx, row) in table.rows.iter().enumerate().skip(1) {
            row_to_html(
                table,
                row,
                row_idx,
                &rowspan_map,
                &skip_cells,
                buf,
                dropped_reasons,
            );
        }
        buf.push_raw("</tbody>");
    } else {
        for (row_idx, row) in table.rows.iter().enumerate() {
            row_to_html(
                table,
                row,
                row_idx,
                &rowspan_map,
                &skip_cells,
                buf,
                dropped_reasons,
            );
        }
    }

    buf.push_close_tag("table");
}

/// Converts a table row to HTML.
///
/// Handles horizontal merge normalization by skipping continuation cells
/// that are covered by a start cell's colspan, and vertical merge skipping.
fn row_to_html(
    table: &TableBlock,
    row: &TableRow,
    row_idx: usize,
    rowspan_map: &[Vec<usize>],
    skip_cells: &HashSet<(usize, usize)>,
    buf: &mut HtmlBuffer,
    dropped_reasons: &mut Vec<String>,
) {
    buf.push_open_tag("tr", &[]);

    let mut expected_h_continuations = 0usize;

    for (col_idx, cell) in row.cells.iter().enumerate() {
        // Check if this cell should be skipped due to vertical merge
        if skip_cells.contains(&(row_idx, col_idx)) {
            continue;
        }

        match cell.merge {
            Some(CellMerge::HorizontalStart { span }) if span > 1 => {
                // Start of horizontal merge - emit the cell with colspan
                let rowspan = rowspan_map
                    .get(row_idx)
                    .and_then(|r| r.get(col_idx))
                    .copied()
                    .unwrap_or(0);
                cell_to_html(table, row_idx, col_idx, cell, rowspan, buf, dropped_reasons);
                expected_h_continuations = span.saturating_sub(1) as usize;
            }
            Some(CellMerge::HorizontalStart { .. }) => {
                // Defensive: span=0/1 is not a real merge, emit as standalone
                expected_h_continuations = 0;
                let mut standalone = cell.clone();
                standalone.merge = None;
                cell_to_html(
                    table,
                    row_idx,
                    col_idx,
                    &standalone,
                    0,
                    buf,
                    dropped_reasons,
                );
            }
            Some(CellMerge::HorizontalContinue) if expected_h_continuations > 0 => {
                // Valid continuation - skip this cell (covered by colspan)
                expected_h_continuations -= 1;
            }
            Some(CellMerge::HorizontalContinue) => {
                // Orphan continuation - preserve content rather than silently dropping
                let mut standalone = cell.clone();
                standalone.merge = None;
                cell_to_html(
                    table,
                    row_idx,
                    col_idx,
                    &standalone,
                    0,
                    buf,
                    dropped_reasons,
                );
            }
            Some(CellMerge::VerticalStart) | Some(CellMerge::VerticalContinue) => {
                // Vertical merge - emit with computed rowspan (already handled skip_cells above)
                let rowspan = rowspan_map
                    .get(row_idx)
                    .and_then(|r| r.get(col_idx))
                    .copied()
                    .unwrap_or(0);
                cell_to_html(table, row_idx, col_idx, cell, rowspan, buf, dropped_reasons);
            }
            _ => {
                // No merge - emit normally
                expected_h_continuations = 0;
                cell_to_html(table, row_idx, col_idx, cell, 0, buf, dropped_reasons);
            }
        }
    }

    buf.push_close_tag("tr");
}

/// Converts a single `Border` side to a CSS `border-{side}` declaration.
fn border_side_to_css(side: &str, border: &Border) -> String {
    if border.style == BorderStyle::None {
        return format!("border-{side}: none");
    }
    let width_pt = border
        .width_half_pts
        .map(|hp| hp as f32 / 2.0)
        .unwrap_or(0.5);
    let style_str = match border.style {
        BorderStyle::None => unreachable!(),
        BorderStyle::Single => "solid",
        BorderStyle::Double => "double",
        BorderStyle::Dotted => "dotted",
        BorderStyle::Dashed => "dashed",
    };
    let color = border
        .color
        .as_ref()
        .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
        .unwrap_or_else(|| "currentColor".to_string());
    format!("border-{side}: {width_pt:.1}pt {style_str} {color}")
}

/// Converts a table cell to HTML.
///
/// Handles:
/// - `colspan` for horizontal merges
/// - `rowspan` for vertical merges (computed from VerticalStart/VerticalContinue)
/// - `style="width: Npt"` for explicit widths
/// - `class="rtf-valign-*"` for vertical alignment
/// - `style="background-color: #rrggbb"` for cell shading
/// - `style="border-{side}: ..."` for cell borders
fn cell_to_html(
    table: &TableBlock,
    row_idx: usize,
    col_idx: usize,
    cell: &TableCell,
    rowspan: usize,
    buf: &mut HtmlBuffer,
    dropped_reasons: &mut Vec<String>,
) {
    let mut attrs: Vec<(&str, String)> = Vec::new();
    let mut classes: Vec<&'static str> = Vec::new();
    let mut style_parts: Vec<String> = Vec::new();

    // Handle vertical alignment class
    if let Some(v_align) = cell.v_align {
        let class_name = match v_align {
            CellVerticalAlign::Top => "rtf-valign-top",
            CellVerticalAlign::Center => "rtf-valign-middle",
            CellVerticalAlign::Bottom => "rtf-valign-bottom",
        };
        classes.push(class_name);
    }

    // Handle colspan for horizontal merges
    if let Some(CellMerge::HorizontalStart { span }) = cell.merge
        && span > 1
    {
        attrs.push(("colspan", span.to_string()));
    }

    // Handle rowspan for vertical merges (pre-computed value)
    if rowspan > 1 {
        attrs.push(("rowspan", rowspan.to_string()));
    }

    // Handle width styling
    if let Some(width_twips) = cell.width_twips
        && width_twips > 0
    {
        let points = twips_to_points(width_twips);
        style_parts.push(format!("width: {:.1}pt", points));
    }

    // Handle cell shading (background color)
    let shading_style = build_paragraph_style(cell.shading.as_ref());
    if !shading_style.is_empty() {
        // Remove trailing semicolon from build_paragraph_style output
        // and add to style parts
        style_parts.push(shading_style.trim_end_matches(';').to_string());
    }

    // Handle cell borders
    if let Some(ref borders) = resolve_effective_cell_borders(table, row_idx, col_idx) {
        let sides: [(&str, Option<&Border>); 4] = [
            ("top", borders.top.as_ref()),
            ("left", borders.left.as_ref()),
            ("bottom", borders.bottom.as_ref()),
            ("right", borders.right.as_ref()),
        ];
        for (side, maybe_border) in sides {
            if let Some(b) = maybe_border {
                style_parts.push(border_side_to_css(side, b));
            }
        }
    }

    // Combine style parts into a single style attribute
    if !style_parts.is_empty() {
        attrs.push(("style", style_parts.join("; ") + ";"));
    }

    // Combine classes into a single attribute
    if !classes.is_empty() {
        let class_str = classes.join(" ");
        attrs.insert(0, ("class", class_str));
    }

    // Convert attrs to the format expected by push_open_tag
    let attrs_ref: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();

    buf.push_open_tag("td", &attrs_ref);

    // Emit cell content
    for block in &cell.blocks {
        match block {
            Block::Paragraph(para) => paragraph_to_html(para, buf),
            Block::ListBlock(list) => {
                super::list::list_to_html_with_warnings(list, buf, dropped_reasons)
            }
            Block::TableBlock(nested) => table_to_html_with_warnings(nested, buf, dropped_reasons),
            Block::ImageBlock(image) => super::image::image_to_html(image, buf),
        }
    }

    buf.push_close_tag("td");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{Paragraph, Run, TableCell, TableRow};

    #[test]
    fn empty_table() {
        let table = TableBlock::new();
        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        assert_eq!(buf.as_str(), r#"<table class="rtf-table"></table>"#);
    }

    #[test]
    fn simple_2x2_table() {
        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
            ]),
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("C")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("D")])),
            ]),
        ]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.starts_with(r#"<table class="rtf-table">"#));
        assert!(html.ends_with("</table>"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("</thead>"));
        assert!(html.contains("<tbody>"));
        assert!(html.contains("</tbody>"));
        assert!(html.contains("<tr>"));
        assert!(html.contains("</tr>"));
        assert!(html.contains("<td>"));
        assert!(html.contains("</td>"));
        assert!(html.contains("A"));
        assert!(html.contains("B"));
        assert!(html.contains("C"));
        assert!(html.contains("D"));
    }

    #[test]
    fn table_with_horizontal_merge_colspan() {
        let mut start_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        start_cell.merge = Some(CellMerge::HorizontalStart { span: 2 });

        let mut continue_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Hidden")]));
        continue_cell.merge = Some(CellMerge::HorizontalContinue);

        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![start_cell, continue_cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should have colspan="2" on the start cell
        assert!(html.contains(r#"colspan="2""#));
        // Should contain "Merged" but not "Hidden" (continuation is skipped)
        assert!(html.contains("Merged"));
        assert!(!html.contains("Hidden"));
    }

    #[test]
    fn table_with_vertical_merge_rowspan_single_row() {
        // Single row with VerticalStart but no continuations - no rowspan needed
        let mut top_cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top")]));
        top_cell.merge = Some(CellMerge::VerticalStart);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![top_cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Single row with no continuations - no rowspan attribute needed
        assert!(!html.contains("rowspan"));
        assert!(html.contains("Top"));
    }

    #[test]
    fn table_with_vertical_merge_rowspan_multi_row() {
        // Two rows with vertical merge - should have rowspan="2"
        let mut top_cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top")]));
        top_cell.merge = Some(CellMerge::VerticalStart);

        let mut bottom_cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("")]));
        bottom_cell.merge = Some(CellMerge::VerticalContinue);

        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![top_cell]),
            TableRow::from_cells(vec![bottom_cell]),
        ]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should have rowspan="2" on the vertical start cell
        assert!(html.contains(r#"rowspan="2""#));
        assert!(html.contains("Top"));
        assert!(!html.contains("<thead>"));
        assert!(!html.contains("<tbody>"));
        // Bottom cell content should not appear (it's skipped)
        // Note: empty string won't show anyway, but the cell is properly skipped
    }

    #[test]
    fn table_with_vertical_merge_three_rows() {
        // Three rows with vertical merge - should have rowspan="3"
        let mut top_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        top_cell.merge = Some(CellMerge::VerticalStart);

        let mut mid_cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Mid")]));
        mid_cell.merge = Some(CellMerge::VerticalContinue);

        let mut bottom_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bottom")]));
        bottom_cell.merge = Some(CellMerge::VerticalContinue);

        let right_cell1 = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R1")]));
        let right_cell2 = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R2")]));
        let right_cell3 = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R3")]));

        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![top_cell, right_cell1]),
            TableRow::from_cells(vec![mid_cell, right_cell2]),
            TableRow::from_cells(vec![bottom_cell, right_cell3]),
        ]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should have rowspan="3" on the vertical start cell
        assert!(html.contains(r#"rowspan="3""#));
        assert!(html.contains("Merged"));
        assert!(html.contains("R1"));
        assert!(html.contains("R2"));
        assert!(html.contains("R3"));
        assert!(!html.contains("<thead>"));
        assert!(!html.contains("<tbody>"));
        // Mid and Bottom content should not appear (continuation cells are skipped)
        assert!(!html.contains(">Mid<"));
        assert!(!html.contains(">Bottom<"));
    }

    #[test]
    fn table_with_mixed_merge() {
        // Cell with both horizontal and vertical merge semantics
        // Note: In the current IR, a cell can only have one merge type at a time
        // This test verifies horizontal merge takes precedence in emission
        let mut h_start =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("H-Merge")]));
        h_start.merge = Some(CellMerge::HorizontalStart { span: 3 });

        let mut h_cont1 = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cont1")]));
        h_cont1.merge = Some(CellMerge::HorizontalContinue);

        let mut h_cont2 = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cont2")]));
        h_cont2.merge = Some(CellMerge::HorizontalContinue);

        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![h_start, h_cont1, h_cont2])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains(r#"colspan="3""#));
        assert!(html.contains("H-Merge"));
        assert!(!html.contains("Cont1"));
        assert!(!html.contains("Cont2"));
    }

    #[test]
    fn table_with_vertical_alignment_classes() {
        let mut top_cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top")]));
        top_cell.v_align = Some(CellVerticalAlign::Top);

        let mut middle_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Middle")]));
        middle_cell.v_align = Some(CellVerticalAlign::Center);

        let mut bottom_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bottom")]));
        bottom_cell.v_align = Some(CellVerticalAlign::Bottom);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            top_cell,
            middle_cell,
            bottom_cell,
        ])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains(r#"class="rtf-valign-top""#));
        assert!(html.contains(r#"class="rtf-valign-middle""#));
        assert!(html.contains(r#"class="rtf-valign-bottom""#));
    }

    #[test]
    fn table_with_width_styling() {
        let cell = TableCell::from_paragraph_with_width(
            Paragraph::from_runs(vec![Run::new("Width")]),
            1440, // 1 inch = 72 points (1440 twips / 20)
        );

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // 1440 twips = 72.0 points (style now ends with semicolon)
        assert!(html.contains(r#"style="width: 72.0pt;""#));
    }

    #[test]
    fn table_with_zero_width_no_style() {
        let cell = TableCell::from_paragraph_with_width(
            Paragraph::from_runs(vec![Run::new("No Width")]),
            0, // Zero width should not emit style
        );

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should not have style attribute for zero width
        assert!(!html.contains("style="));
    }

    #[test]
    fn orphan_horizontal_continue_preserves_text() {
        // Orphan continuation (no horizontal start before it) should preserve content
        let start = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Alpha")]));

        let mut orphan = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bravo")]));
        orphan.merge = Some(CellMerge::HorizontalContinue);

        let end = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Charlie")]));

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![start, orphan, end])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // All content should be preserved
        assert!(html.contains("Alpha"));
        assert!(html.contains("Bravo"));
        assert!(html.contains("Charlie"));
    }

    #[test]
    fn table_with_list_in_cell() {
        use rtfkit_core::{ListBlock, ListItem, ListKind};

        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 2")]),
        ));

        let cell = TableCell::from_blocks(vec![Block::ListBlock(list)], None);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains("<ul"));
        assert!(html.contains("Item 1"));
        assert!(html.contains("Item 2"));
    }

    #[test]
    fn nested_table() {
        // Create inner table
        let inner_table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Inner")]),
            )])]);

        // Create outer table with nested table
        let outer_table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Outer")])),
            TableCell::from_blocks(vec![Block::TableBlock(inner_table)], None),
        ])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&outer_table, &mut buf);
        let html = buf.as_str();

        // Should have two tables
        assert!(html.contains("Outer"));
        assert!(html.contains("Inner"));
        // Count table tags - should have 2 opening and 2 closing
        assert_eq!(html.matches(r#"<table class="rtf-table">"#).count(), 2);
        assert_eq!(html.matches("</table>").count(), 2);
    }

    #[test]
    fn malformed_table_missing_row_terminator() {
        // Test that malformed tables still produce valid HTML
        // The IR should already have normalized this, but we test robustness
        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Cell")]),
            )])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains("<tr>"));
        assert!(html.contains("</tr>"));
        assert!(html.contains("Cell"));
    }

    #[test]
    fn table_cell_with_multiple_blocks() {
        let cell = TableCell::from_blocks(
            vec![
                Block::Paragraph(Paragraph::from_runs(vec![Run::new("Para 1")])),
                Block::Paragraph(Paragraph::from_runs(vec![Run::new("Para 2")])),
            ],
            None,
        );

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains("Para 1"));
        assert!(html.contains("Para 2"));
    }

    #[test]
    fn combined_merge_and_alignment() {
        // Cell with both merge and vertical alignment
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Combined")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 2 });
        cell.v_align = Some(CellVerticalAlign::Center);

        let mut cont = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Hidden")]));
        cont.merge = Some(CellMerge::HorizontalContinue);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell, cont])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should have both class and colspan
        assert!(html.contains(r#"class="rtf-valign-middle""#));
        assert!(html.contains(r#"colspan="2""#));
        assert!(html.contains("Combined"));
        assert!(!html.contains("Hidden"));
    }

    #[test]
    fn twips_to_points_conversion() {
        assert_eq!(twips_to_points(20), 1.0);
        assert_eq!(twips_to_points(1440), 72.0); // 1 inch
        assert_eq!(twips_to_points(720), 36.0); // 0.5 inch
        assert_eq!(twips_to_points(0), 0.0);
    }

    // =========================================================================
    // Cell shading tests
    // =========================================================================

    #[test]
    fn table_cell_with_shading() {
        use rtfkit_core::{Color, Shading};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Shaded")]));
        cell.shading = Some(Shading::solid(Color::new(255, 255, 0))); // Yellow

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains(r#"style="background-color: #ffff00;""#));
        assert!(html.contains("Shaded"));
    }

    #[test]
    fn table_cell_with_shading_and_width() {
        use rtfkit_core::{Color, Shading};

        let mut cell = TableCell::from_paragraph_with_width(
            Paragraph::from_runs(vec![Run::new("Styled")]),
            1440, // 72pt
        );
        cell.shading = Some(Shading::solid(Color::new(0, 128, 255))); // Blue

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Both width and background-color in style, deterministic order
        assert!(html.contains(r#"style="width: 72.0pt; background-color: #0080ff;""#));
        assert!(html.contains("Styled"));
    }

    #[test]
    fn table_cell_with_shading_and_alignment() {
        use rtfkit_core::{Color, Shading};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Combined")]));
        cell.shading = Some(Shading::solid(Color::new(255, 0, 0))); // Red
        cell.v_align = Some(CellVerticalAlign::Center);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should have both class and style
        assert!(html.contains(r#"class="rtf-valign-middle""#));
        assert!(html.contains(r#"style="background-color: #ff0000;""#));
        assert!(html.contains("Combined"));
    }

    #[test]
    fn table_cell_without_shading_no_style() {
        let cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal")]));

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should NOT have style attribute
        assert!(!html.contains("style="));
        assert!(html.contains("Normal"));
    }

    #[test]
    fn table_cell_shading_deterministic() {
        use rtfkit_core::{Color, Shading};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Test")]));
        cell.shading = Some(Shading::solid(Color::new(128, 64, 32)));

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        // Generate HTML multiple times
        let mut buf1 = HtmlBuffer::new();
        table_to_html(&table, &mut buf1);

        let mut buf2 = HtmlBuffer::new();
        table_to_html(&table, &mut buf2);

        // Output should be identical
        assert_eq!(buf1.as_str(), buf2.as_str());
        assert!(buf1.as_str().contains("background-color: #804020"));
    }

    // =========================================================================
    // Pattern degradation tests (Slice B)
    // =========================================================================

    #[test]
    fn table_cell_with_percent_patterned_shading_uses_blended_fill() {
        use rtfkit_core::{Color, Shading, ShadingPattern};

        // Create a patterned shading (25% pattern with black on white)
        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 255)); // White background
        shading.pattern_color = Some(Color::new(0, 0, 0)); // Black foreground
        shading.pattern = Some(ShadingPattern::Percent25);

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Patterned")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Percent pattern should be approximated as blended fill.
        assert!(html.contains(r#"style="background-color: #bfbfbf;""#));
        assert!(html.contains("Patterned"));
    }

    #[test]
    fn table_cell_with_horz_stripe_pattern_degrades_to_flat_fill() {
        use rtfkit_core::{Color, Shading, ShadingPattern};

        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(200, 200, 200)); // Light gray
        shading.pattern_color = Some(Color::new(100, 100, 100)); // Dark gray
        shading.pattern = Some(ShadingPattern::HorzStripe);

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Striped")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Only fill_color should be emitted
        assert!(html.contains(r#"style="background-color: #c8c8c8;""#));
        assert!(!html.contains(&format!("#{}", "646464"))); // Pattern color not emitted
    }

    #[test]
    fn table_cell_with_diag_cross_pattern_degrades_to_flat_fill() {
        use rtfkit_core::{Color, Shading, ShadingPattern};

        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 0)); // Yellow
        shading.pattern_color = Some(Color::new(255, 0, 0)); // Red
        shading.pattern = Some(ShadingPattern::DiagCross);

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Crosshatch")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Only fill_color should be emitted, pattern ignored
        assert!(html.contains(r#"style="background-color: #ffff00;""#));
        assert!(!html.contains(&format!("#{}", "ff0000"))); // Pattern color not emitted
    }

    #[test]
    fn table_cell_pattern_with_width_and_alignment() {
        use rtfkit_core::{Color, Shading, ShadingPattern};

        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(0, 128, 0)); // Green
        shading.pattern_color = Some(Color::new(255, 255, 255)); // White
        shading.pattern = Some(ShadingPattern::Percent50);

        let mut cell = TableCell::from_paragraph_with_width(
            Paragraph::from_runs(vec![Run::new("Complex")]),
            720,
        ); // 36pt
        cell.shading = Some(shading);
        cell.v_align = Some(CellVerticalAlign::Center);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Should have class, width, and blended background-color.
        assert!(html.contains(r#"class="rtf-valign-middle""#));
        assert!(html.contains(r#"width: 36.0pt"#));
        assert!(html.contains(r#"background-color: #80c080"#));
    }

    // =========================================================================
    // Cell border CSS tests
    // =========================================================================

    #[test]
    fn cell_with_single_top_border_emits_css() {
        use rtfkit_core::{Border, BorderSet, BorderStyle};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Border")]));
        cell.borders = Some(BorderSet {
            top: Some(Border {
                style: BorderStyle::Single,
                width_half_pts: Some(4),
                color: None,
            }),
            ..Default::default()
        });

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains("border-top: 2.0pt solid currentColor"));
        assert!(!html.contains("border-left"));
        assert!(!html.contains("border-bottom"));
        assert!(!html.contains("border-right"));
    }

    #[test]
    fn cell_border_none_style_emits_border_none() {
        use rtfkit_core::{Border, BorderSet, BorderStyle};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("None")]));
        cell.borders = Some(BorderSet {
            top: Some(Border {
                style: BorderStyle::None,
                width_half_pts: None,
                color: None,
            }),
            ..Default::default()
        });

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains("border-top: none"));
    }

    #[test]
    fn cell_border_with_color_and_width() {
        use rtfkit_core::{Border, BorderSet, BorderStyle, Color};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Color")]));
        cell.borders = Some(BorderSet {
            right: Some(Border {
                style: BorderStyle::Dashed,
                width_half_pts: Some(6),
                color: Some(Color::new(255, 0, 0)),
            }),
            ..Default::default()
        });

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains("border-right: 3.0pt dashed #ff0000"));
    }

    #[test]
    fn cell_border_style_variants_css() {
        use rtfkit_core::{Border, BorderSet, BorderStyle};

        let make_cell_with_top = |style: BorderStyle| {
            let mut c = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("x")]));
            c.borders = Some(BorderSet {
                top: Some(Border {
                    style,
                    width_half_pts: Some(4),
                    color: None,
                }),
                ..Default::default()
            });
            c
        };

        // double
        let mut buf = HtmlBuffer::new();
        table_to_html(
            &TableBlock::from_rows(vec![TableRow::from_cells(vec![make_cell_with_top(
                BorderStyle::Double,
            )])]),
            &mut buf,
        );
        assert!(buf.as_str().contains("double"));

        // dotted
        let mut buf = HtmlBuffer::new();
        table_to_html(
            &TableBlock::from_rows(vec![TableRow::from_cells(vec![make_cell_with_top(
                BorderStyle::Dotted,
            )])]),
            &mut buf,
        );
        assert!(buf.as_str().contains("dotted"));
    }

    #[test]
    fn cell_border_after_shading_stable_order() {
        use rtfkit_core::{Border, BorderSet, BorderStyle, Color, Shading};

        let mut cell =
            TableCell::from_paragraph_with_width(Paragraph::from_runs(vec![Run::new("x")]), 720);
        cell.shading = Some(Shading::solid(Color::new(0, 255, 0)));
        cell.borders = Some(BorderSet {
            bottom: Some(Border {
                style: BorderStyle::Single,
                width_half_pts: Some(2),
                color: None,
            }),
            ..Default::default()
        });

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Width, shading, border — in that order
        let width_pos = html.find("width:").unwrap();
        let bg_pos = html.find("background-color:").unwrap();
        let border_pos = html.find("border-bottom:").unwrap();
        assert!(
            width_pos < bg_pos,
            "width should come before background-color"
        );
        assert!(
            bg_pos < border_pos,
            "background-color should come before border"
        );
    }

    #[test]
    fn no_borders_no_border_css() {
        let cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal")]));
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        // Default table CSS may contain "border" from .rtf-table rule in <style>,
        // but the <td> element itself should not have an inline border style.
        // The table element tag should not contain "border-top:", "border-left:", etc.
        assert!(!html.contains("border-top:"));
        assert!(!html.contains("border-left:"));
        assert!(!html.contains("border-bottom:"));
        assert!(!html.contains("border-right:"));
    }

    #[test]
    fn row_outer_borders_fallback_to_cells() {
        use rtfkit_core::{Border, BorderSet, BorderStyle, RowProps};

        let row = TableRow {
            cells: vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
            ],
            row_props: Some(RowProps {
                borders: Some(BorderSet {
                    top: Some(Border {
                        style: BorderStyle::Single,
                        width_half_pts: Some(4),
                        color: None,
                    }),
                    bottom: Some(Border {
                        style: BorderStyle::Single,
                        width_half_pts: Some(4),
                        color: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        let table = TableBlock::from_rows(vec![row]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();
        assert!(html.contains("border-top: 2.0pt solid currentColor"));
        assert!(html.contains("border-bottom: 2.0pt solid currentColor"));
    }

    #[test]
    fn inside_vertical_fallback_prefers_left_cell_edge() {
        use rtfkit_core::{Border, BorderSet, BorderStyle, RowProps};

        let row = TableRow {
            cells: vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
            ],
            row_props: Some(RowProps {
                borders: Some(BorderSet {
                    inside_v: Some(Border {
                        style: BorderStyle::Single,
                        width_half_pts: Some(4),
                        color: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        let table = TableBlock::from_rows(vec![row]);

        let mut buf = HtmlBuffer::new();
        table_to_html(&table, &mut buf);
        let html = buf.as_str();

        assert!(html.contains("border-right: 2.0pt solid currentColor"));
        assert!(!html.contains("border-left: 2.0pt solid currentColor"));
    }
}
