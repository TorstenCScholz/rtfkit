//! List State Module
//!
//! This module contains list parsing and resolution state for handling
//! RTF list tables and list override tables.

use crate::{ListId, ListKind};
use std::collections::HashMap;

/// Parsed list level from \listtable.
///
/// Represents a single level within a list definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedListLevel {
    /// The level number (0-8)
    pub level: u8,
    /// The kind of numbering for this level
    pub kind: ListKind,
}

/// Parsed list definition from \listtable.
///
/// Represents a complete list definition with all its levels.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedListDefinition {
    /// The list ID from RTF (\listidN)
    pub list_id: i32,
    /// The levels defined for this list
    pub levels: Vec<ParsedListLevel>,
}

impl ParsedListDefinition {
    /// Creates a new empty list definition with the given ID.
    pub fn new(list_id: i32) -> Self {
        Self {
            list_id,
            levels: Vec::new(),
        }
    }

    /// Gets the kind for a specific level, defaulting to Bullet.
    pub fn kind_for_level(&self, level: u8) -> ListKind {
        self.levels
            .iter()
            .find(|l| l.level == level)
            .map(|l| l.kind)
            .unwrap_or_default()
    }
}

/// Parsed list override from \listoverridetable.
///
/// List overrides map an ls_id to a list definition and can optionally
/// override the starting number.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedListOverride {
    /// The ls_id used in \lsN control words
    pub ls_id: i32,
    /// The list_id this override references
    pub list_id: i32,
    /// Optional start number override
    pub start_override: Option<i32>,
}

impl ParsedListOverride {
    /// Creates a new list override.
    pub fn new(ls_id: i32, list_id: i32) -> Self {
        Self {
            ls_id,
            list_id,
            start_override: None,
        }
    }
}

/// Resolved list reference for the current paragraph.
///
/// This is populated when \lsN and \ilvlN are encountered,
/// and used during paragraph finalization to create list blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphListRef {
    /// The resolved list ID
    pub list_id: ListId,
    /// The nesting level (0-8)
    pub level: u8,
    /// The kind of list
    pub kind: ListKind,
}

impl ParagraphListRef {
    /// Creates a new paragraph list reference.
    pub fn new(list_id: ListId, level: u8, kind: ListKind) -> Self {
        Self {
            list_id,
            level: level.min(8), // Clamp to DOCX max
            kind,
        }
    }
}

/// List parsing state.
///
/// Tracks list definitions, overrides, and current paragraph list state.
#[derive(Debug, Clone, Default)]
pub struct ListState {
    // =============================================================================
    // Resolved List Data
    // =============================================================================
    /// Parsed list definitions from \listtable
    pub list_table: HashMap<i32, ParsedListDefinition>,
    /// Parsed list overrides from \listoverridetable
    pub list_overrides: HashMap<i32, ParsedListOverride>,

    // =============================================================================
    // Current Paragraph List State
    // =============================================================================
    /// Current paragraph list reference (from \lsN and \ilvlN)
    pub current_list_ref: Option<ParagraphListRef>,
    /// Current ls_id (from \lsN, used to resolve list reference)
    pub pending_ls_id: Option<i32>,
    /// Current level (from \ilvlN)
    pub pending_level: u8,

    // =============================================================================
    // List Table Parsing State
    // =============================================================================
    /// Whether we're currently parsing a list table destination
    pub parsing_list_table: bool,
    /// Whether we're currently parsing a list override table destination
    pub parsing_list_override_table: bool,
    /// Current list definition being parsed (for listtable)
    pub current_list_def: Option<ParsedListDefinition>,
    /// Current list level being parsed (for listtable)
    pub current_list_level: Option<ParsedListLevel>,
    /// Current list override being parsed (for listoverridetable)
    pub current_list_override: Option<ParsedListOverride>,
}

impl ListState {
    /// Creates a new default list state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset paragraph-level list state (called on \pard).
    pub fn reset_paragraph_state(&mut self) {
        self.current_list_ref = None;
        self.pending_ls_id = None;
        self.pending_level = 0;
    }

    /// Start parsing a list table.
    pub fn start_list_table(&mut self) {
        self.parsing_list_table = true;
    }

    /// Start parsing a list override table.
    pub fn start_list_override_table(&mut self) {
        self.parsing_list_override_table = true;
    }

    /// End parsing of list tables (called when exiting destination).
    pub fn end_list_parsing(&mut self) {
        self.parsing_list_table = false;
        self.parsing_list_override_table = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_list_definition_new() {
        let def = ParsedListDefinition::new(42);
        assert_eq!(def.list_id, 42);
        assert!(def.levels.is_empty());
    }

    #[test]
    fn test_parsed_list_definition_kind_for_level() {
        let mut def = ParsedListDefinition::new(1);
        def.levels.push(ParsedListLevel {
            level: 0,
            kind: ListKind::Bullet,
        });
        def.levels.push(ParsedListLevel {
            level: 1,
            kind: ListKind::OrderedDecimal,
        });

        assert_eq!(def.kind_for_level(0), ListKind::Bullet);
        assert_eq!(def.kind_for_level(1), ListKind::OrderedDecimal);
        // Default for undefined level
        assert_eq!(def.kind_for_level(2), ListKind::Bullet);
    }

    #[test]
    fn test_parsed_list_override_new() {
        let override_def = ParsedListOverride::new(1, 100);
        assert_eq!(override_def.ls_id, 1);
        assert_eq!(override_def.list_id, 100);
        assert!(override_def.start_override.is_none());
    }

    #[test]
    fn test_paragraph_list_ref_new() {
        let list_ref = ParagraphListRef::new(42, 3, ListKind::OrderedDecimal);
        assert_eq!(list_ref.list_id, 42);
        assert_eq!(list_ref.level, 3);
        assert_eq!(list_ref.kind, ListKind::OrderedDecimal);
    }

    #[test]
    fn test_paragraph_list_ref_level_clamped() {
        // Level should be clamped to 8
        let list_ref = ParagraphListRef::new(1, 15, ListKind::Bullet);
        assert_eq!(list_ref.level, 8);
    }

    #[test]
    fn test_list_state_default() {
        let state = ListState::new();
        assert!(state.list_table.is_empty());
        assert!(state.list_overrides.is_empty());
        assert!(state.current_list_ref.is_none());
        assert!(state.pending_ls_id.is_none());
        assert_eq!(state.pending_level, 0);
        assert!(!state.parsing_list_table);
        assert!(!state.parsing_list_override_table);
    }

    #[test]
    fn test_list_state_reset_paragraph() {
        let mut state = ListState::new();
        state.current_list_ref = Some(ParagraphListRef::new(1, 0, ListKind::Bullet));
        state.pending_ls_id = Some(5);
        state.pending_level = 2;

        state.reset_paragraph_state();

        assert!(state.current_list_ref.is_none());
        assert!(state.pending_ls_id.is_none());
        assert_eq!(state.pending_level, 0);
    }
}
