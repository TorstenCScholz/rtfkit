//! Focused detection of page-related semantics for Typst mapping.
//!
//! This module answers a single question:
//! does the document *require* page numbering to be enabled in Typst?

use rtfkit_core::{Block, Document, GeneratedBlockKind, Inline, Note, Paragraph, TableBlock};

/// Returns true when the document semantics require page numbering to be enabled.
///
/// This is the case when:
/// - Any `Inline::PageField(_)` is present in body, headers, footers, or notes.
/// - Any table-of-contents generated block is present, either inline (`GeneratedBlockMarker`)
///   or via `PageManagement.generated_blocks`.
pub fn requires_page_numbering(doc: &Document) -> bool {
    if doc.page_management.as_ref().is_some_and(|pm| {
        pm.generated_blocks
            .iter()
            .any(|g| matches!(g.kind, GeneratedBlockKind::TableOfContents { .. }))
    }) {
        return true;
    }

    if any_page_semantics_in_blocks(&doc.blocks) {
        return true;
    }

    if let Some(structure) = &doc.structure {
        if any_page_semantics_in_blocks(&structure.headers.default)
            || any_page_semantics_in_blocks(&structure.headers.first)
            || any_page_semantics_in_blocks(&structure.headers.even)
            || any_page_semantics_in_blocks(&structure.footers.default)
            || any_page_semantics_in_blocks(&structure.footers.first)
            || any_page_semantics_in_blocks(&structure.footers.even)
            || any_page_semantics_in_notes(&structure.notes)
        {
            return true;
        }
    }

    false
}

fn any_page_semantics_in_notes(notes: &[Note]) -> bool {
    notes
        .iter()
        .any(|note| any_page_semantics_in_blocks(&note.blocks))
}

fn any_page_semantics_in_blocks(blocks: &[Block]) -> bool {
    blocks.iter().any(any_page_semantics_in_block)
}

fn any_page_semantics_in_block(block: &Block) -> bool {
    match block {
        Block::Paragraph(p) => any_page_semantics_in_paragraph(p),
        Block::ListBlock(list) => list
            .items
            .iter()
            .any(|item| any_page_semantics_in_blocks(&item.blocks)),
        Block::TableBlock(table) => any_page_semantics_in_table(table),
        Block::ImageBlock(_) => false,
    }
}

fn any_page_semantics_in_paragraph(paragraph: &Paragraph) -> bool {
    paragraph.inlines.iter().any(any_page_semantics_in_inline)
}

fn any_page_semantics_in_inline(inline: &Inline) -> bool {
    match inline {
        Inline::PageField(_) => true,
        Inline::GeneratedBlockMarker(kind) => {
            matches!(kind, GeneratedBlockKind::TableOfContents { .. })
        }
        Inline::Run(_) | Inline::Hyperlink(_) | Inline::BookmarkAnchor(_) | Inline::NoteRef(_) => {
            false
        }
    }
}

fn any_page_semantics_in_table(table: &TableBlock) -> bool {
    table.rows.iter().any(|row| {
        row.cells
            .iter()
            .any(|cell| any_page_semantics_in_blocks(&cell.blocks))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{
        Block, DocumentStructure, GeneratedBlock, HeaderFooterSet, Inline, ListBlock, ListItem,
        ListKind, NoteKind, PageFieldRef, PageManagement, PageNumberFormat, Paragraph, Run,
        SectionPlan, TableCell, TableRow, TocOptions,
    };

    #[test]
    fn empty_document_does_not_require_numbering() {
        let doc = Document::new();
        assert!(!requires_page_numbering(&doc));
    }

    #[test]
    fn simple_document_without_page_fields_does_not_require_numbering() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello"),
        ]))]);
        assert!(!requires_page_numbering(&doc));
    }

    #[test]
    fn body_page_field_requires_numbering() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_inlines(vec![
            Inline::Run(Run::new("Page ")),
            Inline::PageField(PageFieldRef::CurrentPage {
                format: PageNumberFormat::Arabic,
            }),
        ]))]);
        assert!(requires_page_numbering(&doc));
    }

    #[test]
    fn table_cell_page_field_requires_numbering() {
        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_inlines(vec![Inline::PageField(PageFieldRef::TotalPages {
                    format: PageNumberFormat::RomanUpper,
                })]),
            )])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        assert!(requires_page_numbering(&doc));
    }

    #[test]
    fn header_footer_page_field_requires_numbering() {
        let header_para =
            Paragraph::from_inlines(vec![Inline::PageField(PageFieldRef::CurrentPage {
                format: PageNumberFormat::Arabic,
            })]);

        let structure = DocumentStructure {
            headers: HeaderFooterSet {
                default: vec![Block::Paragraph(header_para)],
                ..HeaderFooterSet::default()
            },
            footers: HeaderFooterSet::default(),
            notes: Vec::new(),
        };

        let mut doc = Document::from_blocks(vec![]);
        doc.structure = Some(structure);

        assert!(requires_page_numbering(&doc));
    }

    #[test]
    fn note_body_page_field_requires_numbering() {
        let note_para =
            Paragraph::from_inlines(vec![Inline::PageField(PageFieldRef::CurrentPage {
                format: PageNumberFormat::Arabic,
            })]);

        let structure = DocumentStructure {
            headers: HeaderFooterSet::default(),
            footers: HeaderFooterSet::default(),
            notes: vec![Note {
                id: 1,
                kind: NoteKind::Footnote,
                blocks: vec![Block::Paragraph(note_para)],
            }],
        };

        let mut doc = Document::from_blocks(vec![]);
        doc.structure = Some(structure);

        assert!(requires_page_numbering(&doc));
    }

    #[test]
    fn generated_toc_block_requires_numbering_from_page_management() {
        let mut doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Intro"),
        ]))]);

        doc.page_management = Some(PageManagement {
            sections: vec![SectionPlan {
                index: 0,
                restart_page_numbering: false,
                start_page: None,
                number_format: PageNumberFormat::Arabic,
            }],
            running_content: Default::default(),
            generated_blocks: vec![GeneratedBlock {
                insertion_index: 0,
                kind: GeneratedBlockKind::TableOfContents {
                    options: TocOptions::default(),
                },
                explicit: true,
            }],
        });

        assert!(requires_page_numbering(&doc));
    }

    #[test]
    fn inline_generated_toc_marker_requires_numbering() {
        let para = Paragraph::from_inlines(vec![Inline::GeneratedBlockMarker(
            GeneratedBlockKind::TableOfContents {
                options: TocOptions::default(),
            },
        )]);

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        assert!(requires_page_numbering(&doc));
    }

    #[test]
    fn explicit_page_management_without_fields_does_not_require_numbering() {
        let mut doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Body without fields"),
        ]))]);

        doc.page_management = Some(PageManagement {
            sections: vec![SectionPlan {
                index: 0,
                restart_page_numbering: true,
                start_page: Some(5),
                number_format: PageNumberFormat::RomanLower,
            }],
            running_content: Default::default(),
            generated_blocks: Vec::new(),
        });

        assert!(!requires_page_numbering(&doc));
    }

    #[test]
    fn lists_without_page_fields_do_not_require_numbering() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 1")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        assert!(!requires_page_numbering(&doc));
    }
}
