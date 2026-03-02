//! DOCX document writer: top-level orchestration and public entry points.
//!
//! This module provides `write_docx` and `write_docx_to_bytes`, which drive
//! the two-pass conversion pipeline:
//!
//! 1. **Registration pass** — walk every block and pre-register all list IDs
//!    with `NumberingAllocator` so that numIds are assigned deterministically.
//! 2. **Conversion pass** — walk blocks again and convert each one to docx-rs
//!    elements, using the pre-built allocator through `ConvertCtx`.

use crate::allocators::{ImageAllocator, NumberingAllocator};
use crate::context::ConvertCtx;
use crate::image::convert_image_block;
use crate::paragraph::{convert_paragraph, convert_paragraph_with_numbering};
use crate::structure::{
    apply_document_structure, register_lists_in_header_footer_set, register_lists_in_notes,
};
use crate::table::convert_table;
use crate::{DocxError, DocxWriterOptions};
use docx_rs::{Docx, IndentLevel, LineSpacing, LineSpacingType, NumberingId, RunFonts};
use rtfkit_core::{Block, Document, DocumentStructure};
use rtfkit_style_tokens::StyleProfile;
use rtfkit_style_tokens::builtins::resolve_profile;
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
/// use rtfkit_docx::{write_docx, DocxWriterOptions};
/// use std::path::Path;
///
/// let doc = Document::from_blocks(vec![
///     Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello, World!")])),
/// ]);
///
/// write_docx(&doc, Path::new("output.docx"), &DocxWriterOptions::default()).unwrap();
/// ```
pub fn write_docx(
    document: &Document,
    path: &Path,
    options: &DocxWriterOptions,
) -> Result<(), DocxError> {
    let bytes = write_docx_to_bytes(document, options)?;
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
/// use rtfkit_docx::{write_docx_to_bytes, DocxWriterOptions};
///
/// let doc = Document::from_blocks(vec![
///     Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello, World!")])),
/// ]);
///
/// let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
/// assert!(!bytes.is_empty());
/// ```
pub fn write_docx_to_bytes(
    document: &Document,
    options: &DocxWriterOptions,
) -> Result<Vec<u8>, DocxError> {
    let docx = convert_document(document, options)?;
    let mut cursor = Cursor::new(Vec::new());
    docx.pack(&mut cursor)?;
    Ok(cursor.into_inner())
}

/// Converts an IR document to a docx-rs XMLDocx.
fn convert_document(
    document: &Document,
    options: &DocxWriterOptions,
) -> Result<docx_rs::XMLDocx, DocxError> {
    let mut doc = Docx::new();

    // Resolve style profile (None = no profile defaults applied).
    let profile: Option<StyleProfile> = options.style_profile.as_ref().map(resolve_profile);

    // Apply document-level defaults from profile when explicitly set.
    if let Some(ref p) = profile {
        let font_name = p
            .typography
            .font_body
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !font_name.is_empty() {
            doc = doc.default_fonts(RunFonts::new().ascii(&font_name).hi_ansi(&font_name));
        }
        doc = doc.default_size((p.typography.size_body * 2.0).round() as usize);
        doc = doc.default_line_spacing(
            LineSpacing::new()
                .line_rule(LineSpacingType::Auto)
                .line((p.typography.line_height_body * 240.0).round() as i32)
                .after((p.spacing.paragraph_gap * 20.0).round() as u32),
        );
    }

    let mut numbering = match profile.as_ref() {
        Some(p) => NumberingAllocator::with_profile(p),
        None => NumberingAllocator::new(),
    };
    let mut images = ImageAllocator::new();
    let note_lookup = build_note_lookup(document.structure.as_ref());
    let note_lookup = (!note_lookup.is_empty()).then_some(&note_lookup);

    // First pass: collect all list blocks (including nested table-cell lists)
    // and register them for deterministic numbering allocation.
    for block in &document.blocks {
        register_lists_in_block(block, &mut numbering);
    }
    // Also pre-register lists from structure (headers, footers, notes).
    if let Some(ref s) = document.structure {
        register_lists_in_header_footer_set(&s.headers, &mut numbering);
        register_lists_in_header_footer_set(&s.footers, &mut numbering);
        register_lists_in_notes(&s.notes, &mut numbering);
    }

    // Second pass: convert blocks.
    // All lists are already registered; use num_id_for for list lookup.
    let mut bookmark_id: usize = 0;
    let mut ctx = ConvertCtx {
        numbering: &numbering,
        images: &mut images,
        bookmark_id: &mut bookmark_id,
        note_lookup,
        profile: profile.as_ref(),
    };

    for block in &document.blocks {
        match block {
            Block::Paragraph(para) => {
                doc = doc.add_paragraph(convert_paragraph(para, &mut ctx));
            }
            Block::ListBlock(list) => {
                let num_id = ctx.numbering.num_id_for(list.list_id).unwrap_or(0);
                for item in &list.items {
                    for item_block in &item.blocks {
                        match item_block {
                            Block::Paragraph(para) => {
                                let paragraph = convert_paragraph_with_numbering(
                                    para, num_id, item.level, &mut ctx,
                                );
                                doc = doc.add_paragraph(paragraph);
                            }
                            Block::ImageBlock(image) => {
                                let paragraph =
                                    convert_image_block(image, ctx.images)?.numbering(
                                        NumberingId::new(num_id as usize),
                                        IndentLevel::new(item.level as usize),
                                    );
                                doc = doc.add_paragraph(paragraph);
                            }
                            Block::TableBlock(table) => {
                                let table = convert_table(table, &mut ctx)?;
                                doc = doc.add_table(table);
                            }
                            Block::ListBlock(_) => {}
                        }
                    }
                }
            }
            Block::TableBlock(table) => {
                let docx_table = convert_table(table, &mut ctx)?;
                doc = doc.add_table(docx_table);
            }
            Block::ImageBlock(image) => {
                let para = convert_image_block(image, ctx.images)?;
                doc = doc.add_paragraph(para);
            }
        }
    }

    // Apply document structure (headers, footers).
    if let Some(ref s) = document.structure {
        doc = apply_document_structure(
            doc,
            s,
            &numbering,
            ctx.images,
            note_lookup,
            profile.as_ref(),
        )?;
    }

    // Add numbering part if needed.
    if numbering.has_numbering() {
        doc = doc.numberings(numbering.build_numberings());
    }

    Ok(doc.build())
}

/// Recursively registers all list blocks for deterministic numbering pre-allocation.
pub(crate) fn register_lists_in_block(block: &Block, numbering: &mut NumberingAllocator) {
    match block {
        Block::Paragraph(_) => {}
        Block::ListBlock(list) => {
            numbering.register_list(list);
            for item in &list.items {
                for item_block in &item.blocks {
                    register_lists_in_block(item_block, numbering);
                }
            }
        }
        Block::TableBlock(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    for cell_block in &cell.blocks {
                        register_lists_in_block(cell_block, numbering);
                    }
                }
            }
        }
        Block::ImageBlock(_) => {}
    }
}

