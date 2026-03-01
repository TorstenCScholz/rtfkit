//! Reporting Module
//!
//! This module provides types for collecting warnings and statistics during
//! RTF interpretation. The `Report` structure is returned alongside the parsed
//! `Document` to provide insight into the conversion process.
//!
//! # Example
//!
//! ```ignore
//! use rtfkit_core::report::{Report, Warning, Stats};
//!
//! let report = Report::new();
//! // During interpretation, warnings and stats are collected
//! ```

use crate::limits::ParserLimits;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// =============================================================================
// Warning Severity
// =============================================================================

/// The severity level of a warning.
///
/// Warnings are categorized by severity to help users understand
/// the impact of issues encountered during conversion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WarningSeverity {
    /// Informational message, no impact on output
    Info,
    /// Warning that may affect output quality
    #[default]
    Warning,
    /// Error that significantly affects output
    Error,
}

// =============================================================================
// Warning Types
// =============================================================================

/// A warning encountered during RTF interpretation.
///
/// Warnings represent issues that don't prevent parsing but may
/// affect the quality or completeness of the output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Warning {
    /// An unsupported control word was encountered.
    ///
    /// This indicates a control word that is recognized but not
    /// yet implemented in the interpreter.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This is a **cosmetic loss** warning and does NOT cause strict mode to fail.
    /// It indicates that a formatting control word was not applied, but the text
    /// content is still preserved in the output.
    UnsupportedControlWord {
        /// The control word that was encountered (without the leading backslash)
        word: String,
        /// Optional parameter that was provided with the control word
        parameter: Option<i32>,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// An unknown destination was encountered.
    ///
    /// Destinations are special RTF groups that contain content
    /// not part of the main document flow (e.g., headers, footers).
    ///
    /// # Strict-Mode Behavior
    ///
    /// This is an **informational** warning and does NOT cause strict mode to fail
    /// on its own. However, unknown destinations typically result in their content
    /// being dropped, which will emit a separate `DroppedContent` warning that
    /// WILL trigger strict mode failure.
    UnknownDestination {
        /// The name of the destination
        destination: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// Content was dropped during conversion.
    ///
    /// This indicates that some content could not be represented
    /// in the output format and was discarded.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This warning **always** causes strict mode to fail (exit code 4).
    /// It represents semantic loss where content or structure could not
    /// be faithfully represented in the output.
    ///
    /// # Warning Cap Behavior
    ///
    /// When the warning count limit is reached, `DroppedContent` warnings
    /// are specially preserved to ensure the strict-mode signal is not lost.
    /// If a `DroppedContent` warning arrives after the cap, it will replace
    /// the last non-`DroppedContent` warning to maintain the strict-mode signal.
    ///
    /// # Stable Reason Strings
    ///
    /// The following reason strings are part of the stable API contract:
    /// - `"merge_semantics"` - Merge semantics were lost or degraded
    /// - `"Dropped unsupported RTF destination content"` - Unknown destination
    /// - `"Dropped unsupported binary RTF data"` - Binary data
    /// - `"Dropped legacy paragraph numbering content"` - Legacy \\pn controls
    /// - `"Unresolved list override ls_id=N"` - List reference could not be resolved
    DroppedContent {
        /// Description of what was dropped
        reason: String,
        /// Approximate size of dropped content (if known)
        size_hint: Option<usize>,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A list-related control word was encountered but not fully supported.
    ///
    /// This indicates list functionality that is recognized but partially implemented.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This is a **cosmetic loss** warning and does NOT cause strict mode to fail
    /// on its own. However, if the unsupported control leads to content being dropped,
    /// a separate `DroppedContent` warning will be emitted which will trigger strict mode.
    UnsupportedListControl {
        /// The control word that was encountered
        control_word: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A list override could not be resolved.
    ///
    /// This indicates a reference to a list definition that doesn't exist or is malformed.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This warning indicates semantic loss. The interpreter always emits
    /// `DroppedContent("Unresolved list override ls_id=N")` alongside this warning,
    /// which will cause strict mode to fail (exit code 4).
    UnresolvedListOverride {
        /// The \ls index that couldn't be resolved
        ls_id: i32,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// List nesting level exceeds supported range.
    ///
    /// DOCX supports levels 0-8; levels beyond this are clamped.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This is a **cosmetic loss** warning and does NOT cause strict mode to fail.
    /// The level is clamped to the maximum supported value (8), and the list content
    /// is still rendered at the clamped level.
    UnsupportedNestingLevel {
        /// The level that was encountered
        level: u8,
        /// The maximum supported level
        max: u8,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A table-related control word was encountered but not fully supported.
    ///
    /// This indicates table functionality that is recognized but partially implemented
    /// or intentionally deferred to a later phase.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This is a **cosmetic loss** warning and does NOT cause strict mode to fail.
    /// It indicates that a table control word was not mapped to output, but the
    /// table structure and content are still preserved.
    UnsupportedTableControl {
        /// The control word that was encountered (without leading backslash)
        control_word: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// The table structure is malformed or incomplete.
    ///
    /// This indicates structural issues like mismatched cell counts,
    /// missing terminators, or invalid nesting.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This warning indicates structural issues with the table. When accompanied
    /// by `DroppedContent`, it will cause strict mode to fail (exit code 4).
    /// The interpreter emits `DroppedContent` for cases where content or structure
    /// is lost (e.g., cell count mismatch, orphan controls).
    MalformedTableStructure {
        /// Human-readable description of the issue
        reason: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A table cell was not properly closed before row/document end.
    ///
    /// This indicates a missing `\cell` control word. The interpreter
    /// auto-closes the cell to preserve content.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This warning indicates a structural issue. When accompanied by `DroppedContent`,
    /// it will cause strict mode to fail (exit code 4). The interpreter emits
    /// `DroppedContent` for unclosed cells to signal potential content reordering.
    UnclosedTableCell {
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A table row was not properly closed before next row/document end.
    ///
    /// This indicates a missing `\row` control word. The interpreter
    /// auto-closes the row to preserve content.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This warning indicates a structural issue. When accompanied by `DroppedContent`,
    /// it will cause strict mode to fail (exit code 4). The interpreter emits
    /// `DroppedContent` for unclosed rows to signal potential content reordering.
    UnclosedTableRow {
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// Merge semantics conflict or invalid merge structure.
    ///
    /// This indicates issues like orphan merge continuations,
    /// conflicting merge directions, or invalid merge chains.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This warning indicates potential semantic loss and will cause
    /// strict mode to fail (exit code 4) when accompanied by `DroppedContent`.
    /// The interpreter always emits `DroppedContent("merge_semantics")` alongside
    /// this warning to ensure strict-mode compliance.
    ///
    /// # Stable Reason Strings
    ///
    /// The following reason strings are part of the stable API contract:
    /// - `"Orphan merge continuation without start - treating as standalone cell"`
    MergeConflict {
        /// Human-readable description of the issue
        reason: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// Table geometry conflict (e.g., non-monotonic cellx, impossible spans).
    ///
    /// This indicates structural issues with table geometry that
    /// required adjustment or clamping.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This warning indicates structural changes to the table and will cause
    /// strict mode to fail (exit code 4) when accompanied by `DroppedContent`.
    /// The interpreter always emits `DroppedContent("merge_semantics")` alongside
    /// this warning for span-related conflicts.
    ///
    /// # Stable Reason Strings
    ///
    /// The following reason strings are part of the stable API contract:
    /// - `"Merge span N exceeds available cells M - clamped"`
    TableGeometryConflict {
        /// Human-readable description of the issue
        reason: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// An unrecognized field type was encountered, but the result text was preserved.
    ///
    /// This indicates a field instruction (e.g. `DATE`, `SEQ`, `TOC`) that is not
    /// yet supported. When the field's `\fldrslt` text is present, it is emitted
    /// as plain text so visible content is not lost.
    ///
    /// # Strict-Mode Behavior
    ///
    /// This is a **cosmetic degradation** warning and does NOT cause strict mode to
    /// fail. The visible result text is preserved; only the field semantics are lost.
    UnsupportedField {
        /// Description of the issue
        reason: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A page-management field instruction was only partially supported.
    ///
    /// Non-strict mode keeps rendering with best effort output.
    UnsupportedPageField {
        /// Description of the issue
        reason: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A TOC field switch was parsed but not supported in v1 mapping.
    ///
    /// This is a partial-support warning and does not imply dropped content.
    UnsupportedTocSwitch {
        /// The unsupported TOC switch token (without leading backslash).
        switch: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// A page reference target could not be resolved.
    ///
    /// The renderer should preserve visible fallback text where possible.
    UnresolvedPageReference {
        /// Bookmark target that could not be resolved.
        target: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },

    /// Section numbering required fallback behavior.
    ///
    /// This indicates that section numbering semantics were approximated.
    SectionNumberingFallback {
        /// Description of the fallback.
        reason: String,
        /// Severity of this warning
        severity: WarningSeverity,
    },
}

impl Warning {
    /// Creates a new `UnsupportedControlWord` warning.
    pub fn unsupported_control_word(word: impl Into<String>, parameter: Option<i32>) -> Self {
        Warning::UnsupportedControlWord {
            word: word.into(),
            parameter,
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnknownDestination` warning.
    pub fn unknown_destination(destination: impl Into<String>) -> Self {
        Warning::UnknownDestination {
            destination: destination.into(),
            severity: WarningSeverity::Info,
        }
    }

    /// Creates a new `DroppedContent` warning.
    pub fn dropped_content(reason: impl Into<String>, size_hint: Option<usize>) -> Self {
        Warning::DroppedContent {
            reason: reason.into(),
            size_hint,
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnsupportedListControl` warning.
    pub fn unsupported_list_control(control_word: impl Into<String>) -> Self {
        Warning::UnsupportedListControl {
            control_word: control_word.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnresolvedListOverride` warning.
    pub fn unresolved_list_override(ls_id: i32) -> Self {
        Warning::UnresolvedListOverride {
            ls_id,
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnsupportedNestingLevel` warning.
    pub fn unsupported_nesting_level(level: u8, max: u8) -> Self {
        Warning::UnsupportedNestingLevel {
            level,
            max,
            severity: WarningSeverity::Info,
        }
    }

    /// Creates a new `UnsupportedTableControl` warning.
    pub fn unsupported_table_control(control_word: impl Into<String>) -> Self {
        Warning::UnsupportedTableControl {
            control_word: control_word.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `MalformedTableStructure` warning.
    pub fn malformed_table_structure(reason: impl Into<String>) -> Self {
        Warning::MalformedTableStructure {
            reason: reason.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnclosedTableCell` warning.
    pub fn unclosed_table_cell() -> Self {
        Warning::UnclosedTableCell {
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnclosedTableRow` warning.
    pub fn unclosed_table_row() -> Self {
        Warning::UnclosedTableRow {
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `MergeConflict` warning.
    pub fn merge_conflict(reason: impl Into<String>) -> Self {
        Warning::MergeConflict {
            reason: reason.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `TableGeometryConflict` warning.
    pub fn table_geometry_conflict(reason: impl Into<String>) -> Self {
        Warning::TableGeometryConflict {
            reason: reason.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnsupportedField` warning.
    pub fn unsupported_field(reason: impl Into<String>) -> Self {
        Warning::UnsupportedField {
            reason: reason.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnsupportedPageField` warning.
    pub fn unsupported_page_field(reason: impl Into<String>) -> Self {
        Warning::UnsupportedPageField {
            reason: reason.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnsupportedTocSwitch` warning.
    pub fn unsupported_toc_switch(switch: impl Into<String>) -> Self {
        Warning::UnsupportedTocSwitch {
            switch: switch.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `UnresolvedPageReference` warning.
    pub fn unresolved_page_reference(target: impl Into<String>) -> Self {
        Warning::UnresolvedPageReference {
            target: target.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Creates a new `SectionNumberingFallback` warning.
    pub fn section_numbering_fallback(reason: impl Into<String>) -> Self {
        Warning::SectionNumberingFallback {
            reason: reason.into(),
            severity: WarningSeverity::Warning,
        }
    }

    /// Returns the severity of this warning.
    pub fn severity(&self) -> WarningSeverity {
        match self {
            Warning::UnsupportedControlWord { severity, .. } => *severity,
            Warning::UnknownDestination { severity, .. } => *severity,
            Warning::DroppedContent { severity, .. } => *severity,
            Warning::UnsupportedListControl { severity, .. } => *severity,
            Warning::UnresolvedListOverride { severity, .. } => *severity,
            Warning::UnsupportedNestingLevel { severity, .. } => *severity,
            Warning::UnsupportedTableControl { severity, .. } => *severity,
            Warning::MalformedTableStructure { severity, .. } => *severity,
            Warning::UnclosedTableCell { severity } => *severity,
            Warning::UnclosedTableRow { severity } => *severity,
            Warning::MergeConflict { severity, .. } => *severity,
            Warning::TableGeometryConflict { severity, .. } => *severity,
            Warning::UnsupportedField { severity, .. } => *severity,
            Warning::UnsupportedPageField { severity, .. } => *severity,
            Warning::UnsupportedTocSwitch { severity, .. } => *severity,
            Warning::UnresolvedPageReference { severity, .. } => *severity,
            Warning::SectionNumberingFallback { severity, .. } => *severity,
        }
    }
}

// =============================================================================
// Statistics
// =============================================================================

/// Statistics collected during RTF interpretation.
///
/// These metrics provide insight into the conversion process
/// and can be used for performance monitoring and debugging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    /// Number of paragraphs processed.
    pub paragraph_count: usize,

    /// Number of text runs processed.
    pub run_count: usize,

    /// Total bytes read from input.
    pub bytes_processed: usize,

    /// Processing duration in milliseconds.
    pub duration_ms: u64,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    /// Creates a new `Stats` instance with zero values.
    pub fn new() -> Self {
        Self {
            paragraph_count: 0,
            run_count: 0,
            bytes_processed: 0,
            duration_ms: 0,
        }
    }
}

// =============================================================================
// Report
// =============================================================================

/// A report containing warnings and statistics from RTF interpretation.
///
/// The `Report` is returned alongside the parsed `Document` to provide
/// information about the conversion process.
///
/// # Example
///
/// ```ignore
/// use rtfkit_core::report::Report;
///
/// let report = Report::new();
/// // During interpretation, warnings and stats are collected
/// assert!(report.warnings.is_empty());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Report {
    /// Warnings encountered during interpretation.
    pub warnings: Vec<Warning>,

    /// Statistics collected during interpretation.
    pub stats: Stats,
}

impl Report {
    /// Creates a new empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a warning to the report.
    pub fn add_warning(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// Returns the number of warnings.
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Returns warnings of a specific severity.
    pub fn warnings_by_severity(&self, severity: WarningSeverity) -> Vec<&Warning> {
        self.warnings
            .iter()
            .filter(|w| w.severity() == severity)
            .collect()
    }

    /// Returns true if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| w.severity() == WarningSeverity::Error)
    }
}

// =============================================================================
// Report Builder (for internal use during interpretation)
// =============================================================================

/// Internal helper for building a report during interpretation.
///
/// This struct tracks the start time and provides methods for
/// incrementally building the report as interpretation progresses.
pub struct ReportBuilder {
    warnings: Vec<Warning>,
    paragraph_count: usize,
    run_count: usize,
    bytes_processed: usize,
    start_time: Instant,
    /// Parser limits for warning count enforcement
    limits: Option<ParserLimits>,
    /// Whether warning limit has been reached (to avoid repeated checks)
    warning_limit_reached: bool,
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportBuilder {
    /// Creates a new report builder, starting the timer.
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
            paragraph_count: 0,
            run_count: 0,
            bytes_processed: 0,
            start_time: Instant::now(),
            limits: None,
            warning_limit_reached: false,
        }
    }

    /// Sets the parser limits for this report builder.
    pub fn set_limits(&mut self, limits: ParserLimits) {
        self.limits = Some(limits);
    }

    /// Records an unsupported control word.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unsupported_control_word(&mut self, word: &str, parameter: Option<i32>) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::unsupported_control_word(word, parameter));
        }
    }

    /// Records an unknown destination.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unknown_destination(&mut self, destination: &str) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::unknown_destination(destination));
        }
    }

    /// Records dropped content.
    ///
    /// If the warning limit is reached, this preserves strict-mode behavior by
    /// ensuring at least one `DroppedContent` warning remains in the report.
    pub fn dropped_content(&mut self, reason: &str, size_hint: Option<usize>) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::dropped_content(reason, size_hint));
            return;
        }

        // Preserve strict-mode signal even when warning collection is capped.
        // If we have no dropped-content warning yet, replace the last warning.
        if !self
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::DroppedContent { .. }))
            && let Some(last) = self.warnings.last_mut()
        {
            *last = Warning::dropped_content(reason, size_hint);
        }
    }

    /// Records an unsupported list control word.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unsupported_list_control(&mut self, control_word: &str) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::unsupported_list_control(control_word));
        }
    }

    /// Records an unresolved list override.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unresolved_list_override(&mut self, ls_id: i32) {
        if self.can_add_warning() {
            self.warnings.push(Warning::unresolved_list_override(ls_id));
        }
    }

    /// Records an unsupported nesting level.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unsupported_nesting_level(&mut self, level: u8, max: u8) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::unsupported_nesting_level(level, max));
        }
    }

    /// Records an unsupported table control word.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unsupported_table_control(&mut self, control_word: &str) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::unsupported_table_control(control_word));
        }
    }

    /// Records a malformed table structure issue.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn malformed_table_structure(&mut self, reason: &str) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::malformed_table_structure(reason));
        }
    }

    /// Records an unclosed table cell warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unclosed_table_cell(&mut self) {
        if self.can_add_warning() {
            self.warnings.push(Warning::unclosed_table_cell());
        }
    }

    /// Records an unclosed table row warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unclosed_table_row(&mut self) {
        if self.can_add_warning() {
            self.warnings.push(Warning::unclosed_table_row());
        }
    }

    /// Records a merge conflict warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn merge_conflict(&mut self, reason: &str) {
        if self.can_add_warning() {
            self.warnings.push(Warning::merge_conflict(reason));
        }
    }

    /// Records a table geometry conflict warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn table_geometry_conflict(&mut self, reason: &str) {
        if self.can_add_warning() {
            self.warnings.push(Warning::table_geometry_conflict(reason));
        }
    }

    /// Records an unsupported field warning (non-strict: result text was preserved).
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unsupported_field(&mut self, reason: &str) {
        if self.can_add_warning() {
            self.warnings.push(Warning::unsupported_field(reason));
        }
    }

    /// Records a page-field partial-support warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unsupported_page_field(&mut self, reason: &str) {
        if self.can_add_warning() {
            self.warnings.push(Warning::unsupported_page_field(reason));
        }
    }

    /// Records an unsupported TOC switch warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unsupported_toc_switch(&mut self, switch: &str) {
        if self.can_add_warning() {
            self.warnings.push(Warning::unsupported_toc_switch(switch));
        }
    }

    /// Records an unresolved page-reference warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn unresolved_page_reference(&mut self, target: &str) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::unresolved_page_reference(target));
        }
    }

    /// Records a section numbering fallback warning.
    ///
    /// If the warning count limit has been reached, this is a no-op.
    pub fn section_numbering_fallback(&mut self, reason: &str) {
        if self.can_add_warning() {
            self.warnings
                .push(Warning::section_numbering_fallback(reason));
        }
    }

    /// Check if we can add another warning (respects warning count limit).
    fn can_add_warning(&mut self) -> bool {
        if self.warning_limit_reached {
            return false;
        }

        if let Some(ref limits) = self.limits
            && self.warnings.len() >= limits.max_warning_count
        {
            self.warning_limit_reached = true;
            return false;
        }
        true
    }

    /// Increments the paragraph count.
    pub fn increment_paragraph_count(&mut self) {
        self.paragraph_count += 1;
    }

    /// Adds to the run count.
    pub fn add_runs(&mut self, count: usize) {
        self.run_count += count;
    }

    /// Sets the bytes processed.
    pub fn set_bytes_processed(&mut self, bytes: usize) {
        self.bytes_processed = bytes;
    }

    /// Builds the final report.
    pub fn build(self) -> Report {
        // Keep report JSON deterministic across repeated runs.
        // Runtime wall-clock duration is intentionally excluded from stable output.
        let _ = self.start_time;
        let duration_ms = 0;
        Report {
            warnings: self.warnings,
            stats: Stats {
                paragraph_count: self.paragraph_count,
                run_count: self.run_count,
                bytes_processed: self.bytes_processed,
                duration_ms,
            },
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
    fn test_warning_severity_default() {
        let severity = WarningSeverity::default();
        assert_eq!(severity, WarningSeverity::Warning);
    }

    #[test]
    fn test_warning_unsupported_control_word() {
        let warning = Warning::unsupported_control_word("fonttbl", None);
        assert!(matches!(
            warning,
            Warning::UnsupportedControlWord {
                word,
                parameter: None,
                severity: WarningSeverity::Warning
            } if word == "fonttbl"
        ));
    }

    #[test]
    fn test_warning_unknown_destination() {
        let warning = Warning::unknown_destination("header");
        assert!(matches!(
            warning,
            Warning::UnknownDestination {
                destination,
                severity: WarningSeverity::Info
            } if destination == "header"
        ));
    }

    #[test]
    fn test_warning_dropped_content() {
        let warning = Warning::dropped_content("binary data", Some(100));
        assert!(matches!(
            warning,
            Warning::DroppedContent {
                reason,
                size_hint: Some(100),
                severity: WarningSeverity::Warning
            } if reason == "binary data"
        ));
    }

    #[test]
    fn test_stats_new() {
        let stats = Stats::new();
        assert_eq!(stats.paragraph_count, 0);
        assert_eq!(stats.run_count, 0);
        assert_eq!(stats.bytes_processed, 0);
        assert_eq!(stats.duration_ms, 0);
    }

    #[test]
    fn test_report_new() {
        let report = Report::new();
        assert!(report.warnings.is_empty());
        assert_eq!(report.stats.paragraph_count, 0);
    }

    #[test]
    fn test_report_add_warning() {
        let mut report = Report::new();
        report.add_warning(Warning::unsupported_control_word("test", None));
        assert_eq!(report.warning_count(), 1);
    }

    #[test]
    fn test_report_warnings_by_severity() {
        let mut report = Report::new();
        report.add_warning(Warning::unsupported_control_word("test1", None));
        report.add_warning(Warning::unknown_destination("test2"));

        let warnings = report.warnings_by_severity(WarningSeverity::Warning);
        assert_eq!(warnings.len(), 1);

        let info_warnings = report.warnings_by_severity(WarningSeverity::Info);
        assert_eq!(info_warnings.len(), 1);
    }

    #[test]
    fn test_report_has_errors() {
        let mut report = Report::new();
        assert!(!report.has_errors());

        report.add_warning(Warning::unsupported_control_word("test", None));
        assert!(!report.has_errors());

        // Add an error-level warning
        report.warnings.push(Warning::DroppedContent {
            reason: "critical data".to_string(),
            size_hint: None,
            severity: WarningSeverity::Error,
        });
        assert!(report.has_errors());
    }

    #[test]
    fn test_report_builder() {
        let mut builder = ReportBuilder::new();
        builder.unsupported_control_word("fonttbl", None);
        builder.unknown_destination("header");
        builder.increment_paragraph_count();
        builder.increment_paragraph_count();
        builder.add_runs(5);
        builder.set_bytes_processed(1000);

        let report = builder.build();

        assert_eq!(report.warnings.len(), 2);
        assert_eq!(report.stats.paragraph_count, 2);
        assert_eq!(report.stats.run_count, 5);
        assert_eq!(report.stats.bytes_processed, 1000);
        // duration_ms is non-zero but we can't predict exact value
    }

    #[test]
    fn test_warning_cap_preserves_dropped_content_signal() {
        let mut builder = ReportBuilder::new();
        builder.set_limits(ParserLimits::new().with_max_warning_count(2));

        // Fill the warning budget with non-dropped warnings.
        builder.unsupported_control_word("foo", None);
        builder.unsupported_control_word("bar", None);

        // This arrives after the cap; strict mode still needs to see it.
        builder.dropped_content("Dropped unsupported destination", None);

        let report = builder.build();
        assert_eq!(report.warnings.len(), 2);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_warning_serialization() {
        let warning = Warning::unsupported_control_word("fonttbl", Some(42));
        let json = serde_json::to_string(&warning).unwrap();

        assert!(json.contains("unsupported_control_word"));
        assert!(json.contains("fonttbl"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_stats_serialization() {
        let stats = Stats {
            paragraph_count: 10,
            run_count: 25,
            bytes_processed: 1000,
            duration_ms: 50,
        };
        let json = serde_json::to_string(&stats).unwrap();

        assert!(json.contains("paragraph_count"));
        assert!(json.contains("run_count"));
        assert!(json.contains("bytes_processed"));
        assert!(json.contains("duration_ms"));
    }

    #[test]
    fn test_report_serialization() {
        let mut report = Report::new();
        report.add_warning(Warning::unsupported_control_word("test", None));
        report.stats.paragraph_count = 5;

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("warnings"));
        assert!(json.contains("stats"));
    }
}
