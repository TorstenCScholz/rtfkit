//! Error types for HTML generation.
//!
//! This module provides the [`HtmlWriterError`] type which represents all possible
//! errors that can occur during HTML document generation.
//!
//! # CLI Exit Code Mapping
//!
//! The CLI maps this error to exit code `3` for writer-level failures.

use thiserror::Error;

/// Errors that can occur during HTML generation.
///
/// This error type is designed to be mappable to CLI exit code `3`
/// for writer-level failures.
#[derive(Debug, Error)]
pub enum HtmlWriterError {
    /// An error occurred during HTML generation.
    #[error("HTML generation error: {0}")]
    Generation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = HtmlWriterError::Generation("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HtmlWriterError>();
    }
}
