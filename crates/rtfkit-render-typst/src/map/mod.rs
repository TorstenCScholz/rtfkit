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
//! - `image`: Image mapping with in-memory assets and virtual paths

mod image;
mod list;
mod paragraph;
mod structure;
mod table;

use std::collections::BTreeMap;
use std::collections::HashSet;

use rtfkit_core::{Block as IrBlock, Document, GeneratedBlockKind, Inline, NoteKind};
use rtfkit_style_tokens::{StyleProfile, StyleProfileName, builtins, serialize::to_typst_preamble};

use crate::error::WarningKind;
use crate::options::{Margins, RenderOptions};

use image::map_image_block_with_assets;
pub use image::{ImageOutput, map_image_block};
use list::map_list_with_assets;
pub use list::{ListOutput, map_list};
use paragraph::{ParagraphMapContext, map_paragraph_with_context};
pub use paragraph::{ParagraphOutput, map_paragraph};
use table::map_table_with_assets;
pub use table::{TableOutput, map_table};

/// Structured mapping warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingWarning {
    ListMixedKindFallbackToBullet,
    ListLevelSkip {
        from: u8,
        to: u8,
    },
    OrphanHorizontalContinue,
    CellVerticalAlignDropped,
    /// Pattern was degraded to flat fill (Typst doesn't support patterns)
    PatternDegraded {
        /// Context where the pattern was found (e.g., "paragraph shading", "cell shading")
        context: String,
        /// The pattern that was degraded
        pattern: String,
    },
    /// Image payload bytes are malformed for the declared PNG format.
    MalformedPngImagePayload,
    /// Image payload bytes are malformed for the declared JPEG format.
    MalformedJpegImagePayload,
    /// A feature has partial support in Typst (best-effort mapping, no content loss).
    PartialSupport {
        /// The feature name (e.g., "exact row height").
        feature: String,
        /// Human-readable reason for the degradation.
        reason: String,
    },
    /// A page reference target could not be resolved to a known label.
    UnresolvedPageReference {
        /// Bookmark/label target name.
        target: String,
    },
}

impl MappingWarning {
    /// Stable machine-readable warning code.
    pub fn code(&self) -> String {
        match self {
            Self::ListMixedKindFallbackToBullet => "list_mixed_kind_fallback_to_bullet".to_string(),
            Self::ListLevelSkip { from, to } => format!("list_level_skip_{}_to_{}", from, to),
            Self::OrphanHorizontalContinue => "orphan_horizontal_continue".to_string(),
            Self::CellVerticalAlignDropped => "cell_vertical_align_dropped".to_string(),
            Self::PatternDegraded { context, .. } => {
                format!("pattern_degraded_{}", context.replace(' ', "_"))
            }
            Self::MalformedPngImagePayload => "Dropped malformed PNG image payload".to_string(),
            Self::MalformedJpegImagePayload => "Dropped malformed JPEG image payload".to_string(),
            Self::PartialSupport { feature, .. } => {
                format!("partial_support_{}", feature.replace(' ', "_"))
            }
            Self::UnresolvedPageReference { target } => {
                format!("unresolved_page_reference_{}", target.replace(' ', "_"))
            }
        }
    }

    /// Semantic class for strict-mode behavior.
    pub fn kind(&self) -> WarningKind {
        match self {
            // Mixed list kinds lose ordered-vs-bullet semantics.
            Self::ListMixedKindFallbackToBullet => WarningKind::DroppedContent,
            Self::MalformedPngImagePayload | Self::MalformedJpegImagePayload => {
                WarningKind::DroppedContent
            }
            // Pattern degradation is partial support (not dropped content).
            Self::PatternDegraded { .. } => WarningKind::PartialSupport,
            // Explicit partial support variant.
            Self::PartialSupport { .. } => WarningKind::PartialSupport,
            // Unresolved refs preserve visible fallback text.
            Self::UnresolvedPageReference { .. } => WarningKind::PartialSupport,
            // Remaining current mapper degradations are partial support.
            _ => WarningKind::PartialSupport,
        }
    }
}

/// Deterministic in-memory asset bundle used by Typst rendering.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypstAssetBundle {
    /// Virtual path -> raw file bytes.
    pub files: BTreeMap<String, Vec<u8>>,
}

