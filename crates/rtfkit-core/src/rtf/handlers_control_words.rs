//! Control Word Handlers Module
//!
//! This module contains control word dispatch and handling for RTF parsing.

use super::state::RuntimeState;
use crate::{Alignment, CellMerge, CellVerticalAlign, RowAlignment};

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
        // List Control Words
        // =============================================================================
        // \lsN - List identifier reference (from listoverridetable)
        "ls" => {
            state.lists.pending_ls_id = parameter;
        }
        // \ilvlN - List level (0-indexed)
        "ilvl" => {
            let level = parameter.and_then(|p| u8::try_from(p).ok()).unwrap_or(0);

            // Emit warning if level exceeds max
            if level > 8 {
                state.report_builder.unsupported_nesting_level(level, 8);
            }

            state.lists.pending_level = level.min(8); // Clamp to DOCX max
        }
        // Legacy paragraph numbering controls are intentionally unsupported.
        "pnlvl" | "pnlvlblt" | "pnlvlbody" | "pnlvlcont" | "pnstart" | "pnindent" | "pntxta"
        | "pntxtb" => {
            state.report_builder.unsupported_list_control(word);
            state
                .report_builder
                .dropped_content("Dropped legacy paragraph numbering content", None);
        }

        // =============================================================================
        // Table Control Words
        // =============================================================================
        // \trowd - Start of a new table row definition
        "trowd" => {
            handle_trowd(state);
        }
        // \cellxN - Cell boundary position in twips
        "cellx" => {
            if let Some(boundary) = parameter {
                state.tables.record_cellx(boundary);
            }
        }
        // \intbl - Paragraph is inside a table
        "intbl" => {
            state.tables.seen_intbl_in_paragraph = true;
        }
        // \cell - End of a table cell
        "cell" => {
            // Note: cell handling will be done by the event processor
        }
        // \row - End of a table row
        "row" => {
            // Note: row handling will be done by the event processor
        }

        // =============================================================================
        // Row Property Controls
        // =============================================================================
        // Row alignment
        "trql" => {
            state.tables.pending_row_props.alignment = Some(RowAlignment::Left);
        }
        "trqc" => {
            state.tables.pending_row_props.alignment = Some(RowAlignment::Center);
        }
        "trqr" => {
            state.tables.pending_row_props.alignment = Some(RowAlignment::Right);
        }
        // Row left indent (in twips)
        "trleft" => {
            if let Some(value) = parameter {
                state.tables.pending_row_props.left_indent = Some(value);
            }
        }
        // Row formatting controls - recognized but not fully supported
        "trgaph" => {
            state.report_builder.unsupported_table_control(word);
        }

        // =============================================================================
        // Cell Vertical Alignment Controls
        // =============================================================================
        "clvertalt" => {
            state.tables.pending_cell_v_align = Some(CellVerticalAlign::Top);
        }
        "clvertalc" => {
            state.tables.pending_cell_v_align = Some(CellVerticalAlign::Center);
        }
        "clvertalb" => {
            state.tables.pending_cell_v_align = Some(CellVerticalAlign::Bottom);
        }

        // =============================================================================
        // Cell Merge Controls
        // =============================================================================
        // Horizontal merge start - this cell starts a merge
        "clmgf" => {
            // Mark this cell as merge start
            // Span will be calculated when we see continuation markers
            state.tables.pending_cell_merge = Some(CellMerge::HorizontalStart { span: 1 });
        }
        // Horizontal merge continuation - this cell is merged with previous
        "clmrg" => {
            state.tables.pending_cell_merge = Some(CellMerge::HorizontalContinue);
        }
        // Vertical merge start - this cell starts a vertical merge
        "clvmgf" => {
            state.tables.pending_cell_merge = Some(CellMerge::VerticalStart);
        }
        // Vertical merge continuation - this cell continues vertical merge
        "clvmrg" => {
            state.tables.pending_cell_merge = Some(CellMerge::VerticalContinue);
        }

        // =============================================================================
        // Paragraph Shading Controls
        // =============================================================================
        // \cbpatN - Paragraph background color index
        "cbpat" => {
            state.style.paragraph_cbpat = parameter;
        }
        // \cfpatN - Paragraph pattern color index (for Slice B)
        "cfpat" => {
            state.style.paragraph_cfpat = parameter;
        }
        // \shadingN - Paragraph shading percentage (for Slice B)
        "shading" => {
            state.style.paragraph_shading = parameter;
        }

        // =============================================================================
        // Cell Shading Controls
        // =============================================================================
        // \clcbpatN - Cell background color index
        "clcbpat" => {
            state.tables.pending_cell_cbpat = parameter;
        }
        // \clcfpatN - Cell pattern color index (for Slice B)
        "clcfpat" => {
            state.tables.pending_cell_cfpat = parameter;
        }
        // \clshdngN - Cell shading percentage (for Slice B)
        "clshdng" => {
            state.tables.pending_cell_shading = parameter;
        }

        // =============================================================================
        // Row/Table Fallback Shading Controls
        // =============================================================================
        // \trcbpatN - Row background color index
        "trcbpat" => {
            // Capture for row-level shading (applied when row is finalized)
            state.tables.pending_row_cbpat = parameter;
            // Also capture for table-level shading if no table shading set yet
            if state.tables.pending_table_cbpat.is_none() {
                state.tables.pending_table_cbpat = parameter;
            }
        }
        // \trcfpatN - Row pattern color index (for Slice B)
        "trcfpat" => {
            state.tables.pending_row_cfpat = parameter;
            // Capture first-row value as table-level default
            if state.tables.pending_table_cfpat.is_none() {
                state.tables.pending_table_cfpat = parameter;
            }
        }
        // \trshdngN - Row shading percentage (for Slice B)
        "trshdng" => {
            state.tables.pending_row_shading = parameter;
            // Capture first-row value as table-level default
            if state.tables.pending_table_shading.is_none() {
                state.tables.pending_table_shading = parameter;
            }
        }

        // =============================================================================
        // Field Control Words
        // =============================================================================
        // \field - Start of a field group
        "field" => {
            // Flush any pending text to the paragraph before starting the field
            // This ensures text like "Click " before a hyperlink stays outside the link
            super::handlers_text::flush_current_text_as_run(state);
            state
                .fields
                .start_field(state.current_depth, state.style.snapshot());
        }
        // \fldinst - Field instruction (contains HYPERLINK "url" for hyperlinks)
        "fldinst" => {
            state.fields.start_fldinst(state.current_depth);
        }
        // \fldrslt - Field result (visible content)
        "fldrslt" => {
            state.fields.start_fldrslt(state.current_depth);
        }

        // =============================================================================
        // RTF Header Control Words - Silently Ignored
        // =============================================================================
        "rtf" | "ansi" | "ansicpg" | "deflang" | "deflangfe" | "adeflang" | "result" | "hwid"
        | "emdash" | "endash" | "emspace" | "enspace" | "qmspace" | "bullet" | "lquote"
        | "rquote" | "ldblquote" | "rdblquote" | "tab" | "strike" | "striked" | "sub" | "super"
        | "nosupersub" | "caps" | "scaps" | "outl" | "shad" | "expnd" | "expndtw" | "kerning"
        | "charscalex" | "lang" | "langfe" | "langnp" | "langfenp" => {
            // Silently ignore these structural/formatting control words
        }

        // =============================================================================
        // Font and Color Controls
        // =============================================================================
        // \deffN - Default font index
        "deff" => {
            if let Some(index) = parameter {
                state.resources.default_font_index = Some(index);
                // Also set as current font if no font is currently set
                if state.style.font_index.is_none() {
                    state.style.font_index = Some(index);
                }
            }
        }
        // \fN - Font index
        "f" => {
            state.style.font_index = parameter;
        }
        // \fsN - Font size in half-points
        "fs" => {
            state.style.font_size_half_points = parameter;
        }
        // \cfN - Foreground color index
        "cf" => {
            state.style.color_index = parameter;
        }
        // \highlightN - Highlight color index
        "highlight" => {
            state.style.highlight_color_index =
                parameter.and_then(|n| if n > 0 { Some(n) } else { None });
        }
        // \cbN - Background color index
        "cb" => {
            state.style.background_color_index =
                parameter.and_then(|n| if n > 0 { Some(n) } else { None });
        }
        // \plain - Reset character formatting only
        "plain" => {
            state.reset_character_formatting();
        }

        // =============================================================================
        // Unknown Control Words
        // =============================================================================
        _ => {
            state
                .report_builder
                .unsupported_control_word(word, parameter);
        }
    }
}

// =============================================================================
// Table Row Definition Handler
// =============================================================================

/// Handle \trowd - start of a new table row definition.
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
    // If this is the first text in the paragraph, capture the alignment
    if state.current_paragraph.inlines.is_empty() && state.current_text.is_empty() {
        state.paragraph_alignment = state.style.alignment;
    }

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
        state.style.font_index = Some(5);
        state.style.color_index = Some(3);

        handle_control_word(&mut state, "plain", None);

        assert!(!state.style.bold);
        assert!(!state.style.italic);
        assert!(!state.style.underline);
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
