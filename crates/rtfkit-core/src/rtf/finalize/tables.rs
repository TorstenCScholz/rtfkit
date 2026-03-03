//! Table/cell/row finalization helpers.

use super::super::state::RuntimeState;
use crate::error::ParseError;
use crate::{Block, BoxSpacingTwips, CellMerge, RowHeightRule, TableProps, TableRow, WidthUnit};

/// Resolve an (fts, w) pair into a `WidthUnit`.
///
/// RTF ftsWidth values:
/// - 0 or 1: auto
/// - 2: percent (units are 0.02% each, i.e. 5000 = 100%)
/// - 3: twips/dxa (absolute)
fn resolve_width_unit(
    fts: Option<i32>,
    w: Option<i32>,
    word: &str,
    state: &mut RuntimeState,
) -> Option<WidthUnit> {
    let fts_val = fts?; // if no fts, no preferred width
    match fts_val {
        0 | 1 => Some(WidthUnit::Auto),
        2 => {
            // Percent: RTF stores in units where 10000 = 100%
            let pct = w.unwrap_or(0).clamp(0, 10000) as u16;
            Some(WidthUnit::Percent(pct))
        }
        3 => {
            let twips = w.unwrap_or(0);
            Some(WidthUnit::Twips(twips))
        }
        _ => {
            state.report_builder.unsupported_table_control(word);
            None
        }
    }
}

/// Apply row-default padding precedence to a side value.
///
/// Returns `Some(v)` if the unit is dxa (3) or unset (treat as dxa).
/// Returns `None` if unit is set to a non-dxa value (already warned at parse time).
fn apply_padding_side(value: Option<i32>, fmt: Option<i32>) -> Option<i32> {
    match fmt {
        None | Some(3) => value,
        _ => None, // unsupported unit, already warned
    }
}

/// Finalize the current cell and attach it to the current row.
pub fn finalize_current_cell(state: &mut RuntimeState) {
    // Create cell if needed (for empty cells or cells with content)
    if state.tables.current_cell.is_none() {
        state.tables.current_cell = Some(crate::TableCell::new());
    }

    if let Some(cell) = state.tables.current_cell.take() {
        // Convert \\cellx right-boundary positions to actual cell widths.
        let cell_index = state
            .tables
            .current_row
            .as_ref()
            .map(|r| r.cells.len())
            .unwrap_or(0);
        let width = state
            .tables
            .pending_cellx
            .get(cell_index)
            .copied()
            .and_then(|right_boundary| {
                if cell_index == 0 {
                    Some(right_boundary)
                } else {
                    state
                        .tables
                        .pending_cellx
                        .get(cell_index - 1)
                        .map(|left_boundary| right_boundary - *left_boundary)
                }
            });

        let mut cell_with_props = cell;

        // Apply width
        if let Some(w) = width {
            if w > 0 {
                cell_with_props.width_twips = Some(w);
            } else {
                state.report_builder.malformed_table_structure(&format!(
                    "Non-increasing \\cellx boundaries at cell {}",
                    cell_index
                ));
            }
        }

        // Apply merge state from stored per-cellx state
        cell_with_props.merge = state
            .tables
            .pending_cell_merges
            .get(cell_index)
            .cloned()
            .flatten();

        // Apply vertical alignment from stored per-cellx state
        cell_with_props.v_align = state
            .tables
            .pending_cell_v_aligns
            .get(cell_index)
            .copied()
            .flatten();

        // Apply shading with fallback precedence: cell > row > table
        // 1. Check for explicit cell shading first
        let cell_cbpat = state
            .tables
            .pending_cell_cbpats
            .get(cell_index)
            .copied()
            .flatten();
        let cell_cfpat = state
            .tables
            .pending_cell_cfpats
            .get(cell_index)
            .copied()
            .flatten();
        let cell_shading = state
            .tables
            .pending_cell_shadings
            .get(cell_index)
            .copied()
            .flatten();

        if let Some(shading) =
            super::shading::build_shading(state, cell_cbpat, cell_cfpat, cell_shading)
        {
            cell_with_props.shading = Some(shading);
        }
        // 2. Fall back to row shading if cell has no explicit shading
        else if let Some(shading) = super::shading::build_shading(
            state,
            state.tables.pending_row_cbpat,
            state.tables.pending_row_cfpat,
            state.tables.pending_row_shading,
        ) {
            cell_with_props.shading = Some(shading);
        }
        // 3. Fall back to table shading if no cell or row shading
        else if let Some(shading) = super::shading::build_shading(
            state,
            state.tables.pending_table_cbpat,
            state.tables.pending_table_cfpat,
            state.tables.pending_table_shading,
        ) {
            cell_with_props.shading = Some(shading);
        }

        // Apply cell borders from per-cellx capture
        if let Some(capture) = state.tables.pending_cell_border_captures.get(cell_index)
            && let Some(borders) = super::borders::build_border_set_from_cell(capture, state)
        {
            cell_with_props.borders = Some(borders);
        }

        // Apply preferred cell width from per-cellx capture
        let cell_fts = state
            .tables
            .pending_cell_fts_widths
            .get(cell_index)
            .copied()
            .flatten();
        let cell_w = state
            .tables
            .pending_cell_w_widths
            .get(cell_index)
            .copied()
            .flatten();
        if let Some(wu) = resolve_width_unit(cell_fts, cell_w, "clftswidth", state) {
            cell_with_props.preferred_width = Some(wu);
        }

        // Apply cell padding (cell override of row default), per side
        let row_padding = state.tables.pending_row_padding.clone();
        let row_fmt = state.tables.pending_row_padding_fmt;
        let cell_padding_capture = state
            .tables
            .pending_cell_padding_captures
            .get(cell_index)
            .cloned()
            .unwrap_or_default();
        let cell_padding_fmt = state
            .tables
            .pending_cell_padding_fmt_captures
            .get(cell_index)
            .copied()
            .unwrap_or([None; 4]);
        // Build effective padding: cell value overrides row default per side
        let eff_top = apply_padding_side(cell_padding_capture.top, cell_padding_fmt[1])
            .or_else(|| apply_padding_side(row_padding.top, row_fmt[1]));
        let eff_right = apply_padding_side(cell_padding_capture.right, cell_padding_fmt[2])
            .or_else(|| apply_padding_side(row_padding.right, row_fmt[2]));
        let eff_bottom = apply_padding_side(cell_padding_capture.bottom, cell_padding_fmt[3])
            .or_else(|| apply_padding_side(row_padding.bottom, row_fmt[3]));
        let eff_left = apply_padding_side(cell_padding_capture.left, cell_padding_fmt[0])
            .or_else(|| apply_padding_side(row_padding.left, row_fmt[0]));
        let eff_padding = BoxSpacingTwips {
            top: eff_top,
            right: eff_right,
            bottom: eff_bottom,
            left: eff_left,
        };
        if !eff_padding.is_empty() {
            cell_with_props.padding = Some(eff_padding);
        }

        if let Some(ref mut row) = state.tables.current_row {
            row.cells.push(cell_with_props);
        }
    }

    // Reset pending cell state for next cell
    state.tables.reset_for_new_cell();
}

