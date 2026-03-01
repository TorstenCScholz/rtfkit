//! Control Word Handlers Module
//!
//! This module contains control word dispatch and handling for RTF parsing.

use super::state::RuntimeState;
use crate::Alignment;
use crate::ImageFormat;
use crate::PageNumberFormat;

// =============================================================================
// Main Control Word Handler
// =============================================================================

/// Handle a control word.
///
/// This is the main dispatch function for control word processing.
pub fn handle_control_word(state: &mut RuntimeState, word: &str, parameter: Option<i32>) {
    // Check if this starts a destination
    if super::handlers_destinations::maybe_start_destination(state, word) {
        return;
    }
    state.mark_current_group_non_destination();

    // Inside fldinst, treat control words as instruction text tokens.
    // This avoids applying formatting semantics to instruction groups and
    // preserves switches like \o, \h, etc. for hyperlink target parsing.
    if super::handlers_fields::capture_fldinst_control_word(state, word, parameter) {
        return;
    }

    match word {
        // =============================================================================
        // Character Formatting
        // =============================================================================
        // Bold: \b or \bN (N=0 turns off, N>0 turns on)
        "b" => {
            state.style.bold = parameter.map(|p| p != 0).unwrap_or(true);
        }
        // Italic: \i or \iN
        "i" => {
            state.style.italic = parameter.map(|p| p != 0).unwrap_or(true);
        }
        // Underline: \ul or \ulN
        "ul" => {
            state.style.underline = parameter.map(|p| p != 0).unwrap_or(true);
        }
        // Underline none: \ulnone
        "ulnone" => {
            state.style.underline = false;
        }
        // Strikethrough: \strike, \strikeN, \striked, or \strikedN
        // We degrade \striked (double strikethrough) to regular strikethrough.
        "strike" | "striked" => {
            state.style.strikethrough = parameter.map(|p| p != 0).unwrap_or(true);
        }
        // Small caps: \scaps or \scapsN
        "scaps" => {
            state.style.small_caps = parameter.map(|p| p != 0).unwrap_or(true);
        }
        // All caps: \caps or \capsN
        "caps" => {
            state.style.all_caps = parameter.map(|p| p != 0).unwrap_or(true);
        }

        // =============================================================================
        // Paragraph Breaks and Resets
        // =============================================================================
        // Paragraph break
        "par" | "pard" => {
            // \pard resets paragraph properties
            if word == "pard" {
                state.style.alignment = Alignment::default();
                // Also reset list state on \pard
                state.lists.reset_paragraph_state();
                // Reset paragraph-local table marker.
                state.tables.seen_intbl_in_paragraph = false;
                // Reset paragraph shading state (reset by \pard, NOT by \plain)
                state.style.paragraph_cbpat = None;
                state.style.paragraph_cfpat = None;
                state.style.paragraph_shading = None;
            } else {
                // Note: finalize_paragraph will be called by the event processor
                // This is a placeholder for the actual finalization logic
            }
        }
        // Line break (treated as paragraph for MVP)
        "line" => {
            // Note: finalize_paragraph will be called by the event processor
        }

        // =============================================================================
        // Alignment
        // =============================================================================
        "ql" => {
            state.style.alignment = Alignment::Left;
        }
        "qc" => {
            state.style.alignment = Alignment::Center;
        }
        "qr" => {
            state.style.alignment = Alignment::Right;
        }
        "qj" => {
            state.style.alignment = Alignment::Justify;
        }

        // =============================================================================
        // Unicode Handling
        // =============================================================================
        // Unicode escape: \uN (N is a signed 16-bit integer)
        "u" => {
            if let Some(code) = parameter {
                // RTF uses signed 16-bit integers for Unicode codepoints
                // Negative values represent codepoints in the range U+FFFF8000 to U+FFFFFFFF
                // which should be interpreted as unsigned for the actual Unicode codepoint
                let codepoint = if code < 0 {
                    (code as u16) as u32
                } else {
                    code as u32
                };
                if let Some(c) = char::from_u32(codepoint) {
                    // Note: Text handling will be done by the event processor
                    // Store the character for later processing
                    state.current_text.push(c);
                }
                // Mark that we need to skip the fallback characters
                state.skip_next_chars = state.unicode_skip_count;
            }
        }
        // Unicode skip count: \ucN (number of fallback characters after \u)
        "uc" => {
            state.unicode_skip_count = parameter.and_then(|p| usize::try_from(p).ok()).unwrap_or(1);
        }

        // =============================================================================
        // Section / Page Numbering
        // =============================================================================
        // Section break
        "sect" => {
            // Keep paragraph boundaries stable at section transitions.
            super::finalize::finalize_paragraph(state);
            state.start_new_section();
        }
        // Section defaults group marker (tracked by section controls below).
        "sectd" => {}
        // Restart page numbering in current section.
        "pgnrestart" => {
            state.set_current_section_restart(true);
        }
        // Explicit section page start.
        "pgnstarts" => {
            let start = parameter.and_then(|p| u32::try_from(p).ok()).filter(|p| *p > 0);
            state.set_current_section_start_page(start);
            if start.is_none() && parameter.is_some() {
                state
                    .report_builder
                    .section_numbering_fallback("Invalid \\pgnstarts value; ignored");
            }
        }
        // Page-number format controls.
        "pgndec" => {
            state.set_current_section_number_format(PageNumberFormat::Arabic);
        }
        "pgnlcrm" => {
            state.set_current_section_number_format(PageNumberFormat::RomanLower);
        }
        "pgnucrm" => {
            state.set_current_section_number_format(PageNumberFormat::RomanUpper);
        }

        // =============================================================================
        // RTF Header Control Words - Silently Ignored
        // =============================================================================
        "rtf" | "ansi" | "ansicpg" | "deflang" | "deflangfe" | "adeflang" | "result" | "hwid"
        | "emdash" | "endash" | "emspace" | "enspace" | "qmspace" | "bullet" | "lquote"
        | "rquote" | "ldblquote" | "rdblquote" | "tab" | "sub" | "super" | "nosupersub"
        | "outl" | "shad" | "expnd" | "expndtw" | "kerning" | "charscalex" | "lang" | "langfe"
        | "langnp" | "langfenp" => {
            // Silently ignore these structural/formatting control words
        }

        // \plain - Reset character formatting only
        "plain" => {
            state.reset_character_formatting();
        }

        // =============================================================================
        // Domain Delegation + Unknown Control Words
        // =============================================================================
        _ => {
            if super::handlers_lists::handle_paragraph_list_control_word(state, word, parameter) {
                return;
            }
            if super::handlers_tables::handle_table_control_word(state, word, parameter) {
                return;
            }
            if super::handlers_fields::handle_field_control_word(state, word) {
                return;
            }
            if super::handlers_resources::handle_resource_control_word(state, word, parameter) {
                return;
            }
            if handle_image_control_word(state, word, parameter) {
                return;
            }
            state
                .report_builder
                .unsupported_control_word(word, parameter);
        }
    }
}

