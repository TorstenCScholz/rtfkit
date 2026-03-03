//! DOCX writer for rtfkit IR documents.
//!
//! This crate provides functionality to convert rtfkit IR `Document` instances
//! from `rtfkit_core` to DOCX format using the `docx-rs` library.
//!
//! # Overview
//!
//! The crate maps the IR types from `rtfkit-core` to their DOCX equivalents:
//!
//! | IR Type | DOCX Element |
//! |---------|--------------|
//! | `Document.blocks` | Sequence of `w:p` elements |
//! | `Paragraph.alignment` | `w:pPr/w:jc` |
//! | `Run.text` | `w:r/w:t` |
//! | `Run.bold` | `w:rPr/w:b` |
//! | `Run.italic` | `w:rPr/w:i` |
//! | `Run.underline` | `w:rPr/w:u w:val="single"` |
//! | `Inline::Hyperlink` | `w:hyperlink` + external relationship |
//!
//! # Example
//!
//! ```rust
//! use rtfkit_core::{Document, Block, Paragraph, Run};
//! use rtfkit_docx::{write_docx_to_bytes, DocxWriterOptions};
//!
//! // Create a document with a single paragraph
//! let doc = Document::from_blocks(vec![
//!     Block::Paragraph(Paragraph::from_runs(vec![
//!         Run::new("Hello, World!"),
//!     ])),
//! ]);
//!
//! // Convert to DOCX bytes
//! let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
//! assert!(!bytes.is_empty());
//! ```
//!
//! # Features
//!
//! - Converts IR documents to valid OOXML DOCX files
//! - Supports paragraph alignment (left, center, right, justify)
//! - Supports inline formatting (bold, italic, underline)
//! - Supports hyperlinks (`http://`, `https://`, `mailto:`)
//! - Handles Unicode text correctly
//! - Produces deterministic output
//!
//! # Errors
//!
//! All functions return [`Result`] with [`DocxError`] on failure.
//! See [`DocxError`] for the error types that can occur.

pub(crate) mod allocators;
pub(crate) mod context;
pub mod error;
pub(crate) mod image;
pub mod options;
pub(crate) mod paragraph;
pub(crate) mod shading;
pub(crate) mod structure;
pub(crate) mod table;
pub(crate) mod utils;
pub mod writer;

pub use error::DocxError;
pub use options::DocxWriterOptions;
pub use writer::{write_docx, write_docx_to_bytes};

// Re-export docx-rs types that users might need
pub use docx_rs::AlignmentType;

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{Block, Document, Paragraph, Run};

    #[test]
    fn test_public_api_write_docx_to_bytes() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test"),
        ]))]);
        let result = write_docx_to_bytes(&doc, &DocxWriterOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_public_api_write_docx() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test"),
        ]))]);

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.docx");

        let result = write_docx(&doc, &path, &DocxWriterOptions::default());
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_error_from_io() {
        use std::io;

        let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
        let docx_err: DocxError = io_err.into();
        assert!(matches!(docx_err, DocxError::Io(_)));
    }
}
