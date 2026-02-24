//! Destination Handlers Module
//!
//! This module contains destination detection and skip logic for handling
//! RTF destination groups like `{\*\destination ...}`.

use super::events::RtfEvent;
use super::state::RuntimeState;
use crate::error::{ConversionError, ParseError};

// =============================================================================
// Destination Behavior Classification
// =============================================================================

/// Destination behavior classification.
///
/// Defines how different RTF destinations should be handled during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationBehavior {
    /// Metadata destination (stylesheet, info, etc.) - skip silently
    Metadata,
    /// Dropped destination with a reason (pict, obj, etc.)
    Dropped(&'static str),
    /// List table destination - parse list definitions
    ListTable,
    /// List override table destination - parse list overrides
    ListOverrideTable,
    /// Field instruction destination - handled specially for hyperlink parsing
    FldInst,
    /// Field result destination - handled specially for hyperlink parsing
    FldRslt,
    /// Font table destination - parse font definitions
    FontTable,
    /// Color table destination - parse color definitions
    ColorTable,
}

// =============================================================================
// Destination Behavior Lookup
// =============================================================================

/// Get destination behavior for a word.
///
/// Returns `None` if the word is not a recognized destination.
pub fn destination_behavior(word: &str) -> Option<DestinationBehavior> {
    match word {
        // List table destinations - need special parsing
        "listtable" => Some(DestinationBehavior::ListTable),
        "listoverridetable" => Some(DestinationBehavior::ListOverrideTable),
        // Field destinations - handled specially for hyperlink parsing
        "fldinst" => Some(DestinationBehavior::FldInst),
        "fldrslt" => Some(DestinationBehavior::FldRslt),
        // Font table - parse font definitions
        "fonttbl" => Some(DestinationBehavior::FontTable),
        // Color table - parse color definitions
        "colortbl" => Some(DestinationBehavior::ColorTable),
        // Metadata destinations that are intentionally excluded from body content.
        "stylesheet" | "info" | "title" | "author" | "operator" | "keywords" | "comment"
        | "version" | "vern" | "creatim" | "revtim" | "printim" | "buptim" | "edmins"
        | "nofpages" | "nofwords" | "nofchars" | "nofcharsws" | "id" => {
            Some(DestinationBehavior::Metadata)
        }
        // Destinations that represent currently unsupported visible content.
        "pict" | "obj" | "objclass" | "objdata" | "shppict" | "nonshppict" | "picprop"
        | "datafield" | "header" | "headerl" | "headerr" | "footer" | "footerl" | "footerr"
        | "footnote" | "annotation" | "pn" | "pntext" | "pntxtb" | "pntxta" | "pnseclvl" => Some(
            DestinationBehavior::Dropped("Dropped unsupported RTF destination content"),
        ),
        _ => None,
    }
}

// =============================================================================
// Destination Start Detection
// =============================================================================

/// Attempt to start a destination, returns true if handled.
///
/// This function checks if the current position can start a destination
/// and handles the destination based on its behavior type.
pub fn maybe_start_destination(state: &mut RuntimeState, word: &str) -> bool {
    if !state.can_start_destination() {
        state.destinations.destination_marker = false;
        return false;
    }

    if let Some(behavior) = destination_behavior(word) {
        match behavior {
            DestinationBehavior::Dropped(reason) => {
                state.report_builder.dropped_content(reason, None);
                state.destinations.enter_destination();
            }
            DestinationBehavior::Metadata => {
                state.destinations.enter_destination();
            }
            DestinationBehavior::ListTable => {
                state.lists.start_list_table();
                state.destinations.enter_destination();
            }
            DestinationBehavior::ListOverrideTable => {
                state.lists.start_list_override_table();
                state.destinations.enter_destination();
            }
            DestinationBehavior::FontTable => {
                state.resources.start_font_table();
                state.destinations.enter_destination();
            }
            DestinationBehavior::ColorTable => {
                state.resources.start_color_table();
                state.destinations.enter_destination();
            }
            DestinationBehavior::FldInst => {
                // Field instruction - handle specially for hyperlink parsing
                // Set state but don't skip - we need to capture the instruction text
                state.fields.start_fldinst(state.current_depth);
                state.destinations.destination_marker = false;
                state.mark_current_group_non_destination();
                return false; // Don't skip - continue processing
            }
            DestinationBehavior::FldRslt => {
                // Field result - handle specially for hyperlink parsing
                // Set state but don't skip - we need to capture the result content
                state.fields.start_fldrslt(state.current_depth);
                state.destinations.destination_marker = false;
                state.mark_current_group_non_destination();
                return false; // Don't skip - continue processing
            }
        }
        state.destinations.destination_marker = false;
        state.mark_current_group_non_destination();
        return true;
    }

    if state.destinations.destination_marker {
        state.report_builder.unknown_destination(word);
        let reason = format!("Dropped unknown destination group \\{word}");
        state.report_builder.dropped_content(&reason, None);
        state.destinations.enter_destination();
        state.destinations.destination_marker = false;
        state.mark_current_group_non_destination();
        return true;
    }

    false
}

