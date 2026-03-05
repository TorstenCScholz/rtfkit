//! Paragraph, inline, run, and hyperlink conversion from IR to docx-rs types.

use crate::context::ConvertCtx;
use crate::shading::convert_shading;
use docx_rs::{
    AlignmentType, Hyperlink as DocxHyperlink, HyperlinkType, IndentLevel, LineSpacing,
    NumberingId, Paragraph as DocxParagraph, Run as DocxRun, RunFonts, RunProperty, Shading,
    ShdType,
};
use rtfkit_core::{
    Alignment, GeneratedBlockKind, Hyperlink as IrHyperlink, HyperlinkTarget, Inline, PageFieldRef,
    Paragraph, Run, SemanticField, SemanticFieldRef,
};

/// Converts an IR paragraph to a docx-rs paragraph without numbering.
pub(crate) fn convert_paragraph(para: &Paragraph, ctx: &mut ConvertCtx<'_>) -> DocxParagraph {
    convert_paragraph_with_numbering(para, 0, 0, ctx)
}

/// Converts an IR paragraph to a docx-rs paragraph with optional numbering.
pub(crate) fn convert_paragraph_with_numbering(
    para: &Paragraph,
    num_id: u32,
    level: u8,
    ctx: &mut ConvertCtx<'_>,
) -> DocxParagraph {
    let mut p = DocxParagraph::new();

    // Add numbering properties if this is a list item
    if num_id > 0 {
        p = p.numbering(
            NumberingId::new(num_id as usize),
            IndentLevel::new(level as usize),
        );
        if let Some(pf) = ctx.profile {
            p = p.line_spacing(
                LineSpacing::new().after((pf.spacing.list_item_gap * 20.0).round() as u32),
            );
        }
    }

    // Map alignment
    p = p.align(convert_alignment(para.alignment));

    // Map paragraph shading through paragraph default run properties.
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
                let id = *ctx.bookmark_id;
                *ctx.bookmark_id += 1;
                p = p.add_bookmark_start(id, &anchor.name);
                p = p.add_bookmark_end(id);
            }
            Inline::NoteRef(note_ref) => {
                p = p.add_run(crate::structure::note_ref_to_docx_run(note_ref, ctx));
            }
            Inline::PageField(page_field) => {
                p = p.add_run(page_field_to_docx_run(page_field));
            }
            Inline::SemanticField(sf) => {
                p = add_semantic_field_to_para(p, sf);
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

/// Add a semantic field to a paragraph, using formatted runs when available.
fn add_semantic_field_to_para(mut p: DocxParagraph, sf: &SemanticField) -> DocxParagraph {
    match &sf.reference {
        SemanticFieldRef::Ref {
            target,
            fallback_text,
        }
        | SemanticFieldRef::NoteRef {
            target,
            fallback_text,
        } => {
            if sf.resolved {
                let mut link = DocxHyperlink::new(target, HyperlinkType::Anchor);
                if sf.runs.is_empty() {
                    let visible = fallback_text.as_deref().unwrap_or(target.as_str());
                    link = link.add_run(DocxRun::new().add_text(visible));
                } else {
                    for run in &sf.runs {
                        link = link.add_run(convert_run(run));
                    }
                }
                p = p.add_hyperlink(link);
            } else {
                // Unresolved: emit plain runs, avoid broken link.
                if sf.runs.is_empty() {
                    let visible = fallback_text.as_deref().unwrap_or(target.as_str());
                    p = p.add_run(DocxRun::new().add_text(visible));
                } else {
                    for run in &sf.runs {
                        p = p.add_run(convert_run(run));
                    }
                }
            }
        }
        SemanticFieldRef::Sequence {
            identifier,
            fallback_text,
        }
        | SemanticFieldRef::DocProperty {
            name: identifier,
            fallback_text,
        }
        | SemanticFieldRef::MergeField {
            name: identifier,
            fallback_text,
        } => {
            if sf.runs.is_empty() {
                p = p.add_run(
                    DocxRun::new()
                        .add_text(fallback_text.as_deref().unwrap_or(identifier.as_str())),
                );
            } else {
                for run in &sf.runs {
                    p = p.add_run(convert_run(run));
                }
            }
        }
    }
    p
}

/// Converts a page field reference to a placeholder run (page fields are not
/// fully supported in DOCX output; a static fallback value is emitted).
pub(crate) fn page_field_to_docx_run(page_field: &PageFieldRef) -> DocxRun {
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

/// Converts an IR Hyperlink to a docx-rs Hyperlink.
pub(crate) fn convert_hyperlink(hyperlink: &IrHyperlink) -> DocxHyperlink {
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
pub(crate) fn convert_alignment(align: Alignment) -> AlignmentType {
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
///
/// Run property order follows OOXML convention:
/// 1. Font family (`w:rFonts`)
/// 2. Font size (`w:sz`, `w:szCs`)
/// 3. Color (`w:color`)
/// 4. Bold (`w:b`)
/// 5. Italic (`w:i`)
/// 6. Underline (`w:u`)
/// 7. Strikethrough (`w:strike`)
/// 8. Caps (`w:caps`)
/// 9. Shading (`w:shd`) for background color
pub(crate) fn convert_run(run: &Run) -> DocxRun {
    let mut r = DocxRun::new().add_text(&run.text);

    // Apply font family (w:rFonts)
    if let Some(ref font_family) = run.font_family {
        r = r.fonts(
            RunFonts::new()
                .ascii(font_family)
                .hi_ansi(font_family)
                .cs(font_family),
        );
    }

    // Apply font size (w:sz, w:szCs)
    // Size is in half-points (24 = 12pt). Clamp to OOXML spec: 1-1638 half-points.
    if let Some(font_size) = run.font_size {
        let half_points = (font_size * 2.0).round() as usize;
        let clamped = half_points.clamp(1, 1638);
        r = r.size(clamped);
    }

    // Apply color (w:color) — 6-character hex RGB, no alpha.
    if let Some(ref color) = run.color {
        let hex = format!("{:02X}{:02X}{:02X}", color.r, color.g, color.b);
        r = r.color(hex);
    }

    if run.bold {
        r = r.bold();
    }

    if run.italic {
        r = r.italic();
    }

    if run.underline {
        r = r.underline("single");
    }

    if run.strikethrough {
        r = r.strike();
    }

    // Apply caps (w:caps).
    // docx-rs does not currently expose w:smallCaps, so small caps degrades to caps.
    if run.all_caps || run.small_caps {
        r.run_property = r.run_property.caps();
    }

    // Apply background color (w:shd) with w:val="clear" and w:fill for arbitrary RGB.
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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::{DocxWriterOptions, write_docx_to_bytes};
    use rtfkit_core::{
        Alignment, Block, Document, Hyperlink as IrHyperlink, HyperlinkTarget, Inline, ListBlock,
        ListItem, ListKind, Paragraph, Run, SemanticField, SemanticFieldRef, ShadingPattern,
    };

    fn zip_entry_string(bytes: &[u8], entry_name: &str) -> String {
        use std::io::Cursor;
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

    // =========================================================================
    // Paragraph / Run Tests
    // =========================================================================

    #[test]
    fn test_simple_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());

        use std::io::Cursor;
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
    // Alignment Tests
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
    // Run Formatting Tests
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
    // Whitespace Tests
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
    // Unicode / Special Characters
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
        run.font_size = Some(12.0);
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:sz w:val="24" />"#));
        assert!(document_xml.contains(r#"<w:szCs w:val="24" />"#));
    }

    #[test]
    fn test_run_with_color_only() {
        let mut run = Run::new("Colored text");
        run.color = Some(rtfkit_core::Color::new(255, 0, 0));
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
        run.font_size = Some(14.0);
        run.color = Some(rtfkit_core::Color::new(0, 128, 255));
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"w:ascii="Helvetica""#));
        assert!(document_xml.contains(r#"<w:sz w:val="28" />"#));
        assert!(document_xml.contains(r#"<w:color w:val="0080FF" />"#));
    }

    #[test]
    fn test_run_with_combined_formatting() {
        let mut run = Run::new("Bold colored font text");
        run.bold = true;
        run.font_family = Some("Times New Roman".to_string());
        run.font_size = Some(16.0);
        run.color = Some(rtfkit_core::Color::new(0, 255, 0));
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"w:ascii="Times New Roman""#));
        assert!(document_xml.contains(r#"<w:sz w:val="32" />"#));
        assert!(document_xml.contains(r#"<w:color w:val="00FF00" />"#));
        assert!(document_xml.contains("<w:b"));
    }

    #[test]
    fn test_font_size_clamping_min() {
        let mut run = Run::new("Tiny text");
        run.font_size = Some(0.1);
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:sz w:val="1" />"#));
    }

    #[test]
    fn test_font_size_clamping_max() {
        let mut run = Run::new("Huge text");
        run.font_size = Some(1000.0);
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:sz w:val="1638" />"#));
    }

    #[test]
    fn test_font_size_rounding() {
        let mut run = Run::new("Rounded text");
        run.font_size = Some(12.4);
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:sz w:val="25" />"#));
    }

    // =========================================================================
    // Hyperlink Tests
    // =========================================================================

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

    #[test]
    fn test_hyperlink_with_font_color_styling() {
        let mut styled_run = Run::new("Styled Link");
        styled_run.font_family = Some("Verdana".to_string());
        styled_run.font_size = Some(10.0);
        styled_run.color = Some(rtfkit_core::Color::new(0, 0, 255));
        styled_run.underline = true;
        let para = Paragraph::from_inlines(vec![Inline::Hyperlink(IrHyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![styled_run],
        })]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:hyperlink"));
        assert!(document_xml.contains(r#"w:ascii="Verdana""#));
        assert!(document_xml.contains(r#"<w:sz w:val="20" />"#));
        assert!(document_xml.contains(r#"<w:color w:val="0000FF" />"#));
        assert!(document_xml.contains(r#"<w:u w:val="single""#));
    }

    #[test]
    fn test_hyperlink_with_background_color() {
        let mut run = Run::new("Highlighted link");
        run.background_color = Some(rtfkit_core::Color::new(0, 255, 0));
        run.underline = true;
        let para = Paragraph::from_inlines(vec![Inline::Hyperlink(IrHyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![run],
        })]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:hyperlink"));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="00FF00""#));
        assert!(document_xml.contains(r#"<w:u w:val="single""#));
    }

    // =========================================================================
    // Background Color (Shading) Tests
    // =========================================================================

    #[test]
    fn test_paragraph_with_shading() {
        let mut para = Paragraph::from_runs(vec![Run::new("Shaded paragraph")]);
        para.shading = Some(rtfkit_core::Shading::solid(rtfkit_core::Color::new(
            255, 255, 0,
        )));
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
        shading.fill_color = Some(rtfkit_core::Color::new(255, 255, 255));
        shading.pattern_color = Some(rtfkit_core::Color::new(0, 0, 0));
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
        run.background_color = Some(rtfkit_core::Color::new(255, 255, 0));
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
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
        assert!(!document_xml.contains("<w:shd"));
    }

    #[test]
    fn test_run_with_foreground_and_background_color() {
        let mut run = Run::new("Colored text on colored background");
        run.color = Some(rtfkit_core::Color::new(255, 0, 0));
        run.background_color = Some(rtfkit_core::Color::new(0, 0, 255));
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:color w:val="FF0000" />"#));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="0000FF""#));
    }

    #[test]
    fn test_background_color_with_all_formatting() {
        let mut run = Run::new("Fully formatted");
        run.bold = true;
        run.italic = true;
        run.underline = true;
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(14.0);
        run.color = Some(rtfkit_core::Color::new(128, 0, 128));
        run.background_color = Some(rtfkit_core::Color::new(255, 192, 203));
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![run]))]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
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
        use std::io::Cursor;
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
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
        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
        use std::io::Cursor;
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
        let mut list2 = ListBlock::new(2, ListKind::OrderedDecimal);
        list2.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Number 1")]),
        ));
        let doc = Document::from_blocks(vec![
            Block::ListBlock(list1),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Between lists")])),
            Block::ListBlock(list2),
        ]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
        use std::io::Cursor;
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
    fn test_semantic_ref_emits_anchor_hyperlink() {
        let para = Paragraph::from_inlines(vec![Inline::SemanticField(SemanticField::new(
            SemanticFieldRef::Ref {
                target: "sec_intro".to_string(),
                fallback_text: Some("Introduction".to_string()),
            },
        ))]);
        let doc = Document::from_blocks(vec![Block::Paragraph(para)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:hyperlink"));
        assert!(document_xml.contains("w:anchor=\"sec_intro\""));
        assert!(document_xml.contains("Introduction"));
    }
}
