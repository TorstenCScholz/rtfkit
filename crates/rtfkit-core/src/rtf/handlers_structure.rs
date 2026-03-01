//! Structure Group-End Handler Module
//!
//! Called from `handle_group_end` in `pipeline.rs` to finalize header, footer,
//! or note content when the corresponding RTF destination group closes.

use super::state::RuntimeState;

/// Process a group-end event for structure destinations.
///
/// When the parser exits a header, footer, or note group, this function:
/// 1. Finalizes any remaining paragraph content into the current sink buffer.
/// 2. Calls `exit_sink_if_at_depth` to move accumulated blocks into their
///    final channel and reset the sink to `Body`.
/// 3. For note sinks, restores the saved body paragraph state so parsing
///    continues in the body paragraph after the note group.
///
/// Is a no-op when the current sink is `Body`.
pub fn process_structure_group_end(state: &mut RuntimeState) {
    if state.structure.in_body() {
        return;
    }

    // current_depth has already been decremented by handle_group_end before
    // this function is called.  We exit the sink when the depth drops below
    // the depth at which the sink group was opened.
    if state.current_depth < state.structure.sink_group_depth {
        // Finalize any remaining paragraph into the sink buffer.
        // finalize_paragraph internally flushes pending text and routes
        // through push_block_to_current_sink → structure sink when not in body.
        super::finalize::finalize_paragraph(state);

        // If there is still an open table context inside the structure group
        // (unusual but possible), close it.
        if state.tables.in_table() {
            super::finalize::finalize_current_table(state);
        }

        // Exit the sink: moves note blocks to completed_notes; resets to Body.
        state.structure.exit_sink_if_at_depth(state.current_depth);

        // Restore the body paragraph state that was saved when the structure
        // group was entered.  All structure sinks (header, footer, note) push
        // a saved state so that body content surrounding the group is
        // preserved correctly.
        if let Some((saved_para, saved_text, saved_alignment)) =
            state.structure.pop_saved_paragraph()
        {
            state.current_paragraph = saved_para;
            state.current_text = saved_text;
            state.paragraph_alignment = saved_alignment;
        }
    }
}
