//! Error types for Typst-based PDF rendering.
//!
//! This module provides the [`RenderError`] type which represents all possible
//! errors that can occur during PDF document rendering using the Typst engine.
//!
//! # CLI Exit Code Mapping
//!
//! The CLI maps this error to exit code `3` for writer-level failures.

use thiserror::Error;

/// Errors that can occur during PDF rendering.
///
/// This error type is designed to be mappable to CLI exit code `3`
/// for writer-level failures.
#[derive(Debug, Error)]
pub enum RenderError {
    /// Invalid renderer option value.
    #[error("Invalid render option: {0}")]
    InvalidOption(String),

    /// An error occurred during IR to Typst mapping.
    #[error("IR to Typst mapping error: {0}")]
    Mapping(String),

    /// An error occurred during Typst rendering.
    #[error("Typst rendering error: {0}")]
    Rendering(String),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl RenderError {
    /// Returns the CLI exit code for this error.
    ///
    /// All render errors map to exit code 3 (writer-level failure).
    pub fn exit_code(&self) -> i32 {
        3
    }
}

/// A warning generated during rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// The warning message.
    pub message: String,
    /// The kind of warning.
    pub kind: WarningKind,
}

/// Kinds of warnings that can occur during rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningKind {
    /// Content was dropped during conversion.
    DroppedContent,
    /// A feature is not fully supported.
    PartialSupport,
    /// A fallback was used for unsupported content.
    Fallback,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = RenderError::Mapping("unsupported element".to_string());
        assert!(err.to_string().contains("mapping"));
        assert!(err.to_string().contains("unsupported element"));
    }

    #[test]
    fn rendering_error_display() {
        let err = RenderError::Rendering("failed to compile".to_string());
        assert!(err.to_string().contains("rendering"));
        assert!(err.to_string().contains("failed to compile"));
    }

    #[test]
    fn error_exit_code() {
        let err = RenderError::InvalidOption("bad option".to_string());
        assert_eq!(err.exit_code(), 3);

        let err = RenderError::Mapping("test".to_string());
        assert_eq!(err.exit_code(), 3);

        let err = RenderError::Rendering("test".to_string());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RenderError>();
    }

    #[test]
    fn warning_creation() {
        let warning = Warning {
            message: "Table border style simplified".to_string(),
            kind: WarningKind::PartialSupport,
        };
        assert_eq!(warning.message, "Table border style simplified");
        assert_eq!(warning.kind, WarningKind::PartialSupport);
    }
}
