//! Finalization Module (Phase 4)
//!
//! This module contains paragraph, cell, row, and table finalization logic
//! extracted from the monolithic interpreter.

use crate::error::ParseError;
use crate::{
    Block, CellMerge, Inline, ListBlock, ListId, ListItem, ListKind, Paragraph, Run, Shading,
    ShadingPattern, TableCell, TableProps, TableRow,
};

use super::state::RuntimeState;

// =============================================================================
// Run Creation
// =============================================================================

/// Create a run from the current text and run style.
pub fn create_run(state: &RuntimeState) -> Run {
    // Resolve font_family from font_index -> font_table
    let font_family = state.resolve_font_family();

    // Resolve font_size from half-points to points
    let font_size = state.resolve_font_size();

    // Resolve color from color_index -> color_table
    let color = state.resolve_color();

    // Resolve background_color with precedence: highlight > background
    let background_color = state.resolve_background_color();

    Run {
        text: state.current_text.clone(),
        bold: state.current_run_style.bold,
        italic: state.current_run_style.italic,
        underline: state.current_run_style.underline,
        font_family,
        font_size,
        color,
        background_color,
    }
}

/// Flush current text as a run into the current paragraph.
pub fn flush_current_text_as_run(state: &mut RuntimeState) {
    if !state.current_text.is_empty() {
        let run = create_run(state);
        state.current_paragraph.inlines.push(Inline::Run(run));
        state.current_text.clear();
    }
}

// =============================================================================
// Shading Helpers
// =============================================================================

/// Map RTF shading percentage (0-10000) to ShadingPattern.
///
/// RTF `\shadingN` and `\clshdngN` use percentage values where:
/// - 0 = Clear (transparent)
/// - 10000 = Solid (100%)
/// - Other values map to Percent patterns
pub fn shading_percentage_to_pattern(percentage: i32) -> Option<ShadingPattern> {
    // Clamp to valid range
    let clamped = percentage.clamp(0, 10000);

    match clamped {
        0 => Some(ShadingPattern::Clear),
        10000 => Some(ShadingPattern::Solid),
        // Map percentage to closest Percent pattern
        // RTF uses 0-10000, we map to discrete percentages
        p if p <= 75 => Some(ShadingPattern::Percent5),
        p if p <= 150 => Some(ShadingPattern::Percent10),
        p if p <= 250 => Some(ShadingPattern::Percent20),
        p if p <= 375 => Some(ShadingPattern::Percent25),
        p if p <= 450 => Some(ShadingPattern::Percent30),
        p if p <= 550 => Some(ShadingPattern::Percent40),
        p if p <= 650 => Some(ShadingPattern::Percent50),
        p if p <= 750 => Some(ShadingPattern::Percent60),
        p if p <= 825 => Some(ShadingPattern::Percent70),
        p if p <= 875 => Some(ShadingPattern::Percent75),
        p if p <= 950 => Some(ShadingPattern::Percent80),
        p if p < 10000 => Some(ShadingPattern::Percent90),
        _ => Some(ShadingPattern::Solid),
    }
}

/// Build a Shading object from fill color index, pattern color index, and shading percentage.
///
/// This combines the three shading-related RTF controls into a single Shading struct:
/// - `cbpat`/`clcbpat`: fill/background color index
/// - `cfpat`/`clcfpat`: pattern/foreground color index
/// - `shading`/`clshdng`: shading percentage (0-10000)
pub fn build_shading(
    state: &RuntimeState,
    fill_color_idx: Option<i32>,
    pattern_color_idx: Option<i32>,
    shading_percentage: Option<i32>,
) -> Option<Shading> {
    // Resolve fill color (required for any shading)
    let fill_color = fill_color_idx.and_then(|idx| state.resolve_color_from_index(idx));

    // If no fill color, no shading
    fill_color.map(|fill| {
        // Resolve pattern color (optional)
        let pattern_color = pattern_color_idx.and_then(|idx| state.resolve_color_from_index(idx));

        // Map shading percentage to pattern
        let pattern = shading_percentage.and_then(shading_percentage_to_pattern);

        // Determine the final pattern:
        // - If we have an explicit shading percentage, use the mapped pattern
        // - If we have a pattern color but no shading percentage, default to Solid
        // - If we have neither, leave pattern as None (flat fill, no pattern overlay)
        let final_pattern = match (pattern, pattern_color.is_some()) {
            (Some(p), _) => Some(p),
            (None, true) => Some(ShadingPattern::Solid),
            (None, false) => None,
        };

        Shading {
            fill_color: Some(fill),
            pattern_color,
            pattern: final_pattern,
        }
    })
}