// =============================================================================
// Skipped Destination Event Processing
// =============================================================================

/// Process an event while skipping a destination.
///
/// This handles events within skipped destination groups, including
/// nested group tracking and special handling for list/font/color tables.
pub fn process_skipped_destination_event(
    state: &mut RuntimeState,
    event: RtfEvent,
) -> Result<(), ConversionError> {
    if let Some(err) = state.hard_failure.take() {
        return Err(ConversionError::Parse(err));
    }

    match event {
        RtfEvent::GroupStart => {
            // Check depth limit
            state.current_depth += 1;
            if state.current_depth > state.limits.max_group_depth {
                return Err(ConversionError::Parse(ParseError::GroupDepthExceeded {
                    depth: state.current_depth,
                    limit: state.limits.max_group_depth,
                }));
            }
            state.push_group();
            state.group_can_start_destination.push(false);
            state.destinations.enter_destination();

            // Handle nested groups in list table parsing
            if state.lists.parsing_list_table && state.destinations.skip_destination_depth == 2 {
                // Starting a new \list or \listlevel group
                // The depth check helps us know we're inside the listtable destination
            }
            if state.lists.parsing_list_override_table
                && state.destinations.skip_destination_depth == 2
            {
                // Starting a new \listoverride group
            }
            // Handle nested groups in font table parsing (each font entry is in a group)
            if state.resources.parsing_font_table && state.destinations.skip_destination_depth == 2
            {
                // Starting a new font entry group - reset font parsing state
                state.resources.current_font_index = None;
                state.resources.current_font_name.clear();
            }
        }
        RtfEvent::GroupEnd => {
            // Finalize list definition when closing a \list group
            if state.lists.parsing_list_table
                && state.destinations.skip_destination_depth == 2
                && let Some(list_def) = state.lists.current_list_def.take()
            {
                state.lists.list_table.insert(list_def.list_id, list_def);
            }

            // Finalize list override when closing a \listoverride group
            if state.lists.parsing_list_override_table
                && state.destinations.skip_destination_depth == 2
                && let Some(list_override) = state.lists.current_list_override.take()
            {
                state
                    .lists
                    .list_overrides
                    .insert(list_override.ls_id, list_override);
            }

            // Finalize list level when closing a \listlevel group
            if state.lists.parsing_list_table
                && state.destinations.skip_destination_depth == 3
                && let Some(level) = state.lists.current_list_level.take()
                && let Some(ref mut list_def) = state.lists.current_list_def
            {
                list_def.levels.push(level);
            }

            // Finalize font entry when closing a font group
            if state.resources.parsing_font_table && state.destinations.skip_destination_depth == 2
            {
                state.resources.finalize_font_entry();
            }

            if let Some(previous_style) = state.pop_group() {
                state.style = previous_style;
            }
            state.current_depth = state.current_depth.saturating_sub(1);
            state.destinations.exit_destination();

            if !state.destinations.is_skipping() {
                state.destinations.destination_marker = false;
                state.lists.end_list_parsing();
                state.resources.parsing_font_table = false;
                state.resources.parsing_color_table = false;
            }
        }
        RtfEvent::ControlWord { word, parameter } => {
            handle_destination_control_word(state, &word, parameter);
        }
        RtfEvent::Text(text) => {
            // Handle text in font/color table destinations
            if state.resources.parsing_font_table {
                // Accumulate font name text
                state.resources.append_font_name(&text);
            } else if state.resources.parsing_color_table {
                // Handle semicolons in color table
                // Semicolons separate color entries
                for ch in text.chars() {
                    if ch == ';' {
                        state.resources.finalize_color();
                    }
                }
            }
            // Other destinations ignore text
        }
        RtfEvent::ControlSymbol(_) => {}
    }

    if let Some(err) = state.hard_failure.take() {
        return Err(ConversionError::Parse(err));
    }

    Ok(())
}

