//! Paragraph finalization helpers.

use super::super::state::RuntimeState;
use crate::{Block, TableCell};

/// Finalize the current paragraph and add it to the document.
pub fn finalize_paragraph(state: &mut RuntimeState) {
    // Active row context always wins over \\intbl marker state.
    if state.tables.in_row() {
        finalize_paragraph_for_table(state);
        return;
    }

    // \\intbl without an active row is malformed. Keep text as normal paragraph.
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
        super::tables::finalize_current_table(state);
    }

    super::runs::flush_current_text_as_run(state);

    // Add paragraph to document if it has content
    if !state.current_paragraph.inlines.is_empty() {
        state.current_paragraph.alignment = state.paragraph_alignment;

        // Apply paragraph shading from current style state
        state.current_paragraph.shading = super::shading::build_shading(
            state,
            state.style.paragraph_cbpat,
            state.style.paragraph_cfpat,
            state.style.paragraph_shading,
        );

        // Resolve list reference if we have pending list state
        super::lists::resolve_list_reference(state);

        // Check if this paragraph belongs to a list
        if let Some(list_ref) = &state.lists.current_list_ref {
            // Add as list item to existing or new list block
            super::lists::add_list_item(
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
            .add_runs(super::runs::inline_run_count(&state.current_paragraph.inlines));
    }

    state.reset_paragraph_state();
}

/// Finalize paragraph for table context (routes to current cell instead of document).
pub fn finalize_paragraph_for_table(state: &mut RuntimeState) {
    super::runs::flush_current_text_as_run(state);

    // Add paragraph to current cell if it has content
    if !state.current_paragraph.inlines.is_empty() {
        state.current_paragraph.alignment = state.paragraph_alignment;

        // Apply paragraph shading from current style state
        state.current_paragraph.shading = super::shading::build_shading(
            state,
            state.style.paragraph_cbpat,
            state.style.paragraph_cfpat,
            state.style.paragraph_shading,
        );

        super::lists::resolve_list_reference(state);

        let paragraph = state.current_paragraph.clone();

        if let Some(list_ref) = state.lists.current_list_ref.clone() {
            super::lists::add_list_item_to_current_cell(
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
            .add_runs(super::runs::inline_run_count(&state.current_paragraph.inlines));
    }

    state.reset_paragraph_state();
}
