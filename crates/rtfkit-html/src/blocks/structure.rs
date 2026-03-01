//! Document structure HTML emission.
//!
//! Handles rendering of headers, footers, footnotes, and endnotes
//! from `DocumentStructure` into semantic HTML elements.
//!
//! ## Output structure
//!
//! Headers emit as `<header class="rtf-header rtf-header-{kind}">`.
//! Footers emit as `<footer class="rtf-footer rtf-footer-{kind}">`.
//! Notes emit as `<section class="rtf-notes">` containing individual
//! `<div id="note-{id}" class="rtf-note rtf-{footnote|endnote}">` elements.

use rtfkit_core::{Block, DocumentStructure, HeaderFooterSet, Note, NoteKind};

use crate::serialize::HtmlBuffer;

/// Emits headers and footers from a `DocumentStructure` into the buffer.
///
/// Headers appear before body content; footers after. Only non-empty
/// channels are emitted.
pub fn structure_to_html(
    structure: &DocumentStructure,
    buf: &mut HtmlBuffer,
    emit_headers: bool,
    dropped_content_reasons: &mut Vec<String>,
) {
    if emit_headers {
        emit_header_set(&structure.headers, buf, dropped_content_reasons);
    } else {
        emit_footer_set(&structure.footers, buf, dropped_content_reasons);
        emit_notes(&structure.notes, buf, dropped_content_reasons);
    }
}

/// Emits a `HeaderFooterSet` as `<header>` elements.
fn emit_header_set(
    set: &HeaderFooterSet,
    buf: &mut HtmlBuffer,
    dropped_content_reasons: &mut Vec<String>,
) {
    emit_header_footer_channel(&set.default, "header", "rtf-header-default", buf, dropped_content_reasons);
    emit_header_footer_channel(&set.first, "header", "rtf-header-first", buf, dropped_content_reasons);
    emit_header_footer_channel(&set.even, "header", "rtf-header-even", buf, dropped_content_reasons);
}

/// Emits a `HeaderFooterSet` as `<footer>` elements.
fn emit_footer_set(
    set: &HeaderFooterSet,
    buf: &mut HtmlBuffer,
    dropped_content_reasons: &mut Vec<String>,
) {
    emit_header_footer_channel(&set.default, "footer", "rtf-footer-default", buf, dropped_content_reasons);
    emit_header_footer_channel(&set.first, "footer", "rtf-footer-first", buf, dropped_content_reasons);
    emit_header_footer_channel(&set.even, "footer", "rtf-footer-even", buf, dropped_content_reasons);
}

/// Emits a single header/footer channel as an HTML landmark element.
///
/// Skips emission when `blocks` is empty.
fn emit_header_footer_channel(
    blocks: &[Block],
    tag: &str,
    class: &str,
    buf: &mut HtmlBuffer,
    dropped_content_reasons: &mut Vec<String>,
) {
    if blocks.is_empty() {
        return;
    }
    buf.push_raw(&format!("<{tag} class=\"rtf-header-footer {class}\">\n"));
    for block in blocks {
        // emit_block is infallible in practice (only returns Ok); ignore result
        let _ = crate::writer::emit_block(block, buf, dropped_content_reasons);
    }
    buf.push_raw(&format!("</{tag}>\n"));
}

/// Emits a `<section class="rtf-notes">` containing all note bodies.
///
/// Skips emission when `notes` is empty.
pub fn emit_notes(
    notes: &[Note],
    buf: &mut HtmlBuffer,
    dropped_content_reasons: &mut Vec<String>,
) {
    if notes.is_empty() {
        return;
    }
    buf.push_raw("<section class=\"rtf-notes\">\n");
    for note in notes {
        let kind_class = match note.kind {
            NoteKind::Footnote => "rtf-footnote",
            NoteKind::Endnote => "rtf-endnote",
        };
        buf.push_raw(&format!(
            "<div id=\"note-{}\" class=\"rtf-note {kind_class}\">\n",
            note.id
        ));
        for block in &note.blocks {
            let _ = crate::writer::emit_block(block, buf, dropped_content_reasons);
        }
        buf.push_raw("</div>\n");
    }
    buf.push_raw("</section>\n");
}
