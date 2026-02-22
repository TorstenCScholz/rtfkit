//! Typst-based PDF renderer for rtfkit IR documents.
//!
//! This crate provides functionality to convert rtfkit IR `Document` instances
//! from `rtfkit_core` to PDF format using the in-process Typst library.
//!
//! # Overview
//!
//! The crate uses a multi-stage pipeline:
//!
//! 1. **IR Mapping**: Convert rtfkit-core IR to Typst source code
//! 2. **Typst Rendering**: Render the Typst source to PDF using the in-process Typst engine
//!
//! # Example
//!
//! ```rust
//! use rtfkit_core::{Document, Block, Paragraph, Run};
//! use rtfkit_render_typst::{document_to_pdf_with_warnings, RenderOptions};
//!
//! // Create a document with a single paragraph
//! let doc = Document::from_blocks(vec![
//!     Block::Paragraph(Paragraph::from_runs(vec![
//!         Run::new("Hello, World!"),
//!     ])),
//! ]);
//!
//! // Convert to PDF with default options
//! let options = RenderOptions::default();
//! let output = document_to_pdf_with_warnings(&doc, &options);
//! ```
//!
//! # Features
//!
//! - **In-process rendering**: No shell-out to external binaries
//! - **No PATH dependency**: Works even with empty PATH
//! - **Offline capable**: No network access needed
//! - **Bundled fonts**: Consistent output across systems
//! - **Deterministic**: Same input → same output (with fixed timestamp)
//!
//! # Errors
//!
//! All functions return [`Result`] with [`RenderError`] on failure.
//! See [`RenderError`] for the error types that can occur.

pub mod engine;
pub mod error;
pub mod map;
pub mod options;

// Re-export public API types
pub use error::{RenderError, Warning, WarningKind};
pub use options::{DeterminismOptions, Margins, PageSize, RenderOptions};

// Re-export mapping functions
pub use map::{BlockOutput, DocumentOutput, map_document, map_list, map_paragraph, map_table};

// Re-export engine functions
pub use engine::compile_to_pdf;

use rtfkit_core::Document;

/// Output from rendering a document to PDF.
#[derive(Debug)]
pub struct RenderOutput {
    /// The rendered PDF bytes.
    pub pdf_bytes: Vec<u8>,
    /// Warnings generated during rendering.
    pub warnings: Vec<Warning>,
}

