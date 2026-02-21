//! DOCX document writer implementation.
//!
//! This module provides the core conversion logic from rtfkit IR [`Document`]
//! to DOCX format using the `docx-rs` library.

use crate::DocxError;
use docx_rs::{
    AbstractNumbering, AlignmentType, Docx, IndentLevel, Level, LevelJc, LevelText, NumberFormat,
    Numbering, NumberingId, Numberings, Paragraph as DocxParagraph, Run as DocxRun, Start, Table,
    TableCell, TableRow, VAlignType, VMergeType, WidthType,
};
use indexmap::IndexMap;
use rtfkit_core::{
    Alignment, Block, CellMerge, CellVerticalAlign, Document, ListBlock, ListId, ListKind,
    Paragraph, Run, TableBlock, TableCell as IrTableCell, TableRow as IrTableRow,
};
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
}

impl NumberingAllocator {
    /// Creates a new empty NumberingAllocator.
    pub fn new() -> Self {
        Self {
            abstract_num_ids: IndexMap::new(),
            list_to_num: IndexMap::new(),
            next_abstract_num_id: 0,
            next_num_id: 2, // Start at 2, since docx-rs reserves 1 for default
        }
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
        let left_indent = 420 * (level_idx as i32 + 1);
        let hanging_indent = 420;

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
    let mut numbering = NumberingAllocator::new();

    // First pass: collect all list blocks (including nested table-cell lists)
    // and register them for deterministic numbering allocation.
    for block in &document.blocks {
        register_lists_in_block(block, &mut numbering);
    }

    // Second pass: convert blocks
    for block in &document.blocks {
        match block {
            Block::Paragraph(para) => {
                doc = doc.add_paragraph(convert_paragraph(para));
            }
            Block::ListBlock(list) => {
                let num_id = numbering.register_list(list);
                for item in &list.items {
                    for item_block in &item.blocks {
                        if let Block::Paragraph(para) = item_block {
                            let paragraph =
                                convert_paragraph_with_numbering(para, num_id, item.level);
                            doc = doc.add_paragraph(paragraph);
                        }
                    }
                }
            }
            Block::TableBlock(table) => {
                let docx_table = convert_table(table, &numbering);
                doc = doc.add_table(docx_table);
            }
        }
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
    }
}

/// Converts an IR paragraph to a docx-rs paragraph without numbering.
fn convert_paragraph(para: &Paragraph) -> DocxParagraph {
    convert_paragraph_with_numbering(para, 0, 0)
}

/// Converts an IR paragraph to a docx-rs paragraph with optional numbering.
fn convert_paragraph_with_numbering(para: &Paragraph, num_id: u32, level: u8) -> DocxParagraph {
    let mut p = DocxParagraph::new();

    // Add numbering properties if this is a list item
    if num_id > 0 {
        p = p.numbering(
            NumberingId::new(num_id as usize),
            IndentLevel::new(level as usize),
        );
    }

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
fn convert_table(table: &TableBlock, numbering: &NumberingAllocator) -> Table {
    let rows: Vec<TableRow> = table
        .rows
        .iter()
        .map(|row| convert_table_row(row, numbering))
        .collect();
    Table::new(rows)
}

/// Converts an IR TableRow to a docx-rs TableRow.
///
/// Handles row properties and horizontal merge normalization.
///
/// Valid horizontal continuation cells are skipped because they are
/// represented by the start cell's gridSpan. Orphan continuation cells
/// are preserved as standalone cells to avoid silent text loss.
fn convert_table_row(row: &IrTableRow, numbering: &NumberingAllocator) -> TableRow {
    let mut cells = Vec::with_capacity(row.cells.len());
    let mut expected_continuations = 0usize;

    for cell in &row.cells {
        match cell.merge {
            Some(CellMerge::HorizontalStart { span }) if span > 1 => {
                cells.push(convert_table_cell(cell, numbering));
                expected_continuations = span.saturating_sub(1) as usize;
            }
            Some(CellMerge::HorizontalStart { .. }) => {
                // Defensive: span=0/1 is not a real merge, emit as standalone.
                expected_continuations = 0;
                let mut standalone = cell.clone();
                standalone.merge = None;
                cells.push(convert_table_cell(&standalone, numbering));
            }
            Some(CellMerge::HorizontalContinue) if expected_continuations > 0 => {
                expected_continuations -= 1;
            }
            Some(CellMerge::HorizontalContinue) => {
                // Orphan continuation: preserve content rather than silently dropping it.
                let mut standalone = cell.clone();
                standalone.merge = None;
                cells.push(convert_table_cell(&standalone, numbering));
            }
            _ => {
                expected_continuations = 0;
                cells.push(convert_table_cell(cell, numbering));
            }
        }
    }

    // Note: docx-rs does not support row-level justification or left indent directly.
    // These properties (row_props.alignment, row_props.left_indent) would require
    // custom XML generation or a different DOCX library.
    // The row properties are preserved in the IR for potential future use.

    TableRow::new(cells)
}

/// Converts an IR TableCell to a docx-rs TableCell.
///
/// Handles cell content (paragraphs and lists), width mapping, merge semantics,
/// and vertical alignment.
///
/// Width is stored in twips in the IR and mapped to DXA for DOCX (1:1 ratio).
fn convert_table_cell(cell: &IrTableCell, numbering: &NumberingAllocator) -> TableCell {
    let mut docx_cell = TableCell::new();

    // Apply width if specified
    if let Some(width_twips) = cell.width_twips {
        // Ensure width is non-negative before casting to usize
        if width_twips >= 0 {
            docx_cell = docx_cell.width(width_twips as usize, WidthType::Dxa);
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

    // Convert cell content
    for block in &cell.blocks {
        match block {
            Block::Paragraph(para) => {
                docx_cell = docx_cell.add_paragraph(convert_paragraph(para));
            }
            Block::ListBlock(list) => {
                if let Some(num_id) = numbering.num_id_for(list.list_id) {
                    for item in &list.items {
                        for item_block in &item.blocks {
                            if let Block::Paragraph(para) = item_block {
                                let paragraph =
                                    convert_paragraph_with_numbering(para, num_id, item.level);
                                docx_cell = docx_cell.add_paragraph(paragraph);
                            }
                        }
                    }
                } else {
                    // Fallback for malformed IR without registered numbering.
                    for item in &list.items {
                        for item_block in &item.blocks {
                            if let Block::Paragraph(para) = item_block {
                                docx_cell = docx_cell.add_paragraph(convert_paragraph(para));
                            }
                        }
                    }
                }
            }
            Block::TableBlock(nested_table) => {
                // Support for nested tables
                docx_cell = docx_cell.add_table(convert_table(nested_table, numbering));
            }
        }
    }

    docx_cell
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{ListItem, ListKind};

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
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("    "),
        ]))]);
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
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new(""),
        ]))]);
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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

        let bytes = write_docx_to_bytes(&doc).unwrap();
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

        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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

        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_empty_table() {
        let table = TableBlock::new();

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();

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
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_merge_continue() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Below")]));
        cell.merge = Some(CellMerge::VerticalContinue);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_merge_none() {
        use rtfkit_core::{TableCell, TableRow};

        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal")]));
        cell.merge = Some(CellMerge::None);

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc).unwrap();
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
        let bytes = write_docx_to_bytes(&doc).unwrap();
        assert!(!bytes.is_empty());
    }
}