// =============================================================================
// Paragraph Finalization
// =============================================================================

/// Finalize the current paragraph and add it to the document.
pub fn finalize_paragraph(state: &mut RuntimeState) {
    // Active row context always wins over \intbl marker state.
    if state.tables.in_row() {
        finalize_paragraph_for_table(state);
        return;
    }

    // \intbl without an active row is malformed. Keep text as normal paragraph.
    if state.tables.seen_intbl_in_paragraph && state.has_pending_paragraph_content() {
        state
            .report_builder
            .malformed_table_structure("\\intbl content without table row context");
        state
            .report_builder
            .dropped_content("Table paragraph without row context", None);
        state.tables.seen_intbl_in_paragraph = false;
    }

    // If a completed table is pending, close it before adding prose blocks.
    if state.tables.in_table() {
        finalize_current_table(state);
    }

    flush_current_text_as_run(state);

    // Add paragraph to document if it has content
    if !state.current_paragraph.inlines.is_empty() {
        state.current_paragraph.alignment = state.paragraph_alignment;

        // Apply paragraph shading from current style state
        state.current_paragraph.shading = build_shading(
            state,
            state.style.paragraph_cbpat,
            state.style.paragraph_cfpat,
            state.style.paragraph_shading,
        );

        // Resolve list reference if we have pending list state
        resolve_list_reference(state);

        // Check if this paragraph belongs to a list
        if let Some(list_ref) = &state.lists.current_list_ref {
            // Add as list item to existing or new list block
            add_list_item(
                state,
                list_ref.list_id,
                list_ref.level,
                list_ref.kind,
                state.current_paragraph.clone(),
            );
        } else {
            // Regular paragraph
            state
                .document
                .blocks
                .push(Block::Paragraph(state.current_paragraph.clone()));
        }

        // Track stats
        state.report_builder.increment_paragraph_count();
        state
            .report_builder
            .add_runs(inline_run_count(&state.current_paragraph.inlines));
    }

    state.reset_paragraph_state();
}

/// Finalize paragraph for table context (routes to current cell instead of document).
pub fn finalize_paragraph_for_table(state: &mut RuntimeState) {
    flush_current_text_as_run(state);

    // Add paragraph to current cell if it has content
    if !state.current_paragraph.inlines.is_empty() {
        state.current_paragraph.alignment = state.paragraph_alignment;

        // Apply paragraph shading from current style state
        state.current_paragraph.shading = build_shading(
            state,
            state.style.paragraph_cbpat,
            state.style.paragraph_cfpat,
            state.style.paragraph_shading,
        );

        resolve_list_reference(state);

        let paragraph = state.current_paragraph.clone();

        if let Some(list_ref) = state.lists.current_list_ref.clone() {
            add_list_item_to_current_cell(
                state,
                list_ref.list_id,
                list_ref.level,
                list_ref.kind,
                paragraph,
            );
        } else {
            // Create cell if needed.
            if state.tables.current_cell.is_none() {
                state.tables.current_cell = Some(TableCell::new());
            }

            if let Some(ref mut cell) = state.tables.current_cell {
                cell.blocks.push(Block::Paragraph(paragraph));
            }
        }

        // Track stats
        state.report_builder.increment_paragraph_count();
        state
            .report_builder
            .add_runs(inline_run_count(&state.current_paragraph.inlines));
    }

    state.reset_paragraph_state();
}