fn build_note_lookup(
    structure: Option<&DocumentStructure>,
) -> crate::allocators::NoteLookup {
    let mut lookup = crate::allocators::NoteLookup::new();
    if let Some(structure) = structure {
        for note in &structure.notes {
            lookup.insert(note.id, note.clone());
        }
    }
    lookup
}

// =============================================================================
// Integration / document-level tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{Block, Document, Paragraph, Run, TableBlock, TableCell, TableRow};
    use rtfkit_style_tokens::StyleProfileName;
    use std::fs::File;
    use std::io::Cursor;

    fn zip_entry_string(bytes: &[u8], entry_name: &str) -> String {
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        let mut entry = archive
            .by_name(entry_name)
            .unwrap_or_else(|_| panic!("missing ZIP entry: {entry_name}"));
        let mut xml = String::new();
        use std::io::Read;
        entry
            .read_to_string(&mut xml)
            .expect("Failed to read ZIP entry as UTF-8 string");
        xml
    }

    fn tiny_png_bytes() -> Vec<u8> {
        vec![
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H',
            b'D', b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, b'I', b'D', b'A', b'T', 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82,
        ]
    }

    #[test]
    fn test_empty_document() {
        let doc = Document::new();
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn test_empty_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::new())]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_empty_run() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new(""),
        ]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_write_to_file() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_output.docx");
        write_docx(&doc, &file_path, &DocxWriterOptions::default()).unwrap();
        assert!(file_path.exists());
        let file = File::open(&file_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn style_profile_report_applies_table_border_width_and_color() {
        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("A")]),
            )])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(
            &doc,
            &DocxWriterOptions {
                style_profile: Some(StyleProfileName::Report),
            },
        )
        .unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:tblBorders>"));
        assert!(document_xml.contains(
            r#"w:top w:val="single" w:sz="4" w:space="0" w:color="D1D5DB""#
        ));
    }

    #[test]
    fn style_profile_report_applies_list_item_gap_on_numbered_paragraphs() {
        use rtfkit_core::{ListBlock, ListItem, ListKind};
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 1")]),
        ));
        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        let bytes = write_docx_to_bytes(
            &doc,
            &DocxWriterOptions {
                style_profile: Some(StyleProfileName::Report),
            },
        )
        .unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:numPr>"));
        assert!(document_xml.contains(r#"w:spacing w:after="120""#));
    }

    #[test]
    fn style_profile_report_applies_table_profile_in_header() {
        use rtfkit_core::{DocumentStructure, HeaderFooterSet};
        let header_table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("H")]),
            )])]);
        let doc = Document {
            blocks: vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new(
                "Body",
            )]))],
            structure: Some(DocumentStructure {
                headers: HeaderFooterSet {
                    default: vec![Block::TableBlock(header_table)],
                    first: Vec::new(),
                    even: Vec::new(),
                },
                footers: HeaderFooterSet::default(),
                notes: Vec::new(),
            }),
            page_management: None,
        };
        let bytes = write_docx_to_bytes(
            &doc,
            &DocxWriterOptions {
                style_profile: Some(StyleProfileName::Report),
            },
        )
        .unwrap();
        let header_xml = zip_entry_string(&bytes, "word/header1.xml");
        assert!(header_xml.contains("<w:tblCellMar>"));
        assert!(header_xml.contains(r#"w:left w:w="160" w:type="dxa""#));
        assert!(header_xml.contains(r#"w:color="D1D5DB""#));
    }

    #[test]
    fn test_image_block_in_document() {
        use rtfkit_core::{ImageBlock, ImageFormat};
        let image = ImageBlock::new(ImageFormat::Png, tiny_png_bytes());
        let doc = Document::from_blocks(vec![Block::ImageBlock(image)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:drawing>"));
    }
}