/// Finalize the current row and attach it to the current table.
pub fn finalize_current_row(state: &mut RuntimeState) {
    if let Some(mut row) = state.tables.current_row.take() {
        // Check for cellx count mismatch
        if !state.tables.pending_cellx.is_empty()
            && row.cells.len() != state.tables.pending_cellx.len()
        {
            let reason = format!(
                "Cell count ({}) does not match \\cellx count ({})",
                row.cells.len(),
                state.tables.pending_cellx.len()
            );
            state.report_builder.malformed_table_structure(&reason);
            state
                .report_builder
                .dropped_content("Table cell count mismatch", None);
        }

        // Apply row-level shading to RowProps if present
        if let Some(shading) = super::shading::build_shading(
            state,
            state.tables.pending_row_cbpat,
            state.tables.pending_row_cfpat,
            state.tables.pending_row_shading,
        ) {
            state.tables.pending_row_props.shading = Some(shading);
        }

        // Apply row-level borders to RowProps
        let row_border_capture = std::mem::take(&mut state.tables.pending_row_borders);
        if let Some(borders) = super::borders::build_border_set_from_row(&row_border_capture, state)
        {
            state.tables.pending_row_props.borders = Some(borders);
        }

        // Apply row height from \trrh (positive = AtLeast, negative = Exact, 0 = unset)
        if let Some(raw) = state.tables.pending_row_height_raw {
            if raw > 0 {
                state.tables.pending_row_props.height_rule = Some(RowHeightRule::AtLeast);
                state.tables.pending_row_props.height_twips = Some(raw);
            } else if raw < 0 {
                state.tables.pending_row_props.height_rule = Some(RowHeightRule::Exact);
                state.tables.pending_row_props.height_twips = Some(-raw);
            }
            // raw == 0 → unset
        }

        // Apply row-default padding to RowProps if any side is set
        {
            let fmt = state.tables.pending_row_padding_fmt;
            let raw = state.tables.pending_row_padding.clone();
            let eff = BoxSpacingTwips {
                top: apply_padding_side(raw.top, fmt[1]),
                right: apply_padding_side(raw.right, fmt[2]),
                bottom: apply_padding_side(raw.bottom, fmt[3]),
                left: apply_padding_side(raw.left, fmt[0]),
            };
            if !eff.is_empty() {
                state.tables.pending_row_props.default_padding = Some(eff);
            }
        }

        // Apply pending row properties
        if state.tables.pending_row_props != Default::default() {
            row.row_props = Some(state.tables.pending_row_props.clone());
        }

        // Normalize merge semantics
        normalize_row_merges(&mut row, &mut state.report_builder);

        // Resolve conflicts
        resolve_merge_conflicts(&mut row, &mut state.report_builder);

        // Check table limits
        if let Err(e) = check_table_limits(&row, &state.limits) {
            state.set_hard_failure(e);
            return;
        }

        if let Some(existing_row_count) = state.tables.current_table.as_ref().map(|t| t.rows.len())
            && existing_row_count >= state.limits.max_rows_per_table
        {
            state.set_hard_failure(ParseError::InvalidStructure(format!(
                "Table has {} rows, maximum is {}",
                existing_row_count + 1,
                state.limits.max_rows_per_table
            )));
            return;
        }

        if let Some(ref mut table) = state.tables.current_table {
            table.rows.push(row);
        }

        // Reset pending row state
        state.tables.pending_row_props = Default::default();
        state.tables.pending_row_cbpat = None;
        state.tables.pending_row_cfpat = None;
        state.tables.pending_row_shading = None;
    }
}

