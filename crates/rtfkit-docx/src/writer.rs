//! DOCX document writer implementation.
//!
//! This module provides the core conversion logic from rtfkit IR [`Document`]
//! to DOCX format using the `docx-rs` library.

use crate::{DocxError, DocxWriterOptions};
use docx_rs::{
    AbstractNumbering, AlignmentType, BorderType, Docx, Footer as DocxFooter,
    Footnote as DocxFootnote, Header as DocxHeader, HeightRule as DocxHeightRule,
    Hyperlink as DocxHyperlink, HyperlinkType, IndentLevel, Level, LevelJc, LevelText, LineSpacing,
    LineSpacingType, NumberFormat, Numbering, NumberingId, Numberings, Paragraph as DocxParagraph,
    Pic, Run as DocxRun, RunFonts, RunProperty, Shading, ShdType, Start, Table, TableAlignmentType,
    TableBorder, TableBorderPosition, TableBorders, TableCell, TableCellBorder,
    TableCellBorderPosition, TableCellBorders, TableCellMargins, TableRow, VAlignType, VMergeType,
    VertAlignType, WidthType,
};
use image::{GenericImageView, ImageFormat as RasterFormat};
use indexmap::IndexMap;
use rtfkit_core::{
    Alignment, Block, Border as IrBorder, BorderSet as IrBorderSet, BorderStyle as IrBorderStyle,
    CellMerge, CellVerticalAlign, Document, DocumentStructure, GeneratedBlockKind, HeaderFooterSet,
    Hyperlink as IrHyperlink, HyperlinkTarget, ImageBlock, Inline, ListBlock, ListId, ListKind,
    Note, NoteRef as IrNoteRef, PageFieldRef, Paragraph, RowAlignment, RowHeightRule, Run,
    Shading as IrShading, ShadingPattern, TableBlock, TableCell as IrTableCell,
    TableRow as IrTableRow, WidthUnit, resolve_effective_cell_borders,
};
use rtfkit_style_tokens::{StyleProfile, TableStripeMode, builtins::resolve_profile};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;

// =============================================================================
// Numbering Allocator
// =============================================================================

/// Allocates numbering IDs deterministically for DOCX output.
///
/// The `NumberingAllocator` tracks abstract numbering definitions and concrete
/// numbering instances, ensuring deterministic ID assignment for reproducible
/// DOCX output.
#[derive(Debug, Clone)]
pub struct NumberingAllocator {
    /// Maps (ListKind, level pattern) -> abstractNumId
    /// Uses IndexMap for deterministic iteration order
    abstract_num_ids: IndexMap<(ListKind, Vec<u8>), u32>,
    /// Maps ListId -> (numId, abstractNumId)
    /// Tracks which numId and abstractNumId each list uses
    list_to_num: IndexMap<ListId, (u32, u32)>,
    /// Next abstractNumId to assign
    next_abstract_num_id: u32,
    /// Next numId to assign (starts at 2 since 1 is reserved by docx-rs)
    next_num_id: u32,
    /// Left indent step per level in twips (default: 420 = 21pt).
    indent_step_twips: i32,
    /// Hanging indent (marker gap) in twips (default: 420 = 21pt).
    marker_gap_twips: i32,
}

impl NumberingAllocator {
    /// Creates a new empty NumberingAllocator with default indentation (21pt / 420 twips).
    pub fn new() -> Self {
        Self {
            abstract_num_ids: IndexMap::new(),
            list_to_num: IndexMap::new(),
            next_abstract_num_id: 0,
            next_num_id: 2, // Start at 2, since docx-rs reserves 1 for default
            indent_step_twips: 420,
            marker_gap_twips: 420,
        }
    }

    /// Creates a `NumberingAllocator` with indentation values from a style profile.
    pub fn with_profile(profile: &StyleProfile) -> Self {
        let mut s = Self::new();
        s.indent_step_twips = (profile.components.list.indentation_step * 20.0).round() as i32;
        s.marker_gap_twips = (profile.components.list.marker_gap * 20.0).round() as i32;
        s
    }

    /// Registers a list and returns its numId.
    ///
    /// This method ensures that:
    /// - Each unique (ListKind, levels) combination gets a unique abstractNumId
    /// - Each unique ListId gets a unique numId
    /// - The same ListId always maps to the same numId (determinism)
    pub fn register_list(&mut self, list: &ListBlock) -> u32 {
        // Check if we've already registered this list
        if let Some((num_id, _)) = self.list_to_num.get(&list.list_id) {
            return *num_id;
        }

        // Determine the levels used in this list
        let levels = self.extract_levels(list);

        // Get or create abstractNumId for this (kind, levels) combination
        let key = (list.kind, levels.clone());
        let abstract_num_id = if let Some(&id) = self.abstract_num_ids.get(&key) {
            id
        } else {
            let id = self.next_abstract_num_id;
            self.next_abstract_num_id += 1;
            self.abstract_num_ids.insert(key, id);
            id
        };

        // Assign a new numId for this list
        let num_id = self.next_num_id;
        self.next_num_id += 1;
        self.list_to_num
            .insert(list.list_id, (num_id, abstract_num_id));

        num_id
    }

    /// Returns the assigned numId for a given ListId, if registered.
    pub fn num_id_for(&self, list_id: ListId) -> Option<u32> {
        self.list_to_num.get(&list_id).map(|(num_id, _)| *num_id)
    }

    /// Extracts the unique levels used in a list.
    fn extract_levels(&self, list: &ListBlock) -> Vec<u8> {
        let mut levels: Vec<u8> = list.items.iter().map(|item| item.level).collect();
        levels.sort();
        levels.dedup();
        levels
    }

    /// Returns true if any lists have been registered.
    pub fn has_numbering(&self) -> bool {
        !self.list_to_num.is_empty()
    }

    /// Builds the Numberings structure for DOCX output.
    pub fn build_numberings(&self) -> Numberings {
        let mut numberings = Numberings::new();

        // Add abstract numbering definitions
        for ((kind, levels), abstract_num_id) in &self.abstract_num_ids {
            let abstract_num = self.build_abstract_num(*kind, levels, *abstract_num_id);
            numberings = numberings.add_abstract_numbering(abstract_num);
        }

        // Add numbering instances
        for (_list_id, (num_id, abstract_num_id)) in &self.list_to_num {
            numberings = numberings
                .add_numbering(Numbering::new(*num_id as usize, *abstract_num_id as usize));
        }

        numberings
    }

    /// Builds an AbstractNumbering definition for a list kind.
    fn build_abstract_num(
        &self,
        kind: ListKind,
        levels: &[u8],
        abstract_num_id: u32,
    ) -> AbstractNumbering {
        let mut abstract_num = AbstractNumbering::new(abstract_num_id as usize);

        // Determine max level needed
        let max_level = levels.iter().max().copied().unwrap_or(0);

        // Add level definitions for all levels up to max_level
        for level_idx in 0..=max_level {
            let level = self.build_level(kind, level_idx);
            abstract_num = abstract_num.add_level(level);
        }

        abstract_num
    }

    /// Builds a Level definition for a specific level index.
    fn build_level(&self, kind: ListKind, level_idx: u8) -> Level {
        let (format, text) = match kind {
            ListKind::Bullet => ("bullet", "•".to_string()),
            ListKind::OrderedDecimal => ("decimal", format!("%{}.", level_idx + 1)),
            ListKind::Mixed => ("decimal", format!("%{}.", level_idx + 1)),
        };

        // Calculate indentation based on level
        let left_indent = self.indent_step_twips * (level_idx as i32 + 1);
        let hanging_indent = self.marker_gap_twips;

        Level::new(
            level_idx as usize,
            Start::new(1),
            NumberFormat::new(format),
            LevelText::new(&text),
            LevelJc::new("left"),
        )
        .indent(
            Some(left_indent),
            Some(docx_rs::SpecialIndentType::Hanging(hanging_indent)),
            None,
            None,
        )
    }
}

impl Default for NumberingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Image Allocator
// =============================================================================

/// Allocates image IDs and tracks images for DOCX output.
///
/// This allocator only tracks deterministic relationship IDs. Media packaging
/// is handled natively by `docx-rs` drawing support.
#[derive(Debug, Clone)]
pub struct ImageAllocator {
    /// Next image number for deterministic relationship IDs.
    next_image_num: u32,
}

impl ImageAllocator {
    /// Creates a new empty ImageAllocator.
    pub fn new() -> Self {
        Self { next_image_num: 1 }
    }

    /// Allocate the next deterministic image ID (1-based).
    pub fn allocate_image_id(&mut self) -> u32 {
        let id = self.next_image_num;
        self.next_image_num += 1;
        id
    }
}

impl Default for ImageAllocator {
    fn default() -> Self {
        Self::new()
    }
}

type NoteLookup = BTreeMap<u32, Note>;

// =============================================================================
// Unit Conversion Utilities
// =============================================================================

/// Converts twips to English Metric Units (EMUs).
///
/// EMUs are used in DrawingML for image dimensions.
/// 1 twip = 635 EMUs (1 twip = 1/20 point, 1 point = 914400 EMUs / 72)
/// Therefore: 1 twip = 914400 / 72 / 20 = 635 EMUs
fn twips_to_emu(twips: i32) -> u32 {
    (twips.max(1) as i64 * 635).clamp(1, u32::MAX as i64) as u32
}

