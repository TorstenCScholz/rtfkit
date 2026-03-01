//! List finalization helpers.

use super::super::state::RuntimeState;
use crate::{Block, ListBlock, ListId, ListItem, ListKind, Paragraph, TableCell};

/// Resolve list reference from pending state.
pub fn resolve_list_reference(state: &mut RuntimeState) {
    // First, try to resolve from \\lsN (modern list mechanism)
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
            state.lists.current_list_ref = Some(super::super::state_lists::ParagraphListRef::new(
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
    if state.structure.in_body() {
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
    } else {
        // Inside a structure sink — push a fresh list block (no merging with
        // preceding blocks, which is acceptable for header/footer/note content).
        let mut list_block = ListBlock::new(list_id, kind);
        list_block.add_item(ListItem::from_paragraph(level, paragraph));
        state
            .structure
            .push_block_to_sink(Block::ListBlock(list_block));
    }
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
