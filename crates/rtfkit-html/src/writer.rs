//! Top-level HTML writer orchestration.
//!
//! This module provides the main entry point for converting a `Document`
//! to HTML string output.

use crate::error::HtmlWriterError;
use crate::options::{CssMode, HtmlWriterOptions};
use crate::serialize::HtmlBuffer;
use crate::style::stylesheet_for_profile;
use rtfkit_core::{Block, Document};

/// HTML rendering result with writer-level semantic degradation reasons.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HtmlWriterOutput {
    /// Rendered HTML payload.
    pub html: String,
    /// Stable dropped-content reason strings produced by the writer.
    pub dropped_content_reasons: Vec<String>,
}

/// Converts a document to an HTML string.
///
/// This is the main entry point for HTML generation. It orchestrates
/// the conversion of a `Document` to a complete HTML string based on
/// the provided options.
///
/// # Example
///
/// ```rust
/// use rtfkit_core::{Document, Block, Paragraph, Run};
/// use rtfkit_html::{document_to_html, HtmlWriterOptions};
///
/// let doc = Document::from_blocks(vec![
///     Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello, World!")])),
/// ]);
///
/// let options = HtmlWriterOptions::default();
/// let html = document_to_html(&doc, &options).unwrap();
/// assert!(html.contains("Hello, World!"));
/// ```
pub fn document_to_html(
    doc: &Document,
    options: &HtmlWriterOptions,
) -> Result<String, HtmlWriterError> {
    Ok(document_to_html_with_warnings(doc, options)?.html)
}

/// Converts a document to HTML while collecting writer-level dropped-content reasons.
pub fn document_to_html_with_warnings(
    doc: &Document,
    options: &HtmlWriterOptions,
) -> Result<HtmlWriterOutput, HtmlWriterError> {
    let mut buf = HtmlBuffer::new();
    let mut dropped_content_reasons = Vec::new();

    // Emit document wrapper if requested
    if options.emit_document_wrapper {
        emit_document_start(&mut buf, options);
    }

    // Emit headers before body content
    if let Some(ref structure) = doc.structure {
        crate::blocks::structure::structure_to_html(
            structure,
            &mut buf,
            true, // emit_headers = true
            &mut dropped_content_reasons,
        );
    }

    // Emit body content
    emit_blocks(&doc.blocks, &mut buf, &mut dropped_content_reasons)?;

    // Emit footers and notes after body content
    if let Some(ref structure) = doc.structure {
        crate::blocks::structure::structure_to_html(
            structure,
            &mut buf,
            false, // emit_headers = false → footers + notes
            &mut dropped_content_reasons,
        );
    }

    // Close document wrapper if requested
    if options.emit_document_wrapper {
        emit_document_end(&mut buf);
    }

    Ok(HtmlWriterOutput {
        html: buf.into_string(),
        dropped_content_reasons,
    })
}

/// Emits the document start (doctype, html, head, body opening).
fn emit_document_start(buf: &mut HtmlBuffer, options: &HtmlWriterOptions) {
    buf.push_raw("<!doctype html>\n");
    buf.push_raw("<html lang=\"en\">\n");
    buf.push_raw("<head>\n");
    buf.push_raw("<meta charset=\"utf-8\">\n");

    // Emit CSS based on mode
    match options.css_mode {
        CssMode::Default => {
            buf.push_raw("<style>\n");
            buf.push_raw(&stylesheet_for_profile(&options.style_profile));
            buf.push_raw("\n</style>\n");
        }
        CssMode::None => {
            // No built-in CSS
        }
    }

    // Append custom CSS if provided
    if let Some(ref custom_css) = options.custom_css {
        buf.push_raw("<style>\n");
        buf.push_raw(custom_css);
        buf.push_raw("\n</style>\n");
    }

    buf.push_raw("</head>\n");
    buf.push_raw("<body>\n");
    buf.push_raw("<div class=\"rtf-doc\">\n");
    buf.push_raw("<div class=\"rtf-content\">\n");
}

/// Emits the document end (body and html closing).
fn emit_document_end(buf: &mut HtmlBuffer) {
    buf.push_raw("</div>\n");
    buf.push_raw("</div>\n");
    buf.push_raw("</body>\n");
    buf.push_raw("</html>\n");
}