// =============================================================================
// Image Control Word Handler
// =============================================================================

/// Handle image-related control words.
///
/// These control words are only effective when `state.image.parsing_pict` is true.
/// Returns `true` if the control word was handled.
pub fn handle_image_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) -> bool {
    // Only process image control words when parsing a pict group
    if !state.image.parsing_pict {
        return false;
    }

    match word {
        // Format identification
        "pngblip" => {
            state.image.format = Some(ImageFormat::Png);
            true
        }
        "jpegblip" => {
            state.image.format = Some(ImageFormat::Jpeg);
            true
        }
        // Dimensions in twips
        "picw" => {
            state.image.picw = parameter;
            true
        }
        "pich" => {
            state.image.pich = parameter;
            true
        }
        // Goal dimensions in twips (preferred over picw/pich)
        "picwgoal" => {
            state.image.picwgoal = parameter;
            true
        }
        "pichgoal" => {
            state.image.pichgoal = parameter;
            true
        }
        // Scaling percentages (default 100)
        "picscalex" => {
            state.image.picscalex = parameter.unwrap_or(100);
            true
        }
        "picscaley" => {
            state.image.picscaley = parameter.unwrap_or(100);
            true
        }
        _ => false,
    }
}

// =============================================================================
// Control Symbol Handler
// =============================================================================