/// Normalize merge semantics in a row.
///
/// This calculates the span for horizontal merge start cells based on
/// the number of continuation cells that follow.
pub fn normalize_row_merges(row: &mut TableRow, report_builder: &mut crate::report::ReportBuilder) {
    // First, collect merge information
    let merge_info: Vec<Option<CellMerge>> = row.cells.iter().map(|c| c.merge.clone()).collect();

    let mut span_count: u16 = 0;
    let mut merge_start_idx: Option<usize> = None;

    // Process merge chains and calculate spans
    for (idx, merge) in merge_info.iter().enumerate() {
        match merge {
            Some(CellMerge::HorizontalStart { .. }) => {
                // Close out any previous merge chain
                if let Some(start_idx) = merge_start_idx {
                    if span_count > 1 {
                        row.cells[start_idx].merge =
                            Some(CellMerge::HorizontalStart { span: span_count });
                    } else {
                        // Single cell "merge" - clear it
                        row.cells[start_idx].merge = None;
                    }
                }
                merge_start_idx = Some(idx);
                span_count = 1;
            }
            Some(CellMerge::HorizontalContinue) => {
                span_count += 1;
            }
            _ => {
                // Not in a merge chain - close out previous if any
                if let Some(start_idx) = merge_start_idx {
                    if span_count > 1 {
                        row.cells[start_idx].merge =
                            Some(CellMerge::HorizontalStart { span: span_count });
                    } else {
                        // Single cell "merge" - clear it
                        row.cells[start_idx].merge = None;
                    }
                }
                merge_start_idx = None;
                span_count = 0;
            }
        }
    }

    // Close out any trailing merge
    if let Some(start_idx) = merge_start_idx {
        if span_count > 1 {
            row.cells[start_idx].merge = Some(CellMerge::HorizontalStart { span: span_count });
        } else {
            row.cells[start_idx].merge = None;
        }
    }

    // Suppress unused variable warning
    let _ = report_builder;
}

