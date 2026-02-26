//! In-process Typst engine for PDF rendering.
//!
//! This module provides a `TypstEngine` implementation that enables in-process
//! PDF compilation without shelling out to the `typst` CLI binary.
//!
//! ## Design Goals
//!
//! - **No shell-out**: All rendering happens in-process
//! - **No PATH dependency**: Works even with empty PATH
//! - **Offline capable**: No network access needed
//! - **Bundled fonts**: Consistent output across systems
//! - **Deterministic**: Same input → same output
//!
//! ## Determinism Guarantees
//!
//! When a fixed timestamp is provided, the PDF output is guaranteed to be
//! byte-identical across multiple runs with the same input. This is achieved by:
//!
//! 1. Using embedded fonts only (no system font variation)
//! 2. Passing the fixed timestamp to both Typst compilation and PDF export
//! 3. Using a stable document identifier based on content hash
//! 4. In-memory processing (no file system variation)

use std::sync::OnceLock;

use chrono::{Datelike, Timelike};
use ecow::EcoVec;
use typst::diag::{FileError, FileResult, SourceDiagnostic, SourceResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library as TypstLibrary, LibraryExt as TypstLibraryExt, World as TypstWorld};

use crate::error::{RenderError, Warning, WarningKind};
use crate::map::TypstAssetBundle;

/// Global font book and fonts, loaded once and reused.
static FONTS: OnceLock<typst_kit::fonts::Fonts> = OnceLock::new();

/// Get or initialize the global fonts.
///
/// This uses embedded fonts only for determinism and offline capability.
fn get_fonts() -> &'static typst_kit::fonts::Fonts {
    FONTS.get_or_init(|| {
        typst_kit::fonts::FontSearcher::new()
            .include_system_fonts(false)
            .search()
    })
}

/// A Typst world for rendering in-memory documents.
///
/// This implementation provides:
/// - A single main source file (the generated Typst markup)
/// - Embedded fonts for offline, deterministic rendering
/// - No network access
/// - Fixed datetime for determinism (optional)
pub struct TypstEngine {
    /// The main source file ID.
    main_id: FileId,
    /// The Typst source content.
    source: Source,
    /// The standard library.
    library: LazyHash<TypstLibrary>,
    /// Font book metadata.
    book: LazyHash<FontBook>,
    /// Font slots.
    fonts: &'static typst_kit::fonts::Fonts,
    /// In-memory assets addressable by Typst virtual path.
    assets: TypstAssetBundle,
    /// Optional fixed timestamp for determinism (Unix timestamp).
    fixed_timestamp: Option<i64>,
}

impl TypstEngine {
    /// Create a new Typst engine with the given source content.
    ///
    /// # Arguments
    ///
    /// * `source` - The Typst markup source code
    /// * `assets` - In-memory assets that Typst may resolve via `#image` etc.
    /// * `fixed_timestamp` - Optional Unix timestamp for deterministic output
    ///
    /// # Returns
    ///
    /// A new `TypstEngine` ready for compilation.
    pub fn new(source: &str, assets: &TypstAssetBundle, fixed_timestamp: Option<i64>) -> Self {
        let fonts = get_fonts();

        // Create a fake file ID for the main source
        let main_id = FileId::new_fake(VirtualPath::new("main.typ"));

        // Create source from the provided content
        let source = Source::new(main_id, source.into());

        // Build the standard library
        let library = TypstLibrary::default();

        Self {
            main_id,
            source,
            library: LazyHash::new(library),
            book: LazyHash::new(fonts.book.clone()),
            fonts,
            assets: assets.clone(),
            fixed_timestamp,
        }
    }

    /// Compile the source to a paged document.
    ///
    /// # Returns
    ///
    /// A `SourceResult` containing the compiled document or errors.
    pub fn compile(&self) -> SourceResult<PagedDocument> {
        let result: SourceResult<PagedDocument> = typst::compile(self).output;
        result
    }

    fn normalize_asset_path(path: &str) -> String {
        path.replace('\\', "/")
    }
}

impl TypstWorld for TypstEngine {
    fn library(&self) -> &LazyHash<TypstLibrary> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path().to_string_lossy();
        let normalized_path = Self::normalize_asset_path(path.as_ref());

        if let Some(bytes) = self.assets.get(normalized_path.as_str()) {
            return Ok(Bytes::new(bytes.to_vec()));
        }

        // We only serve bundled virtual assets.
        Err(FileError::NotFound(
            id.vpath().as_rootless_path().to_path_buf(),
        ))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.fonts.get(index)?.get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        // Use fixed timestamp if provided for determinism
        let datetime = if let Some(ts) = self.fixed_timestamp {
            // Convert Unix timestamp to datetime
            chrono::DateTime::from_timestamp(ts, 0)?
        } else {
            // Use current time
            chrono::Utc::now()
        };

