//! Field State Module
//!
//! This module contains field and hyperlink state for handling
//! RTF field parsing (including hyperlinks).

use super::state_style::StyleState;
use crate::Inline;

/// Nested field tracking state.
///
/// Used to track nested fields within a field result (fldrslt).
/// Nested fields are degraded to plain text.
#[derive(Debug, Clone, Default)]
pub struct NestedFieldState {
    /// Depth of the field group
    pub field_group_depth: usize,
    /// Whether currently parsing fldinst
    pub parsing_fldinst: bool,
    /// Depth of the fldinst group
    pub fldinst_group_depth: usize,
    /// Whether currently parsing fldrslt
    pub parsing_fldrslt: bool,
    /// Depth of the fldrslt group
    pub fldrslt_group_depth: usize,
}

/// Field parsing state.
///
/// Tracks field parsing state including instruction text, result content,
/// and nested field handling.
#[derive(Debug, Clone, Default)]
pub struct FieldState {
    // =============================================================================
    // Field Parsing Flags
    // =============================================================================
    /// Whether we're currently parsing a field group
    pub parsing_field: bool,
    /// Depth of the field group (for tracking nested groups within field)
    pub field_group_depth: usize,
    /// Whether we're currently in the fldinst (instruction) part of a field
    pub parsing_fldinst: bool,
    /// Depth of the fldinst group
    pub fldinst_group_depth: usize,
    /// Whether we're currently in the fldrslt (result) part of a field
    pub parsing_fldrslt: bool,
    /// Depth of the fldrslt group
    pub fldrslt_group_depth: usize,

    // =============================================================================
    // Field Content
    // =============================================================================
    /// Accumulated instruction text from fldinst
    pub field_instruction_text: String,
    /// Accumulated runs from fldrslt (visible content)
    pub field_result_inlines: Vec<Inline>,
    /// Style state at field start (to restore after field)
    pub field_style_snapshot: Option<StyleState>,

    // =============================================================================
    // Nested Fields
    // =============================================================================
    /// Nested field state stack (fields inside fldrslt are degraded to plain text)
    pub nested_fields: Vec<NestedFieldState>,

    // =============================================================================
    // Bookmark Start Tracking
    // =============================================================================
    /// Whether we're currently capturing the name of a \bkmkstart group
    pub parsing_bkmkstart: bool,
    /// Depth of the \bkmkstart group (to detect when we've exited it)
    pub bkmkstart_group_depth: usize,
    /// Accumulated bookmark name text from within the \bkmkstart group
    pub bkmkstart_name: String,
}

impl FieldState {
    /// Creates a new default field state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start parsing a new field.
    pub fn start_field(&mut self, current_depth: usize, current_style: StyleState) {
        if self.parsing_field {
            // Nested fields are degraded: push nested state
            self.nested_fields.push(NestedFieldState {
                field_group_depth: current_depth,
                ..Default::default()
            });
        } else {
            self.parsing_field = true;
            self.field_group_depth = current_depth;
            self.field_instruction_text.clear();
            self.field_result_inlines.clear();
            self.field_style_snapshot = Some(current_style);
            self.parsing_fldinst = false;
            self.parsing_fldrslt = false;
        }
    }

    /// Start parsing fldinst.
    pub fn start_fldinst(&mut self, current_depth: usize) {
        if let Some(nested) = self.nested_fields.last_mut() {
            nested.parsing_fldinst = true;
            nested.fldinst_group_depth = current_depth;
        } else if self.parsing_field {
            self.parsing_fldinst = true;
            self.fldinst_group_depth = current_depth;
        }
    }

    /// Start parsing fldrslt.
    pub fn start_fldrslt(&mut self, current_depth: usize) {
        if let Some(nested) = self.nested_fields.last_mut() {
            nested.parsing_fldrslt = true;
            nested.fldrslt_group_depth = current_depth;
        } else if self.parsing_field {
            self.parsing_fldrslt = true;
            self.fldrslt_group_depth = current_depth;
        }
    }

    /// Start capturing a bookmark name from a \bkmkstart group.
    pub fn start_bkmkstart(&mut self, current_depth: usize) {
        self.parsing_bkmkstart = true;
        self.bkmkstart_group_depth = current_depth;
        self.bkmkstart_name.clear();
    }

    /// Reset bookmark start state after emitting the anchor.
    pub fn reset_bkmkstart(&mut self) {
        self.parsing_bkmkstart = false;
        self.bkmkstart_group_depth = 0;
        self.bkmkstart_name.clear();
    }

    /// Pop nested field state.
    #[cfg(test)]
    pub fn pop_nested_field(&mut self) {
        self.nested_fields.pop();
    }

    /// Reset field state after finalization.
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.parsing_field = false;
        self.field_group_depth = 0;
        self.parsing_fldinst = false;
        self.fldinst_group_depth = 0;
        self.parsing_fldrslt = false;
        self.fldrslt_group_depth = 0;
        self.field_instruction_text.clear();
        self.field_result_inlines.clear();
        self.field_style_snapshot = None;
        self.nested_fields.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_state_default() {
        let state = FieldState::new();
        assert!(!state.parsing_field);
        assert!(state.field_instruction_text.is_empty());
        assert!(state.field_result_inlines.is_empty());
        assert!(state.field_style_snapshot.is_none());
        assert!(state.nested_fields.is_empty());
    }

    #[test]
    fn test_field_state_start_field() {
        let mut state = FieldState::new();
        let style = StyleState::new();
        state.start_field(5, style);

        assert!(state.parsing_field);
        assert_eq!(state.field_group_depth, 5);
        assert!(state.field_style_snapshot.is_some());
    }

    #[test]
    fn test_field_state_start_fldinst() {
        let mut state = FieldState::new();
        state.start_field(5, StyleState::new());
        state.start_fldinst(6);

        assert!(state.parsing_fldinst);
        assert_eq!(state.fldinst_group_depth, 6);
    }

    #[test]
    fn test_field_state_start_fldrslt() {
        let mut state = FieldState::new();
        state.start_field(5, StyleState::new());
        state.start_fldrslt(6);

        assert!(state.parsing_fldrslt);
        assert_eq!(state.fldrslt_group_depth, 6);
    }

    #[test]
    fn test_field_state_reset() {
        let mut state = FieldState::new();
        state.start_field(5, StyleState::new());
        state.start_fldinst(6);
        state.field_instruction_text.push_str("HYPERLINK");
        state
            .field_result_inlines
            .push(crate::Inline::Run(crate::Run::new("Link")));

        state.reset();

        assert!(!state.parsing_field);
        assert!(!state.parsing_fldinst);
        assert!(!state.parsing_fldrslt);
        assert!(state.field_instruction_text.is_empty());
        assert!(state.field_result_inlines.is_empty());
        assert!(state.field_style_snapshot.is_none());
    }

    #[test]
    fn test_field_state_nested_fields() {
        let mut state = FieldState::new();
        state.start_field(5, StyleState::new());

        // Start nested field (simulates encountering another \field while in a field)
        state.start_field(6, StyleState::new());

        // Should have pushed nested state
        assert_eq!(state.nested_fields.len(), 1);
        assert_eq!(state.nested_fields[0].field_group_depth, 6);

        // Pop nested field
        state.pop_nested_field();
        assert!(state.nested_fields.is_empty());
    }
}