// =============================================================================
// Document Writing Functions
// =============================================================================

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
    let profile: Option<StyleProfile> = options
        .style_profile
        .as_ref()
        .map(|name| resolve_profile(name));

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
    // Also pre-register lists from structure (headers, footers, notes)
    if let Some(ref s) = document.structure {
        register_lists_in_header_footer_set(&s.headers, &mut numbering);
        register_lists_in_header_footer_set(&s.footers, &mut numbering);
        register_lists_in_notes(&s.notes, &mut numbering);
    }

    // Second pass: convert blocks
    // Note: docx-rs handles hyperlink relationships internally
    let mut bookmark_id: usize = 0;
    for block in &document.blocks {
        match block {
            Block::Paragraph(para) => {
                doc = doc.add_paragraph(convert_paragraph(
                    para,
                    &mut bookmark_id,
                    &numbering,
                    note_lookup,
                ));
            }
            Block::ListBlock(list) => {
                let num_id = numbering.register_list(list);
                for item in &list.items {
                    for item_block in &item.blocks {
                        match item_block {
                            Block::Paragraph(para) => {
                                let paragraph = convert_paragraph_with_numbering(
                                    para,
                                    num_id,
                                    item.level,
                                    &mut bookmark_id,
                                    &numbering,
                                    note_lookup,
                                    profile.as_ref(),
                                );
                                doc = doc.add_paragraph(paragraph);
                            }
                            Block::ImageBlock(image) => {
                                let paragraph = convert_image_block(image, &mut images)?.numbering(
                                    NumberingId::new(num_id as usize),
                                    IndentLevel::new(item.level as usize),
                                );
                                doc = doc.add_paragraph(paragraph);
                            }
                            Block::TableBlock(table) => {
                                let table = convert_table(
                                    table,
                                    &numbering,
                                    &mut images,
                                    &mut bookmark_id,
                                    note_lookup,
                                    profile.as_ref(),
                                )?;
                                doc = doc.add_table(table);
                            }
                            Block::ListBlock(_) => {}
                        }
                    }
                }
            }
            Block::TableBlock(table) => {
                let docx_table = convert_table(
                    table,
                    &numbering,
                    &mut images,
                    &mut bookmark_id,
                    note_lookup,
                    profile.as_ref(),
                )?;
                doc = doc.add_table(docx_table);
            }
            Block::ImageBlock(image) => {
                // Convert image block to a paragraph with drawing element
                let para = convert_image_block(image, &mut images)?;
                doc = doc.add_paragraph(para);
            }
        }
    }

    // Apply document structure (headers, footers)
    if let Some(ref s) = document.structure {
        doc = apply_document_structure(
            doc,
            s,
            &numbering,
            &mut images,
            note_lookup,
            profile.as_ref(),
        )?;
    }

    // Add numbering part if needed
    if numbering.has_numbering() {
        doc = doc.numberings(numbering.build_numberings());
    }

    Ok(doc.build())
}

fn register_lists_in_block(block: &Block, numbering: &mut NumberingAllocator) {
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
        Block::ImageBlock(_) => {
            // Images don't contain lists
        }
    }
}

fn build_note_lookup(structure: Option<&DocumentStructure>) -> NoteLookup {
    let mut lookup = NoteLookup::new();
    if let Some(structure) = structure {
        for note in &structure.notes {
            lookup.insert(note.id, note.clone());
        }
    }
    lookup
}

/// Converts an IR paragraph to a docx-rs paragraph without numbering.
fn convert_paragraph(
    para: &Paragraph,
    bookmark_id: &mut usize,
    numbering: &NumberingAllocator,
    note_lookup: Option<&NoteLookup>,
) -> DocxParagraph {
    convert_paragraph_with_numbering(para, 0, 0, bookmark_id, numbering, note_lookup, None)
}