        // Apply offset if specified (offset is in hours)
        let datetime = if let Some(hours) = offset {
            datetime + chrono::Duration::hours(hours)
        } else {
            datetime
        };

        // Extract date components
        let year = datetime.year();
        let month = datetime.month() as u8;
        let day = datetime.day() as u8;

        // Create Typst Datetime using from_ymd
        Datetime::from_ymd(year, month, day)
    }
}

/// Compile Typst source to PDF bytes.
///
/// This is the main entry point for in-process PDF generation.
///
/// # Arguments
///
/// * `source` - The Typst markup source code
/// * `assets` - In-memory assets referenced by Typst virtual paths
/// * `fixed_timestamp` - Optional Unix timestamp for deterministic output
///
/// # Returns
///
/// A `Result` containing the PDF bytes and any warnings, or a `RenderError`.
///
/// # Determinism
///
/// When `fixed_timestamp` is provided, the output is guaranteed to be byte-identical
/// across multiple runs with the same input. The timestamp is used for:
///
/// - PDF creation/modification metadata
/// - Document date (if not explicitly set in the Typst source)
///
/// The PDF metadata includes:
/// - Creator: "Typst {version}" (set by typst-pdf)
/// - Producer: Not explicitly set (handled by typst-pdf)
/// - Creation date: The fixed timestamp (when provided)
pub fn compile_to_pdf(
    source: &str,
    assets: &TypstAssetBundle,
    fixed_timestamp: Option<i64>,
) -> Result<(Vec<u8>, Vec<Warning>), RenderError> {
    let engine = TypstEngine::new(source, assets, fixed_timestamp);

    // Compile to paged document
    let warned = typst::compile::<PagedDocument>(&engine);

    // Convert warnings
    let warnings = convert_warnings(&warned.warnings);

    // Get the document or error
    let document = warned.output.map_err(convert_errors)?;

    // Build PDF options with timestamp for determinism
    let pdf_options = build_pdf_options(fixed_timestamp);

    // Export to PDF
    let pdf_bytes = typst_pdf::pdf(&document, &pdf_options).map_err(convert_errors)?;

    Ok((pdf_bytes, warnings))
}

/// Build PDF options with optional fixed timestamp for determinism.
///
/// When a fixed timestamp is provided, it's converted to a Typst Timestamp
/// and passed to the PDF exporter to ensure consistent metadata.
fn build_pdf_options(fixed_timestamp: Option<i64>) -> typst_pdf::PdfOptions<'static> {
    let mut options = typst_pdf::PdfOptions::default();

    if let Some(ts) = fixed_timestamp {
        // Convert Unix timestamp to Typst Timestamp for PDF metadata
        if let Some(timestamp) = unix_to_typst_timestamp(ts) {
            options.timestamp = Some(timestamp);
        }
    }

    options
}

/// Convert a Unix timestamp to a Typst PDF Timestamp.
///
/// This creates a UTC timestamp suitable for PDF metadata.
fn unix_to_typst_timestamp(unix_ts: i64) -> Option<typst_pdf::Timestamp> {
    // Convert Unix timestamp to chrono datetime
    let datetime = chrono::DateTime::from_timestamp(unix_ts, 0)?;

    // Extract date and time components
    let year = datetime.year();
    let month = datetime.month() as u8;
    let day = datetime.day() as u8;
    let hour = datetime.hour() as u8;
    let minute = datetime.minute() as u8;
    let second = datetime.second() as u8;

    // Create Typst Datetime with full date and time
    let typst_datetime = Datetime::from_ymd_hms(year, month, day, hour, minute, second)?;

    // Create UTC timestamp
    Some(typst_pdf::Timestamp::new_utc(typst_datetime))
}

/// Convert Typst warnings to our Warning type.
fn convert_warnings(warnings: &EcoVec<SourceDiagnostic>) -> Vec<Warning> {
    warnings
        .iter()
        .map(|diag| Warning {
            message: diag.message.to_string(),
            kind: WarningKind::PartialSupport, // Default kind for Typst warnings
        })
        .collect()
}

