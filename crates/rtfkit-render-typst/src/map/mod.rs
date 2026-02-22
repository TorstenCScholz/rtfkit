//! Mapping from rtfkit-core IR to Typst source code.
//!
//! This module provides functions to convert rtfkit-core `Document` types
//! to Typst markup source code that can be rendered to PDF.
//!
//! ## Design Principles
//!
//! 1. **Determinism**: Same input always produces same output
//! 2. **Semantic preservation**: Don't lose semantic content; emit warnings for dropped features
//! 3. **Typst-specific**: No generic backend abstraction; direct Typst code generation
//!
//! ## Module Structure
//!
//! - `paragraph`: Paragraph and run mapping
//! - `list`: List mapping with level preservation
//! - `table`: Table mapping with merge support

mod list;
mod paragraph;
mod table;

use rtfkit_core::{Block as IrBlock, Document};

use crate::error::WarningKind;
use crate::options::RenderOptions;

pub use list::{ListOutput, map_list};
pub use paragraph::{ParagraphOutput, map_paragraph};
pub use table::{TableOutput, map_table};

/// Structured mapping warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingWarning {
    FontSizeDropped,
    ColorDropped,
    ListMixedKindFallbackToBullet,
    ListLevelSkip { from: u8, to: u8 },
    OrphanHorizontalContinue,
    CellVerticalAlignDropped,
}

impl MappingWarning {
    /// Stable machine-readable warning code.
    pub fn code(&self) -> String {
        match self {
            Self::FontSizeDropped => "font_size_dropped".to_string(),
            Self::ColorDropped => "color_dropped".to_string(),
            Self::ListMixedKindFallbackToBullet => "list_mixed_kind_fallback_to_bullet".to_string(),
            Self::ListLevelSkip { from, to } => format!("list_level_skip_{}_to_{}", from, to),
            Self::OrphanHorizontalContinue => "orphan_horizontal_continue".to_string(),
            Self::CellVerticalAlignDropped => "cell_vertical_align_dropped".to_string(),
        }
    }

    /// Semantic class for strict-mode behavior.
    pub fn kind(&self) -> WarningKind {
        match self {
            // Mixed list kinds lose ordered-vs-bullet semantics.
            Self::ListMixedKindFallbackToBullet => WarningKind::DroppedContent,
            // Remaining current mapper degradations are partial support.
            _ => WarningKind::PartialSupport,
        }
    }
}

/// Result of mapping a document to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated during mapping.
    pub warnings: Vec<MappingWarning>,
}

/// Result of mapping a block to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated during mapping.
    pub warnings: Vec<MappingWarning>,
}

/// Map a rtfkit-core Document to Typst source code.
///
/// This is the main entry point for the mapping layer. It iterates over
/// all blocks in the source document and maps each to Typst markup,
/// then wraps the result in a valid Typst document structure.
///
/// # Arguments
///
/// * `doc` - The source document from rtfkit-core.
/// * `options` - Render options for page setup.
///
/// # Returns
///
/// A `DocumentOutput` containing the complete Typst source and any warnings.
///
/// # Determinism
///
/// This function is deterministic: the same input always produces the same output.
pub fn map_document(doc: &Document, options: &RenderOptions) -> DocumentOutput {
    let mut warnings = Vec::new();
    let mut block_sources = Vec::new();

    // Map each block
    for block in &doc.blocks {
        let block_output = map_block(block);
        if !block_output.typst_source.is_empty() {
            block_sources.push(block_output.typst_source);
        }
        warnings.extend(block_output.warnings);
    }

    // Generate document structure
    let typst_source = generate_document_source(&block_sources, options);

    DocumentOutput {
        typst_source,
        warnings,
    }
}

/// Map a single IR block to Typst source.
///
/// Returns empty source for blocks that cannot be mapped (e.g., empty content).
pub fn map_block(block: &IrBlock) -> BlockOutput {
    match block {
        IrBlock::Paragraph(para) => {
            let output = map_paragraph(para);
            BlockOutput {
                typst_source: output.typst_source,
                warnings: output.warnings,
            }
        }
        IrBlock::ListBlock(list) => {
            let output = map_list(list);
            BlockOutput {
                typst_source: output.typst_source,
                warnings: output.warnings,
            }
        }
        IrBlock::TableBlock(table) => {
            let output = map_table(table);
            BlockOutput {
                typst_source: output.typst_source,
                warnings: output.warnings,
            }
        }
    }
}

/// Generate the complete Typst document source.
fn generate_document_source(block_sources: &[String], options: &RenderOptions) -> String {
    let mut lines = Vec::new();

    // Add page setup
    lines.push(generate_page_setup(options));
    lines.push(String::new()); // Empty line after setup

    // Add content blocks
    for (i, source) in block_sources.iter().enumerate() {
        if i > 0 {
            lines.push(String::new()); // Empty line between blocks
        }
        lines.push(source.clone());
    }

    lines.join("\n")
}

