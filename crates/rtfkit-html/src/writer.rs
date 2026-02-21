//! Top-level HTML writer orchestration.
//!
//! This module provides the main entry point for converting a `Document`
//! to HTML string output.

use rtfkit_core::{Document, Block};
use crate::error::HtmlWriterError;
use crate::options::HtmlWriterOptions;
use crate::serialize::HtmlBuffer;
use crate::style::default_stylesheet;

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
pub fn document_to_html(doc: &Document, options: &HtmlWriterOptions) -> Result<String, HtmlWriterError> {
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
    
    // Emit body content
    emit_blocks(&doc.blocks, &mut buf, &mut dropped_content_reasons)?;
    
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
    
    if options.include_default_css {
        buf.push_raw("<style>\n");
        buf.push_raw(default_stylesheet());
        buf.push_raw("\n</style>\n");
    }
    
    buf.push_raw("</head>\n");
    buf.push_raw("<body>\n");
}

/// Emits the document end (body and html closing).
fn emit_document_end(buf: &mut HtmlBuffer) {
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
fn emit_block(
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
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{ListBlock, ListItem, ListKind, Paragraph, Run};

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
    }

    #[test]
    fn empty_document_fragment() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: false,
            include_default_css: false,
        };
        let html = document_to_html(&doc, &options).unwrap();
        
        assert!(!html.contains("<!doctype html>"));
        assert!(!html.contains("<html>"));
        assert!(html.is_empty());
    }

    #[test]
    fn document_with_paragraph() {
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello")])),
        ]);
        let options = HtmlWriterOptions::default();
        let html = document_to_html(&doc, &options).unwrap();
        
        assert!(html.contains("<p>Hello</p>"));
    }

    #[test]
    fn document_without_css() {
        let doc = Document::new();
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            include_default_css: false,
        };
        let html = document_to_html(&doc, &options).unwrap();
        
        assert!(!html.contains("<style>"));
    }

    #[test]
    fn document_with_warnings_api_empty_reasons_for_normal_input() {
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello")])),
        ]);
        let options = HtmlWriterOptions::default();
        let output = document_to_html_with_warnings(&doc, &options).unwrap();
        assert!(output.html.contains("<p>Hello</p>"));
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
}