/// Convert Typst errors to our RenderError type.
fn convert_errors(errors: EcoVec<SourceDiagnostic>) -> RenderError {
    let messages: Vec<String> = errors
        .iter()
        .map(|diag| {
            let location = diag
                .span
                .id()
                .map(|id| format!("{:?}: ", id))
                .unwrap_or_default();
            format!("{}{}", location, diag.message)
        })
        .collect();

    RenderError::Rendering(messages.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let source = "#set page(width: 210mm, height: 297mm)\nHello, World!";
        let engine = TypstEngine::new(source, &TypstAssetBundle::default(), None);
        assert!(
            engine
                .source(FileId::new_fake(VirtualPath::new("main.typ")))
                .is_ok()
                || engine.source(engine.main_id).is_ok()
        );
    }

    #[test]
    fn test_engine_with_fixed_timestamp() {
        let source = "Hello, World!";
        let engine = TypstEngine::new(source, &TypstAssetBundle::default(), Some(1609459200)); // 2021-01-01 00:00:00 UTC
        assert!(engine.source(engine.main_id).is_ok());
    }

    #[test]
    fn test_engine_reads_bundled_asset_file() {
        let mut assets = TypstAssetBundle::default();
        assets
            .files
            .insert("assets/image-000001.png".to_string(), vec![1, 2, 3]);

        let engine = TypstEngine::new("Hello", &assets, None);
        let file_id = FileId::new_fake(VirtualPath::new("assets/image-000001.png"));
        let bytes = engine.file(file_id).expect("asset should resolve");
        assert_eq!(bytes.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn test_engine_reads_bundled_asset_file_with_windows_separator() {
        let mut assets = TypstAssetBundle::default();
        assets
            .files
            .insert("assets/image-000001.png".to_string(), vec![1, 2, 3]);

        let engine = TypstEngine::new("Hello", &assets, None);
        let file_id = FileId::new_fake(VirtualPath::new("assets\\image-000001.png"));
        let bytes = engine.file(file_id).expect("asset should resolve");
        assert_eq!(bytes.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn test_engine_missing_asset_returns_not_found() {
        let engine = TypstEngine::new("Hello", &TypstAssetBundle::default(), None);
        let file_id = FileId::new_fake(VirtualPath::new("assets/missing.png"));
        let err = engine.file(file_id).unwrap_err();
        assert!(matches!(err, FileError::NotFound(_)));
    }

    #[test]
    fn test_compile_simple_document() {
        let source = "#set page(width: 210mm, height: 297mm)\nHello, World!";
        let result = compile_to_pdf(source, &TypstAssetBundle::default(), None);
        assert!(result.is_ok());

        let (pdf_bytes, warnings) = result.unwrap();
        // PDF should start with %PDF-
        assert!(pdf_bytes.starts_with(b"%PDF-"));
        // Should have no warnings for simple content
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_compile_with_error() {
        // Invalid Typst syntax
        let source = "#invalid_function_that_does_not_exist()";
        let result = compile_to_pdf(source, &TypstAssetBundle::default(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_output() {
        let source = "#set page(width: 210mm, height: 297mm)\nHello, World!";
        let fixed_ts = Some(1609459200); // Fixed timestamp

        let (pdf1, _) = compile_to_pdf(source, &TypstAssetBundle::default(), fixed_ts).unwrap();
        let (pdf2, _) = compile_to_pdf(source, &TypstAssetBundle::default(), fixed_ts).unwrap();

        // Same input with fixed timestamp should produce identical output
        assert_eq!(pdf1, pdf2);
    }

    #[test]
    fn test_deterministic_output_multiple_runs() {
        // Run the same conversion 5 times to ensure consistency
        let source = "#set page(width: 210mm, height: 297mm)\nDeterministic test content!";
        let fixed_ts = Some(1609459200); // 2021-01-01 00:00:00 UTC

        let results: Vec<Vec<u8>> = (0..5)
            .map(|_| {
                compile_to_pdf(source, &TypstAssetBundle::default(), fixed_ts)
                    .unwrap()
                    .0
            })
            .collect();

        // All results should be identical
        for (i, result) in results.iter().enumerate().skip(1) {
            assert_eq!(
                &results[0], result,
                "PDF output should be identical across runs (run 0 vs run {})",
                i
            );
        }
    }

    #[test]
    fn test_pdf_valid_header() {
        let source = "Test content";
        let (pdf_bytes, _) = compile_to_pdf(source, &TypstAssetBundle::default(), None).unwrap();

        // PDF must start with %PDF- and end with %%EOF
        assert!(pdf_bytes.starts_with(b"%PDF-"));
        // Note: The EOF check is more complex as there can be trailing whitespace
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_str.contains("%%EOF"));
    }

    #[test]
    fn test_unix_to_typst_timestamp() {
        // Test conversion of Unix timestamp to Typst timestamp
        let ts = unix_to_typst_timestamp(1609459200); // 2021-01-01 00:00:00 UTC
        assert!(ts.is_some());

        let ts = unix_to_typst_timestamp(0); // 1970-01-01 00:00:00 UTC
        assert!(ts.is_some());
    }

    #[test]
    fn test_build_pdf_options_with_timestamp() {
        let options = build_pdf_options(Some(1609459200));
        assert!(options.timestamp.is_some());

        let options = build_pdf_options(None);
        assert!(options.timestamp.is_none());
    }
}
