//! List Handlers Module
//!
//! This module contains list-specific control handling for both paragraph
//! controls (e.g. `\\ls`, `\\ilvl`) and list table destinations.

use super::state::RuntimeState;

/// Handle paragraph-level list control words.
///
/// Returns `true` if `word` was recognized and handled.
pub fn handle_paragraph_list_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) -> bool {
    match word {
        // \\lsN - List identifier reference (from listoverridetable)
        "ls" => {
            state.lists.pending_ls_id = parameter;
            true
        }
        // \\ilvlN - List level (0-indexed)
        "ilvl" => {
            let level = parameter.and_then(|p| u8::try_from(p).ok()).unwrap_or(0);

            // Emit warning if level exceeds max
            if level > 8 {
                state.report_builder.unsupported_nesting_level(level, 8);
            }

            state.lists.pending_level = level.min(8); // Clamp to DOCX max
            true
        }
        // Legacy paragraph numbering controls are intentionally unsupported.
        "pnlvl" | "pnlvlblt" | "pnlvlbody" | "pnlvlcont" | "pnstart" | "pnindent" | "pntxta"
        | "pntxtb" => {
            state.report_builder.unsupported_list_control(word);
            state
                .report_builder
                .dropped_content("Dropped legacy paragraph numbering content", None);
            true
        }
        _ => false,
    }
}

/// Handle control words within list table/override table destinations.
pub fn handle_list_table_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) {
    // Only process if we're in list table parsing mode
    if !state.lists.parsing_list_table && !state.lists.parsing_list_override_table {
        return;
    }

    match word {
        // List table control words
        "list" => {
            // Start of a new list definition
            // We'll set the list_id when we encounter \\listidN
            state.lists.current_list_def = Some(super::state_lists::ParsedListDefinition::new(0));
        }
        "listlevel" => {
            // Start of a new level definition
            // \\listlevel has no numeric parameter; infer level by declaration order.
            let level = state
                .lists
                .current_list_def
                .as_ref()
                .map(|def| def.levels.len() as u8)
                .unwrap_or(0)
                .min(8);
            state.lists.current_list_level = Some(super::state_lists::ParsedListLevel {
                level,
                kind: crate::ListKind::Bullet, // Default, will be updated by \\levelnfcN
            });
        }
        "levelnfc" => {
            // Number format code determines the list kind
            // 0 = decimal, 23 = bullet, etc.
            if let Some(nfc) = parameter {
                let kind = match nfc {
                    0 => crate::ListKind::OrderedDecimal, // Decimal (1, 2, 3)
                    1 => crate::ListKind::OrderedDecimal, // Upper Roman (I, II, III)
                    2 => crate::ListKind::OrderedDecimal, // Lower Roman (i, ii, iii)
                    3 => crate::ListKind::OrderedDecimal, // Upper Alpha (A, B, C)
                    4 => crate::ListKind::OrderedDecimal, // Lower Alpha (a, b, c)
                    23 => crate::ListKind::Bullet,        // Bullet
                    _ => crate::ListKind::Bullet,         // Default to bullet for unknown
                };
                if let Some(ref mut level) = state.lists.current_list_level {
                    level.kind = kind;
                }
            }
        }

        // List override table control words
        "listoverride" => {
            // Start of a new list override
            // We'll set the values when we encounter \\listidN and \\lsN
            state.lists.current_list_override =
                Some(super::state_lists::ParsedListOverride::new(0, 0));
        }
        "ls" => {
            // In override table, this sets the ls_id
            if let Some(id) = parameter
                && state.lists.parsing_list_override_table
                && let Some(ref mut list_override) = state.lists.current_list_override
            {
                list_override.ls_id = id;
            }
        }
        "listoverridepad" => {
            // Padding/hack for Word compatibility - ignore
        }

        // listid is used in both listtable and listoverridetable
        "listid" => {
            if let Some(id) = parameter {
                if state.lists.parsing_list_table {
                    // In listtable, this sets the list definition's ID
                    if let Some(ref mut list_def) = state.lists.current_list_def {
                        list_def.list_id = id;
                    }
                } else if state.lists.parsing_list_override_table {
                    // In listoverridetable, this sets the referenced list_id
                    if let Some(ref mut list_override) = state.lists.current_list_override {
                        list_override.list_id = id;
                    }
                }
            }
        }

        // Other list-related control words we recognize but don't fully process
        "listtemplateid" | "listsimple" | "listhybrid" | "listname" | "liststyleid" | "levels"
        | "levelstartat" | "levelindent" | "levelspace" | "levelfollow" | "levellegal"
        | "levelnorestart" | "leveljcn" | "levelnfcn" | "levelnfcN" | "listoverridestart" => {
            // These are recognized but not fully processed - no warning needed
        }

        _ => {
            // Unknown control word in list table context
            // Report as unsupported list control
            state.report_builder.unsupported_list_control(word);
        }
    }
}

/// Handle destination group end actions for list destinations.
pub fn handle_destination_group_end(state: &mut RuntimeState, skip_destination_depth: usize) {
    // Finalize list definition when closing a \\list group
    if state.lists.parsing_list_table
        && skip_destination_depth == 2
        && let Some(list_def) = state.lists.current_list_def.take()
    {
        state.lists.list_table.insert(list_def.list_id, list_def);
    }

    // Finalize list override when closing a \\listoverride group
    if state.lists.parsing_list_override_table
        && skip_destination_depth == 2
        && let Some(list_override) = state.lists.current_list_override.take()
    {
        state
            .lists
            .list_overrides
            .insert(list_override.ls_id, list_override);
    }

    // Finalize list level when closing a \\listlevel group
    if state.lists.parsing_list_table
        && skip_destination_depth == 3
        && let Some(level) = state.lists.current_list_level.take()
        && let Some(ref mut list_def) = state.lists.current_list_def
    {
        list_def.levels.push(level);
    }
}
