//! Runtime State Module
//!
//! This module contains the top-level `RuntimeState` container that aggregates
//! all sub-states for RTF parsing.

use super::state_destinations::DestinationState;
use super::state_fields::FieldState;
use super::state_images::ImageParsingState;
use super::state_lists::ListState;
use super::state_resources::ResourceState;
use super::state_style::StyleState;
use super::state_tables::TableState;
use crate::error::ParseError;
use crate::limits::ParserLimits;
use crate::report::ReportBuilder;
use crate::{Alignment, Document, Paragraph};

/// Top-level runtime state for RTF parsing.
///
/// This struct aggregates all sub-states and provides the main state container
/// used during RTF parsing.
pub struct RuntimeState {
    // =============================================================================
    // Sub-States
    // =============================================================================
    /// Style state (character and paragraph formatting)
    pub style: StyleState,
    /// Destination state (skip tracking)
    pub destinations: DestinationState,
    /// List state (list tables and references)
    pub lists: ListState,
    /// Table state (tables, cells, merges, shading)
    pub tables: TableState,
    /// Field state (hyperlinks and other fields)
    pub fields: FieldState,
    /// Resource state (font and color tables)
    pub resources: ResourceState,
    /// Image state (embedded pictures)
    pub image: ImageParsingState,

    // =============================================================================
    // Core Parsing State
    // =============================================================================
    /// Stack of style states for group handling
    pub group_stack: Vec<StyleState>,
    /// Whether each open group still allows destination detection at its start
    pub group_can_start_destination: Vec<bool>,
    /// Current group depth (for limit enforcement)
    pub current_depth: usize,
    /// Parser limits for resource protection
    pub limits: ParserLimits,

    // =============================================================================
    // Document Building
    // =============================================================================
    /// The document being built
    pub document: Document,
    /// Current paragraph being built
    pub current_paragraph: Paragraph,
    /// Current text being aggregated
    pub current_text: String,
    /// Current run style (for detecting style changes)
    pub current_run_style: StyleState,
    /// Paragraph alignment (captured when paragraph starts)
    pub paragraph_alignment: Alignment,

    // =============================================================================
    // Unicode Handling
    // =============================================================================
    /// Number of fallback characters to skip after a \u escape (from \ucN)
    pub unicode_skip_count: usize,
    /// How many characters to skip next (set after processing \u)
    pub skip_next_chars: usize,

    // =============================================================================
    // Report Building
    // =============================================================================
    /// Report builder for collecting warnings and stats
    pub report_builder: ReportBuilder,

    // =============================================================================
    // Error Tracking
    // =============================================================================
    /// Fatal parser failure encountered mid-parse.
    ///
    /// Used for hard-limit violations discovered in helper methods that
    /// don't return `Result` directly.
    pub hard_failure: Option<ParseError>,

    // =============================================================================
    // Image Parsing State
    // =============================================================================
    /// Cumulative bytes used by decoded images (for limit enforcement)
    pub image_bytes_used: usize,
}

impl RuntimeState {
    /// Creates a new runtime state with the given limits.
    pub fn new(limits: ParserLimits) -> Self {
        Self {
            // Sub-states
            style: StyleState::new(),
            destinations: DestinationState::new(),
            lists: ListState::new(),
            tables: TableState::new(),
            fields: FieldState::new(),
            resources: ResourceState::new(),
            image: ImageParsingState::new(),

            // Core parsing state
            group_stack: Vec::new(),
            group_can_start_destination: Vec::new(),
            current_depth: 0,
            limits,

            // Document building
            document: Document::new(),
            current_paragraph: Paragraph::new(),
            current_text: String::new(),
            current_run_style: StyleState::new(),
            paragraph_alignment: Alignment::default(),

            // Unicode handling
            unicode_skip_count: 1, // Default: 1 fallback character
            skip_next_chars: 0,

            // Report building
            report_builder: ReportBuilder::new(),

            // Error tracking
            hard_failure: None,

            // Image parsing
            image_bytes_used: 0,
        }
    }

    /// Set a hard failure error.
    ///
    /// Only sets the error if one isn't already set (first failure wins).
    pub fn set_hard_failure(&mut self, err: ParseError) {
        if self.hard_failure.is_none() {
            self.hard_failure = Some(err);
        }
    }

    // =============================================================================
    // Group Stack Methods
    // =============================================================================

    /// Push current style onto the group stack (entering a group).
    pub fn push_group(&mut self) {
        self.group_stack.push(self.style.snapshot());
        self.group_can_start_destination.push(true);
    }

    /// Pop style from the group stack (exiting a group).
    pub fn pop_group(&mut self) -> Option<StyleState> {
        self.group_can_start_destination.pop();
        self.group_stack.pop()
    }

    /// Check if current position can start a destination.
    pub fn can_start_destination(&self) -> bool {
        self.group_can_start_destination
            .last()
            .copied()
            .unwrap_or(false)
    }