/// Resolve merge conflicts with deterministic degradation.
///
/// This handles:
/// - Orphan continuations (continuation without start)
/// - Spans exceeding row bounds
pub fn resolve_merge_conflicts(
    row: &mut TableRow,
    report_builder: &mut crate::report::ReportBuilder,
) {
    // Track expected continuation cells from the most recent horizontal start.
    let mut expected_continuations: usize = 0;

    // Detect orphan continuations regardless of column position.
    for idx in 0..row.cells.len() {
        let merge = row.cells[idx].merge.clone();
        match merge {
            Some(CellMerge::HorizontalStart { span }) => {
                expected_continuations = span.saturating_sub(1) as usize;
            }
            Some(CellMerge::HorizontalContinue) => {
                if expected_continuations == 0 {
                    row.cells[idx].merge = None;
                    report_builder.merge_conflict(
                        "Orphan merge continuation without start - treating as standalone cell",
                    );
                    report_builder.dropped_content("merge_semantics", None);
                } else {
                    expected_continuations -= 1;
                }
            }
            _ => {
                expected_continuations = 0;
            }
        }
    }

    // Collect merge info after orphan cleanup to avoid borrow issues.
    let merge_info: Vec<Option<CellMerge>> = row.cells.iter().map(|c| c.merge.clone()).collect();

    // Check for span exceeding row bounds.
    for (idx, merge) in merge_info.iter().enumerate() {
        if let Some(CellMerge::HorizontalStart { span }) = merge {
            let span_val = *span;
            let available_cells = row.cells.len() - idx;
            if span_val as usize > available_cells {
                // Clamp span to available cells
                row.cells[idx].merge = Some(CellMerge::HorizontalStart {
                    span: available_cells as u16,
                });
                report_builder.table_geometry_conflict(&format!(
                    "Merge span {} exceeds available cells {} - clamped",
                    span_val, available_cells
                ));
                report_builder.dropped_content("merge_semantics", None);
            }
        }
    }
}

/// Check table limits for a row.
pub fn check_table_limits(
    row: &TableRow,
    limits: &crate::limits::ParserLimits,
) -> Result<(), ParseError> {
    // Check cells per row limit
    if row.cells.len() > limits.max_cells_per_row {
        return Err(ParseError::InvalidStructure(format!(
            "Row has {} cells, maximum is {}",
            row.cells.len(),
            limits.max_cells_per_row
        )));
    }

    // Check merge spans
    for cell in &row.cells {
        if let Some(CellMerge::HorizontalStart { span }) = cell.merge
            && span > limits.max_merge_span
        {
            return Err(ParseError::InvalidStructure(format!(
                "Merge span {} exceeds maximum {}",
                span, limits.max_merge_span
            )));
        }
    }

    Ok(())
}

/// Finalize the current table and add it to the document.
pub fn finalize_current_table(state: &mut RuntimeState) {
    // Finalize any dangling row/cell.
    if state.tables.in_row() {
        auto_close_table_cell_if_needed(state, "Unclosed table cell at document end");
        state.report_builder.unclosed_table_row();
        state
            .report_builder
            .dropped_content("Unclosed table row at document end", None);
        finalize_current_row(state);
    }

    // Build the finalized table block for this active context (if any content).
    let finalized_table = if let Some(mut table) = state.tables.current_table.take()
        && !table.is_empty()
    {
        // Resolve table-level preferred width from first-row \trftsWidth/\trwWidth
        let table_preferred_width = resolve_width_unit(
            state.tables.pending_table_fts_width,
            state.tables.pending_table_w_width,
            "trftswidth",
            state,
        );

        // Apply table-level shading and preferred width to TableProps
        let table_shading = super::shading::build_shading(
            state,
            state.tables.pending_table_cbpat,
            state.tables.pending_table_cfpat,
            state.tables.pending_table_shading,
        );
        if table_shading.is_some() || table_preferred_width.is_some() {
            let existing = table.table_props.take().unwrap_or_default();
            table.table_props = Some(TableProps {
                shading: table_shading.or(existing.shading),
                borders: existing.borders,
                preferred_width: table_preferred_width.or(existing.preferred_width),
            });
        }
        Some(Block::TableBlock(table))
    } else {
        None
    };

    // Reset table state
    state.tables.clear_table();

    // Nested case: restore parent and append child table to parent cell stream.
    if state.tables.has_parent_context() {
        let restored = state.tables.restore_parent_context();
        if restored && let Some(block) = finalized_table {
            if state.tables.current_cell.is_none() {
                state.tables.current_cell = Some(crate::TableCell::new());
            }
            if let Some(parent_cell) = state.tables.current_cell.as_mut() {
                parent_cell.blocks.push(block);
            }
        }
        return;
    }

    if let Some(block) = finalized_table {
        state.push_block_to_current_sink(block);
    }
}

/// Auto-close table cell if needed.
pub fn auto_close_table_cell_if_needed(state: &mut RuntimeState, dropped_reason: &str) {
    if !state.tables.in_row() || !has_open_or_pending_table_cell_content(state) {
        return;
    }

    state.report_builder.unclosed_table_cell();
    state.report_builder.dropped_content(dropped_reason, None);
    super::paragraphs::finalize_paragraph_for_table(state);
    finalize_current_cell(state);
}

/// Check if there's open or pending table cell content.
pub fn has_open_or_pending_table_cell_content(state: &RuntimeState) -> bool {
    state.tables.in_cell() || state.has_pending_paragraph_content()
}