impl TypstAssetBundle {
    /// Returns bytes for the given virtual path if present.
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }
}

/// Internal deterministic image asset allocator.
#[derive(Debug, Default)]
pub(crate) struct TypstAssetAllocator {
    next_image_num: u32,
    bundle: TypstAssetBundle,
}

impl TypstAssetAllocator {
    fn new() -> Self {
        Self {
            next_image_num: 1,
            bundle: TypstAssetBundle::default(),
        }
    }

    pub(crate) fn allocate_image_path_and_store(
        &mut self,
        extension: &str,
        bytes: &[u8],
    ) -> String {
        let index = self.next_image_num;
        self.next_image_num += 1;
        let path = format!("assets/image-{index:06}.{extension}");
        self.bundle.files.insert(path.clone(), bytes.to_vec());
        path
    }

    fn into_bundle(self) -> TypstAssetBundle {
        self.bundle
    }
}

/// Result of mapping a document to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated during mapping.
    pub warnings: Vec<MappingWarning>,
    /// Image/file assets referenced by typst_source.
    pub assets: TypstAssetBundle,
}

/// Result of mapping a block to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated during mapping.
    pub warnings: Vec<MappingWarning>,
}

/// Resolve a style profile name to an actual StyleProfile.
///
/// This function maps the profile name to the corresponding built-in profile.
/// For custom profiles, it falls back to the Report profile (MVP behavior).
pub fn resolve_style_profile(name: &StyleProfileName) -> StyleProfile {
    builtins::resolve_profile(name)
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
    let mut assets = TypstAssetAllocator::new();
    let note_bodies = build_note_body_map(doc, &mut assets, &mut warnings);
    let known_bookmarks = collect_bookmark_labels(doc);
    let enable_heading_inference = doc
        .page_management
        .as_ref()
        .is_some_and(|pm| !pm.generated_blocks.is_empty());
    let body_context = ParagraphMapContext {
        note_bodies: Some(&note_bodies),
        known_bookmarks: Some(&known_bookmarks),
        enable_heading_inference,
    };
    let running_context = ParagraphMapContext {
        note_bodies: Some(&note_bodies),
        known_bookmarks: Some(&known_bookmarks),
        enable_heading_inference: false,
    };

    // Map document structure (headers/footers) to a page setup override
    let structure_page_setup = if let Some(ref s) = doc.structure {
        structure::map_structure_page_setup(s, &mut assets, &mut warnings, &running_context)
    } else {
        String::new()
    };

    let mut generated_by_index: BTreeMap<usize, Vec<GeneratedBlockKind>> = BTreeMap::new();
    if let Some(pm) = &doc.page_management {
        for generated in &pm.generated_blocks {
            generated_by_index
                .entry(generated.insertion_index)
                .or_default()
                .push(generated.kind.clone());
        }
    }

    // Map generated blocks and document blocks in index order.
    for idx in 0..=doc.blocks.len() {
        if let Some(generated_blocks) = generated_by_index.get(&idx) {
            for generated in generated_blocks {
                let generated_source = map_generated_block_kind(generated, &mut warnings);
                if !generated_source.is_empty() {
                    block_sources.push(generated_source);
                }
            }
        }
        if let Some(block) = doc.blocks.get(idx) {
            let block_output = map_block_with_context(block, &mut assets, &body_context);
            if !block_output.typst_source.is_empty() {
                block_sources.push(block_output.typst_source);
            }
            warnings.extend(block_output.warnings);
        }
    }

    // Generate document structure
    let typst_source = generate_document_source(&block_sources, options, &structure_page_setup);

    DocumentOutput {
        typst_source,
        warnings,
        assets: assets.into_bundle(),
    }
}

/// Map a single IR block to Typst source.
///
/// Returns empty source for blocks that cannot be mapped (e.g., empty content).
pub(crate) fn map_block(block: &IrBlock, assets: &mut TypstAssetAllocator) -> BlockOutput {
    map_block_with_context(block, assets, &ParagraphMapContext::default())
}