/// Emits a sequence of blocks.
fn emit_blocks(
    blocks: &[Block],
    buf: &mut HtmlBuffer,
    dropped_content_reasons: &mut Vec<String>,
) -> Result<(), HtmlWriterError> {
    for block in blocks {
        emit_block(block, buf, dropped_content_reasons)?;
    }
    Ok(())
}

/// Emits a single block.
///
/// `pub(crate)` so that the `blocks::structure` module can reuse it when
/// rendering header/footer/note body blocks.
pub(crate) fn emit_block(
    block: &Block,
    buf: &mut HtmlBuffer,
    dropped_content_reasons: &mut Vec<String>,
) -> Result<(), HtmlWriterError> {
    match block {
        Block::Paragraph(para) => {
            crate::blocks::paragraph::paragraph_to_html(para, buf);
        }
        Block::ListBlock(list) => {
            crate::blocks::list::list_to_html_with_warnings(list, buf, dropped_content_reasons);
        }
        Block::TableBlock(table) => {
            crate::blocks::table::table_to_html_with_warnings(table, buf, dropped_content_reasons);
        }
        Block::ImageBlock(image) => {
            crate::blocks::image::image_to_html(image, buf);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{ListBlock, ListItem, ListKind, Paragraph, Run};
    use rtfkit_style_tokens::StyleProfileName;

    #[test]
    fn empty_document_with_wrapper() {
        let doc = Document::new();
        let options = HtmlWriterOptions::default();
        let html = document_to_html(&doc, &options).unwrap();

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("</body>"));
        assert!(html.contains("<div class=\"rtf-doc\">"));
        assert!(html.contains("<div class=\"rtf-content\">"));
    }

    #[test]
    fn empty_document_fragment() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: false,
            css_mode: CssMode::None,
            custom_css: None,
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        assert!(!html.contains("<!doctype html>"));
        assert!(!html.contains("<html>"));
        assert!(html.is_empty());
    }

    #[test]
    fn document_with_paragraph() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello"),
        ]))]);
        let options = HtmlWriterOptions::default();
        let html = document_to_html(&doc, &options).unwrap();

        assert!(html.contains(r#"<p class="rtf-p">Hello</p>"#));
    }

    #[test]
    fn document_without_css() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::None,
            custom_css: None,
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        assert!(!html.contains("<style>"));
    }

    #[test]
    fn document_with_custom_css_only() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::None,
            custom_css: Some("body { background: #fff; }".to_string()),
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        // Should not contain default stylesheet
        assert!(!html.contains(".rtf-doc"));
        // Should contain custom CSS
        assert!(html.contains("body { background: #fff; }"));
        // Should have exactly one style block
        assert_eq!(html.matches("<style>").count(), 1);
    }

    #[test]
    fn document_with_default_and_custom_css() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::Default,
            custom_css: Some("body { color: red; }".to_string()),
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        // Should contain default stylesheet
        assert!(html.contains(".rtf-doc"));
        // Should also contain custom CSS
        assert!(html.contains("body { color: red; }"));
        // Should have two style blocks
        assert_eq!(html.matches("<style>").count(), 2);
    }

    #[test]
    fn document_with_warnings_api_empty_reasons_for_normal_input() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello"),
        ]))]);
        let options = HtmlWriterOptions::default();
        let output = document_to_html_with_warnings(&doc, &options).unwrap();
        assert!(output.html.contains(r#"<p class="rtf-p">Hello</p>"#));
        assert!(output.dropped_content_reasons.is_empty());
    }

    #[test]
    fn document_with_list_level_jump_emits_dropped_reason() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Top")]),
        ));
        list.add_item(ListItem::from_paragraph(
            3,
            Paragraph::from_runs(vec![Run::new("Deep")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);
        let output = document_to_html_with_warnings(&doc, &HtmlWriterOptions::default()).unwrap();

        assert!(output.html.contains("Deep"));
        assert_eq!(
            output.dropped_content_reasons,
            vec!["list_nesting_semantics".to_string()]
        );
    }

    /// Test that CssMode::None produces HTML without any <style> block for built-in CSS.
    #[test]
    fn css_mode_none_omits_builtin_stylesheet() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::None,
            custom_css: None,
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        // Should not contain any style block
        assert!(
            !html.contains("<style>"),
            "CssMode::None should not emit any <style> block"
        );
        assert!(
            !html.contains(".rtf-doc"),
            "CssMode::None should not include built-in CSS classes"
        );
    }

    /// Test that custom CSS is appended after built-in CSS.
    #[test]
    fn custom_css_appended_after_builtin_css() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::Default,
            custom_css: Some("/* custom css */".to_string()),
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        // Find positions of built-in and custom CSS
        let builtin_pos = html
            .find(".rtf-doc")
            .expect("Built-in CSS should be present");
        let custom_pos = html
            .find("/* custom css */")
            .expect("Custom CSS should be present");

        // Custom CSS should come after built-in CSS
        assert!(
            custom_pos > builtin_pos,
            "Custom CSS should be appended after built-in CSS"
        );
    }

    /// Test that custom CSS appears in its own <style> block.
    #[test]
    fn custom_css_in_own_style_block() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::Default,
            custom_css: Some(".custom { color: red; }".to_string()),
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        // Should have exactly two style blocks
        let style_count = html.matches("<style>").count();
        assert_eq!(
            style_count, 2,
            "Should have two <style> blocks (built-in and custom)"
        );

        // Verify the custom CSS is in its own block
        assert!(
            html.contains(".custom { color: red; }"),
            "Custom CSS should be present"
        );
    }

    /// Test that CssMode::None with custom CSS produces only one style block.
    #[test]
    fn css_mode_none_with_custom_css_single_block() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::None,
            custom_css: Some(".my-style { margin: 0; }".to_string()),
            style_profile: StyleProfileName::Report,
        };
        let html = document_to_html(&doc, &options).unwrap();

        // Should have exactly one style block (custom only)
        let style_count = html.matches("<style>").count();
        assert_eq!(
            style_count, 1,
            "Should have exactly one <style> block for custom CSS"
        );

        // Should contain custom CSS
        assert!(
            html.contains(".my-style { margin: 0; }"),
            "Custom CSS should be present"
        );

        // Should not contain built-in CSS
        assert!(
            !html.contains(".rtf-doc"),
            "Should not contain built-in CSS classes"
        );
    }

    /// Test that HTML output is deterministic across multiple calls.
    #[test]
    fn html_output_is_deterministic() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);
        let options = HtmlWriterOptions::default();

        // Generate HTML multiple times
        let html1 = document_to_html(&doc, &options).unwrap();
        let html2 = document_to_html(&doc, &options).unwrap();
        let html3 = document_to_html(&doc, &options).unwrap();

        // All outputs should be identical
        assert_eq!(html1, html2, "HTML output should be deterministic");
        assert_eq!(html2, html3, "HTML output should be deterministic");
    }

    /// Test that HTML output with custom CSS is deterministic.
    #[test]
    fn html_output_with_custom_css_is_deterministic() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test"),
        ]))]);
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::Default,
            custom_css: Some(".custom {}".to_string()),
            style_profile: StyleProfileName::Report,
        };

        let html1 = document_to_html(&doc, &options).unwrap();
        let html2 = document_to_html(&doc, &options).unwrap();

        assert_eq!(
            html1, html2,
            "HTML output with custom CSS should be deterministic"
        );
    }

    /// Test that different style profiles produce different CSS.
    #[test]
    fn different_style_profiles_produce_different_css() {
        let doc = Document::new();

        let classic_options = HtmlWriterOptions {
            style_profile: StyleProfileName::Classic,
            ..HtmlWriterOptions::default()
        };
        let report_options = HtmlWriterOptions::default();
        let compact_options = HtmlWriterOptions {
            style_profile: StyleProfileName::Compact,
            ..HtmlWriterOptions::default()
        };

        let classic_html = document_to_html(&doc, &classic_options).unwrap();
        let report_html = document_to_html(&doc, &report_options).unwrap();
        let compact_html = document_to_html(&doc, &compact_options).unwrap();

        // All profiles use Libertinus Serif as the body font
        assert!(classic_html.contains("Libertinus Serif"));
        assert!(report_html.contains("Libertinus Serif"));
        // Classic uses 12pt body size
        assert!(classic_html.contains("--rtfkit-size-body: 12pt;"));
        // Report uses 11pt body size
        assert!(report_html.contains("--rtfkit-size-body: 11pt;"));
        // Compact uses 9pt body size
        assert!(compact_html.contains("--rtfkit-size-body: 9pt;"));
    }
}