/// Generate Typst page setup directives.
fn generate_page_setup(options: &RenderOptions) -> String {
    let (width_mm, height_mm) = options.page_size.dimensions_mm();
    let margins = &options.margins;

    // Convert mm to Typst units (1mm = 1mm in Typst)
    // Note: We use width and height directly instead of "paper" parameter
    // because the paper parameter requires the standard library to be available
    format!(
        "#set page(\n  width: {}mm,\n  height: {}mm,\n  margin: (top: {}mm, bottom: {}mm, left: {}mm, right: {}mm),\n)",
        width_mm, height_mm, margins.top, margins.bottom, margins.left, margins.right
    )
}

/// Convert PageSize to Typst paper name.
/// Note: Currently unused as we use explicit width/height instead of paper names.
#[allow(dead_code)]
fn page_size_to_typst(page_size: &crate::options::PageSize) -> &'static str {
    use crate::options::PageSize;
    match page_size {
        PageSize::A4 => "a4",
        PageSize::Letter => "us-letter",
        PageSize::Legal => "us-legal",
        PageSize::Custom { .. } => "a4", // Custom uses explicit width/height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{Block, ListKind as IrListKind, Paragraph, Run, TableBlock as IrTableBlock};

    #[test]
    fn test_map_empty_document() {
        let doc = Document::new();
        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        // Should have page setup even for empty document
        assert!(output.typst_source.contains("#set page("));
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_document_with_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(output.typst_source.contains("#set page("));
        assert!(output.typst_source.contains("Hello, World!"));
    }

    #[test]
    fn test_map_document_with_multiple_blocks() {
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("First paragraph")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Second paragraph")])),
        ]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(output.typst_source.contains("First paragraph"));
        assert!(output.typst_source.contains("Second paragraph"));
    }

    #[test]
    fn test_map_document_with_list() {
        use rtfkit_core::{ListBlock, ListItem};

        let mut list = ListBlock::new(1, IrListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 1")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(output.typst_source.contains("- Item 1"));
    }

    #[test]
    fn test_map_document_with_table() {
        use rtfkit_core::{TableCell, TableRow};

        let table =
            IrTableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Cell")]),
            )])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(output.typst_source.contains("#table("));
        assert!(output.typst_source.contains("[Cell]"));
    }

    #[test]
    fn test_map_document_with_mixed_content() {
        use rtfkit_core::{ListBlock, ListItem, TableCell, TableRow};

        let mut list = ListBlock::new(1, IrListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("List item")]),
        ));

        let table =
            IrTableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Cell")]),
            )])]);

        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Intro")])),
            Block::ListBlock(list),
            Block::TableBlock(table),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Outro")])),
        ]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(output.typst_source.contains("Intro"));
        assert!(output.typst_source.contains("- List item"));
        assert!(output.typst_source.contains("#table("));
        assert!(output.typst_source.contains("Outro"));
    }

    #[test]
    fn test_page_setup_a4() {
        let options = RenderOptions {
            page_size: crate::options::PageSize::A4,
            ..Default::default()
        };

        let setup = generate_page_setup(&options);

        // A4 dimensions
        assert!(setup.contains("210mm"));
        assert!(setup.contains("297mm"));
    }

    #[test]
    fn test_page_setup_letter() {
        let options = RenderOptions {
            page_size: crate::options::PageSize::Letter,
            ..Default::default()
        };

        let setup = generate_page_setup(&options);

        // Letter dimensions
        assert!(setup.contains("215.9mm"));
        assert!(setup.contains("279.4mm"));
    }

    #[test]
    fn test_page_setup_custom_margins() {
        let options = RenderOptions {
            page_size: crate::options::PageSize::A4,
            margins: crate::options::Margins {
                top: 10.0,
                bottom: 15.0,
                left: 20.0,
                right: 25.0,
            },
            ..Default::default()
        };

        let setup = generate_page_setup(&options);

        assert!(setup.contains("top: 10mm"));
        assert!(setup.contains("bottom: 15mm"));
        assert!(setup.contains("left: 20mm"));
        assert!(setup.contains("right: 25mm"));
    }

    #[test]
    fn test_determinism() {
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("First")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Second")])),
        ]);

        let options = RenderOptions::default();

        // Run multiple times to verify determinism
        let output1 = map_document(&doc, &options);
        let output2 = map_document(&doc, &options);
        let output3 = map_document(&doc, &options);

        assert_eq!(output1.typst_source, output2.typst_source);
        assert_eq!(output2.typst_source, output3.typst_source);
        assert_eq!(output1.warnings, output2.warnings);
        assert_eq!(output2.warnings, output3.warnings);
    }

    #[test]
    fn test_empty_paragraph_filtered() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![]))]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        // Should have page setup but no content
        assert!(output.typst_source.contains("#set page("));
        // The empty paragraph should not add any content
        // Just page setup + trailing newline
        let trimmed = output.typst_source.trim();
        assert!(trimmed.starts_with("#set page("));
    }
}
