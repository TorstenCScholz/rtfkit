//! Error types for DOCX generation.
//!
//! This module provides the [`DocxError`] type which represents all possible
//! errors that can occur during DOCX document generation.

use std::io;
use thiserror::Error;

/// Errors that can occur during DOCX generation.
#[derive(Debug, Error)]
pub enum DocxError {
    /// An I/O error occurred while writing the document.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// An error occurred during ZIP compression.
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Failed to embed image in the document.
    #[error("Image embedding failed: {reason}")]
    ImageEmbedding {
        /// Human-readable reason for the failure
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let docx_err: DocxError = io_err.into();
        assert!(matches!(docx_err, DocxError::Io(_)));
    }
}
