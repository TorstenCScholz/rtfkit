//! DOCX document writer implementation.
//!
//! This module provides the core conversion logic from rtfkit IR [`Document`]
//! to DOCX format using the `docx-rs` library.

use crate::DocxError;
use docx_rs::{
    AlignmentType, Docx, Paragraph as DocxParagraph, Run as DocxRun,
};
use rtfkit_core::{Alignment, Block, Document, Paragraph, Run};
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;

/// Writes a document to a file at the specified path.
///
/// # Arguments
///
/// * `document` - The IR document to convert
/// * `path` - The path where the DOCX file will be written
///
/// # Errors
///
/// Returns [`DocxError`] if the file cannot be created or written.
///
/// # Example
///
/// ```no_run
/// use rtfkit_core::{Document, Block, Paragraph, Run};
/// use rtfkit_docx::write_docx;
/// use std::path::Path;
///
/// let doc = Document::from_blocks(vec![
///     Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello, World!")])),
/// ]);
///
/// write_docx(&doc, Path::new("output.docx")).unwrap();
/// ```
pub fn write_docx(document: &Document, path: &Path) -> Result<(), DocxError> {
    let bytes = write_docx_to_bytes(document)?;
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    Ok(())
}

/// Writes a document to a byte vector.
///
/// # Arguments
///
/// * `document` - The IR document to convert
///
/// # Returns
///
/// A `Vec<u8>` containing the DOCX file data.
///
/// # Errors
///
/// Returns [`DocxError`] if the document cannot be converted or written.
///
/// # Example
///
/// ```
/// use rtfkit_core::{Document, Block, Paragraph, Run};
/// use rtfkit_docx::write_docx_to_bytes;
///
/// let doc = Document::from_blocks(vec![
///     Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello, World!")])),
/// ]);
///
/// let bytes = write_docx_to_bytes(&doc).unwrap();
/// assert!(!bytes.is_empty());
/// ```
pub fn write_docx_to_bytes(document: &Document) -> Result<Vec<u8>, DocxError> {
    let docx = convert_document(document)?;

    let mut cursor = Cursor::new(Vec::new());
    docx.pack(&mut cursor)?;
    Ok(cursor.into_inner())
}

/// Converts an IR document to a docx-rs XMLDocx.
fn convert_document(document: &Document) -> Result<docx_rs::XMLDocx, DocxError> {
    let mut doc = Docx::new();

    for block in &document.blocks {
        match block {
            Block::Paragraph(para) => {
                doc = doc.add_paragraph(convert_paragraph(para));
            }
        }
    }

    Ok(doc.build())
}

/// Converts an IR paragraph to a docx-rs paragraph.
fn convert_paragraph(para: &Paragraph) -> DocxParagraph {
    let mut p = DocxParagraph::new();

    // Map alignment
    p = p.align(convert_alignment(para.alignment));

    // Map runs
    for run in &para.runs {
        p = p.add_run(convert_run(run));
    }

    p
}

/// Converts IR alignment to docx-rs alignment.
fn convert_alignment(align: Alignment) -> AlignmentType {
    match align {
        Alignment::Left => AlignmentType::Left,
        Alignment::Center => AlignmentType::Center,
        Alignment::Right => AlignmentType::Right,
        Alignment::Justify => AlignmentType::Both,
    }
}

/// Converts an IR run to a docx-rs run.
///
/// Handles whitespace preservation for runs with leading/trailing spaces.
fn convert_run(run: &Run) -> DocxRun {
    let mut r = DocxRun::new().add_text(&run.text);

    // Apply formatting
    if run.bold {
        r = r.bold();
    }
    if run.italic {
        r = r.italic();
    }
    if run.underline {
        r = r.underline("single");
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Paragraph/Run Mapping Tests
    // =========================================================================

    #[test]
    fn test_simple_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's a valid ZIP (DOCX is a ZIP file)
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn test_multiple_paragraphs() {
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("First paragraph")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Second paragraph")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Third paragraph")])),
        ]);

        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_multiple_runs_in_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, "),
            Run::new("World!"),
        ]))]);

        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Alignment Mapping Tests
    // =========================================================================

    #[test]
    fn test_alignment_left() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Left;
        para.runs.push(Run::new("Left aligned"));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_alignment_center() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Center;
        para.runs.push(Run::new("Center aligned"));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_alignment_right() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Right;
        para.runs.push(Run::new("Right aligned"));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_alignment_justify() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Justify;
        para.runs.push(Run::new("Justified text"));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Style Mapping Tests
    // =========================================================================

    #[test]
    fn test_bold_run() {
        let mut run = Run::new("Bold text");
        run.bold = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_italic_run() {
        let mut run = Run::new("Italic text");
        run.italic = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_underline_run() {
        let mut run = Run::new("Underlined text");
        run.underline = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_bold_italic_run() {
        let mut run = Run::new("Bold and italic");
        run.bold = true;
        run.italic = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_bold_italic_underline_run() {
        let mut run = Run::new("All styles");
        run.bold = true;
        run.italic = true;
        run.underline = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_mixed_styles_in_paragraph() {
        let mut bold_run = Run::new("bold");
        bold_run.bold = true;

        let mut italic_run = Run::new("italic");
        italic_run.italic = true;

        let mut underline_run = Run::new("underline");
        underline_run.underline = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            bold_run,
            italic_run,
            underline_run,
        ]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Whitespace Preservation Tests
    // =========================================================================

    #[test]
    fn test_leading_space() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("  leading spaces"),
        ]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_trailing_space() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("trailing spaces  "),
        ]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_multiple_spaces() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("multiple   spaces   inside"),
        ]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_only_spaces() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new(
            "    ",
        )]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Empty Document Handling Tests
    // =========================================================================

    #[test]
    fn test_empty_document() {
        let doc = Document::new();
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's a valid ZIP
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn test_empty_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::new())]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_empty_run() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new(
            "",
        )]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // File Writing Tests
    // =========================================================================

    #[test]
    fn test_write_to_file() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_output.docx");

        write_docx(&doc, &file_path).unwrap();
        assert!(file_path.exists());

        // Verify the file is a valid DOCX
        let file = File::open(&file_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    // =========================================================================
    // Unicode Tests
    // =========================================================================

    #[test]
    fn test_unicode_text() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello 世界 🌍"),
        ]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_special_characters() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Special: <>&\"'"),
        ]))]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }
}
