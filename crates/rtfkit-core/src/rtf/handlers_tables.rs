//! Table Handlers Module
//!
//! This module contains table-specific control handling and table event
//! processing (`\\cell` and `\\row`).

use super::state::RuntimeState;
use crate::error::ConversionError;
use crate::rtf::state_tables::BorderTarget;
use crate::{BorderStyle, CellMerge, CellVerticalAlign, RowAlignment};

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
        // \itapN - table nesting depth hint (1 = top-level, >1 = nested)
        "itap" => {
            state.tables.pending_itap = parameter;
            true
        }
        // \nesttableprops marker seen in nested-table producers
        "nesttableprops" => {
            state.tables.seen_nesttableprops = true;
            true
        }
        // \nonesttables marker (currently no-op; table semantics remain explicit)
        "nonesttables" => true,
        // \\cell - End of a table cell (handled in pipeline)
        "cell" => true,
        "nestcell" => true,
        // \\row - End of a table row (handled in pipeline)
        "row" => true,
        "nestrow" => true,

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
        // \trgaphN — inter-cell half-gap; normalize to full gap (2 * N)
        "trgaph" => {
            if let Some(n) = parameter {
                state.tables.pending_row_props.cell_gap_twips = Some(n.max(0) * 2);
            }
            true
        }

        // Row height: positive = AtLeast, negative = Exact, zero = unset
        "trrh" => {
            state.tables.pending_row_height_raw = parameter;
            true
        }

        // Table preferred width controls (first-row wins for table-level)
        "trftswidth" => {
            state.tables.pending_row_fts_width = parameter;
            if state.tables.pending_table_fts_width.is_none() {
                state.tables.pending_table_fts_width = parameter;
            }
            true
        }
        "trwwidth" => {
            state.tables.pending_row_w_width = parameter;
            if state.tables.pending_table_w_width.is_none() {
                state.tables.pending_table_w_width = parameter;
            }
            true
        }

        // Cell preferred width controls
        "clftswidth" => {
            state.tables.pending_cell_fts_width = parameter;
            true
        }
        "clwwidth" => {
            state.tables.pending_cell_w_width = parameter;
            true
        }

        // Row-default padding (sides: l=0, t=1, r=2, b=3)
        "trpaddl" => {
            state.tables.pending_row_padding.left = parameter;
            true
        }
        "trpaddt" => {
            state.tables.pending_row_padding.top = parameter;
            true
        }
        "trpaddr" => {
            state.tables.pending_row_padding.right = parameter;
            true
        }
        "trpaddb" => {
            state.tables.pending_row_padding.bottom = parameter;
            true
        }
        // Row padding unit selectors — only dxa (3) supported; others warn and ignore
        "trpaddfl" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_row_padding_fmt[0] = parameter;
            true
        }
        "trpaddfr" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_row_padding_fmt[2] = parameter;
            true
        }
        "trpaddft" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_row_padding_fmt[1] = parameter;
            true
        }
        "trpaddfb" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_row_padding_fmt[3] = parameter;
            true
        }

        // Cell padding
        "clpadl" => {
            state.tables.pending_cell_padding.left = parameter;
            true
        }
        "clpadt" => {
            state.tables.pending_cell_padding.top = parameter;
            true
        }
        "clpadr" => {
            state.tables.pending_cell_padding.right = parameter;
            true
        }
        "clpadb" => {
            state.tables.pending_cell_padding.bottom = parameter;
            true
        }
        // Cell padding unit selectors — only dxa (3) supported; others warn and ignore
        "clpadfl" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_cell_padding_fmt[0] = parameter;
            true
        }
        "clpadfr" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_cell_padding_fmt[2] = parameter;
            true
        }
        "clpadft" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_cell_padding_fmt[1] = parameter;
            true
        }
        "clpadfb" => {
            if parameter != Some(3) {
                state.report_builder.unsupported_table_control(word);
            }
            state.tables.pending_cell_padding_fmt[3] = parameter;
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

        // ---------------------------------------------------------------
        // Cell border side selectors (\clbrdr*)
        // Each commits the previous pending border before setting a new target.
        // ---------------------------------------------------------------
        "clbrdrt" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::CellTop);
            true
        }
        "clbrdrl" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::CellLeft);
            true
        }
        "clbrdrb" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::CellBottom);
            true
        }
        "clbrdrr" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::CellRight);
            true
        }

        // ---------------------------------------------------------------
        // Row border side selectors (\trbrdr*)
        // ---------------------------------------------------------------
        "trbrdrt" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::RowTop);
            true
        }
        "trbrdrl" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::RowLeft);
            true
        }
        "trbrdrb" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::RowBottom);
            true
        }
        "trbrdrr" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::RowRight);
            true
        }
        "trbrdrh" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::RowInsideH);
            true
        }
        "trbrdrv" => {
            state.tables.commit_pending_border();
            state.tables.pending_border_target = Some(BorderTarget::RowInsideV);
            true
        }

        // ---------------------------------------------------------------
        // Border style descriptors (\brdr*)
        // ---------------------------------------------------------------
        "brdrs" => {
            state.tables.pending_border_style = Some(BorderStyle::Single);
            true
        }
        "brdrdb" => {
            state.tables.pending_border_style = Some(BorderStyle::Double);
            true
        }
        "brdrdot" => {
            state.tables.pending_border_style = Some(BorderStyle::Dotted);
            true
        }
        "brdrdash" => {
            state.tables.pending_border_style = Some(BorderStyle::Dashed);
            true
        }
        "brdrnone" => {
            state.tables.pending_border_style = Some(BorderStyle::None);
            true
        }

        // ---------------------------------------------------------------
        // Border width and color descriptors
        // ---------------------------------------------------------------
        "brdrw" => {
            if let Some(v) = parameter {
                state.tables.pending_border_width_hp = Some(v.max(0) as u32);
            }
            true
        }
        "brdrcf" => {
            state.tables.pending_border_color_idx = parameter;
            true
        }

        _ => false,
    }
}

