//! Document structure conversion: headers, footers, footnotes, and endnotes.

use crate::allocators::{ImageAllocator, NoteLookup, NumberingAllocator};
use crate::context::ConvertCtx;
use crate::image::convert_image_block;
use crate::paragraph::{convert_paragraph, convert_paragraph_with_numbering};
use crate::DocxError;
use docx_rs::{
    Footer as DocxFooter, Footnote as DocxFootnote, Header as DocxHeader,
    Paragraph as DocxParagraph, Run as DocxRun, RunProperty, VertAlignType,
};
use rtfkit_core::{Block, DocumentStructure, HeaderFooterSet, Note, NoteRef as IrNoteRef};
use rtfkit_style_tokens::StyleProfile;

/// Converts a `NoteRef` inline to a docx-rs footnote reference run.
///
/// Falls back to a superscript text run when the corresponding note body
/// is unavailable.
pub(crate) fn note_ref_to_docx_run(note_ref: &IrNoteRef, ctx: &ConvertCtx<'_>) -> DocxRun {
    if let Some(notes) = ctx.note_lookup
        && let Some(note) = notes.get(&note_ref.id)
    {
        // docx-rs does not currently expose endnotes, so both kinds are emitted
        // as footnote references to preserve note body content.
        let footnote = note_to_docx_footnote(note, ctx.numbering);
        return DocxRun::new().add_footnote_reference(footnote);
    }

    let mut run = DocxRun::new().add_text(note_ref.id.to_string());
    run.run_property = RunProperty::new().vert_align(VertAlignType::SuperScript);
    run
}

fn note_to_docx_footnote(note: &Note, numbering: &NumberingAllocator) -> DocxFootnote {
    let mut footnote = DocxFootnote {
        id: note.id as usize,
        content: Vec::new(),
    };
    let mut bookmark_id = 0usize;
    let mut dummy_images = ImageAllocator::new();
    let mut inner_ctx = ConvertCtx {
        numbering,
        images: &mut dummy_images,
        bookmark_id: &mut bookmark_id,
        note_lookup: None,
        profile: None,
    };
    append_note_blocks_as_paragraphs(&note.blocks, &mut inner_ctx, &mut footnote.content);
    footnote
}

fn append_note_blocks_as_paragraphs(
    blocks: &[Block],
    ctx: &mut ConvertCtx<'_>,
    out: &mut Vec<DocxParagraph>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(para) => {
                out.push(convert_paragraph(para, ctx));
            }
            Block::ListBlock(list) => {
                let num_id = ctx.numbering.num_id_for(list.list_id).unwrap_or(0);
                for item in &list.items {
                    for item_block in &item.blocks {
                        match item_block {
                            Block::Paragraph(para) => out.push(convert_paragraph_with_numbering(
                                para,
                                num_id,
                                item.level,
                                ctx,
                            )),
                            Block::ListBlock(nested) => {
                                append_note_blocks_as_paragraphs(
                                    &[Block::ListBlock(nested.clone())],
                                    ctx,
                                    out,
                                );
                            }
                            Block::TableBlock(table) => {
                                for row in &table.rows {
                                    for cell in &row.cells {
                                        append_note_blocks_as_paragraphs(
                                            &cell.blocks,
                                            ctx,
                                            out,
                                        );
                                    }
                                }
                            }
                            Block::ImageBlock(_) => {}
                        }
                    }
                }
            }
            Block::TableBlock(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        append_note_blocks_as_paragraphs(&cell.blocks, ctx, out);
                    }
                }
            }
            Block::ImageBlock(_) => {}
        }
    }
}