// =============================================================================
// List Helpers
// =============================================================================

/// Resolve list reference from pending state.
pub fn resolve_list_reference(state: &mut RuntimeState) {
    // First, try to resolve from \lsN (modern list mechanism)
    if let Some(ls_id) = state.lists.pending_ls_id {
        if let Some(override_def) = state.lists.list_overrides.get(&ls_id) {
            let list_id = override_def.list_id as ListId;
            let level = state.lists.pending_level;

            // Emit warning if level exceeds max
            if level > 8 {
                state.report_builder.unsupported_nesting_level(level, 8);
            }

            let kind = state
                .lists
                .list_table
                .get(&override_def.list_id)
                .map(|def| def.kind_for_level(level))
                .unwrap_or_default();
            state.lists.current_list_ref = Some(super::state_lists::ParagraphListRef::new(
                list_id, level, kind,
            ));
            return;
        }

        // If we have ls_id but no override, emit warnings and keep paragraph output.
        state.report_builder.unresolved_list_override(ls_id);

        // Emit DroppedContent for strict mode compatibility
        state
            .report_builder
            .dropped_content(&format!("Unresolved list override ls_id={}", ls_id), None);
    }
}

/// Add a list item to the document, creating or appending to a list block.
pub fn add_list_item(
    state: &mut RuntimeState,
    list_id: ListId,
    level: u8,
    kind: ListKind,
    paragraph: Paragraph,
) {
    // Check if we can append to the last block if it's a list with the same ID
    if let Some(Block::ListBlock(last_list)) = state.document.blocks.last_mut()
        && last_list.list_id == list_id
        && last_list.kind == kind
    {
        last_list.add_item(ListItem::from_paragraph(level, paragraph));
        return;
    }

    // Create a new list block
    let mut list_block = ListBlock::new(list_id, kind);
    list_block.add_item(ListItem::from_paragraph(level, paragraph));
    state.document.blocks.push(Block::ListBlock(list_block));
}

/// Add a list item to the current table cell.
pub fn add_list_item_to_current_cell(
    state: &mut RuntimeState,
    list_id: ListId,
    level: u8,
    kind: ListKind,
    paragraph: Paragraph,
) {
    if state.tables.current_cell.is_none() {
        state.tables.current_cell = Some(TableCell::new());
    }

    if let Some(ref mut cell) = state.tables.current_cell {
        if let Some(Block::ListBlock(last_list)) = cell.blocks.last_mut()
            && last_list.list_id == list_id
            && last_list.kind == kind
        {
            last_list.add_item(ListItem::from_paragraph(level, paragraph));
            return;
        }

        let mut list_block = ListBlock::new(list_id, kind);
        list_block.add_item(ListItem::from_paragraph(level, paragraph));
        cell.blocks.push(Block::ListBlock(list_block));
    }
}

/// Count runs in a list of inlines.
pub fn inline_run_count(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Run(_) => 1,
            Inline::Hyperlink(link) => link.runs.len(),
        })
        .sum()
}

// =============================================================================
// Table Cell Finalization
// =============================================================================

/// Finalize the current cell and attach it to the current row.
pub fn finalize_current_cell(state: &mut RuntimeState) {
    // Create cell if needed (for empty cells or cells with content)
    if state.tables.current_cell.is_none() {
        state.tables.current_cell = Some(TableCell::new());
    }

    if let Some(cell) = state.tables.current_cell.take() {
        // Convert \cellx right-boundary positions to actual cell widths.
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

        if let Some(shading) = build_shading(state, cell_cbpat, cell_cfpat, cell_shading) {
            cell_with_props.shading = Some(shading);
        }
        // 2. Fall back to row shading if cell has no explicit shading
        else if let Some(shading) = build_shading(
            state,
            state.tables.pending_row_cbpat,
            state.tables.pending_row_cfpat,
            state.tables.pending_row_shading,
        ) {
            cell_with_props.shading = Some(shading);
        }
        // 3. Fall back to table shading if no cell or row shading
        else if let Some(shading) = build_shading(
            state,
            state.tables.pending_table_cbpat,
            state.tables.pending_table_cfpat,
            state.tables.pending_table_shading,
        ) {
            cell_with_props.shading = Some(shading);
        }

        if let Some(ref mut row) = state.tables.current_row {
            row.cells.push(cell_with_props);
        }
    }

    // Reset pending cell state for next cell
    state.tables.reset_for_new_cell();
}