    /// Mark current group as non-destination.
    pub fn mark_current_group_non_destination(&mut self) {
        if let Some(can_start) = self.group_can_start_destination.last_mut() {
            *can_start = false;
        }
    }

    // =============================================================================
    // Paragraph State Methods
    // =============================================================================

    /// Check if there's pending paragraph content.
    pub fn has_pending_paragraph_content(&self) -> bool {
        !self.current_text.is_empty() || !self.current_paragraph.inlines.is_empty()
    }

    /// Reset paragraph state for a new paragraph.
    pub fn reset_paragraph_state(&mut self) {
        self.current_paragraph = Paragraph::new();
        self.current_run_style = self.style.snapshot();
        self.paragraph_alignment = self.style.alignment;
        self.lists.reset_paragraph_state();
        self.tables.reset_paragraph_state();
    }

    // =============================================================================
    // Style Methods
    // =============================================================================

    /// Check if character style has changed.
    pub fn character_style_changed(&self) -> bool {
        super::state_style::character_style_changed(&self.style, &self.current_run_style)
    }

    /// Reset character formatting (for \plain).
    pub fn reset_character_formatting(&mut self) {
        super::state_style::reset_character_formatting(
            &mut self.style,
            self.resources.default_font_index,
        );
    }

    // =============================================================================
    // Resource Methods
    // =============================================================================

    /// Resolve font family from current style.
    pub fn resolve_font_family(&self) -> Option<String> {
        // Try current font index first
        if let Some(font_idx) = self.current_run_style.font_index
            && let Some(family) = self.resources.get_font_family(font_idx)
        {
            return Some(family.to_string());
        }

        // Fall back to default font index
        if let Some(default_idx) = self.resources.default_font_index
            && let Some(family) = self.resources.get_font_family(default_idx)
        {
            return Some(family.to_string());
        }

        None
    }

    /// Resolve font size from current style.
    pub fn resolve_font_size(&self) -> Option<f32> {
        self.current_run_style
            .font_size_half_points
            .and_then(|hp| if hp > 0 { Some(hp as f32 / 2.0) } else { None })
    }

    /// Resolve foreground color from current style.
    pub fn resolve_color(&self) -> Option<crate::Color> {
        let color_idx = self.current_run_style.color_index?;
        self.resources.resolve_color(color_idx)
    }

    /// Resolve background color from current style.
    pub fn resolve_background_color(&self) -> Option<crate::Color> {
        // Try highlight_color_index first (takes precedence)
        if let Some(highlight_idx) = self.current_run_style.highlight_color_index
            && let Some(color) = self.resources.resolve_color(highlight_idx)
        {
            return Some(color);
        }

        // Fall back to background_color_index
        if let Some(bg_idx) = self.current_run_style.background_color_index
            && let Some(color) = self.resources.resolve_color(bg_idx)
        {
            return Some(color);
        }

        None
    }

    /// Resolve a color from a color table index.
    pub fn resolve_color_from_index(&self, color_idx: i32) -> Option<crate::Color> {
        self.resources.resolve_color(color_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_state_new() {
        let limits = ParserLimits::default();
        let state = RuntimeState::new(limits.clone());

        assert!(state.group_stack.is_empty());
        assert!(state.group_can_start_destination.is_empty());
        assert_eq!(state.current_depth, 0);
        assert_eq!(state.limits, limits);
        assert!(state.hard_failure.is_none());
        assert_eq!(state.unicode_skip_count, 1);
    }

    #[test]
    fn test_runtime_state_push_pop_group() {
        let mut state = RuntimeState::new(ParserLimits::default());
        state.style.bold = true;

        state.push_group();
        assert_eq!(state.group_stack.len(), 1);
        assert!(state.can_start_destination());

        state.style.bold = false;
        let popped = state.pop_group();
        assert!(popped.is_some());
        assert!(popped.unwrap().bold); // Should have the snapshot
        assert!(state.group_stack.is_empty());
    }

    #[test]
    fn test_runtime_state_mark_non_destination() {
        let mut state = RuntimeState::new(ParserLimits::default());
        state.push_group();

        assert!(state.can_start_destination());
        state.mark_current_group_non_destination();
        assert!(!state.can_start_destination());
    }

    #[test]
    fn test_runtime_state_set_hard_failure() {
        let mut state = RuntimeState::new(ParserLimits::default());
        let err = ParseError::EmptyInput;

        state.set_hard_failure(err.clone());
        assert!(state.hard_failure.is_some());

        // Second call should not overwrite
        state.set_hard_failure(ParseError::UnbalancedGroups);
        assert!(matches!(state.hard_failure, Some(ParseError::EmptyInput)));
    }

    #[test]
    fn test_runtime_state_has_pending_content() {
        let mut state = RuntimeState::new(ParserLimits::default());

        assert!(!state.has_pending_paragraph_content());

        state.current_text = "Hello".to_string();
        assert!(state.has_pending_paragraph_content());

        state.current_text.clear();
        assert!(!state.has_pending_paragraph_content());
    }
}