pub(crate) fn map_block_with_context(
    block: &IrBlock,
    assets: &mut TypstAssetAllocator,
    paragraph_context: &ParagraphMapContext<'_>,
) -> BlockOutput {
    match block {
        IrBlock::Paragraph(para) => {
            let output = map_paragraph_with_context(para, paragraph_context);
            BlockOutput {
                typst_source: output.typst_source,
                warnings: output.warnings,
            }
        }
        IrBlock::ListBlock(list) => {
            let output = map_list_with_assets(list, assets);
            BlockOutput {
                typst_source: output.typst_source,
                warnings: output.warnings,
            }
        }
        IrBlock::TableBlock(table) => {
            let output = map_table_with_assets(table, assets);
            BlockOutput {
                typst_source: output.typst_source,
                warnings: output.warnings,
            }
        }
        IrBlock::ImageBlock(image) => {
            let output = map_image_block_with_assets(image, assets);
            BlockOutput {
                typst_source: output.typst_source,
                warnings: output.warnings,
            }
        }
    }
}

fn map_generated_block_kind(
    kind: &GeneratedBlockKind,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    match kind {
        GeneratedBlockKind::TableOfContents { options } => {
            if !options.hyperlinks {
                warnings.push(MappingWarning::PartialSupport {
                    feature: "toc_hyperlinks_switch".into(),
                    reason: "Typst outline hyperlink toggling is not configurable; default behavior used"
                        .into(),
                });
            }
            if let Some((_, end)) = options.levels {
                format!("#outline(depth: {end})")
            } else {
                "#outline()".to_string()
            }
        }
    }
}