/// Handle `\\cell` event.
pub fn handle_cell_event(state: &mut RuntimeState) -> Result<(), ConversionError> {
    // If a nested table was completed (\nestrow) and we're back at a cell boundary,
    // close child contexts before handling the parent \cell event.
    while state.tables.in_table() && !state.tables.in_row() && state.tables.has_parent_context() {
        super::finalize::finalize_current_table(state);
    }

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
    while state.tables.in_table() && !state.tables.in_row() && state.tables.has_parent_context() {
        super::finalize::finalize_current_table(state);
    }

    if !state.tables.in_table() || !state.tables.in_row() {
        state
            .report_builder
            .malformed_table_structure("\\row encountered outside table context");
        state
            .report_builder
            .dropped_content("Table row control outside table context", None);
        return Ok(());
    }

    // Commit any trailing row border descriptor before finalization
    state.tables.commit_pending_border();

    super::finalize::auto_close_table_cell_if_needed(state, "Unclosed table cell at row end");
    super::finalize::finalize_current_row(state);

    state.tables.seen_intbl_in_paragraph = false;

    Ok(())
}

/// Handle `\\trowd` - start of a new table row definition.
fn handle_trowd(state: &mut RuntimeState) {
    let requested_depth = state
        .tables
        .pending_itap
        .unwrap_or(if state.tables.seen_nesttableprops {
            2
        } else {
            1
        })
        .max(1) as usize;
    state.tables.pending_itap = None;
    state.tables.seen_nesttableprops = false;

    // Unwind to requested depth first.
    while state.tables.nesting_depth() > requested_depth && state.tables.has_parent_context() {
        super::finalize::finalize_current_table(state);
    }

    // Enter nested child contexts when requested and we're inside a parent row/cell stream.
    while state.tables.nesting_depth() < requested_depth {
        if state.tables.nesting_depth() >= state.limits.max_table_nesting_depth {
            state.set_hard_failure(crate::error::ParseError::InvalidStructure(format!(
                "Table nesting depth exceeds maximum {}",
                state.limits.max_table_nesting_depth
            )));
            return;
        }
        if !state.tables.in_row() {
            state
                .report_builder
                .malformed_table_structure("Nested table start without parent row context");
            break;
        }
        super::finalize::finalize_paragraph_for_table(state);
        if state.tables.current_cell.is_none() {
            state.tables.current_cell = Some(crate::TableCell::new());
        }
        state.tables.enter_child_context();
    }

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