/// Handle a control symbol.
///
/// Control symbols are single-character controls like `\*`, `\\`, `\{`, etc.
pub fn handle_control_symbol(state: &mut RuntimeState, symbol: char) {
    match symbol {
        // Destination marker at the start of a group: {\*\destination ...}
        '*' if state.can_start_destination() => {
            state.destinations.destination_marker = true;
        }
        '\\' => {
            state.mark_current_group_non_destination();
            // Handle escaped backslash as text
            handle_text_from_symbol(state, "\\".to_string());
        }
        '{' => {
            state.mark_current_group_non_destination();
            // Handle escaped brace as text
            handle_text_from_symbol(state, "{".to_string());
        }
        '}' => {
            state.mark_current_group_non_destination();
            // Handle escaped brace as text
            handle_text_from_symbol(state, "}".to_string());
        }
        '~' => {
            state.mark_current_group_non_destination();
            // Non-breaking space
            handle_text_from_symbol(state, "\u{00A0}".to_string());
        }
        '_' => {
            state.mark_current_group_non_destination();
            // Non-breaking hyphen
            handle_text_from_symbol(state, "\u{2011}".to_string());
        }
        '-' | '\n' | '\r' => {
            // Optional hyphen / source formatting characters are ignored.
            state.mark_current_group_non_destination();
        }
        _ => {
            state.mark_current_group_non_destination();
        }
    }
}

/// Handle text generated from control symbols.
fn handle_text_from_symbol(state: &mut RuntimeState, text: String) {
    // Skip fallback characters if needed (after \u escape)
    if state.skip_next_chars > 0 {
        let skip = state.skip_next_chars.min(text.chars().count());
        let chars_to_take = text.chars().skip(skip);
        let remaining: String = chars_to_take.collect();
        state.skip_next_chars -= skip;

        if remaining.is_empty() {
            return;
        }

        // Process remaining text
        handle_text_internal(state, remaining);
    } else {
        handle_text_internal(state, text);
    }
}