/// Convert an IR Document to PDF bytes with optional warnings.
///
/// This is the primary entry point for rendering documents to PDF.
/// It performs in-process rendering using the Typst library, with no
/// dependency on external binaries.
///
/// # Arguments
///
/// * `doc` - The IR document to render
/// * `options` - Rendering options controlling page size, margins, and determinism
///
/// # Returns
///
/// A `Result` containing the rendered PDF bytes and any warnings on success,
/// or a `RenderError` on failure.
///
/// # Example
///
/// ```rust
/// use rtfkit_core::{Document, Block, Paragraph, Run};
/// use rtfkit_render_typst::{document_to_pdf_with_warnings, RenderOptions};
///
/// let doc = Document::from_blocks(vec![
///     Block::Paragraph(Paragraph::from_runs(vec![
///         Run::new("Hello, World!"),
///     ])),
/// ]);
///
/// let options = RenderOptions::default();
/// let result = document_to_pdf_with_warnings(&doc, &options);
/// assert!(result.is_ok());
///
/// let output = result.unwrap();
/// assert!(output.pdf_bytes.starts_with(b"%PDF-"));
/// ```
///
/// # Determinism
///
/// For deterministic output, set `options.determinism.fixed_timestamp` to a fixed
/// timestamp string (ISO 8601 format). This ensures the same input always produces
/// the same PDF bytes.
pub fn document_to_pdf_with_warnings(
    doc: &Document,
    options: &RenderOptions,
) -> Result<RenderOutput, RenderError> {
    // Step 1: Map IR to Typst source
    let mapped = map::map_document(doc, options);

    // Convert mapping warnings to render warnings
    let mut warnings: Vec<Warning> = mapped
        .warnings
        .into_iter()
        .map(|mapping_warning| Warning {
            kind: mapping_warning.kind(),
            message: mapping_warning.code(),
        })
        .collect();

    // Parse fixed timestamp if provided
    let fixed_timestamp = match options.determinism.fixed_timestamp.as_ref() {
        Some(s) => {
            let ts = chrono::DateTime::parse_from_rfc3339(s).map_err(|e| {
                RenderError::InvalidOption(format!(
                    "fixed_timestamp must be RFC3339, got '{}': {}",
                    s, e
                ))
            })?;
            Some(ts.timestamp())
        }
        None => None,
    };

    // Step 2: Compile Typst source to PDF
    let (pdf_bytes, compile_warnings) =
        engine::compile_to_pdf(&mapped.typst_source, fixed_timestamp)?;

    // Merge warnings from compilation
    warnings.extend(compile_warnings);

    Ok(RenderOutput {
        pdf_bytes,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{Block, ListBlock, ListItem, ListKind, Paragraph, Run};

    #[test]
    fn test_public_api_compiles() {
        // Verify the API compiles and types are correct
        let doc = Document::new();
        let options = RenderOptions::default();
        let result: Result<RenderOutput, RenderError> =
            document_to_pdf_with_warnings(&doc, &options);

        // Should succeed (empty document is valid)
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_output_type() {
        // Verify RenderOutput has expected fields
        let output = RenderOutput {
            pdf_bytes: vec![1, 2, 3],
            warnings: vec![Warning {
                message: "test".to_string(),
                kind: WarningKind::PartialSupport,
            }],
        };

        assert_eq!(output.pdf_bytes, vec![1, 2, 3]);
        assert_eq!(output.warnings.len(), 1);
    }

    #[test]
    fn test_simple_document() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);

        let options = RenderOptions::default();
        let result = document_to_pdf_with_warnings(&doc, &options);

        // Should succeed
        assert!(result.is_ok());
        let output = result.unwrap();

        // PDF should start with %PDF-
        assert!(output.pdf_bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn test_default_options() {
        let options = RenderOptions::default();
        assert_eq!(options.page_size, PageSize::A4);
        assert!(options.determinism.fixed_timestamp.is_none());
        assert!(!options.determinism.normalize_metadata);
    }

    #[test]
    fn test_error_exit_code() {
        let err = RenderError::Rendering("test".to_string());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_invalid_fixed_timestamp_returns_error() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);
        let options = RenderOptions {
            determinism: DeterminismOptions {
                fixed_timestamp: Some("not-a-timestamp".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = document_to_pdf_with_warnings(&doc, &options);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("fixed_timestamp"));
    }

    #[test]
    fn test_map_document_integration() {
        // Integration test for document mapping
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        // Should contain page setup
        assert!(output.typst_source.contains("#set page("));
        // Should contain the content
        assert!(output.typst_source.contains("Hello, World!"));
        // Should have no warnings for simple content
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_full_render_pipeline() {
        // Full integration test: IR -> Typst -> PDF
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("First paragraph")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Second paragraph")])),
        ]);

        let options = RenderOptions::default();
        let result = document_to_pdf_with_warnings(&doc, &options);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Verify PDF structure
        assert!(output.pdf_bytes.starts_with(b"%PDF-"));
        let pdf_str = String::from_utf8_lossy(&output.pdf_bytes);
        assert!(pdf_str.contains("%%EOF"));

        // Should have no warnings
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_deterministic_rendering() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Deterministic test"),
        ]))]);

        let options = RenderOptions {
            determinism: DeterminismOptions {
                fixed_timestamp: Some("2021-01-01T00:00:00Z".to_string()), // Fixed ISO 8601 timestamp
                ..Default::default()
            },
            ..Default::default()
        };

        // Render twice with same options
        let result1 = document_to_pdf_with_warnings(&doc, &options).unwrap();
        let result2 = document_to_pdf_with_warnings(&doc, &options).unwrap();

        // Should produce identical output
        assert_eq!(result1.pdf_bytes, result2.pdf_bytes);
    }

    #[test]
    fn test_mixed_list_kind_warning_is_dropped_content() {
        let mut list = ListBlock::new(1, ListKind::Mixed);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item")]),
        ));
        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

        let options = RenderOptions::default();
        let output = document_to_pdf_with_warnings(&doc, &options).unwrap();

        assert!(output.warnings.iter().any(|w| {
            w.message == "list_mixed_kind_fallback_to_bullet"
                && w.kind == WarningKind::DroppedContent
        }));
    }
}