fn build_note_body_map(
    doc: &Document,
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> BTreeMap<String, String> {
    let mut note_bodies = BTreeMap::new();
    let Some(structure) = doc.structure.as_ref() else {
        return note_bodies;
    };

    for note in &structure.notes {
        let mut parts = Vec::new();
        for block in &note.blocks {
            let output = map_block(block, assets);
            if !output.typst_source.is_empty() {
                parts.push(output.typst_source);
            }
            warnings.extend(output.warnings);
        }
        if !parts.is_empty() {
            let key = match note.kind {
                NoteKind::Footnote => format!("footnote:{}", note.id),
                NoteKind::Endnote => format!("endnote:{}", note.id),
            };
            note_bodies.insert(key, parts.join(" "));
        }
    }

    note_bodies
}

fn collect_bookmark_labels(doc: &Document) -> HashSet<String> {
    fn collect_from_blocks(blocks: &[IrBlock], labels: &mut HashSet<String>) {
        for block in blocks {
            match block {
                IrBlock::Paragraph(paragraph) => {
                    for inline in &paragraph.inlines {
                        if let Inline::BookmarkAnchor(anchor) = inline {
                            labels.insert(sanitize_label(&anchor.name));
                        }
                    }
                }
                IrBlock::ListBlock(list) => {
                    for item in &list.items {
                        collect_from_blocks(&item.blocks, labels);
                    }
                }
                IrBlock::TableBlock(table) => {
                    for row in &table.rows {
                        for cell in &row.cells {
                            collect_from_blocks(&cell.blocks, labels);
                        }
                    }
                }
                IrBlock::ImageBlock(_) => {}
            }
        }
    }

    fn sanitize_label(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    }

    let mut labels = HashSet::new();
    collect_from_blocks(&doc.blocks, &mut labels);
    if let Some(structure) = &doc.structure {
        collect_from_blocks(&structure.headers.default, &mut labels);
        collect_from_blocks(&structure.headers.first, &mut labels);
        collect_from_blocks(&structure.headers.even, &mut labels);
        collect_from_blocks(&structure.footers.default, &mut labels);
        collect_from_blocks(&structure.footers.first, &mut labels);
        collect_from_blocks(&structure.footers.even, &mut labels);
        for note in &structure.notes {
            collect_from_blocks(&note.blocks, &mut labels);
        }
    }
    labels
}

/// Generate the complete Typst document source.
fn generate_document_source(
    block_sources: &[String],
    options: &RenderOptions,
    structure_page_setup: &str,
) -> String {
    let mut lines = Vec::new();

    // Add style preamble from profile
    let profile = resolve_style_profile(&options.style_profile);
    let preamble = to_typst_preamble(&profile);
    lines.push(preamble);

    // Apply page geometry in one place to avoid option/profile drift.
    let margins = effective_margins(options, &profile);
    lines.push(generate_page_setup(options, &margins));

    // Append header/footer page setup override if present
    if !structure_page_setup.is_empty() {
        lines.push(structure_page_setup.to_string());
    }

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

/// Generate Typst page size setup directives.
///
/// Note: Margins are set via the style preamble. This function only sets
/// the page width and height.
fn generate_page_setup(options: &RenderOptions, margins: &Margins) -> String {
    let (width_mm, height_mm) = options.page_size.dimensions_mm();
    format!(
        "#set page(\n  width: {}mm,\n  height: {}mm,\n  margin: (top: {}mm, bottom: {}mm, left: {}mm, right: {}mm),\n  numbering: \"1\",\n)",
        width_mm, height_mm, margins.top, margins.bottom, margins.left, margins.right
    )
}

fn effective_margins(options: &RenderOptions, profile: &StyleProfile) -> Margins {
    if options.margins == Margins::default() {
        Margins {
            top: profile.layout.page_margin_top_mm,
            bottom: profile.layout.page_margin_bottom_mm,
            left: profile.layout.page_margin_left_mm,
            right: profile.layout.page_margin_right_mm,
        }
    } else {
        options.margins
    }
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
    use rtfkit_core::{
        Block, BookmarkAnchor, DocumentStructure, GeneratedBlock, GeneratedBlockKind,
        HeaderFooterSet, Inline, ListKind as IrListKind, Note, NoteKind, NoteRef, PageFieldRef,
        PageManagement, Paragraph, Run, RunningContentPlan, SectionPlan,
        TableBlock as IrTableBlock, TocOptions,
    };

    fn valid_png_data() -> Vec<u8> {
        let mut bytes = Vec::new();
        let rgba = [255_u8, 0, 0, 255];
        let encoder = ::image::codecs::png::PngEncoder::new(&mut bytes);
        ::image::ImageEncoder::write_image(encoder, &rgba, 1, 1, ::image::ColorType::Rgba8.into())
            .unwrap();
        bytes
    }

    fn valid_jpeg_data() -> Vec<u8> {
        let mut bytes = Vec::new();
        let rgb = [255_u8, 0, 0];
        let mut encoder = ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85);
        encoder
            .encode(&rgb, 1, 1, ::image::ColorType::Rgb8.into())
            .unwrap();
        bytes
    }

    #[test]
    fn test_map_empty_document() {
        let doc = Document::new();
        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        // Should have style preamble
        assert!(output.typst_source.contains("// rtfkit style profile:"));
        // Should have page setup
        assert!(output.typst_source.contains("#set page("));
        assert!(output.warnings.is_empty());
        assert!(output.assets.files.is_empty());
    }

    #[test]
    fn test_map_document_with_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        // Should have style preamble
        assert!(
            output
                .typst_source
                .contains("// rtfkit style profile: report")
        );
        assert!(output.typst_source.contains("#set page("));
        assert!(output.typst_source.contains("Hello, World!"));
    }

    #[test]
    fn test_map_document_renders_page_fields_dynamically() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_inlines(vec![
            Inline::Run(Run::new("Page ")),
            Inline::PageField(PageFieldRef::CurrentPage {
                format: rtfkit_core::PageNumberFormat::Arabic,
            }),
            Inline::Run(Run::new(" of ")),
            Inline::PageField(PageFieldRef::TotalPages {
                format: rtfkit_core::PageNumberFormat::Arabic,
            }),
        ]))]);

        let output = map_document(&doc, &RenderOptions::default());
        assert!(output.typst_source.contains("counter(page).get().at(0)"));
        assert!(output.typst_source.contains("counter(page).final().at(0)"));
    }

    #[test]
    fn test_map_document_inserts_generated_toc() {
        let mut doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Intro")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Body")])),
        ]);
        doc.page_management = Some(PageManagement {
            sections: vec![SectionPlan {
                index: 0,
                restart_page_numbering: false,
                start_page: None,
                number_format: rtfkit_core::PageNumberFormat::Arabic,
            }],
            running_content: RunningContentPlan::default(),
            generated_blocks: vec![GeneratedBlock {
                insertion_index: 1,
                kind: GeneratedBlockKind::TableOfContents {
                    options: TocOptions {
                        levels: Some((1, 2)),
                        hyperlinks: true,
                    },
                },
                explicit: true,
            }],
        });

        let output = map_document(&doc, &RenderOptions::default());
        assert!(output.typst_source.contains("#outline(depth: 2)"));
    }

    #[test]
    fn test_map_document_resolves_pageref_when_bookmark_exists() {
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_inlines(vec![
                Inline::BookmarkAnchor(BookmarkAnchor {
                    name: "sec_program_delivery".to_string(),
                }),
                Inline::Run(Run::new("Program Delivery")),
            ])),
            Block::Paragraph(Paragraph::from_inlines(vec![Inline::PageField(
                PageFieldRef::PageRef {
                    target: "sec_program_delivery".to_string(),
                    format: rtfkit_core::PageNumberFormat::Arabic,
                    fallback_text: Some("7".to_string()),
                },
            )])),
        ]);

        let output = map_document(&doc, &RenderOptions::default());
        assert!(
            output
                .typst_source
                .contains("#ref(<sec_program_delivery>, form: \"page\")")
        );
        assert!(
            !output
                .warnings
                .iter()
                .any(|w| matches!(w, MappingWarning::UnresolvedPageReference { .. }))
        );
    }

    #[test]
    fn test_map_document_pageref_fallback_when_missing() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_inlines(vec![
            Inline::PageField(PageFieldRef::PageRef {
                target: "missing_anchor".to_string(),
                format: rtfkit_core::PageNumberFormat::Arabic,
                fallback_text: Some("11".to_string()),
            }),
        ]))]);

        let output = map_document(&doc, &RenderOptions::default());
        assert!(output.typst_source.contains("11"));
        assert!(
            output.warnings.iter().any(
                |w| matches!(w, MappingWarning::UnresolvedPageReference { target } if target == "missing_anchor")
            )
        );
    }

    #[test]
    fn test_map_document_footnote_body_inlined() {
        let mut doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_inlines(vec![
            Inline::Run(Run::new("Body")),
            Inline::NoteRef(NoteRef {
                id: 1,
                kind: NoteKind::Footnote,
            }),
        ]))]);
        doc.structure = Some(DocumentStructure {
            headers: HeaderFooterSet::default(),
            footers: HeaderFooterSet::default(),
            notes: vec![Note {
                id: 1,
                kind: NoteKind::Footnote,
                blocks: vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new(
                    "Footnote body",
                )]))],
            }],
        });

        let output = map_document(&doc, &RenderOptions::default());
        assert!(output.typst_source.contains("#footnote[Footnote body]"));
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
    fn test_map_document_with_image() {
        use rtfkit_core::{ImageBlock, ImageFormat};

        let image = ImageBlock::with_dimensions(
            ImageFormat::Png,
            valid_png_data(),
            1440, // 1 inch
            720,  // 0.5 inch
        );

        let doc = Document::from_blocks(vec![Block::ImageBlock(image)]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(output.typst_source.contains("#image("));
        assert!(output.typst_source.contains("assets/image-000001.png"));
        assert!(output.typst_source.contains("width: 1.00in"));
        assert!(output.typst_source.contains("height: 0.50in"));
        assert_eq!(output.assets.files.len(), 1);
        assert!(output.assets.files.contains_key("assets/image-000001.png"));
    }

    #[test]
    fn test_map_document_with_image_without_dimensions() {
        use rtfkit_core::{ImageBlock, ImageFormat};

        let image = ImageBlock::new(ImageFormat::Jpeg, valid_jpeg_data());

        let doc = Document::from_blocks(vec![Block::ImageBlock(image)]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(output.typst_source.contains("#image("));
        assert!(output.typst_source.contains("assets/image-000001.jpg"));
        // The image function should not have width/height parameters
        // (but the page setup will have width/height for page dimensions)
        // Check that the image line doesn't contain width/height params
        let image_line = output
            .typst_source
            .lines()
            .find(|line| line.contains("#image("))
            .expect("Should have an image line");
        assert!(!image_line.contains("width:"));
        assert!(!image_line.contains("height:"));
        assert_eq!(output.assets.files.len(), 1);
    }

    #[test]
    fn test_map_document_with_malformed_image_emits_dropped_warning() {
        use rtfkit_core::{ImageBlock, ImageFormat};

        let image = ImageBlock::new(ImageFormat::Png, vec![0x89, 0x50, 0x4E, 0x47]);
        let doc = Document::from_blocks(vec![Block::ImageBlock(image)]);
        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        assert!(!output.typst_source.contains("#image("));
        assert!(output.assets.files.is_empty());
        assert!(
            output
                .warnings
                .contains(&MappingWarning::MalformedPngImagePayload)
        );
    }

    #[test]
    fn test_page_setup_a4() {
        let options = RenderOptions {
            page_size: crate::options::PageSize::A4,
            ..Default::default()
        };

        let setup = generate_page_setup(&options, &Margins::default());

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

        let setup = generate_page_setup(&options, &Margins::default());

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

        let setup = generate_page_setup(&options, &options.margins);

        assert!(setup.contains("top: 10mm"));
        assert!(setup.contains("bottom: 15mm"));
        assert!(setup.contains("left: 20mm"));
        assert!(setup.contains("right: 25mm"));
    }

    #[test]
    fn test_effective_margins_uses_profile_defaults() {
        let options = RenderOptions {
            style_profile: StyleProfileName::Compact,
            ..Default::default()
        };
        let profile = resolve_style_profile(&options.style_profile);
        let margins = effective_margins(&options, &profile);

        assert_eq!(margins.top, profile.layout.page_margin_top_mm);
        assert_eq!(margins.left, profile.layout.page_margin_left_mm);
    }

    #[test]
    fn test_effective_margins_respects_explicit_override() {
        let options = RenderOptions {
            margins: Margins {
                top: 11.0,
                bottom: 12.0,
                left: 13.0,
                right: 14.0,
            },
            ..Default::default()
        };
        let profile = resolve_style_profile(&options.style_profile);
        let margins = effective_margins(&options, &profile);

        assert_eq!(margins.top, 11.0);
        assert_eq!(margins.bottom, 12.0);
        assert_eq!(margins.left, 13.0);
        assert_eq!(margins.right, 14.0);
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

        // Should have style preamble and page setup but no content
        assert!(output.typst_source.contains("// rtfkit style profile:"));
        assert!(output.typst_source.contains("#set page("));
    }

    #[test]
    fn test_style_preamble_included() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test"),
        ]))]);

        let options = RenderOptions::default();
        let output = map_document(&doc, &options);

        // Should contain style preamble elements
        assert!(
            output
                .typst_source
                .contains("// rtfkit style profile: report")
        );
        assert!(output.typst_source.contains("#set text("));
        assert!(output.typst_source.contains("#set par("));
        assert!(output.typst_source.contains("#set table("));
        assert!(output.typst_source.contains("#set list("));
    }

    #[test]
    fn test_different_style_profiles() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test"),
        ]))]);

        // Test Report profile (default)
        let options_report = RenderOptions {
            style_profile: StyleProfileName::Report,
            ..Default::default()
        };
        let output_report = map_document(&doc, &options_report);
        assert!(
            output_report
                .typst_source
                .contains("// rtfkit style profile: report")
        );

        // Test Classic profile
        let options_classic = RenderOptions {
            style_profile: StyleProfileName::Classic,
            ..Default::default()
        };
        let output_classic = map_document(&doc, &options_classic);
        assert!(
            output_classic
                .typst_source
                .contains("// rtfkit style profile: classic")
        );

        // Test Compact profile
        let options_compact = RenderOptions {
            style_profile: StyleProfileName::Compact,
            ..Default::default()
        };
        let output_compact = map_document(&doc, &options_compact);
        assert!(
            output_compact
                .typst_source
                .contains("// rtfkit style profile: compact")
        );
    }

    #[test]
    fn test_resolve_style_profile() {
        let classic = resolve_style_profile(&StyleProfileName::Classic);
        assert_eq!(classic.name, StyleProfileName::Classic);

        let report = resolve_style_profile(&StyleProfileName::Report);
        assert_eq!(report.name, StyleProfileName::Report);

        let compact = resolve_style_profile(&StyleProfileName::Compact);
        assert_eq!(compact.name, StyleProfileName::Compact);

        // Custom falls back to Report
        let custom = resolve_style_profile(&StyleProfileName::Custom("my-theme".to_string()));
        assert_eq!(custom.name, StyleProfileName::Report);
    }
}