// =============================================================================
// Table Row Finalization
// =============================================================================

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
        if let Some(shading) = build_shading(
            state,
            state.tables.pending_row_cbpat,
            state.tables.pending_row_cfpat,
            state.tables.pending_row_shading,
        ) {
            state.tables.pending_row_props.shading = Some(shading);
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

// =============================================================================
// Table Finalization
// =============================================================================

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

    // Add the table to the document if it has content
    if let Some(mut table) = state.tables.current_table.take()
        && !table.is_empty()
    {
        // Apply table-level shading to TableProps if present
        if let Some(shading) = build_shading(
            state,
            state.tables.pending_table_cbpat,
            state.tables.pending_table_cfpat,
            state.tables.pending_table_shading,
        ) {
            table.table_props = Some(TableProps {
                shading: Some(shading),
            });
        }
        state.document.blocks.push(Block::TableBlock(table));
    }

    // Reset table state
    state.tables.clear_table();
}

/// Auto-close table cell if needed.
pub fn auto_close_table_cell_if_needed(state: &mut RuntimeState, dropped_reason: &str) {
    if !state.tables.in_row() || !has_open_or_pending_table_cell_content(state) {
        return;
    }

    state.report_builder.unclosed_table_cell();
    state.report_builder.dropped_content(dropped_reason, None);
    finalize_paragraph_for_table(state);
    finalize_current_cell(state);
}

/// Check if there's open or pending table cell content.
pub fn has_open_or_pending_table_cell_content(state: &RuntimeState) -> bool {
    state.tables.in_cell() || state.has_pending_paragraph_content()
}

// =============================================================================
// Document Finalization
// =============================================================================

/// Finalize the document at end of parsing.
pub fn finalize_document(state: &mut RuntimeState) {
    // Finalize any remaining content
    finalize_paragraph(state);

    // Finalize any remaining table context at document end
    if state.tables.in_table() {
        finalize_current_table(state);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shading_percentage_to_pattern() {
        assert_eq!(
            shading_percentage_to_pattern(0),
            Some(ShadingPattern::Clear)
        );
        assert_eq!(
            shading_percentage_to_pattern(10000),
            Some(ShadingPattern::Solid)
        );
        // The thresholds are designed for specific boundary values
        assert_eq!(
            shading_percentage_to_pattern(75),
            Some(ShadingPattern::Percent5)
        );
        assert_eq!(
            shading_percentage_to_pattern(650),
            Some(ShadingPattern::Percent50)
        );
        assert_eq!(
            shading_percentage_to_pattern(5000),
            Some(ShadingPattern::Percent90)
        );
        assert_eq!(
            shading_percentage_to_pattern(-100),
            Some(ShadingPattern::Clear)
        );
        assert_eq!(
            shading_percentage_to_pattern(20000),
            Some(ShadingPattern::Solid)
        );
    }

    #[test]
    fn test_inline_run_count() {
        let inlines = vec![
            Inline::Run(Run {
                text: "Hello".to_string(),
                bold: false,
                italic: false,
                underline: false,
                font_family: None,
                font_size: None,
                color: None,
                background_color: None,
            }),
            Inline::Run(Run {
                text: "World".to_string(),
                bold: false,
                italic: false,
                underline: false,
                font_family: None,
                font_size: None,
                color: None,
                background_color: None,
            }),
        ];
        assert_eq!(inline_run_count(&inlines), 2);
    }
}
