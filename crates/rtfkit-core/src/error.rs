//! Error Types Module
//!
//! This module provides error types for RTF parsing and conversion operations.
//! Errors are categorized into parse errors (syntax/structure issues) and
//! limit errors (resource protection).

use thiserror::Error;

// =============================================================================
// Parse Error
// =============================================================================

/// Errors that occur during RTF parsing.
///
/// These errors indicate problems with the RTF input itself,
/// such as invalid syntax, malformed structure, or unsupported features.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// Input size exceeds the configured limit.
    #[error("Input too large: {size} bytes exceeds limit of {limit} bytes")]
    InputTooLarge {
        /// Actual size of the input in bytes
        size: usize,
        /// Configured maximum size in bytes
        limit: usize,
    },

    /// Group nesting depth exceeds the configured limit.
    #[error("Group depth exceeded: {depth} levels exceeds limit of {limit}")]
    GroupDepthExceeded {
        /// Current nesting depth
        depth: usize,
        /// Configured maximum depth
        limit: usize,
    },

    /// Invalid RTF structure.
    #[error("Invalid RTF: {0}")]
    InvalidStructure(String),

    /// Tokenization error.
    #[error("Tokenization error: {0}")]
    TokenizationError(String),

    /// Missing RTF header.
    #[error("Invalid RTF: missing \\rtf header")]
    MissingRtfHeader,

    /// Unbalanced groups.
    #[error("Invalid RTF: unbalanced groups")]
    UnbalancedGroups,

    /// Unmatched group end.
    #[error("Invalid RTF: unmatched group end ('}}')")]
    UnmatchedGroupEnd,

    /// Empty input.
    #[error("Invalid RTF: empty input")]
    EmptyInput,
}

// =============================================================================
// Report Error
// =============================================================================

/// Errors that occur during report generation.
///
/// These errors indicate problems during the reporting phase,
/// typically related to resource limits.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReportError {
    /// Warning count exceeds the configured limit.
    #[error("Warning count exceeded: {count} warnings exceeds limit of {limit}")]
    WarningCountExceeded {
        /// Current warning count
        count: usize,
        /// Configured maximum count
        limit: usize,
    },
}

// =============================================================================
// Conversion Error
// =============================================================================

/// Unified error type for RTF conversion operations.
///
/// This enum wraps all possible errors that can occur during
/// RTF parsing and conversion.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConversionError {
    /// A parse error occurred.
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    /// A report error occurred.
    #[error("Report error: {0}")]
    Report(#[from] ReportError),

    /// A generic error with a message.
    #[error("{0}")]
    Other(String),
}

impl From<String> for ConversionError {
    fn from(msg: String) -> Self {
        ConversionError::Other(msg)
    }
}

impl From<&str> for ConversionError {
    fn from(msg: &str) -> Self {
        ConversionError::Other(msg.to_string())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_input_too_large() {
        let err = ParseError::InputTooLarge {
            size: 20_000_000,
            limit: 10_000_000,
        };
        let msg = err.to_string();
        assert!(msg.contains("20"));
        assert!(msg.contains("10"));
        assert!(msg.contains("bytes"));
    }

    #[test]
    fn test_parse_error_group_depth_exceeded() {
        let err = ParseError::GroupDepthExceeded {
            depth: 300,
            limit: 256,
        };
        let msg = err.to_string();
        assert!(msg.contains("300"));
        assert!(msg.contains("256"));
    }

    #[test]
    fn test_parse_error_invalid_structure() {
        let err = ParseError::InvalidStructure("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_report_error_warning_count_exceeded() {
        let err = ReportError::WarningCountExceeded {
            count: 1500,
            limit: 1000,
        };
        let msg = err.to_string();
        assert!(msg.contains("1500"));
        assert!(msg.contains("1000"));
    }

    #[test]
    fn test_conversion_error_from_parse_error() {
        let parse_err = ParseError::MissingRtfHeader;
        let conv_err: ConversionError = parse_err.into();
        assert!(matches!(conv_err, ConversionError::Parse(ParseError::MissingRtfHeader)));
    }

    #[test]
    fn test_conversion_error_from_string() {
        let err: ConversionError = "test error".into();
        assert!(matches!(err, ConversionError::Other(s) if s == "test error"));
    }
}
