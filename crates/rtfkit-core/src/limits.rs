//! Parser Limits Module
//!
//! This module provides safeguards against pathological inputs that could cause
//! resource exhaustion or denial of service. Limits are enforced during parsing
//! and conversion to fail fast with explicit errors.
//!
//! # Conservative Defaults
//!
//! The default limits are chosen to be conservative while still allowing
//! reasonable documents:
//!
//! - `max_input_bytes`: 10 MB - reasonable for text documents
//! - `max_group_depth`: 256 - RTF spec allows deep nesting
//! - `max_warning_count`: 1000 - prevents runaway reports
//!
//! # Example
//!
//! ```ignore
//! use rtfkit_core::limits::ParserLimits;
//!
//! // Use default limits
//! let limits = ParserLimits::default();
//!
//! // Or customize
//! let limits = ParserLimits {
//!     max_input_bytes: 5 * 1024 * 1024,  // 5 MB
//!     max_group_depth: 128,
//!     max_warning_count: 500,
//! };
//! ```

use serde::{Deserialize, Serialize};

// =============================================================================
// Parser Limits
// =============================================================================

/// Configuration for parser/runtime safeguards.
///
/// These limits protect against pathological inputs that could cause
/// resource exhaustion or denial of service. When a limit is exceeded,
/// parsing fails immediately with an explicit error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParserLimits {
    /// Maximum input size in bytes.
    ///
    /// Inputs larger than this limit are rejected immediately.
    /// Default: 10 MB (10,485,760 bytes)
    pub max_input_bytes: usize,

    /// Maximum group nesting depth.
    ///
    /// Prevents stack overflow from deeply nested RTF groups.
    /// Default: 256 levels
    pub max_group_depth: usize,

    /// Maximum number of warnings to collect.
    ///
    /// Prevents unbounded memory growth from pathological documents
    /// that generate many warnings. When this limit is reached,
    /// parsing continues but no more warnings are collected.
    /// Default: 1000 warnings
    pub max_warning_count: usize,

    /// Maximum number of rows per table.
    ///
    /// Prevents resource exhaustion from extremely large tables.
    /// Default: 10000 rows
    pub max_rows_per_table: usize,

    /// Maximum number of cells per row.
    ///
    /// Prevents resource exhaustion from extremely wide tables.
    /// Default: 1000 cells
    pub max_cells_per_row: usize,

    /// Maximum merge span.
    ///
    /// Limits the number of cells that can be merged horizontally.
    /// Default: 1000 cells
    pub max_merge_span: u16,
}

impl Default for ParserLimits {
    /// Returns conservative default limits.
    ///
    /// # Defaults
    ///
    /// - `max_input_bytes`: 10 MB (10,485,760 bytes)
    /// - `max_group_depth`: 256 levels
    /// - `max_warning_count`: 1000 warnings
    /// - `max_rows_per_table`: 10000 rows
    /// - `max_cells_per_row`: 1000 cells
    /// - `max_merge_span`: 1000 cells
    fn default() -> Self {
        Self {
            max_input_bytes: 10 * 1024 * 1024, // 10 MB
            max_group_depth: 256,
            max_warning_count: 1000,
            max_rows_per_table: 10000,
            max_cells_per_row: 1000,
            max_merge_span: 1000,
        }
    }
}

impl ParserLimits {
    /// Creates a new `ParserLimits` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates limits with no restrictions.
    ///
    /// **Warning**: Using these limits may expose the parser to
    /// denial-of-service attacks from pathological inputs.
    pub fn none() -> Self {
        Self {
            max_input_bytes: usize::MAX,
            max_group_depth: usize::MAX,
            max_warning_count: usize::MAX,
            max_rows_per_table: usize::MAX,
            max_cells_per_row: usize::MAX,
            max_merge_span: u16::MAX,
        }
    }

    /// Sets the maximum input size in bytes.
    pub fn with_max_input_bytes(mut self, bytes: usize) -> Self {
        self.max_input_bytes = bytes;
        self
    }

    /// Sets the maximum group nesting depth.
    pub fn with_max_group_depth(mut self, depth: usize) -> Self {
        self.max_group_depth = depth;
        self
    }

    /// Sets the maximum warning count.
    pub fn with_max_warning_count(mut self, count: usize) -> Self {
        self.max_warning_count = count;
        self
    }

    /// Sets the maximum rows per table.
    pub fn with_max_rows_per_table(mut self, count: usize) -> Self {
        self.max_rows_per_table = count;
        self
    }

    /// Sets the maximum cells per row.
    pub fn with_max_cells_per_row(mut self, count: usize) -> Self {
        self.max_cells_per_row = count;
        self
    }

    /// Sets the maximum merge span.
    pub fn with_max_merge_span(mut self, span: u16) -> Self {
        self.max_merge_span = span;
        self
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = ParserLimits::default();
        assert_eq!(limits.max_input_bytes, 10 * 1024 * 1024);
        assert_eq!(limits.max_group_depth, 256);
        assert_eq!(limits.max_warning_count, 1000);
        assert_eq!(limits.max_rows_per_table, 10000);
        assert_eq!(limits.max_cells_per_row, 1000);
        assert_eq!(limits.max_merge_span, 1000);
    }

    #[test]
    fn test_new_is_default() {
        let limits = ParserLimits::new();
        assert_eq!(limits, ParserLimits::default());
    }

    #[test]
    fn test_none_limits() {
        let limits = ParserLimits::none();
        assert_eq!(limits.max_input_bytes, usize::MAX);
        assert_eq!(limits.max_group_depth, usize::MAX);
        assert_eq!(limits.max_warning_count, usize::MAX);
        assert_eq!(limits.max_rows_per_table, usize::MAX);
        assert_eq!(limits.max_cells_per_row, usize::MAX);
        assert_eq!(limits.max_merge_span, u16::MAX);
    }

    #[test]
    fn test_builder_pattern() {
        let limits = ParserLimits::new()
            .with_max_input_bytes(1024)
            .with_max_group_depth(50)
            .with_max_warning_count(100)
            .with_max_rows_per_table(500)
            .with_max_cells_per_row(50)
            .with_max_merge_span(100);

        assert_eq!(limits.max_input_bytes, 1024);
        assert_eq!(limits.max_group_depth, 50);
        assert_eq!(limits.max_warning_count, 100);
        assert_eq!(limits.max_rows_per_table, 500);
        assert_eq!(limits.max_cells_per_row, 50);
        assert_eq!(limits.max_merge_span, 100);
    }

    #[test]
    fn test_serialization() {
        let limits = ParserLimits::default();
        let json = serde_json::to_string(&limits).unwrap();
        assert!(json.contains("max_input_bytes"));
        assert!(json.contains("max_group_depth"));
        assert!(json.contains("max_warning_count"));
        assert!(json.contains("max_rows_per_table"));
        assert!(json.contains("max_cells_per_row"));
        assert!(json.contains("max_merge_span"));
    }
}
