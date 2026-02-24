//! Style State Module
//!
//! This module contains `StyleState` for tracking character and paragraph
//! formatting state during RTF parsing.

use crate::Alignment;

/// Tracks the current formatting state within the RTF document.
///
/// The style state is pushed onto the group stack when entering a new group
/// and popped when exiting, allowing RTF's scoping rules to be properly handled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleState {
    // =============================================================================
    // Character Formatting (reset by \plain)
    // =============================================================================
    /// Whether text is bold
    pub bold: bool,
    /// Whether text is italic
    pub italic: bool,
    /// Whether text is underlined
    pub underline: bool,
    /// Current font index (from \fN)
    pub font_index: Option<i32>,
    /// Font size in half-points (from \fsN)
    pub font_size_half_points: Option<i32>,
    /// Current color index (from \cfN)
    pub color_index: Option<i32>,
    /// Current highlight color index (from \highlightN)
    pub highlight_color_index: Option<i32>,
    /// Current background color index (from \cbN)
    pub background_color_index: Option<i32>,

    // =============================================================================
    // Paragraph Formatting (reset by \pard)
    // =============================================================================
    /// Paragraph alignment
    pub alignment: Alignment,

    // =============================================================================
    // Paragraph Shading State (reset by \pard, NOT reset by \plain)
    // =============================================================================
    /// Paragraph background color index (from \cbpatN)
    pub paragraph_cbpat: Option<i32>,
    /// Paragraph pattern color index (from \cfpatN) - for Slice B
    pub paragraph_cfpat: Option<i32>,
    /// Paragraph shading percentage (from \shadingN) - for Slice B
    pub paragraph_shading: Option<i32>,
}

impl StyleState {
    /// Creates a new default style state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a snapshot of the current style state.
    /// This is used when pushing onto the group stack.
    pub fn snapshot(&self) -> Self {
        self.clone()
    }
}

/// Check if character-level style differs between two states.
pub fn character_style_changed(current: &StyleState, run: &StyleState) -> bool {
    current.bold != run.bold
        || current.italic != run.italic
        || current.underline != run.underline
        || current.font_index != run.font_index
        || current.font_size_half_points != run.font_size_half_points
        || current.color_index != run.color_index
        || current.highlight_color_index != run.highlight_color_index
        || current.background_color_index != run.background_color_index
}

/// Reset character formatting to defaults (called by \plain).
///
/// This resets ONLY character-style fields:
/// - bold, italic, underline
/// - font_index, font_size_half_points, color_index
/// - highlight_color_index, background_color_index
///
/// It does NOT reset:
/// - paragraph alignment
/// - list state
/// - table state
/// - paragraph shading state
pub fn reset_character_formatting(style: &mut StyleState, default_font: Option<i32>) {
    style.bold = false;
    style.italic = false;
    style.underline = false;
    // Reset font to default font index if available
    style.font_index = default_font;
    style.font_size_half_points = None;
    style.color_index = None;
    style.highlight_color_index = None;
    style.background_color_index = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_state_default() {
        let state = StyleState::new();
        assert!(!state.bold);
        assert!(!state.italic);
        assert!(!state.underline);
        assert_eq!(state.alignment, Alignment::Left);
        assert!(state.font_index.is_none());
        assert!(state.font_size_half_points.is_none());
        assert!(state.color_index.is_none());
    }

    #[test]
    fn test_style_state_snapshot() {
        let mut state = StyleState::new();
        state.bold = true;
        state.italic = true;
        state.font_index = Some(5);

        let snapshot = state.snapshot();
        assert_eq!(snapshot.bold, true);
        assert_eq!(snapshot.italic, true);
        assert_eq!(snapshot.font_index, Some(5));
    }

    #[test]
    fn test_character_style_changed() {
        let mut current = StyleState::new();
        let run = StyleState::new();

        // No change
        assert!(!character_style_changed(&current, &run));

        // Bold changed
        current.bold = true;
        assert!(character_style_changed(&current, &run));

        // Reset and test font change
        current.bold = false;
        current.font_index = Some(3);
        assert!(character_style_changed(&current, &run));
    }

    #[test]
    fn test_reset_character_formatting() {
        let mut state = StyleState::new();
        state.bold = true;
        state.italic = true;
        state.underline = true;
        state.font_index = Some(5);
        state.font_size_half_points = Some(24);
        state.color_index = Some(1);
        state.highlight_color_index = Some(2);
        state.background_color_index = Some(3);
        state.alignment = Alignment::Center;
        state.paragraph_cbpat = Some(4);

        reset_character_formatting(&mut state, Some(0));

        // Character formatting should be reset
        assert!(!state.bold);
        assert!(!state.italic);
        assert!(!state.underline);
        assert_eq!(state.font_index, Some(0)); // Set to default
        assert!(state.font_size_half_points.is_none());
        assert!(state.color_index.is_none());
        assert!(state.highlight_color_index.is_none());
        assert!(state.background_color_index.is_none());

        // Paragraph properties should NOT be reset
        assert_eq!(state.alignment, Alignment::Center);
        assert_eq!(state.paragraph_cbpat, Some(4));
    }
}