/// Internal text handling (after skip logic).
fn handle_text_internal(state: &mut RuntimeState, text: String) {
    state.capture_paragraph_alignment_if_start();

    // Check if style has changed
    if state.character_style_changed() {
        // Flush current text as a run if any
        if !state.current_text.is_empty() {
            // Note: Run creation will be handled by the event processor
        }
        state.current_run_style = state.style.snapshot();
    }

    // Append text
    state.current_text.push_str(&text);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CellMerge;
    use crate::limits::ParserLimits;

    #[test]
    fn test_handle_control_word_bold() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "b", None);
        assert!(state.style.bold);

        handle_control_word(&mut state, "b", Some(0));
        assert!(!state.style.bold);

        handle_control_word(&mut state, "b", Some(1));
        assert!(state.style.bold);
    }

    #[test]
    fn test_handle_control_word_italic() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "i", None);
        assert!(state.style.italic);

        handle_control_word(&mut state, "i", Some(0));
        assert!(!state.style.italic);
    }

    #[test]
    fn test_handle_control_word_underline() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "ul", None);
        assert!(state.style.underline);

        handle_control_word(&mut state, "ulnone", None);
        assert!(!state.style.underline);
    }

    #[test]
    fn test_handle_control_word_strikethrough_variants() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "strike", None);
        assert!(state.style.strikethrough);

        handle_control_word(&mut state, "strike", Some(0));
        assert!(!state.style.strikethrough);

        handle_control_word(&mut state, "striked", None);
        assert!(state.style.strikethrough);

        handle_control_word(&mut state, "striked", Some(0));
        assert!(!state.style.strikethrough);
    }

    #[test]
    fn test_handle_control_word_caps_variants() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "scaps", None);
        assert!(state.style.small_caps);

        handle_control_word(&mut state, "scaps", Some(0));
        assert!(!state.style.small_caps);

        handle_control_word(&mut state, "caps", None);
        assert!(state.style.all_caps);

        handle_control_word(&mut state, "caps", Some(0));
        assert!(!state.style.all_caps);
    }

    #[test]
    fn test_handle_control_word_alignment() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "qc", None);
        assert_eq!(state.style.alignment, Alignment::Center);

        handle_control_word(&mut state, "qr", None);
        assert_eq!(state.style.alignment, Alignment::Right);

        handle_control_word(&mut state, "qj", None);
        assert_eq!(state.style.alignment, Alignment::Justify);

        handle_control_word(&mut state, "ql", None);
        assert_eq!(state.style.alignment, Alignment::Left);
    }

    #[test]
    fn test_handle_control_word_font() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "f", Some(5));
        assert_eq!(state.style.font_index, Some(5));

        handle_control_word(&mut state, "fs", Some(24));
        assert_eq!(state.style.font_size_half_points, Some(24));
    }

    #[test]
    fn test_handle_control_word_color() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "cf", Some(3));
        assert_eq!(state.style.color_index, Some(3));

        handle_control_word(&mut state, "highlight", Some(2));
        assert_eq!(state.style.highlight_color_index, Some(2));

        handle_control_word(&mut state, "cb", Some(1));
        assert_eq!(state.style.background_color_index, Some(1));
    }

    #[test]
    fn test_handle_control_word_plain() {
        let mut state = RuntimeState::new(ParserLimits::default());

        state.style.bold = true;
        state.style.italic = true;
        state.style.underline = true;
        state.style.strikethrough = true;
        state.style.small_caps = true;
        state.style.all_caps = true;
        state.style.font_index = Some(5);
        state.style.color_index = Some(3);

        handle_control_word(&mut state, "plain", None);

        assert!(!state.style.bold);
        assert!(!state.style.italic);
        assert!(!state.style.underline);
        assert!(!state.style.strikethrough);
        assert!(!state.style.small_caps);
        assert!(!state.style.all_caps);
        assert!(state.style.font_index.is_none());
        assert!(state.style.color_index.is_none());
    }

    #[test]
    fn test_handle_control_word_unicode_skip() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "uc", Some(2));
        assert_eq!(state.unicode_skip_count, 2);
    }

    #[test]
    fn test_handle_control_word_list() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "ls", Some(5));
        assert_eq!(state.lists.pending_ls_id, Some(5));

        handle_control_word(&mut state, "ilvl", Some(2));
        assert_eq!(state.lists.pending_level, 2);
    }

    #[test]
    fn test_handle_control_word_list_level_clamped() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "ilvl", Some(15));
        assert_eq!(state.lists.pending_level, 8); // Clamped to max
    }

    #[test]
    fn test_handle_control_word_cell_merge() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_word(&mut state, "clmgf", None);
        assert!(matches!(
            state.tables.pending_cell_merge,
            Some(CellMerge::HorizontalStart { span: 1 })
        ));

        handle_control_word(&mut state, "clmrg", None);
        assert!(matches!(
            state.tables.pending_cell_merge,
            Some(CellMerge::HorizontalContinue)
        ));
    }

    #[test]
    fn test_handle_control_symbol_escape() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_symbol(&mut state, '\\');
        assert!(state.current_text.contains('\\'));

        state.current_text.clear();
        handle_control_symbol(&mut state, '{');
        assert!(state.current_text.contains('{'));

        state.current_text.clear();
        handle_control_symbol(&mut state, '}');
        assert!(state.current_text.contains('}'));
    }

    #[test]
    fn test_handle_control_symbol_nbsp() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_control_symbol(&mut state, '~');
        assert!(state.current_text.contains('\u{00A0}'));
    }
}
