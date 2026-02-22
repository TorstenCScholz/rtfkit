//! HTML writer for rtfkit IR documents.
//!
//! This crate provides functionality to convert rtfkit IR `Document` instances
//! from `rtfkit_core` to HTML format.
//!
//! # Overview
//!
//! The crate maps the IR types from `rtfkit-core` to their HTML equivalents:
//!
//! | IR Type | HTML Element |
//! |---------|--------------|
//! | `Document.blocks` | Sequence of block elements |
//! | `Paragraph` | `<p>` element |
//! | `ListBlock` | `<ul>` or `<ol>` with `<li>` children |
//! | `TableBlock` | `<table>` with `<tr>` and `<td>` children |
//! | `Run.text` | Text content with optional `<span>` for formatting |
//!
//! # Example
//!
//! ```rust
//! use rtfkit_core::{Document, Block, Paragraph, Run};
//! use rtfkit_html::{document_to_html, HtmlWriterOptions};
//!
//! // Create a document with a single paragraph
//! let doc = Document::from_blocks(vec![
//!     Block::Paragraph(Paragraph::from_runs(vec![
//!         Run::new("Hello, World!"),
//!     ])),
//! ]);
//!
//! // Convert to HTML with default options
//! let options = HtmlWriterOptions::default();
//! let html = document_to_html(&doc, &options).unwrap();
//! assert!(html.contains("Hello, World!"));
//! ```
//!
//! # Features
//!
//! - Converts IR documents to valid HTML5
//! - Supports paragraph alignment (left, center, right, justify)
//! - Supports inline formatting (bold, italic, underline)
//! - Handles Unicode text correctly
//! - Produces deterministic output
//! - Optional document wrapper and embedded CSS
//!
//! # Errors
//!
//! All functions return [`Result`] with [`HtmlWriterError`] on failure.
//! See [`HtmlWriterError`] for the error types that can occur.

pub mod blocks;
pub mod error;
pub mod escape;
pub mod options;
pub mod serialize;
pub mod style;
pub mod writer;

// Re-export public API types
pub use error::HtmlWriterError;
pub use options::HtmlWriterOptions;
pub use writer::{HtmlWriterOutput, document_to_html, document_to_html_with_warnings};

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{Block, Document, Paragraph, Run};

    #[test]
    fn test_public_api_empty_document() {
        let doc = Document::new();
        let options = HtmlWriterOptions::default();
        let result = document_to_html(&doc, &options);
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("<body>"));
    }

    #[test]
    fn test_public_api_simple_document() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);

        let options = HtmlWriterOptions::default();
        let result = document_to_html(&doc, &options);
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains("Test content"));
        assert!(html.contains("<p>"));
    }

    #[test]
    fn test_fragment_mode() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Fragment"),
        ]))]);

        let options = HtmlWriterOptions {
            emit_document_wrapper: false,
            include_default_css: false,
        };

        let html = document_to_html(&doc, &options).unwrap();
        assert!(!html.contains("<!doctype html>"));
        assert!(html.contains("<p>Fragment</p>"));
    }
}