/// Converts an IR paragraph to a docx-rs paragraph with optional numbering.
fn convert_paragraph_with_numbering(
    para: &Paragraph,
    num_id: u32,
    level: u8,
    bookmark_id: &mut usize,
    numbering: &NumberingAllocator,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> DocxParagraph {
    let mut p = DocxParagraph::new();

    // Add numbering properties if this is a list item
    if num_id > 0 {
        p = p.numbering(
            NumberingId::new(num_id as usize),
            IndentLevel::new(level as usize),
        );
        if let Some(pf) = profile {
            p = p.line_spacing(
                LineSpacing::new().after((pf.spacing.list_item_gap * 20.0).round() as u32),
            );
        }
    }

    // Map alignment
    p = p.align(convert_alignment(para.alignment));

    // Map paragraph shading through paragraph default run properties.
    // docx-rs does not expose w:pPr/w:shd directly, but this still emits w:shd
    // at paragraph property scope (w:pPr/w:rPr/w:shd).
    if let Some(ref shading) = para.shading
        && let Some(docx_shading) = convert_shading(shading)
    {
        p = p.run_property(RunProperty::new().shading(docx_shading));
    }

    // Map inlines
    for inline in &para.inlines {
        match inline {
            Inline::Run(run) => {
                p = p.add_run(convert_run(run));
            }
            Inline::Hyperlink(hyperlink) => {
                let docx_hyperlink = convert_hyperlink(hyperlink);
                p = p.add_hyperlink(docx_hyperlink);
            }
            Inline::BookmarkAnchor(anchor) => {
                let id = *bookmark_id;
                *bookmark_id += 1;
                p = p.add_bookmark_start(id, &anchor.name);
                p = p.add_bookmark_end(id);
            }
            Inline::NoteRef(note_ref) => {
                p = p.add_run(note_ref_to_docx_run(note_ref, numbering, note_lookup));
            }
            Inline::PageField(page_field) => {
                p = p.add_run(page_field_to_docx_run(page_field));
            }
            Inline::GeneratedBlockMarker(kind) => {
                if matches!(kind, GeneratedBlockKind::TableOfContents { .. }) {
                    p = p.add_run(DocxRun::new().add_text("[TOC]"));
                }
            }
        }
    }

    p
}

fn page_field_to_docx_run(page_field: &PageFieldRef) -> DocxRun {
    match page_field {
        PageFieldRef::CurrentPage { .. } => DocxRun::new().add_text("1"),
        PageFieldRef::TotalPages { .. } => DocxRun::new().add_text("1"),
        PageFieldRef::SectionPages { .. } => DocxRun::new().add_text("1"),
        PageFieldRef::PageRef {
            fallback_text,
            target,
            ..
        } => DocxRun::new().add_text(fallback_text.as_deref().unwrap_or(target)),
    }
}

/// Converts a NoteRef inline to a docx-rs footnote reference run.
///
/// Falls back to a superscript text run when the corresponding note body
/// is unavailable.
fn note_ref_to_docx_run(
    note_ref: &IrNoteRef,
    numbering: &NumberingAllocator,
    note_lookup: Option<&NoteLookup>,
) -> DocxRun {
    if let Some(notes) = note_lookup
        && let Some(note) = notes.get(&note_ref.id)
    {
        // docx-rs does not currently expose endnotes, so both kinds are emitted
        // as footnote references to preserve note body content.
        let footnote = note_to_docx_footnote(note, numbering);
        return DocxRun::new().add_footnote_reference(footnote);
    }

    let mut run = DocxRun::new().add_text(&note_ref.id.to_string());
    run.run_property = RunProperty::new().vert_align(VertAlignType::SuperScript);
    run
}

fn note_to_docx_footnote(note: &Note, numbering: &NumberingAllocator) -> DocxFootnote {
    let mut footnote = DocxFootnote {
        id: note.id as usize,
        content: Vec::new(),
    };
    let mut bookmark_id = 0usize;
    append_note_blocks_as_paragraphs(
        &note.blocks,
        numbering,
        &mut bookmark_id,
        &mut footnote.content,
    );
    footnote
}

fn append_note_blocks_as_paragraphs(
    blocks: &[Block],
    numbering: &NumberingAllocator,
    bookmark_id: &mut usize,
    out: &mut Vec<DocxParagraph>,
) {
    for block in blocks {
        match block {
            Block::Paragraph(para) => {
                out.push(convert_paragraph(para, bookmark_id, numbering, None));
            }
            Block::ListBlock(list) => {
                let num_id = numbering.num_id_for(list.list_id).unwrap_or(0);
                for item in &list.items {
                    for item_block in &item.blocks {
                        match item_block {
                            Block::Paragraph(para) => out.push(convert_paragraph_with_numbering(
                                para,
                                num_id,
                                item.level,
                                bookmark_id,
                                numbering,
                                None,
                                None,
                            )),
                            Block::ListBlock(nested) => {
                                append_note_blocks_as_paragraphs(
                                    &[Block::ListBlock(nested.clone())],
                                    numbering,
                                    bookmark_id,
                                    out,
                                );
                            }
                            Block::TableBlock(table) => {
                                for row in &table.rows {
                                    for cell in &row.cells {
                                        append_note_blocks_as_paragraphs(
                                            &cell.blocks,
                                            numbering,
                                            bookmark_id,
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
                        append_note_blocks_as_paragraphs(&cell.blocks, numbering, bookmark_id, out);
                    }
                }
            }
            Block::ImageBlock(_) => {}
        }
    }
}

// =============================================================================
// Document Structure (Headers / Footers / Footnotes)
// =============================================================================

/// Converts a slice of IR blocks to a DOCX `Header`.
///
/// Lists inside headers are registered with a temporary allocator and numbered
/// independently from the body, which is acceptable for header content.
fn blocks_to_docx_header(
    blocks: &[Block],
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<DocxHeader, DocxError> {
    let mut header = DocxHeader::new();
    let mut bookmark_id: usize = 0;
    for block in blocks {
        match block {
            Block::Paragraph(para) => {
                header = header.add_paragraph(convert_paragraph(
                    para,
                    &mut bookmark_id,
                    numbering,
                    note_lookup,
                ));
            }
            Block::TableBlock(table) => {
                header = header.add_table(convert_table(
                    table,
                    numbering,
                    images,
                    &mut bookmark_id,
                    note_lookup,
                    profile,
                )?);
            }
            Block::ListBlock(list) => {
                let num_id = numbering.num_id_for(list.list_id).unwrap_or(0);
                for item in &list.items {
                    for item_block in &item.blocks {
                        if let Block::Paragraph(para) = item_block {
                            let p = convert_paragraph_with_numbering(
                                para,
                                num_id,
                                item.level,
                                &mut bookmark_id,
                                numbering,
                                note_lookup,
                                profile,
                            );
                            header = header.add_paragraph(p);
                        }
                    }
                }
            }
            Block::ImageBlock(image) => {
                header = header.add_paragraph(convert_image_block(image, images)?);
            }
        }
    }
    Ok(header)
}

/// Converts a slice of IR blocks to a DOCX `Footer`.
fn blocks_to_docx_footer(
    blocks: &[Block],
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<DocxFooter, DocxError> {
    let mut footer = DocxFooter::new();
    let mut bookmark_id: usize = 0;
    for block in blocks {
        match block {
            Block::Paragraph(para) => {
                footer = footer.add_paragraph(convert_paragraph(
                    para,
                    &mut bookmark_id,
                    numbering,
                    note_lookup,
                ));
            }
            Block::TableBlock(table) => {
                footer = footer.add_table(convert_table(
                    table,
                    numbering,
                    images,
                    &mut bookmark_id,
                    note_lookup,
                    profile,
                )?);
            }
            Block::ListBlock(list) => {
                let num_id = numbering.num_id_for(list.list_id).unwrap_or(0);
                for item in &list.items {
                    for item_block in &item.blocks {
                        if let Block::Paragraph(para) = item_block {
                            let p = convert_paragraph_with_numbering(
                                para,
                                num_id,
                                item.level,
                                &mut bookmark_id,
                                numbering,
                                note_lookup,
                                profile,
                            );
                            footer = footer.add_paragraph(p);
                        }
                    }
                }
            }
            Block::ImageBlock(image) => {
                footer = footer.add_paragraph(convert_image_block(image, images)?);
            }
        }
    }
    Ok(footer)
}

/// Registers lists from a `HeaderFooterSet` for numbering pre-allocation.
fn register_lists_in_header_footer_set(set: &HeaderFooterSet, numbering: &mut NumberingAllocator) {
    for blocks in [&set.default, &set.first, &set.even] {
        for block in blocks {
            register_lists_in_block(block, numbering);
        }
    }
}

/// Registers lists from structure notes for numbering pre-allocation.
fn register_lists_in_notes(notes: &[Note], numbering: &mut NumberingAllocator) {
    for note in notes {
        for block in &note.blocks {
            register_lists_in_block(block, numbering);
        }
    }
}

/// Applies all structure headers/footers to the Docx builder.
fn apply_document_structure(
    mut doc: Docx,
    structure: &DocumentStructure,
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<Docx, DocxError> {
    // Default headers
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
    // Default footers
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

/// Converts an IR Hyperlink to a docx-rs Hyperlink.
fn convert_hyperlink(hyperlink: &IrHyperlink) -> DocxHyperlink {
    let mut docx_hyperlink = match &hyperlink.target {
        HyperlinkTarget::ExternalUrl(url) => DocxHyperlink::new(url, HyperlinkType::External),
        HyperlinkTarget::InternalBookmark(name) => DocxHyperlink::new(name, HyperlinkType::Anchor),
    };

    for run in &hyperlink.runs {
        docx_hyperlink = docx_hyperlink.add_run(convert_run(run));
    }

    docx_hyperlink
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

/// Converts IR Shading to docx-rs Shading.
///
/// Maps IR `ShadingPattern` to DOCX `w:val` attribute:
/// - Clear → "clear"
/// - Solid → "solid"
/// - HorzStripe → "horzStripe"
/// - VertStripe → "vertStripe"
/// - DiagStripe → "diagStripe"
/// - ReverseDiagStripe → "reverseDiagStripe"
/// - HorzCross → "horzCross"
/// - DiagCross → "diagCross"
/// - Percent5-90 → "pct5"-"pct90"
///
/// Emits full `w:shd` attributes:
/// - `w:val` = pattern type
/// - `w:fill` = fill_color (background)
/// - `w:color` = pattern_color (foreground)
fn convert_shading(shading: &IrShading) -> Option<Shading> {
    // Only emit shading if we have a fill color
    let fill_color = shading.fill_color.as_ref()?;
    let fill_hex = format!(
        "{:02X}{:02X}{:02X}",
        fill_color.r, fill_color.g, fill_color.b
    );

    // Get pattern type, defaulting to Solid if fill_color is present
    let pattern = shading.pattern.unwrap_or(ShadingPattern::Solid);
    let shd_type = pattern_to_shd_type(pattern);

    // Build shading with pattern and fill color
    let mut docx_shading = Shading::new().shd_type(shd_type).fill(fill_hex);

    // Add pattern color if present (foreground for patterns)
    if let Some(ref pattern_color) = shading.pattern_color {
        let pattern_hex = format!(
            "{:02X}{:02X}{:02X}",
            pattern_color.r, pattern_color.g, pattern_color.b
        );
        docx_shading = docx_shading.color(pattern_hex);
    } else {
        // Use "auto" for color when no pattern color specified
        docx_shading = docx_shading.color("auto");
    }

    Some(docx_shading)
}

/// Maps IR ShadingPattern to docx-rs ShdType.
fn pattern_to_shd_type(pattern: ShadingPattern) -> ShdType {
    match pattern {
        ShadingPattern::Clear => ShdType::Clear,
        ShadingPattern::Solid => ShdType::Solid,
        ShadingPattern::HorzStripe => ShdType::HorzStripe,
        ShadingPattern::VertStripe => ShdType::VertStripe,
        ShadingPattern::DiagStripe => ShdType::DiagStripe,
        ShadingPattern::ReverseDiagStripe => ShdType::ReverseDiagStripe,
        ShadingPattern::HorzCross => ShdType::HorzCross,
        ShadingPattern::DiagCross => ShdType::DiagCross,
        ShadingPattern::Percent5 => ShdType::Pct5,
        ShadingPattern::Percent10 => ShdType::Pct10,
        ShadingPattern::Percent20 => ShdType::Pct20,
        ShadingPattern::Percent25 => ShdType::Pct25,
        ShadingPattern::Percent30 => ShdType::Pct30,
        ShadingPattern::Percent40 => ShdType::Pct40,
        ShadingPattern::Percent50 => ShdType::Pct50,
        ShadingPattern::Percent60 => ShdType::Pct60,
        ShadingPattern::Percent70 => ShdType::Pct70,
        ShadingPattern::Percent75 => ShdType::Pct75,
        ShadingPattern::Percent80 => ShdType::Pct80,
        ShadingPattern::Percent90 => ShdType::Pct90,
    }
}

/// Converts an IR run to a docx-rs run.
///
/// Handles whitespace preservation for runs with leading/trailing spaces.
///
/// Run property order follows OOXML convention:
/// 1. Run style (if any)
/// 2. Font family (`w:rFonts`)
/// 3. Font size (`w:sz`, `w:szCs`)
/// 4. Color (`w:color`)
/// 5. Bold (`w:b`)
/// 6. Italic (`w:i`)
/// 7. Underline (`w:u`)
/// 8. Shading (`w:shd`) for background color
fn convert_run(run: &Run) -> DocxRun {
    let mut r = DocxRun::new().add_text(&run.text);

    // Apply font family (w:rFonts)
    // Set w:ascii, w:hAnsi, and w:cs attributes to the font family name
    if let Some(ref font_family) = run.font_family {
        r = r.fonts(
            RunFonts::new()
                .ascii(font_family)
                .hi_ansi(font_family)
                .cs(font_family),
        );
    }

    // Apply font size (w:sz, w:szCs)
    // Size is in half-points (24 = 12pt)
    // Clamp to OOXML spec bounds: 1-1638 half-points
    if let Some(font_size) = run.font_size {
        let half_points = (font_size * 2.0).round() as usize;
        let clamped = half_points.clamp(1, 1638);
        r = r.size(clamped);
    }

    // Apply color (w:color)
    // Color is 6-character hex RGB (no alpha)
    if let Some(ref color) = run.color {
        let hex = format!("{:02X}{:02X}{:02X}", color.r, color.g, color.b);
        r = r.color(hex);
    }

    // Apply bold
    if run.bold {
        r = r.bold();
    }

    // Apply italic
    if run.italic {
        r = r.italic();
    }

    // Apply underline
    if run.underline {
        r = r.underline("single");
    }

    // Apply strikethrough (w:strike)
    if run.strikethrough {
        r = r.strike();
    }

    // Apply caps (w:caps).
    // docx-rs does not currently expose w:smallCaps, so small caps degrades to caps.
    if run.all_caps || run.small_caps {
        r.run_property = r.run_property.caps();
    }

    // Apply background color (w:shd)
    // Use w:shd with w:val="clear" and w:fill for arbitrary RGB background
    if let Some(ref background_color) = run.background_color {
        let hex = format!(
            "{:02X}{:02X}{:02X}",
            background_color.r, background_color.g, background_color.b
        );
        r = r.shading(
            Shading::new()
                .shd_type(ShdType::Clear)
                .color("auto")
                .fill(hex),
        );
    }

    r
}

// =============================================================================
// Image Conversion Functions
// =============================================================================

/// 96 DPI pixel to EMU conversion used by OOXML drawing sizes.
const PX_TO_EMU: u32 = 9525;
const DEFAULT_IMAGE_EMU: u32 = 914_400; // 1 inch
type PreparedImage = (Vec<u8>, Option<(u32, u32)>);

/// Returns image bytes that are safe for docx-rs packaging and optional intrinsic dimensions.
///
/// docx-rs stores image parts under `.png` paths, so JPEG is normalized to PNG.
/// For PNG, bytes are preserved as-is; if decoding fails we still proceed and fall
/// back to explicit RTF dimensions or a default display size.
fn prepare_image_for_docx(image: &ImageBlock) -> Result<PreparedImage, DocxError> {
    match image.format {
        rtfkit_core::ImageFormat::Png => {
            let intrinsic = image::load_from_memory_with_format(&image.data, RasterFormat::Png)
                .ok()
                .map(|img| img.dimensions());
            Ok((image.data.clone(), intrinsic))
        }
        rtfkit_core::ImageFormat::Jpeg => {
            let dyn_image = image::load_from_memory_with_format(&image.data, RasterFormat::Jpeg)
                .map_err(|err| DocxError::ImageEmbedding {
                    reason: format!("failed to decode JPEG image: {err}"),
                })?;

            let (width_px, height_px) = dyn_image.dimensions();
            let mut png_cursor = Cursor::new(Vec::new());
            dyn_image
                .write_to(&mut png_cursor, RasterFormat::Png)
                .map_err(|err| DocxError::ImageEmbedding {
                    reason: format!("failed to encode JPEG as PNG: {err}"),
                })?;
            Ok((
                png_cursor.into_inner(),
                Some((width_px.max(1), height_px.max(1))),
            ))
        }
    }
}

fn px_to_emu(px: u32) -> u32 {
    (px.max(1) as u64 * PX_TO_EMU as u64).min(u32::MAX as u64) as u32
}

fn scale_emu(base_emu: u32, numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return base_emu.max(1);
    }
    ((base_emu as u64 * numerator.max(1) as u64) / denominator as u64).clamp(1, u32::MAX as u64)
        as u32
}

fn resolve_image_size_emu(
    image: &ImageBlock,
    intrinsic_dimensions: Option<(u32, u32)>,
) -> (u32, u32) {
    let (intrinsic_width_px, intrinsic_height_px) = intrinsic_dimensions.unwrap_or((96, 96));
    let width_from_twips = image.width_twips.filter(|w| *w > 0).map(twips_to_emu);
    let height_from_twips = image.height_twips.filter(|h| *h > 0).map(twips_to_emu);

    match (width_from_twips, height_from_twips) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => (
            width,
            scale_emu(width, intrinsic_height_px, intrinsic_width_px),
        ),
        (None, Some(height)) => (
            scale_emu(height, intrinsic_width_px, intrinsic_height_px),
            height,
        ),
        (None, None) => (
            intrinsic_dimensions
                .map(|_| px_to_emu(intrinsic_width_px))
                .unwrap_or(DEFAULT_IMAGE_EMU),
            intrinsic_dimensions
                .map(|_| px_to_emu(intrinsic_height_px))
                .unwrap_or(DEFAULT_IMAGE_EMU),
        ),
    }
}

/// Converts an IR ImageBlock to a docx-rs Paragraph containing a drawing run.
fn convert_image_block(
    image: &ImageBlock,
    images: &mut ImageAllocator,
) -> Result<DocxParagraph, DocxError> {
    let image_id = images.allocate_image_id();
    let (png_bytes, intrinsic_dimensions) = prepare_image_for_docx(image)?;
    let (intrinsic_w, intrinsic_h) = intrinsic_dimensions.unwrap_or((1, 1));
    let (width_emu, height_emu) = resolve_image_size_emu(image, intrinsic_dimensions);

    let pic = Pic::new_with_dimensions(png_bytes, intrinsic_w, intrinsic_h)
        .id(format!("rIdImage{image_id}"))
        .size(width_emu, height_emu);

    Ok(DocxParagraph::new().add_run(DocxRun::new().add_image(pic)))
}

// =============================================================================
// Table Conversion Functions
// =============================================================================

/// Converts an IR TableBlock to a docx-rs Table.
///
/// Maps the IR table structure to DOCX elements:
/// - `TableBlock` -> `w:tbl`
/// - `TableRow` -> `w:tr`
/// - `TableCell` -> `w:tc`
///
/// Cell widths are mapped from twips to DXA (1:1 ratio since both are 1/20th point).
fn convert_table(
    table: &TableBlock,
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    bookmark_id: &mut usize,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<Table, DocxError> {
    let rows: Vec<TableRow> = table
        .rows
        .iter()
        .enumerate()
        .map(|(row_idx, row)| {
            convert_table_row(
                table,
                row_idx,
                row,
                numbering,
                images,
                bookmark_id,
                note_lookup,
                profile,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut docx_table = Table::new(rows);

    // Apply table-level layout from first row's RowProps (RTF stores these per-row)
    let first_row_props = table.rows.first().and_then(|r| r.row_props.as_ref());

    // Alignment
    if let Some(rp) = first_row_props {
        match rp.alignment {
            Some(RowAlignment::Center) => {
                docx_table = docx_table.align(TableAlignmentType::Center);
            }
            Some(RowAlignment::Right) => {
                docx_table = docx_table.align(TableAlignmentType::Right);
            }
            _ => {} // Left is docx-rs default
        }
        // Left indent
        if let Some(indent) = rp.left_indent {
            docx_table = docx_table.indent(indent);
        }
        // Row-default cell padding as table-level cell margins
        if let Some(ref pad) = rp.default_padding {
            let mut margins = TableCellMargins::new();
            if let Some(t) = pad.top {
                margins = margins.margin_top(t.max(0) as usize, WidthType::Dxa);
            }
            if let Some(r) = pad.right {
                // docx-rs TableCellMargins only has margin_top and margin_left;
                // build the full margin via the shorthand if possible.
                let _ = r; // right and bottom exposed via direct cell property below
            }
            if let Some(l) = pad.left {
                margins = margins.margin_left(l.max(0) as usize, WidthType::Dxa);
            }
            // Only apply if at least one supported side was set
            if pad.top.is_some() || pad.left.is_some() {
                docx_table = docx_table.margins(margins);
            }
        }
    }

    // Preferred table width from TableProps
    if let Some(ref pw) = table.table_props.as_ref().and_then(|tp| tp.preferred_width) {
        match pw {
            WidthUnit::Auto => {
                docx_table = docx_table.width(0, WidthType::Auto);
            }
            WidthUnit::Twips(t) => {
                if *t >= 0 {
                    docx_table = docx_table.width(*t as usize, WidthType::Dxa);
                }
            }
            WidthUnit::Percent(bp) => {
                // DOCX pct type: 1/50 of a percent per unit (same basis as RTF)
                docx_table = docx_table.width(*bp as usize, WidthType::Pct);
            }
        }
    }

    // Apply profile table border defaults at table level.
    // Explicit IR borders continue to be applied on cells and take precedence.
    if let Some(p) = profile {
        docx_table = docx_table.set_borders(profile_table_borders(p));
    }

    // Apply profile cell margins when no RTF row-default padding was specified.
    if let Some(p) = profile {
        if first_row_props
            .and_then(|rp| rp.default_padding.as_ref())
            .is_none()
        {
            let pad_x = (p.spacing.table_cell_padding_x * 20.0).round() as usize;
            let pad_y = (p.spacing.table_cell_padding_y * 20.0).round() as usize;
            let margins = TableCellMargins::new()
                .margin_top(pad_y, WidthType::Dxa)
                .margin_right(pad_x, WidthType::Dxa)
                .margin_bottom(pad_y, WidthType::Dxa)
                .margin_left(pad_x, WidthType::Dxa);
            docx_table = docx_table.margins(margins);
        }
    }

    Ok(docx_table)
}

/// Converts an IR TableRow to a docx-rs TableRow.
///
/// Handles row properties and horizontal merge normalization.
///
/// Valid horizontal continuation cells are skipped because they are
/// represented by the start cell's gridSpan. Orphan continuation cells are
/// preserved as standalone cells to avoid silent text loss.
fn convert_table_row(
    table: &TableBlock,
    row_idx: usize,
    row: &IrTableRow,
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    bookmark_id: &mut usize,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<TableRow, DocxError> {
    let mut cells = Vec::with_capacity(row.cells.len());
    let mut expected_continuations = 0usize;

    for (col_idx, cell) in row.cells.iter().enumerate() {
        let effective_borders = resolve_effective_cell_borders(table, row_idx, col_idx);
        match cell.merge {
            Some(CellMerge::HorizontalStart { span }) if span > 1 => {
                cells.push(convert_table_cell(
                    cell,
                    row_idx,
                    effective_borders,
                    numbering,
                    images,
                    bookmark_id,
                    note_lookup,
                    profile,
                )?);
                expected_continuations = span.saturating_sub(1) as usize;
            }
            Some(CellMerge::HorizontalStart { .. }) => {
                // Defensive: span=0/1 is not a real merge, emit as standalone.
                expected_continuations = 0;
                let mut standalone = cell.clone();
                standalone.merge = None;
                cells.push(convert_table_cell(
                    &standalone,
                    row_idx,
                    effective_borders,
                    numbering,
                    images,
                    bookmark_id,
                    note_lookup,
                    profile,
                )?);
            }
            Some(CellMerge::HorizontalContinue) if expected_continuations > 0 => {
                expected_continuations -= 1;
            }
            Some(CellMerge::HorizontalContinue) => {
                // Orphan continuation: preserve content rather than silently dropping it.
                let mut standalone = cell.clone();
                standalone.merge = None;
                cells.push(convert_table_cell(
                    &standalone,
                    row_idx,
                    effective_borders,
                    numbering,
                    images,
                    bookmark_id,
                    note_lookup,
                    profile,
                )?);
            }
            _ => {
                expected_continuations = 0;
                cells.push(convert_table_cell(
                    cell,
                    row_idx,
                    effective_borders,
                    numbering,
                    images,
                    bookmark_id,
                    note_lookup,
                    profile,
                )?);
            }
        }
    }

    let mut docx_row = TableRow::new(cells);

    // Apply row height from RowProps
    if let Some((h, rule)) = row
        .row_props
        .as_ref()
        .and_then(|rp| rp.height_twips.zip(rp.height_rule))
    {
        docx_row = docx_row.row_height(h as f32);
        match rule {
            RowHeightRule::AtLeast => {
                docx_row = docx_row.height_rule(DocxHeightRule::AtLeast);
            }
            RowHeightRule::Exact => {
                docx_row = docx_row.height_rule(DocxHeightRule::Exact);
            }
        }
    }

    Ok(docx_row)
}

/// Maps an IR `BorderStyle` to a docx-rs `BorderType`.
fn border_style_to_docx(style: IrBorderStyle) -> BorderType {
    match style {
        IrBorderStyle::Single => BorderType::Single,
        IrBorderStyle::Double => BorderType::Double,
        IrBorderStyle::Dotted => BorderType::Dotted,
        IrBorderStyle::Dashed => BorderType::Dashed,
        IrBorderStyle::None => BorderType::Nil,
    }
}

/// Build a single `TableCellBorder` from an IR `Border`.
fn convert_border(border: &IrBorder, pos: TableCellBorderPosition) -> TableCellBorder {
    let bt = border_style_to_docx(border.style);
    // docx-rs size unit is 1/8 pt; IR width is half-points, so multiply by 4.
    let size = border
        .width_half_pts
        .map(|hp| hp.saturating_mul(4))
        .unwrap_or(8) as usize;
    let color = border
        .color
        .as_ref()
        .map(|c| format!("{:02X}{:02X}{:02X}", c.r, c.g, c.b))
        .unwrap_or_else(|| "000000".to_string());
    TableCellBorder::new(pos)
        .border_type(bt)
        .size(size)
        .color(color)
}

/// Build a `TableCellBorders` from an IR `BorderSet`.
fn convert_border_set(borders: &IrBorderSet) -> TableCellBorders {
    // Use `with_empty()` so only explicitly-set sides are emitted.
    let mut docx_borders = TableCellBorders::with_empty();
    if let Some(ref b) = borders.top {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Top));
    }
    if let Some(ref b) = borders.left {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Left));
    }
    if let Some(ref b) = borders.bottom {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Bottom));
    }
    if let Some(ref b) = borders.right {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Right));
    }
    if let Some(ref b) = borders.inside_h {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::InsideH));
    }
    if let Some(ref b) = borders.inside_v {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::InsideV));
    }
    docx_borders
}

fn profile_table_borders(profile: &StyleProfile) -> TableBorders {
    let size = (profile.components.table.border_width * 8.0)
        .round()
        .max(1.0) as usize;
    let color = profile.colors.border_default.without_hash().to_string();
    TableBorders::with_empty()
        .set(
            TableBorder::new(TableBorderPosition::Top)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Left)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Bottom)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Right)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::InsideH)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::InsideV)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
}

