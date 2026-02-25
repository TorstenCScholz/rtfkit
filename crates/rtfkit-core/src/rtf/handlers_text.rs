//! Text Handlers Module
//!
//! This module contains text handling and run creation logic for RTF parsing.

use super::state::RuntimeState;

// =============================================================================
// Text Handling
// =============================================================================

/// Handle text content.
///
/// This is the main entry point for text processing during RTF parsing.
pub fn handle_text(state: &mut RuntimeState, text: String) {
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
///
/// This function handles:
/// - Table context management
/// - Paragraph alignment capture
/// - Style change detection
/// - Text accumulation
fn handle_text_internal(state: &mut RuntimeState, text: String) {
    // If we already completed table rows and prose starts, close table first.
    if state.current_paragraph.inlines.is_empty()
        && state.current_text.is_empty()
        && state.tables.current_table.is_some()
        && state.tables.current_row.is_none()
        && !state.tables.seen_intbl_in_paragraph
    {
        // Note: Table finalization will be handled by the event processor
    }

    // If this is the first text in the paragraph, capture the alignment
    if state.current_paragraph.inlines.is_empty() && state.current_text.is_empty() {
        state.paragraph_alignment = state.style.alignment;
    }

    // Check if style has changed
    if state.character_style_changed() {
        // Flush current text as a run if any
        flush_current_text_as_run(state);
        state.current_run_style = state.style.snapshot();
    }

    // Append text
    state.current_text.push_str(&text);
}

/// Flush current text as a run.
///
/// Creates a run from the current text and adds it to the current paragraph.
pub fn flush_current_text_as_run(state: &mut RuntimeState) {
    super::finalize::flush_current_text_as_run(state);
}

// =============================================================================
// Field Result Text Handling
// =============================================================================

/// Handle text within fldrslt (visible content of the field).
pub fn handle_field_result_text(state: &mut RuntimeState, text: String) {
    // Skip fallback characters if needed (after \u escape)
    if state.skip_next_chars > 0 {
        let skip = state.skip_next_chars.min(text.chars().count());
        let chars_to_take = text.chars().skip(skip);
        let remaining: String = chars_to_take.collect();
        state.skip_next_chars -= skip;

        if remaining.is_empty() {
            return;
        }

        handle_field_result_text_internal(state, remaining);
    } else {
        handle_field_result_text_internal(state, text);
    }
}

/// Internal handler for field result text.
fn handle_field_result_text_internal(state: &mut RuntimeState, text: String) {
    // Check if style has changed
    if state.character_style_changed() {
        // Flush current text as a run if any
        flush_current_text_as_field_run(state);
        state.current_run_style = state.style.snapshot();
    }

    // Append text
    state.current_text.push_str(&text);
}

/// Flush current text as a field run.
pub fn flush_current_text_as_field_run(state: &mut RuntimeState) {
    super::finalize::flush_current_text_as_field_run(state);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::ParserLimits;

    #[test]
    fn test_handle_text_basic() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_text(&mut state, "Hello".to_string());
        assert_eq!(state.current_text, "Hello");
    }

    #[test]
    fn test_handle_text_style_change() {
        let mut state = RuntimeState::new(ParserLimits::default());

        handle_text(&mut state, "Hello ".to_string());
        assert_eq!(state.current_text, "Hello ");

        // Change style
        state.style.bold = true;

        handle_text(&mut state, "World".to_string());
        // Should have flushed previous text as a run
        assert_eq!(state.current_paragraph.inlines.len(), 1);
        assert_eq!(state.current_text, "World");
    }

    #[test]
    fn test_create_run() {
        let mut state = RuntimeState::new(ParserLimits::default());

        state.current_text = "Test".to_string();
        state.current_run_style.bold = true;
        state.current_run_style.italic = true;

        let run = super::super::finalize::runs::create_run(&state);
        assert_eq!(run.text, "Test");
        assert!(run.bold);
        assert!(run.italic);
    }

    #[test]
    fn test_flush_current_text_as_run() {
        let mut state = RuntimeState::new(ParserLimits::default());

        state.current_text = "Hello".to_string();
        flush_current_text_as_run(&mut state);

        assert!(state.current_text.is_empty());
        assert_eq!(state.current_paragraph.inlines.len(), 1);
    }

    #[test]
    fn test_flush_current_text_as_run_empty() {
        let mut state = RuntimeState::new(ParserLimits::default());

        flush_current_text_as_run(&mut state);

        assert!(state.current_text.is_empty());
        assert!(state.current_paragraph.inlines.is_empty());
    }

    #[test]
    fn test_handle_text_unicode_skip() {
        let mut state = RuntimeState::new(ParserLimits::default());
        state.skip_next_chars = 2;

        // Should skip first 2 characters
        handle_text(&mut state, "abc".to_string());
        assert_eq!(state.current_text, "c");
        assert_eq!(state.skip_next_chars, 0);
    }

    #[test]
    fn test_handle_text_unicode_skip_all() {
        let mut state = RuntimeState::new(ParserLimits::default());
        state.skip_next_chars = 3;

        // Should skip all characters
        handle_text(&mut state, "abc".to_string());
        assert!(state.current_text.is_empty());
        assert_eq!(state.skip_next_chars, 0);
    }

    #[test]
    fn test_paragraph_alignment_capture() {
        let mut state = RuntimeState::new(ParserLimits::default());

        state.style.alignment = crate::Alignment::Center;
        handle_text(&mut state, "Test".to_string());

        assert_eq!(state.paragraph_alignment, crate::Alignment::Center);
    }
}