// =============================================================================
// Destination Control Word Handling
// =============================================================================

/// Handle control words within destination parsing (list table, font table, color table).
pub fn handle_destination_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) {
    // Font table control words
    if state.resources.parsing_font_table {
        match word {
            "f" => {
                // Font index
                state
                    .resources
                    .set_current_font_index(parameter.unwrap_or(0));
            }
            // Font property controls - we skip these but don't warn
            "fnil" | "froman" | "fswiss" | "fmodern" | "fscript" | "fdecor" | "ftech" | "fbidi" => {
                // Font family - ignore
            }
            "fcharset" => {
                // Character set - ignore
            }
            "fprq" => {
                // Font pitch - ignore
            }
            "panose" | "ftnil" | "fttruetype" => {
                // Font technology - ignore
            }
            _ => {
                // For font table, we don't warn on unknown controls
                // They're likely font-specific properties we don't need
            }
        }
        return;
    }

    // Color table control words
    if state.resources.parsing_color_table {
        match word {
            "red" => {
                if let Some(val) = parameter {
                    state.resources.set_red(val);
                }
            }
            "green" => {
                if let Some(val) = parameter {
                    state.resources.set_green(val);
                }
            }
            "blue" => {
                if let Some(val) = parameter {
                    state.resources.set_blue(val);
                }
            }
            // Theme color controls
            "themecolor" => {
                if let Some(val) = parameter {
                    state.resources.set_theme_color(val);
                }
            }
            "ctint" => {
                if let Some(val) = parameter {
                    state.resources.set_theme_tint(val);
                }
            }
            "cshade" => {
                if let Some(val) = parameter {
                    state.resources.set_theme_shade(val);
                }
            }
            _ => {
                // Unknown control in color table - ignore silently
            }
        }
        return;
    }

    // List table control words (existing logic)
    handle_list_table_control_word(state, word, parameter);
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
            // We'll set the list_id when we encounter \listidN
            state.lists.current_list_def = Some(super::state_lists::ParsedListDefinition::new(0));
        }
        "listlevel" => {
            // Start of a new level definition
            // \listlevel has no numeric parameter; infer level by declaration order.
            let level = state
                .lists
                .current_list_def
                .as_ref()
                .map(|def| def.levels.len() as u8)
                .unwrap_or(0)
                .min(8);
            state.lists.current_list_level = Some(super::state_lists::ParsedListLevel {
                level,
                kind: crate::ListKind::Bullet, // Default, will be updated by \levelnfcN
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
            // We'll set the values when we encounter \listidN and \lsN
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destination_behavior_listtable() {
        assert_eq!(
            destination_behavior("listtable"),
            Some(DestinationBehavior::ListTable)
        );
    }

    #[test]
    fn test_destination_behavior_fonttbl() {
        assert_eq!(
            destination_behavior("fonttbl"),
            Some(DestinationBehavior::FontTable)
        );
    }

    #[test]
    fn test_destination_behavior_colortbl() {
        assert_eq!(
            destination_behavior("colortbl"),
            Some(DestinationBehavior::ColorTable)
        );
    }

    #[test]
    fn test_destination_behavior_metadata() {
        assert_eq!(
            destination_behavior("stylesheet"),
            Some(DestinationBehavior::Metadata)
        );
        assert_eq!(
            destination_behavior("info"),
            Some(DestinationBehavior::Metadata)
        );
    }

    #[test]
    fn test_destination_behavior_dropped() {
        assert!(matches!(
            destination_behavior("pict"),
            Some(DestinationBehavior::Dropped(_))
        ));
        assert!(matches!(
            destination_behavior("obj"),
            Some(DestinationBehavior::Dropped(_))
        ));
    }

    #[test]
    fn test_destination_behavior_unknown() {
        assert_eq!(destination_behavior("unknowndest"), None);
    }

    #[test]
    fn test_destination_behavior_field() {
        assert_eq!(
            destination_behavior("fldinst"),
            Some(DestinationBehavior::FldInst)
        );
        assert_eq!(
            destination_behavior("fldrslt"),
            Some(DestinationBehavior::FldRslt)
        );
    }
}