/// Converts an IR TableCell to a docx-rs TableCell.
///
/// Handles cell content (paragraphs and lists), width mapping, merge semantics,
/// vertical alignment, shading, and borders.
///
/// Width is stored in twips in the IR and mapped to DXA for DOCX (1:1 ratio).
fn convert_table_cell(
    cell: &IrTableCell,
    row_idx: usize,
    effective_borders: Option<IrBorderSet>,
    numbering: &NumberingAllocator,
    images: &mut ImageAllocator,
    bookmark_id: &mut usize,
    note_lookup: Option<&NoteLookup>,
    profile: Option<&StyleProfile>,
) -> Result<TableCell, DocxError> {
    let mut docx_cell = TableCell::new();

    // Apply preferred cell width if present, otherwise fall back to cellx-derived width
    if let Some(ref pw) = cell.preferred_width {
        match pw {
            WidthUnit::Auto => {
                docx_cell = docx_cell.width(0, WidthType::Auto);
            }
            WidthUnit::Twips(t) => {
                if *t >= 0 {
                    docx_cell = docx_cell.width(*t as usize, WidthType::Dxa);
                }
            }
            WidthUnit::Percent(bp) => {
                docx_cell = docx_cell.width(*bp as usize, WidthType::Pct);
            }
        }
    } else if let Some(width_twips) = cell.width_twips {
        // Ensure width is non-negative before casting to usize
        if width_twips >= 0 {
            docx_cell = docx_cell.width(width_twips as usize, WidthType::Dxa);
        }
    }

    // Apply cell-level padding (overrides table default margins)
    if let Some(ref pad) = cell.padding {
        if let Some(t) = pad.top {
            docx_cell.property = docx_cell
                .property
                .margin_top(t.max(0) as usize, WidthType::Dxa);
        }
        if let Some(r) = pad.right {
            docx_cell.property = docx_cell
                .property
                .margin_right(r.max(0) as usize, WidthType::Dxa);
        }
        if let Some(b) = pad.bottom {
            docx_cell.property = docx_cell
                .property
                .margin_bottom(b.max(0) as usize, WidthType::Dxa);
        }
        if let Some(l) = pad.left {
            docx_cell.property = docx_cell
                .property
                .margin_left(l.max(0) as usize, WidthType::Dxa);
        }
    }

    // Handle merge semantics
    if let Some(merge) = &cell.merge {
        match merge {
            CellMerge::HorizontalStart { span } => {
                // Set gridSpan for horizontal merge
                docx_cell = docx_cell.grid_span(*span as usize);
            }
            CellMerge::HorizontalContinue => {
                // This cell is merged with previous - should not appear as separate cell
                // These cells are filtered out in convert_table_row()
            }
            CellMerge::VerticalStart => {
                // Set vMerge="restart"
                docx_cell = docx_cell.vertical_merge(VMergeType::Restart);
            }
            CellMerge::VerticalContinue => {
                // Set vMerge="continue"
                docx_cell = docx_cell.vertical_merge(VMergeType::Continue);
            }
            CellMerge::None => {}
        }
    }

    // Handle vertical alignment
    if let Some(v_align) = cell.v_align {
        match v_align {
            CellVerticalAlign::Top => {
                docx_cell = docx_cell.vertical_align(VAlignType::Top);
            }
            CellVerticalAlign::Center => {
                docx_cell = docx_cell.vertical_align(VAlignType::Center);
            }
            CellVerticalAlign::Bottom => {
                docx_cell = docx_cell.vertical_align(VAlignType::Bottom);
            }
        }
    }

    // Apply cell shading: IR-source shading takes priority; fall back to profile row striping.
    if let Some(ref shading) = cell.shading
        && let Some(docx_shading) = convert_shading(shading)
    {
        docx_cell = docx_cell.shading(docx_shading);
    } else if let Some(p) = profile
        && p.components.table.stripe_mode == TableStripeMode::AlternateRows
        && row_idx % 2 == 1
    {
        // Odd rows (0-indexed: 1, 3, 5, …) receive the stripe color.
        let fill = p.colors.surface_table_stripe.without_hash().to_string();
        docx_cell = docx_cell.shading(
            Shading::new()
                .shd_type(ShdType::Clear)
                .color("auto")
                .fill(fill),
        );
    }

    // Apply cell borders if present
    if let Some(ref borders) = effective_borders {
        docx_cell = docx_cell.set_borders(convert_border_set(borders));
    }

    // Convert cell content
    for block in &cell.blocks {
        match block {
            Block::Paragraph(para) => {
                docx_cell = docx_cell.add_paragraph(convert_paragraph(
                    para,
                    bookmark_id,
                    numbering,
                    note_lookup,
                ));
            }
            Block::ListBlock(list) => {
                if let Some(num_id) = numbering.num_id_for(list.list_id) {
                    for item in &list.items {
                        for item_block in &item.blocks {
                            match item_block {
                                Block::Paragraph(para) => {
                                    let paragraph = convert_paragraph_with_numbering(
                                        para,
                                        num_id,
                                        item.level,
                                        bookmark_id,
                                        numbering,
                                        note_lookup,
                                        profile,
                                    );
                                    docx_cell = docx_cell.add_paragraph(paragraph);
                                }
                                Block::ImageBlock(image) => {
                                    let paragraph = convert_image_block(image, images)?.numbering(
                                        NumberingId::new(num_id as usize),
                                        IndentLevel::new(item.level as usize),
                                    );
                                    docx_cell = docx_cell.add_paragraph(paragraph);
                                }
                                Block::TableBlock(nested_table) => {
                                    docx_cell = docx_cell.add_table(convert_table(
                                        nested_table,
                                        numbering,
                                        images,
                                        bookmark_id,
                                        note_lookup,
                                        profile,
                                    )?);
                                }
                                Block::ListBlock(_) => {}
                            }
                        }
                    }
                } else {
                    // Fallback for malformed IR without registered numbering.
                    for item in &list.items {
                        for item_block in &item.blocks {
                            match item_block {
                                Block::Paragraph(para) => {
                                    docx_cell = docx_cell.add_paragraph(convert_paragraph(
                                        para,
                                        bookmark_id,
                                        numbering,
                                        note_lookup,
                                    ));
                                }
                                Block::ImageBlock(image) => {
                                    docx_cell = docx_cell
                                        .add_paragraph(convert_image_block(image, images)?);
                                }
                                Block::TableBlock(nested_table) => {
                                    docx_cell = docx_cell.add_table(convert_table(
                                        nested_table,
                                        numbering,
                                        images,
                                        bookmark_id,
                                        note_lookup,
                                        profile,
                                    )?);
                                }
                                Block::ListBlock(_) => {}
                            }
                        }
                    }
                }
            }
            Block::TableBlock(nested_table) => {
                // Support for nested tables
                docx_cell = docx_cell.add_table(convert_table(
                    nested_table,
                    numbering,
                    images,
                    bookmark_id,
                    note_lookup,
                    profile,
                )?);
            }
            Block::ImageBlock(image) => {
                docx_cell = docx_cell.add_paragraph(convert_image_block(image, images)?);
            }
        }
    }

    Ok(docx_cell)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{
        Block, Document, DocumentStructure, HeaderFooterSet, ImageBlock, ImageFormat, Inline,
        ListBlock, ListItem, ListKind, Note, NoteKind, NoteRef, Paragraph, Run, TableBlock,
        TableCell, TableRow,
    };
    use rtfkit_style_tokens::StyleProfileName;
    use std::io::Read;

    fn zip_entry_string(bytes: &[u8], entry_name: &str) -> String {
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        let mut entry = archive
            .by_name(entry_name)
            .unwrap_or_else(|_| panic!("missing ZIP entry: {entry_name}"));
        let mut xml = String::new();
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

    // =========================================================================
    // NumberingAllocator Tests
    // =========================================================================

    #[test]
    fn test_numbering_allocator_new() {
        let allocator = NumberingAllocator::new();
        assert!(!allocator.has_numbering());
        assert!(allocator.abstract_num_ids.is_empty());
        assert!(allocator.list_to_num.is_empty());
    }

    #[test]
    fn test_numbering_allocator_single_list() {
        let mut allocator = NumberingAllocator::new();

        let list = ListBlock::new(1, ListKind::Bullet);
        let num_id = allocator.register_list(&list);

        assert!(allocator.has_numbering());
        assert_eq!(num_id, 2); // First numId (starts at 2)
        assert_eq!(allocator.abstract_num_ids.len(), 1);
    }

    #[test]
    fn test_numbering_allocator_same_list_twice() {
        let mut allocator = NumberingAllocator::new();

        let list = ListBlock::new(1, ListKind::Bullet);
        let num_id1 = allocator.register_list(&list);
        let num_id2 = allocator.register_list(&list);

        // Same list should return same numId
        assert_eq!(num_id1, num_id2);
        assert_eq!(allocator.list_to_num.len(), 1);
    }

    #[test]
    fn test_numbering_allocator_different_lists_same_kind() {
        let mut allocator = NumberingAllocator::new();

        let list1 = ListBlock::new(1, ListKind::Bullet);
        let list2 = ListBlock::new(2, ListKind::Bullet);

        let num_id1 = allocator.register_list(&list1);
        let num_id2 = allocator.register_list(&list2);

        // Different lists should get different numIds
        assert_ne!(num_id1, num_id2);
        // But same abstractNumId (same kind)
        let (_, abs1) = allocator.list_to_num.get(&1).unwrap();
        let (_, abs2) = allocator.list_to_num.get(&2).unwrap();
        assert_eq!(abs1, abs2);
    }

    #[test]
    fn test_numbering_allocator_different_kinds() {
        let mut allocator = NumberingAllocator::new();

        let bullet_list = ListBlock::new(1, ListKind::Bullet);
        let decimal_list = ListBlock::new(2, ListKind::OrderedDecimal);

        allocator.register_list(&bullet_list);
        allocator.register_list(&decimal_list);

        // Different kinds should get different abstractNumIds
        assert_eq!(allocator.abstract_num_ids.len(), 2);
    }

    #[test]
    fn test_numbering_allocator_determinism() {
        // Test that the same input always produces the same output
        let mut allocator1 = NumberingAllocator::new();
        let mut allocator2 = NumberingAllocator::new();

        let list1 = ListBlock::new(1, ListKind::Bullet);
        let list2 = ListBlock::new(2, ListKind::OrderedDecimal);
        let list3 = ListBlock::new(3, ListKind::Bullet);

        let num_id1_1 = allocator1.register_list(&list1);
        let num_id1_2 = allocator1.register_list(&list2);
        let num_id1_3 = allocator1.register_list(&list3);

        let num_id2_1 = allocator2.register_list(&list1);
        let num_id2_2 = allocator2.register_list(&list2);
        let num_id2_3 = allocator2.register_list(&list3);

        assert_eq!(num_id1_1, num_id2_1);
        assert_eq!(num_id1_2, num_id2_2);
        assert_eq!(num_id1_3, num_id2_3);
    }

    #[test]
    fn test_numbering_allocator_build_numberings() {
        let mut allocator = NumberingAllocator::new();

        let list = ListBlock::new(1, ListKind::Bullet);
        allocator.register_list(&list);

        let numberings = allocator.build_numberings();

        // Should have one abstract numbering and one numbering instance
        assert_eq!(numberings.abstract_nums.len(), 1);
        assert_eq!(numberings.numberings.len(), 1);
    }

    #[test]
    fn test_numbering_allocator_levels_extraction() {
        let mut allocator = NumberingAllocator::new();

        let mut list = ListBlock::new(1, ListKind::OrderedDecimal);
        list.add_item(ListItem::new(0));
        list.add_item(ListItem::new(1));
        list.add_item(ListItem::new(0)); // Duplicate level

        allocator.register_list(&list);

        // Should have one abstract num with levels 0 and 1
        let key = (ListKind::OrderedDecimal, vec![0, 1]);
        assert!(allocator.abstract_num_ids.contains_key(&key));
    }

    // =========================================================================
    // Paragraph/Run Mapping Tests
    // =========================================================================

    #[test]
    fn test_simple_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
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

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_multiple_runs_in_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, "),
            Run::new("World!"),
        ]))]);

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Alignment Mapping Tests
    // =========================================================================

    #[test]
    fn test_alignment_left() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Left;
        para.inlines.push(Inline::Run(Run::new("Left aligned")));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_alignment_center() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Center;
        para.inlines.push(Inline::Run(Run::new("Center aligned")));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_alignment_right() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Right;
        para.inlines.push(Inline::Run(Run::new("Right aligned")));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_alignment_justify() {
        let mut para = Paragraph::new();
        para.alignment = Alignment::Justify;
        para.inlines.push(Inline::Run(Run::new("Justified text")));

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
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
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_italic_run() {
        let mut run = Run::new("Italic text");
        run.italic = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_underline_run() {
        let mut run = Run::new("Underlined text");
        run.underline = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_bold_italic_run() {
        let mut run = Run::new("Bold and italic");
        run.bold = true;
        run.italic = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_bold_italic_underline_run() {
        let mut run = Run::new("All styles");
        run.bold = true;
        run.italic = true;
        run.underline = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
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
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
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
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_trailing_space() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("trailing spaces  "),
        ]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_multiple_spaces() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("multiple   spaces   inside"),
        ]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_only_spaces() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("    "),
        ]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Empty Document Handling Tests
    // =========================================================================

    #[test]
    fn test_empty_document() {
        let doc = Document::new();
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's a valid ZIP
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

        write_docx(&doc, &file_path, &DocxWriterOptions::default()).unwrap();
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
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_special_characters() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Special: <>&\"'"),
        ]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // List Block Tests
    // =========================================================================

    #[test]
    fn test_bullet_list() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 2")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 3")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's a valid ZIP
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
        // Should have numbering.xml
        assert!(archive.by_name("word/numbering.xml").is_ok());
    }

    #[test]
    fn test_ordered_list() {
        let mut list = ListBlock::new(1, ListKind::OrderedDecimal);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("First")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Second")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Third")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());

        // Verify numbering.xml exists
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/numbering.xml").is_ok());
    }

    #[test]
    fn test_nested_list() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Level 0")]),
        ));
        list.add_item(ListItem::from_paragraph(
            1,
            Paragraph::from_runs(vec![Run::new("Level 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            2,
            Paragraph::from_runs(vec![Run::new("Level 2")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Back to 0")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_mixed_paragraphs_and_lists() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("List item")]),
        ));

        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Before list")])),
            Block::ListBlock(list),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("After list")])),
        ]);

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_multiple_lists() {
        let mut list1 = ListBlock::new(1, ListKind::Bullet);
        list1.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Bullet 1")]),
        ));
        list1.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Bullet 2")]),
        ));

        let mut list2 = ListBlock::new(2, ListKind::OrderedDecimal);
        list2.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Number 1")]),
        ));
        list2.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Number 2")]),
        ));

        let doc = Document::from_blocks(vec![
            Block::ListBlock(list1),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Between lists")])),
            Block::ListBlock(list2),
        ]);

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());

        // Verify numbering.xml exists
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/numbering.xml").is_ok());
    }

    #[test]
    fn test_list_with_formatted_text() {
        let mut bold_run = Run::new("bold");
        bold_run.bold = true;
        let mut italic_run = Run::new("italic");
        italic_run.italic = true;

        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![bold_run, italic_run]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_numbering_xml_content() {
        let mut allocator = NumberingAllocator::new();

        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::new(0));

        allocator.register_list(&list);

        let numberings = allocator.build_numberings();

        // Check that we have the expected structure
        assert_eq!(numberings.abstract_nums.len(), 1);
        assert_eq!(numberings.numberings.len(), 1);

        // Check abstract numbering ID
        let abstract_num = &numberings.abstract_nums[0];
        assert_eq!(abstract_num.id, 0);

        // Check numbering instance
        let numbering = &numberings.numberings[0];
        assert_eq!(numbering.id, 2); // numId starts at 2
        assert_eq!(numbering.abstract_num_id, 0);
    }

    // =========================================================================
    // Table Block Tests
    // =========================================================================

    #[test]
    fn test_simple_table() {
        use rtfkit_core::{TableCell, TableRow};

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 1")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 2")])),
        ])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's a valid ZIP
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn test_table_multiple_rows() {
        use rtfkit_core::{TableCell, TableRow};

        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R1C1")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R1C2")])),
            ]),
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R2C1")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R2C2")])),
            ]),
        ]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_with_width() {
        use rtfkit_core::{TableCell, TableRow};

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph_with_width(
                Paragraph::from_runs(vec![Run::new("Cell with width")]),
                2880, // 2 inches in twips (1440 twips = 1 inch)
            ),
        ])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_with_formatted_content() {
        use rtfkit_core::{TableCell, TableRow};

        let mut bold_run = Run::new("bold");
        bold_run.bold = true;

        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![bold_run]),
            )])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_mixed_with_paragraphs() {
        use rtfkit_core::{TableCell, TableRow};

        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Table cell")]),
            )])]);

        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Before table")])),
            Block::TableBlock(table),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("After table")])),
        ]);

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_empty_table() {
        let table = TableBlock::new();

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_empty_cell() {
        use rtfkit_core::{TableCell, TableRow};

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::new(),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Content")])),
        ])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_nested_table() {
        use rtfkit_core::{TableCell, TableRow};

        // Create inner table
        let inner_table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Inner cell")]),
            )])]);

        // Create outer table with nested table
        let outer_table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Outer cell")])),
            TableCell::from_blocks(vec![Block::TableBlock(inner_table)], None),
        ])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(outer_table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Table Merge Tests
    // =========================================================================

    #[test]
    fn test_table_cell_horizontal_merge_start() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 2 });

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_horizontal_merge_continue_filtered() {
        use rtfkit_core::{TableCell, TableRow};

        // Create a row with a start cell and continuation cell
        let mut start_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Start")]));
        start_cell.merge = Some(CellMerge::HorizontalStart { span: 2 });

        let mut continue_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Continue")]));
        continue_cell.merge = Some(CellMerge::HorizontalContinue);

        let row = TableRow::from_cells(vec![start_cell, continue_cell]);

        let table = TableBlock::from_rows(vec![row]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's a valid DOCX
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn test_orphan_horizontal_continue_preserves_text() {
        use rtfkit_core::{TableCell, TableRow};
        use std::io::Read;

        // Middle cell is an orphan continuation (no horizontal start before it).
        let start = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Alpha")]));

        let mut orphan = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bravo")]));
        orphan.merge = Some(CellMerge::HorizontalContinue);

        let end = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Charlie")]));

        let row = TableRow::from_cells(vec![start, orphan, end]);
        let table = TableBlock::from_rows(vec![row]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .unwrap();

        assert!(document_xml.contains("Alpha"));
        assert!(document_xml.contains("Bravo"));
        assert!(document_xml.contains("Charlie"));
    }

    #[test]
    fn test_table_cell_vertical_merge_start() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top")]));
        cell.merge = Some(CellMerge::VerticalStart);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_merge_continue() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Below")]));
        cell.merge = Some(CellMerge::VerticalContinue);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_align_top() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top aligned")]));
        cell.v_align = Some(CellVerticalAlign::Top);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_align_center() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Center aligned")]));
        cell.v_align = Some(CellVerticalAlign::Center);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_align_bottom() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bottom aligned")]));
        cell.v_align = Some(CellVerticalAlign::Bottom);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_merge_none() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal")]));
        cell.merge = Some(CellMerge::None);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_combined_merge_and_alignment() {
        use rtfkit_core::{TableCell, TableRow};

        // Cell with both merge and vertical alignment
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Combined")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 3 });
        cell.v_align = Some(CellVerticalAlign::Center);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_hyperlink_emits_w_hyperlink_in_document_xml() {
        let para = Paragraph::from_inlines(vec![
            Inline::Run(Run::new("Visit ")),
            Inline::Hyperlink(IrHyperlink {
                target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
                runs: vec![Run::new("Example")],
            }),
        ]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:hyperlink"));
        assert!(document_xml.contains("r:id=\"rIdHyperlink"));
        assert!(document_xml.contains("Example"));
    }

    #[test]
    fn test_hyperlink_emits_relationship_entry() {
        let para = Paragraph::from_inlines(vec![Inline::Hyperlink(IrHyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![Run::new("Example")],
        })]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let rels_xml = zip_entry_string(&bytes, "word/_rels/document.xml.rels");
        assert!(rels_xml.contains(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
        ));
        assert!(rels_xml.contains("Target=\"https://example.com\""));
        assert!(rels_xml.contains("TargetMode=\"External\""));
    }

    #[test]
    fn test_multiple_hyperlinks_emit_multiple_targets() {
        let para = Paragraph::from_inlines(vec![
            Inline::Hyperlink(IrHyperlink {
                target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
                runs: vec![Run::new("Example")],
            }),
            Inline::Run(Run::new(" and ")),
            Inline::Hyperlink(IrHyperlink {
                target: HyperlinkTarget::ExternalUrl("https://docs.example.com".to_string()),
                runs: vec![Run::new("Docs")],
            }),
        ]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let rels_xml = zip_entry_string(&bytes, "word/_rels/document.xml.rels");
        assert!(rels_xml.contains("Target=\"https://example.com\""));
        assert!(rels_xml.contains("Target=\"https://docs.example.com\""));
    }

    #[test]
    fn test_hyperlink_preserves_run_formatting() {
        let mut bold = Run::new("Bold");
        bold.bold = true;
        let mut italic = Run::new("Italic");
        italic.italic = true;

        let para = Paragraph::from_inlines(vec![Inline::Hyperlink(IrHyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![bold, italic],
        })]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:hyperlink"));
        assert!(document_xml.contains("Bold"));
        assert!(document_xml.contains("Italic"));
        assert!(document_xml.contains("<w:b"));
        assert!(document_xml.contains("<w:i"));
    }

    // =========================================================================
    // Font Family, Size, and Color Tests
    // =========================================================================

    #[test]
    fn test_run_with_font_family_only() {
        let mut run = Run::new("Font family text");
        run.font_family = Some("Arial".to_string());

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"w:ascii="Arial""#));
        assert!(document_xml.contains(r#"w:hAnsi="Arial""#));
        assert!(document_xml.contains(r#"w:cs="Arial""#));
    }

    #[test]
    fn test_run_with_font_size_only() {
        let mut run = Run::new("Font size text");
        run.font_size = Some(12.0); // 12pt = 24 half-points

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:sz w:val="24" />"#));
        assert!(document_xml.contains(r#"<w:szCs w:val="24" />"#));
    }

    #[test]
    fn test_run_with_color_only() {
        let mut run = Run::new("Colored text");
        run.color = Some(rtfkit_core::Color::new(255, 0, 0)); // Red

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:color w:val="FF0000" />"#));
    }

    #[test]
    fn test_run_with_strikethrough_emits_w_strike() {
        let mut run = Run::new("Struck text");
        run.strikethrough = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:strike"));
    }

    #[test]
    fn test_run_with_all_caps_emits_w_caps() {
        let mut run = Run::new("All caps");
        run.all_caps = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:caps"));
    }

    #[test]
    fn test_run_with_small_caps_falls_back_to_w_caps() {
        let mut run = Run::new("Small caps");
        run.small_caps = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:caps"));
    }

    #[test]
    fn test_run_with_all_font_properties() {
        let mut run = Run::new("Styled text");
        run.font_family = Some("Helvetica".to_string());
        run.font_size = Some(14.0); // 14pt = 28 half-points
        run.color = Some(rtfkit_core::Color::new(0, 128, 255)); // Blue

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Font family
        assert!(document_xml.contains(r#"w:ascii="Helvetica""#));
        assert!(document_xml.contains(r#"w:hAnsi="Helvetica""#));
        assert!(document_xml.contains(r#"w:cs="Helvetica""#));
        // Font size
        assert!(document_xml.contains(r#"<w:sz w:val="28" />"#));
        assert!(document_xml.contains(r#"<w:szCs w:val="28" />"#));
        // Color
        assert!(document_xml.contains(r#"<w:color w:val="0080FF" />"#));
    }

    #[test]
    fn test_run_with_combined_formatting() {
        let mut run = Run::new("Bold colored font text");
        run.bold = true;
        run.font_family = Some("Times New Roman".to_string());
        run.font_size = Some(16.0); // 16pt = 32 half-points
        run.color = Some(rtfkit_core::Color::new(0, 255, 0)); // Green

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Font family
        assert!(document_xml.contains(r#"w:ascii="Times New Roman""#));
        // Font size
        assert!(document_xml.contains(r#"<w:sz w:val="32" />"#));
        // Color
        assert!(document_xml.contains(r#"<w:color w:val="00FF00" />"#));
        // Bold
        assert!(document_xml.contains("<w:b"));
    }

    #[test]
    fn test_hyperlink_with_font_color_styling() {
        let mut styled_run = Run::new("Styled Link");
        styled_run.font_family = Some("Verdana".to_string());
        styled_run.font_size = Some(10.0); // 10pt = 20 half-points
        styled_run.color = Some(rtfkit_core::Color::new(0, 0, 255)); // Blue
        styled_run.underline = true;

        let para = Paragraph::from_inlines(vec![Inline::Hyperlink(IrHyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![styled_run],
        })]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Verify hyperlink is present
        assert!(document_xml.contains("<w:hyperlink"));
        // Verify font family
        assert!(document_xml.contains(r#"w:ascii="Verdana""#));
        // Verify font size
        assert!(document_xml.contains(r#"<w:sz w:val="20" />"#));
        // Verify color
        assert!(document_xml.contains(r#"<w:color w:val="0000FF" />"#));
        // Verify underline
        assert!(document_xml.contains(r#"<w:u w:val="single""#));
    }

    #[test]
    fn test_font_size_clamping_min() {
        let mut run = Run::new("Tiny text");
        run.font_size = Some(0.1); // Very small, should clamp to 1 half-point

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should be clamped to 1
        assert!(document_xml.contains(r#"<w:sz w:val="1" />"#));
    }

    #[test]
    fn test_font_size_clamping_max() {
        let mut run = Run::new("Huge text");
        run.font_size = Some(1000.0); // Very large, should clamp to 1638 half-points

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should be clamped to 1638
        assert!(document_xml.contains(r#"<w:sz w:val="1638" />"#));
    }

    #[test]
    fn test_font_size_rounding() {
        let mut run = Run::new("Rounded text");
        run.font_size = Some(12.4); // Should round to 25 half-points (12.5 * 2 = 25)

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // 12.4 * 2 = 24.8, rounds to 25
        assert!(document_xml.contains(r#"<w:sz w:val="25" />"#));
    }

    // =========================================================================
    // Background Color (Shading) Tests
    // =========================================================================

    #[test]
    fn test_paragraph_with_shading() {
        let mut para = Paragraph::from_runs(vec![Run::new("Shaded paragraph")]);
        para.shading = Some(rtfkit_core::Shading::solid(rtfkit_core::Color::new(
            255, 255, 0,
        ))); // Yellow

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:pPr>"));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="solid""#));
        assert!(document_xml.contains(r#"w:fill="FFFF00""#));
    }

    #[test]
    fn test_paragraph_with_patterned_shading() {
        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(rtfkit_core::Color::new(255, 255, 255)); // White
        shading.pattern_color = Some(rtfkit_core::Color::new(0, 0, 0)); // Black
        shading.pattern = Some(ShadingPattern::Percent25);

        let mut para = Paragraph::from_runs(vec![Run::new("Patterned paragraph")]);
        para.shading = Some(shading);

        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="pct25""#));
        assert!(document_xml.contains(r#"w:fill="FFFFFF""#));
        assert!(document_xml.contains(r#"w:color="000000""#));
    }

    #[test]
    fn test_run_with_background_color() {
        let mut run = Run::new("Highlighted text");
        run.background_color = Some(rtfkit_core::Color::new(255, 255, 0)); // Yellow

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain w:shd with w:fill attribute
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="clear""#));
        assert!(document_xml.contains(r#"w:color="auto""#));
        assert!(document_xml.contains(r#"w:fill="FFFF00""#));
    }

    #[test]
    fn test_run_without_background_color_no_shd() {
        let run = Run::new("Normal text");

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should NOT contain w:shd element
        assert!(!document_xml.contains("<w:shd"));
    }

    #[test]
    fn test_run_with_foreground_and_background_color() {
        let mut run = Run::new("Colored text on colored background");
        run.color = Some(rtfkit_core::Color::new(255, 0, 0)); // Red foreground
        run.background_color = Some(rtfkit_core::Color::new(0, 0, 255)); // Blue background

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain both w:color and w:shd
        assert!(document_xml.contains(r#"<w:color w:val="FF0000" />"#));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="0000FF""#));
    }

    #[test]
    fn test_hyperlink_with_background_color() {
        let mut run = Run::new("Highlighted link");
        run.background_color = Some(rtfkit_core::Color::new(0, 255, 0)); // Green background
        run.underline = true;

        let para = Paragraph::from_inlines(vec![Inline::Hyperlink(IrHyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![run],
        })]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Verify hyperlink is present
        assert!(document_xml.contains("<w:hyperlink"));
        // Verify background color is applied
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="00FF00""#));
        // Verify underline
        assert!(document_xml.contains(r#"<w:u w:val="single""#));
    }

    #[test]
    fn test_background_color_with_all_formatting() {
        let mut run = Run::new("Fully formatted");
        run.bold = true;
        run.italic = true;
        run.underline = true;
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(14.0);
        run.color = Some(rtfkit_core::Color::new(128, 0, 128)); // Purple foreground
        run.background_color = Some(rtfkit_core::Color::new(255, 192, 203)); // Pink background

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Verify all formatting is present
        assert!(document_xml.contains("<w:b"));
        assert!(document_xml.contains("<w:i"));
        assert!(document_xml.contains(r#"<w:u w:val="single""#));
        assert!(document_xml.contains(r#"w:ascii="Arial""#));
        assert!(document_xml.contains(r#"<w:sz w:val="28" />"#));
        assert!(document_xml.contains(r#"<w:color w:val="800080" />"#));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="FFC0CB""#));
    }

    // =========================================================================
    // Table Cell Shading Tests
    // =========================================================================

    #[test]
    fn test_table_cell_with_shading() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Shaded cell")]));
        cell.shading = Some(rtfkit_core::Shading::solid(rtfkit_core::Color::new(
            255, 0, 0,
        ))); // Red

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain w:shd in cell properties
        assert!(document_xml.contains("<w:tcPr>"));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="FF0000""#));
    }

    #[test]
    fn test_table_cell_without_shading_no_shd() {
        use rtfkit_core::{TableCell, TableRow};

        let cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal cell")]));

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should NOT contain w:shd in cell properties
        assert!(!document_xml.contains("<w:shd"));
    }

    #[test]
    fn test_table_cell_shading_with_merge() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged and shaded")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 2 });
        cell.shading = Some(rtfkit_core::Shading::solid(rtfkit_core::Color::new(
            0, 0, 255,
        ))); // Blue

        let mut cont_cell = TableCell::new();
        cont_cell.merge = Some(CellMerge::HorizontalContinue);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell, cont_cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain both gridSpan and shading
        assert!(document_xml.contains(r#"<w:gridSpan w:val="2""#));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="0000FF""#));
    }

    #[test]
    fn test_table_cell_shading_with_vertical_align() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Aligned and shaded")]));
        cell.v_align = Some(CellVerticalAlign::Center);
        cell.shading = Some(rtfkit_core::Shading::solid(rtfkit_core::Color::new(
            128, 128, 128,
        ))); // Gray

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain both vertical alignment and shading
        assert!(document_xml.contains(r#"<w:vAlign w:val="center""#));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="808080""#));
    }

    // =========================================================================
    // Shading Pattern Tests
    // =========================================================================

    #[test]
    fn test_table_cell_shading_with_percent_pattern() {
        use rtfkit_core::{TableCell, TableRow};

        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(rtfkit_core::Color::new(255, 255, 255)); // White background
        shading.pattern_color = Some(rtfkit_core::Color::new(0, 0, 0)); // Black foreground
        shading.pattern = Some(ShadingPattern::Percent25);

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("25% pattern")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain w:shd with pct25 pattern
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="pct25""#));
        assert!(document_xml.contains(r#"w:fill="FFFFFF""#));
        assert!(document_xml.contains(r#"w:color="000000""#));
    }

    #[test]
    fn test_table_cell_shading_with_horz_stripe_pattern() {
        use rtfkit_core::{TableCell, TableRow};

        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(rtfkit_core::Color::new(200, 200, 200)); // Light gray background
        shading.pattern_color = Some(rtfkit_core::Color::new(100, 100, 100)); // Dark gray foreground
        shading.pattern = Some(ShadingPattern::HorzStripe);

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Horizontal stripes")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain w:shd with horzStripe pattern
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="horzStripe""#));
        assert!(document_xml.contains(r#"w:fill="C8C8C8""#));
        assert!(document_xml.contains(r#"w:color="646464""#));
    }

    #[test]
    fn test_table_cell_shading_with_diag_cross_pattern() {
        use rtfkit_core::{TableCell, TableRow};

        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(rtfkit_core::Color::new(255, 255, 0)); // Yellow background
        shading.pattern_color = Some(rtfkit_core::Color::new(255, 0, 0)); // Red foreground
        shading.pattern = Some(ShadingPattern::DiagCross);

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Diagonal cross")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain w:shd with diagCross pattern
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="diagCross""#));
        assert!(document_xml.contains(r#"w:fill="FFFF00""#));
        assert!(document_xml.contains(r#"w:color="FF0000""#));
    }

    #[test]
    fn test_table_cell_shading_solid_without_pattern_color() {
        use rtfkit_core::{TableCell, TableRow};

        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(rtfkit_core::Color::new(0, 128, 0)); // Green
        shading.pattern = Some(ShadingPattern::Solid);
        // No pattern_color set

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Solid green")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain w:shd with solid pattern and auto color
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="solid""#));
        assert!(document_xml.contains(r#"w:fill="008000""#));
        assert!(document_xml.contains(r#"w:color="auto""#));
    }

    #[test]
    fn test_table_cell_shading_clear_pattern() {
        use rtfkit_core::{TableCell, TableRow};

        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(rtfkit_core::Color::new(200, 200, 255)); // Light blue
        shading.pattern = Some(ShadingPattern::Clear);

        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Clear pattern")]));
        cell.shading = Some(shading);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();

        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        // Should contain w:shd with clear pattern
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="clear""#));
        assert!(document_xml.contains(r#"w:fill="C8C8FF""#));
    }

    #[test]
    fn test_pattern_to_shd_type_all_patterns() {
        // Test all pattern mappings
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Clear),
            ShdType::Clear
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Solid),
            ShdType::Solid
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::HorzStripe),
            ShdType::HorzStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::VertStripe),
            ShdType::VertStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::DiagStripe),
            ShdType::DiagStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::ReverseDiagStripe),
            ShdType::ReverseDiagStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::HorzCross),
            ShdType::HorzCross
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::DiagCross),
            ShdType::DiagCross
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent5),
            ShdType::Pct5
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent10),
            ShdType::Pct10
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent20),
            ShdType::Pct20
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent25),
            ShdType::Pct25
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent30),
            ShdType::Pct30
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent40),
            ShdType::Pct40
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent50),
            ShdType::Pct50
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent60),
            ShdType::Pct60
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent70),
            ShdType::Pct70
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent75),
            ShdType::Pct75
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent80),
            ShdType::Pct80
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent90),
            ShdType::Pct90
        ));
    }

    // =========================================================================
    // Border conversion tests
    // =========================================================================

    #[test]
    fn border_style_single_maps_to_single() {
        assert!(matches!(
            border_style_to_docx(IrBorderStyle::Single),
            BorderType::Single
        ));
    }

    #[test]
    fn border_style_none_maps_to_nil() {
        assert!(matches!(
            border_style_to_docx(IrBorderStyle::None),
            BorderType::Nil
        ));
    }

    #[test]
    fn border_style_double_maps_to_double() {
        assert!(matches!(
            border_style_to_docx(IrBorderStyle::Double),
            BorderType::Double
        ));
    }

    #[test]
    fn convert_border_color_is_uppercase_hex() {
        use rtfkit_core::Color;
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: Some(4),
            color: Some(Color::new(255, 128, 0)),
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.color, "FF8000");
    }

    #[test]
    fn convert_border_no_color_defaults_to_black() {
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: Some(4),
            color: None,
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.color, "000000");
    }

    #[test]
    fn convert_border_size_maps_half_points_to_eighth_points() {
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: Some(4), // 2pt
            color: None,
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.size, 16); // 2pt = 16 eighth-points
    }

    #[test]
    fn convert_border_default_size_is_one_point() {
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: None,
            color: None,
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.size, 8); // 1pt default
    }

    #[test]
    fn convert_border_set_top_left_only() {
        let borders = IrBorderSet {
            top: Some(IrBorder {
                style: IrBorderStyle::Single,
                width_half_pts: Some(4),
                color: None,
            }),
            left: Some(IrBorder {
                style: IrBorderStyle::Dashed,
                width_half_pts: Some(8),
                color: None,
            }),
            ..Default::default()
        };
        let docx_borders = convert_border_set(&borders);
        // We can't easily inspect individual sides without docx-rs internals,
        // but we verify the call succeeds and the struct is built
        let _ = docx_borders;
    }

    #[test]
    fn cell_with_borders_builds_docx_without_error() {
        use rtfkit_core::{Border, BorderSet, Paragraph, Run, TableBlock, TableCell, TableRow};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("bordered")]));
        cell.borders = Some(BorderSet {
            top: Some(Border {
                style: IrBorderStyle::Single,
                width_half_pts: Some(4),
                color: None,
            }),
            right: Some(Border {
                style: IrBorderStyle::Double,
                width_half_pts: Some(8),
                color: None,
            }),
            ..Default::default()
        });

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = rtfkit_core::Document {
            blocks: vec![rtfkit_core::Block::TableBlock(table)],
            structure: None,
            page_management: None,
        };

        // Should not panic
        let result = super::write_docx_to_bytes(&doc, &DocxWriterOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn row_border_fallback_emits_tc_borders() {
        use rtfkit_core::{
            Border, BorderSet, Paragraph, RowProps, Run, TableBlock, TableCell, TableRow,
        };

        let row = TableRow {
            cells: vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
            ],
            row_props: Some(RowProps {
                borders: Some(BorderSet {
                    top: Some(Border {
                        style: IrBorderStyle::Single,
                        width_half_pts: Some(4),
                        color: None,
                    }),
                    bottom: Some(Border {
                        style: IrBorderStyle::Single,
                        width_half_pts: Some(4),
                        color: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        let table = TableBlock::from_rows(vec![row]);
        let doc = rtfkit_core::Document {
            blocks: vec![rtfkit_core::Block::TableBlock(table)],
            structure: None,
            page_management: None,
        };
        let bytes = super::write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");

        assert!(document_xml.contains("<w:tcBorders"));
        assert!(document_xml.contains("w:top"));
        assert!(document_xml.contains("w:bottom"));
        assert!(document_xml.contains("w:sz=\"16\""));
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

        let bytes = super::write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
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

        let bytes = super::write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let header_xml = zip_entry_string(&bytes, "word/header1.xml");
        assert!(header_xml.contains("<w:drawing>"));
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
        assert!(
            document_xml.contains(r#"w:top w:val="single" w:sz="4" w:space="0" w:color="D1D5DB""#)
        );
    }

    #[test]
    fn style_profile_report_applies_list_item_gap_on_numbered_paragraphs() {
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
}
