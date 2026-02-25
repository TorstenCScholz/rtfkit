//! Document-end finalization helpers.

use super::super::state::RuntimeState;

/// Finalize the document at end of parsing.
pub fn finalize_document(state: &mut RuntimeState) {
    // Finalize any remaining content
    super::paragraphs::finalize_paragraph(state);

    // Finalize any remaining table context at document end
    if state.tables.in_table() {
        super::tables::finalize_current_table(state);
    }
}
