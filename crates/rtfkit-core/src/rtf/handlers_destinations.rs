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
    /// Dropped destination with a reason (obj, etc.)
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
    /// Picture destination - parse embedded image data
    Pict,
    /// Shape picture destination - preferred over nonshppict
    Shppict,
    /// Non-shape picture destination - fallback when no shppict
    Nonshppict,
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
        // Picture destinations - parse embedded images
        "pict" => Some(DestinationBehavior::Pict),
        "shppict" => Some(DestinationBehavior::Shppict),
        "nonshppict" => Some(DestinationBehavior::Nonshppict),
        // Destinations that represent currently unsupported visible content.
        "obj" | "objclass" | "objdata" | "picprop" | "datafield" | "header" | "headerl"
        | "headerr" | "footer" | "footerl" | "footerr" | "footnote" | "annotation" | "pn"
        | "pntext" | "pntxtb" | "pntxta" | "pnseclvl" => Some(DestinationBehavior::Dropped(
            "Dropped unsupported RTF destination content",
        )),
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
            DestinationBehavior::Pict => {
                // Picture destination - enter image parsing mode
                // Don't skip - we need to capture the image data
                state.image.start_pict(state.current_depth);
                state.destinations.destination_marker = false;
                state.mark_current_group_non_destination();
                return false; // Don't skip - continue processing
            }
            DestinationBehavior::Shppict => {
                // Prefer `\shppict` over sibling `\nonshppict` for the same parent group.
                let parent_depth = state.current_depth.saturating_sub(1);
                state.image.mark_shppict_parent(parent_depth);
                state.destinations.destination_marker = false;
                state.mark_current_group_non_destination();
                return false; // Don't skip - continue processing
            }
            DestinationBehavior::Nonshppict => {
                // Skip fallback nonshppict only when a sibling shppict exists.
                let parent_depth = state.current_depth.saturating_sub(1);
                if state.image.should_skip_nonshppict(parent_depth) {
                    state
                        .report_builder
                        .dropped_content("Dropped nonshppict in favor of shppict", None);
                    state.destinations.enter_destination();
                } else {
                    // Otherwise, process normally (may contain pict)
                    state.destinations.destination_marker = false;
                    state.mark_current_group_non_destination();
                    return false;
                }
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
            super::handlers_resources::handle_destination_group_start(
                state,
                state.destinations.skip_destination_depth,
            );
        }
        RtfEvent::GroupEnd => {
            super::handlers_lists::handle_destination_group_end(
                state,
                state.destinations.skip_destination_depth,
            );
            super::handlers_resources::handle_destination_group_end(
                state,
                state.destinations.skip_destination_depth,
            );

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
            super::handlers_resources::handle_destination_text(state, &text);
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
    if super::handlers_resources::handle_destination_control_word(state, word, parameter) {
        return;
    }

    super::handlers_lists::handle_list_table_control_word(state, word, parameter);
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
            destination_behavior("obj"),
            Some(DestinationBehavior::Dropped(_))
        ));
    }

    #[test]
    fn test_destination_behavior_pict() {
        assert_eq!(
            destination_behavior("pict"),
            Some(DestinationBehavior::Pict)
        );
        assert_eq!(
            destination_behavior("shppict"),
            Some(DestinationBehavior::Shppict)
        );
        assert_eq!(
            destination_behavior("nonshppict"),
            Some(DestinationBehavior::Nonshppict)
        );
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