/// Converts a slice of IR blocks to a DOCX `Header`.
pub(crate) fn blocks_to_docx_header(
    blocks: &[Block],
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<DocxHeader, DocxError> {
    let mut header = DocxHeader::new();
    let mut bookmark_id: usize = 0;
    let mut ctx = ConvertCtx {
        numbering,
        images,
        bookmark_id: &mut bookmark_id,
        note_lookup,
        profile,
    };
    for block in blocks {
        match block {
            Block::Paragraph(para) => {
                header = header.add_paragraph(convert_paragraph(para, &mut ctx));
            }
            Block::TableBlock(table) => {
                header = header.add_table(crate::table::convert_table(table, &mut ctx)?);
            }
            Block::ListBlock(list) => {
                let num_id = ctx.numbering.num_id_for(list.list_id).unwrap_or(0);
                for item in &list.items {
                    for item_block in &item.blocks {
                        if let Block::Paragraph(para) = item_block {
                            let p = convert_paragraph_with_numbering(
                                para, num_id, item.level, &mut ctx,
                            );
                            header = header.add_paragraph(p);
                        }
                    }
                }
            }
            Block::ImageBlock(image) => {
                header = header.add_paragraph(convert_image_block(image, ctx.images)?);
            }
        }
    }
    Ok(header)
}

/// Converts a slice of IR blocks to a DOCX `Footer`.
pub(crate) fn blocks_to_docx_footer(
    blocks: &[Block],
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<DocxFooter, DocxError> {
    let mut footer = DocxFooter::new();
    let mut bookmark_id: usize = 0;
    let mut ctx = ConvertCtx {
        numbering,
        images,
        bookmark_id: &mut bookmark_id,
        note_lookup,
        profile,
    };
    for block in blocks {
        match block {
            Block::Paragraph(para) => {
                footer = footer.add_paragraph(convert_paragraph(para, &mut ctx));
            }
            Block::TableBlock(table) => {
                footer = footer.add_table(crate::table::convert_table(table, &mut ctx)?);
            }
            Block::ListBlock(list) => {
                let num_id = ctx.numbering.num_id_for(list.list_id).unwrap_or(0);
                for item in &list.items {
                    for item_block in &item.blocks {
                        if let Block::Paragraph(para) = item_block {
                            let p = convert_paragraph_with_numbering(
                                para, num_id, item.level, &mut ctx,
                            );
                            footer = footer.add_paragraph(p);
                        }
                    }
                }
            }
            Block::ImageBlock(image) => {
                footer = footer.add_paragraph(convert_image_block(image, ctx.images)?);
            }
        }
    }
    Ok(footer)
}

/// Registers lists from a `HeaderFooterSet` for numbering pre-allocation.
pub(crate) fn register_lists_in_header_footer_set(
    set: &HeaderFooterSet,
    numbering: &mut NumberingAllocator,
) {
    for blocks in [&set.default, &set.first, &set.even] {
        for block in blocks {
            crate::writer::register_lists_in_block(block, numbering);
        }
    }
}

/// Registers lists from structure notes for numbering pre-allocation.
pub(crate) fn register_lists_in_notes(notes: &[Note], numbering: &mut NumberingAllocator) {
    for note in notes {
        for block in &note.blocks {
            crate::writer::register_lists_in_block(block, numbering);
        }
    }
}

/// Applies all structure headers/footers to the Docx builder.
pub(crate) fn apply_document_structure(
    mut doc: docx_rs::Docx,
    structure: &DocumentStructure,
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<docx_rs::Docx, DocxError> {
    if !structure.headers.default.is_empty() {
        doc = doc.header(blocks_to_docx_header(
            &structure.headers.default,
            numbering,
            images,
            note_lookup,
            profile,
        )?);
    }
    if !structure.headers.first.is_empty() {
        doc = doc.first_header(blocks_to_docx_header(
            &structure.headers.first,
            numbering,
            images,
            note_lookup,
            profile,
        )?);
    }
    if !structure.headers.even.is_empty() {
        doc = doc.even_header(blocks_to_docx_header(
            &structure.headers.even,
            numbering,
            images,
            note_lookup,
            profile,
        )?);
    }
    if !structure.footers.default.is_empty() {
        doc = doc.footer(blocks_to_docx_footer(
            &structure.footers.default,
            numbering,
            images,
            note_lookup,
            profile,
        )?);
    }
    if !structure.footers.first.is_empty() {
        doc = doc.first_footer(blocks_to_docx_footer(
            &structure.footers.first,
            numbering,
            images,
            note_lookup,
            profile,
        )?);
    }
    if !structure.footers.even.is_empty() {
        doc = doc.even_footer(blocks_to_docx_footer(
            &structure.footers.even,
            numbering,
            images,
            note_lookup,
            profile,
        )?);
    }
    Ok(doc)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::{write_docx_to_bytes, DocxWriterOptions};
    use rtfkit_core::{
        Block, Document, DocumentStructure, HeaderFooterSet, ImageBlock, ImageFormat, Inline,
        Note, NoteKind, NoteRef, Paragraph, Run,
    };

    fn zip_entry_string(bytes: &[u8], entry_name: &str) -> String {
        use std::io::{Cursor, Read};
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        let mut entry = archive
            .by_name(entry_name)
            .unwrap_or_else(|_| panic!("missing ZIP entry: {entry_name}"));
        let mut xml = String::new();
        entry.read_to_string(&mut xml).unwrap();
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
    fn note_ref_emits_docx_footnote_reference_and_body() {
        let body_para = Paragraph::from_inlines(vec![
            Inline::Run(Run::new("Body")),
            Inline::NoteRef(NoteRef {
                id: 1,
                kind: NoteKind::Footnote,
            }),
        ]);
        let note = Note {
            id: 1,
            kind: NoteKind::Footnote,
            blocks: vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new(
                "Footnote body",
            )]))],
        };
        let doc = Document {
            blocks: vec![Block::Paragraph(body_para)],
            structure: Some(DocumentStructure {
                headers: HeaderFooterSet::default(),
                footers: HeaderFooterSet::default(),
                notes: vec![note],
            }),
            page_management: None,
        };

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        let footnotes_xml = zip_entry_string(&bytes, "word/footnotes.xml");

        assert!(document_xml.contains("<w:footnoteReference w:id=\"1\""));
        assert!(footnotes_xml.contains("Footnote body"));
    }

    #[test]
    fn header_image_is_emitted_to_docx_header_xml() {
        let image = ImageBlock::new(ImageFormat::Png, tiny_png_bytes());
        let doc = Document {
            blocks: vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new(
                "Body",
            )]))],
            structure: Some(DocumentStructure {
                headers: HeaderFooterSet {
                    default: vec![Block::ImageBlock(image)],
                    first: Vec::new(),
                    even: Vec::new(),
                },
                footers: HeaderFooterSet::default(),
                notes: Vec::new(),
            }),
            page_management: None,
        };

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let header_xml = zip_entry_string(&bytes, "word/header1.xml");
        assert!(header_xml.contains("<w:drawing>"));
    }
}
