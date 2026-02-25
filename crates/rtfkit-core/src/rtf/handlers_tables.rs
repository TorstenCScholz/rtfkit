//! Table Handlers Module
//!
//! This module contains table-specific control handling and table event
//! processing (`\\cell` and `\\row`).

use super::state::RuntimeState;
use crate::error::ConversionError;
use crate::{CellMerge, CellVerticalAlign, RowAlignment};

/// Handle table-related control words.
///
/// Returns `true` if `word` was recognized and handled.
pub fn handle_table_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) -> bool {
    match word {
        // \\trowd - Start of a new table row definition
        "trowd" => {
            handle_trowd(state);
            true
        }
        // \\cellxN - Cell boundary position in twips
        "cellx" => {
            if let Some(boundary) = parameter {
                state.tables.record_cellx(boundary);
            }
            true
        }
        // \\intbl - Paragraph is inside a table
        "intbl" => {
            state.tables.seen_intbl_in_paragraph = true;
            true
        }
        // \\cell - End of a table cell (handled in pipeline)
        "cell" => true,
        // \\row - End of a table row (handled in pipeline)
        "row" => true,

        // Row alignment
        "trql" => {
            state.tables.pending_row_props.alignment = Some(RowAlignment::Left);
            true
        }
        "trqc" => {
            state.tables.pending_row_props.alignment = Some(RowAlignment::Center);
            true
        }
        "trqr" => {
            state.tables.pending_row_props.alignment = Some(RowAlignment::Right);
            true
        }
        // Row left indent (in twips)
        "trleft" => {
            if let Some(value) = parameter {
                state.tables.pending_row_props.left_indent = Some(value);
            }
            true
        }
        // Row formatting controls - recognized but not fully supported
        "trgaph" => {
            state.report_builder.unsupported_table_control(word);
            true
        }

        // Cell vertical alignment controls
        "clvertalt" => {
            state.tables.pending_cell_v_align = Some(CellVerticalAlign::Top);
            true
        }
        "clvertalc" => {
            state.tables.pending_cell_v_align = Some(CellVerticalAlign::Center);
            true
        }
        "clvertalb" => {
            state.tables.pending_cell_v_align = Some(CellVerticalAlign::Bottom);
            true
        }

        // Cell merge controls
        "clmgf" => {
            state.tables.pending_cell_merge = Some(CellMerge::HorizontalStart { span: 1 });
            true
        }
        "clmrg" => {
            state.tables.pending_cell_merge = Some(CellMerge::HorizontalContinue);
            true
        }
        "clvmgf" => {
            state.tables.pending_cell_merge = Some(CellMerge::VerticalStart);
            true
        }
        "clvmrg" => {
            state.tables.pending_cell_merge = Some(CellMerge::VerticalContinue);
            true
        }

        // Paragraph shading controls
        "cbpat" => {
            state.style.paragraph_cbpat = parameter;
            true
        }
        "cfpat" => {
            state.style.paragraph_cfpat = parameter;
            true
        }
        "shading" => {
            state.style.paragraph_shading = parameter;
            true
        }

        // Cell shading controls
        "clcbpat" => {
            state.tables.pending_cell_cbpat = parameter;
            true
        }
        "clcfpat" => {
            state.tables.pending_cell_cfpat = parameter;
            true
        }
        "clshdng" => {
            state.tables.pending_cell_shading = parameter;
            true
        }

        // Row/table fallback shading controls
        "trcbpat" => {
            state.tables.pending_row_cbpat = parameter;
            if state.tables.pending_table_cbpat.is_none() {
                state.tables.pending_table_cbpat = parameter;
            }
            true
        }
        "trcfpat" => {
            state.tables.pending_row_cfpat = parameter;
            if state.tables.pending_table_cfpat.is_none() {
                state.tables.pending_table_cfpat = parameter;
            }
            true
        }
        "trshdng" => {
            state.tables.pending_row_shading = parameter;
            if state.tables.pending_table_shading.is_none() {
                state.tables.pending_table_shading = parameter;
            }
            true
        }

        _ => false,
    }
}

/// Handle `\\cell` event.
pub fn handle_cell_event(state: &mut RuntimeState) -> Result<(), ConversionError> {
    if !state.tables.in_table() || !state.tables.in_row() {
        state
            .report_builder
            .malformed_table_structure("\\cell encountered outside table context");
        state
            .report_builder
            .dropped_content("Table cell control outside table context", None);
        super::finalize::finalize_paragraph(state);
        return Ok(());
    }

    super::finalize::finalize_paragraph_for_table(state);

    if state.tables.current_cell.is_none() {
        state.tables.current_cell = Some(crate::TableCell::new());
    }

    super::finalize::finalize_current_cell(state);
    Ok(())
}

/// Handle `\\row` event.
pub fn handle_row_event(state: &mut RuntimeState) -> Result<(), ConversionError> {
    if !state.tables.in_table() || !state.tables.in_row() {
        state
            .report_builder
            .malformed_table_structure("\\row encountered outside table context");
        state
            .report_builder
            .dropped_content("Table row control outside table context", None);
        return Ok(());
    }

    super::finalize::auto_close_table_cell_if_needed(state, "Unclosed table cell at row end");
    super::finalize::finalize_current_row(state);

    state.tables.pending_cellx.clear();
    state.tables.pending_cell_merges.clear();
    state.tables.pending_cell_v_aligns.clear();
    state.tables.seen_intbl_in_paragraph = false;

    Ok(())
}

/// Handle `\\trowd` - start of a new table row definition.
fn handle_trowd(state: &mut RuntimeState) {
    // Finalize any dangling row/cell with warning.
    if state.tables.current_row.is_some() {
        super::finalize::auto_close_table_cell_if_needed(
            state,
            "Unclosed table cell at row boundary",
        );
        state.report_builder.unclosed_table_row();
        state
            .report_builder
            .dropped_content("Unclosed table row at new row definition", None);
        super::finalize::finalize_current_row(state);
    }

    // Start fresh row context
    state.tables.reset_for_new_row();

    // Ensure we have a table context
    state.tables.ensure_table();
}
