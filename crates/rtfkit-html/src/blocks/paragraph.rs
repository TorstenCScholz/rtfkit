//! Paragraph block HTML emission.
//!
//! This module handles converting IR paragraphs to HTML with proper
//! semantic tags and run merging.

use rtfkit_core::{
    BookmarkAnchor, Color, GeneratedBlockKind, Hyperlink, HyperlinkTarget, Inline, NoteRef,
    PageFieldRef, Paragraph, Run, SemanticField, SemanticFieldRef, Shading, ShadingRenderPolicy,
    resolve_shading_fill_color,
};

use crate::escape::{escape_attribute, sanitize_font_family};
use crate::serialize::HtmlBuffer;
use crate::style::alignment_class;
#[cfg(test)]
use rtfkit_core::Alignment;

/// Builds a CSS style string for run-level formatting properties.
///
/// This helper centralizes run style string building to ensure:
/// 1. Deterministic property order for stable output
/// 2. Proper CSS sanitization for font-family names
/// 3. Consistent formatting across regular runs and hyperlink runs
///
/// Properties are emitted in this order:
/// 1. font-family (if present)
/// 2. font-size (if present)
/// 3. color (if present)
/// 4. background-color (if present)
///
/// # Security
///
/// Font family names are sanitized using [`sanitize_font_family`] to prevent
/// CSS injection attacks from untrusted RTF input.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::blocks::paragraph::build_run_style;
/// use rtfkit_core::{Run, Color};
///
/// let mut run = Run::new("text");
/// run.font_family = Some("Arial".to_string());
/// run.font_size = Some(12.0);
/// run.color = Some(Color::new(255, 0, 0));
///
/// let style = build_run_style(&run);
/// assert_eq!(style, "font-family: \"Arial\"; font-size: 12pt; color: #ff0000;");
/// ```
pub fn build_run_style(run: &Run) -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1. Font family (sanitized for CSS safety)
    if let Some(ref font_family) = run.font_family {
        let sanitized = sanitize_font_family(font_family);
        if !sanitized.is_empty() {
            parts.push(format!("font-family: \"{}\"", sanitized));
        }
    }

    // 2. Font size in points
    if let Some(font_size) = run.font_size {
        parts.push(format!("font-size: {}pt", font_size));
    }

    // 3. Color as hex RGB
    if let Some(ref color) = run.color {
        parts.push(format!(
            "color: #{:02x}{:02x}{:02x}",
            color.r, color.g, color.b
        ));
    }

    // 4. Background color as hex RGB
    if let Some(ref background_color) = run.background_color {
        parts.push(format!(
            "background-color: #{:02x}{:02x}{:02x}",
            background_color.r, background_color.g, background_color.b
        ));
    }

    if parts.is_empty() {
        String::new()
    } else {
        parts.join("; ") + ";"
    }
}

/// Converts a color to its CSS hex representation.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::blocks::paragraph::color_to_hex;
/// use rtfkit_core::Color;
///
/// assert_eq!(color_to_hex(&Color::new(255, 0, 0)), format!("#{}", "ff0000"));
/// assert_eq!(color_to_hex(&Color::new(0, 128, 255)), format!("#{}", "0080ff"));
/// ```
pub fn color_to_hex(color: &Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

/// Builds a CSS style string for paragraph-level properties.
///
/// This helper centralizes paragraph style string building to ensure:
/// 1. Deterministic property order for stable output
/// 2. Consistent formatting with run-level styles
///
/// Properties are emitted in this order:
/// 1. background-color (if present from shading)
///
/// # Pattern Degradation (Slice B)
///
/// CSS does not natively support pattern fills without images. For percentage
/// patterns, we emit a deterministic blended approximation. Non-percent patterns
/// still degrade to fill color only.
///
/// # Security
///
/// Color values are formatted as hex strings, preventing CSS injection.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::blocks::paragraph::build_paragraph_style;
/// use rtfkit_core::{Shading, Color};
///
/// let shading = Shading::solid(Color::new(255, 255, 0));
/// let style = build_paragraph_style(Some(&shading));
/// assert_eq!(style, "background-color: #ffff00;");
/// ```
pub fn build_paragraph_style(shading: Option<&Shading>) -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1. Background color from shading (pattern degradation: emit fill_color only)
    if let Some(fill_color) =
        resolve_shading_fill_color(shading, ShadingRenderPolicy::ApproximatePercentPatterns)
    {
        parts.push(format!(
            "background-color: #{:02x}{:02x}{:02x}",
            fill_color.r, fill_color.g, fill_color.b
        ));
    }

    if parts.is_empty() {
        String::new()
    } else {
        parts.join("; ") + ";"
    }
}

/// Checks if a URL scheme is safe for HTML href attributes.
///
/// Only allows http, https, and mailto schemes to prevent XSS attacks.
fn is_safe_url(url: &str) -> bool {
    let url_lower = url.trim().to_ascii_lowercase();
    url_lower.starts_with("http://")
        || url_lower.starts_with("https://")
        || url_lower.starts_with("mailto:")
}

/// Converts a paragraph to HTML.
///
/// Emits a `<p>` element with `rtf-p` class and appropriate alignment class if needed.
/// Inlines are processed in order, with hyperlinks emitting `<a href>` tags.
/// Paragraph shading is emitted as inline `background-color` style.
pub fn paragraph_to_html(para: &Paragraph, buf: &mut HtmlBuffer) {
    // Build class attribute - always include rtf-p, add alignment if non-default
    let mut classes: Vec<&'static str> = vec!["rtf-p"];
    if let Some(align_class) = alignment_class(para.alignment) {
        classes.push(align_class);
    }
    let class_str = classes.join(" ");

    // Build style attribute for shading
    let style = build_paragraph_style(para.shading.as_ref());
    let has_style = !style.is_empty();

    // Build attributes in deterministic order: class, style
    let mut attrs: Vec<(&str, &str)> = vec![("class", &class_str)];
    if has_style {
        attrs.push(("style", style.as_str()));
    }

    buf.push_open_tag("p", &attrs);

    // Emit inlines in order, handling hyperlinks and bookmark anchors specially
    for inline in &para.inlines {
        match inline {
            Inline::Run(run) => {
                run_to_html(run, buf);
            }
            Inline::Hyperlink(hyperlink) => {
                hyperlink_to_html(hyperlink, buf);
            }
            Inline::BookmarkAnchor(anchor) => {
                bookmark_anchor_to_html(anchor, buf);
            }
            Inline::NoteRef(note_ref) => {
                note_ref_to_html(note_ref, buf);
            }
            Inline::PageField(page_field) => {
                page_field_to_html(page_field, buf);
            }
            Inline::SemanticField(sf) => {
                semantic_field_to_html(sf, buf);
            }
            Inline::GeneratedBlockMarker(kind) => {
                generated_block_marker_to_html(kind, buf);
            }
        }
    }

    buf.push_close_tag("p");
}

/// Converts a note reference to an HTML superscript link.
///
/// Emits `<a href="#note-{id}" class="rtf-note-ref rtf-{kind}-ref"><sup>{id}</sup></a>`.
fn note_ref_to_html(note_ref: &NoteRef, buf: &mut HtmlBuffer) {
    use rtfkit_core::NoteKind;
    let kind_class = match note_ref.kind {
        NoteKind::Footnote => "rtf-footnote-ref",
        NoteKind::Endnote => "rtf-endnote-ref",
    };
    buf.push_raw(&format!(
        "<a href=\"#note-{}\" class=\"rtf-note-ref {kind_class}\"><sup>{}</sup></a>",
        note_ref.id, note_ref.id
    ));
}

fn page_field_to_html(page_field: &PageFieldRef, buf: &mut HtmlBuffer) {
    match page_field {
        PageFieldRef::CurrentPage { .. } => {
            buf.push_raw("<span class=\"rtf-page-field\" data-field=\"page\">1</span>");
        }
        PageFieldRef::TotalPages { .. } => {
            buf.push_raw("<span class=\"rtf-page-field\" data-field=\"numpages\">1</span>");
        }
        PageFieldRef::SectionPages { .. } => {
            buf.push_raw("<span class=\"rtf-page-field\" data-field=\"sectionpages\">1</span>");
        }
        PageFieldRef::PageRef {
            target,
            fallback_text,
            ..
        } => {
            let value = fallback_text.as_deref().unwrap_or(target);
            let escaped = escape_attribute(value);
            buf.push_raw(&format!(
                "<span class=\"rtf-page-field\" data-field=\"pageref\">{escaped}</span>"
            ));
        }
    }
}

fn semantic_field_to_html(sf: &SemanticField, buf: &mut HtmlBuffer) {
    match &sf.reference {
        SemanticFieldRef::Ref {
            target,
            fallback_text,
        }
        | SemanticFieldRef::NoteRef {
            target,
            fallback_text,
        } => {
            let fallback = fallback_text.as_deref().unwrap_or(target);
            if sf.resolved {
                let href = escape_attribute(target);
                buf.push_raw(&format!(
                    "<a href=\"#{href}\" class=\"rtf-link rtf-semantic-field\" data-field=\"ref\">"
                ));
                emit_sf_content(sf, fallback, buf);
                buf.push_raw("</a>");
            } else {
                emit_sf_content(sf, fallback, buf);
            }
        }
        SemanticFieldRef::Sequence {
            identifier,
            fallback_text,
        } => {
            let fallback = fallback_text.as_deref().unwrap_or(identifier);
            buf.push_raw("<span class=\"rtf-semantic-field\" data-field=\"seq\">");
            emit_sf_content(sf, fallback, buf);
            buf.push_raw("</span>");
        }
        SemanticFieldRef::DocProperty {
            name,
            fallback_text,
        } => {
            let fallback = fallback_text.as_deref().unwrap_or(name);
            buf.push_raw("<span class=\"rtf-semantic-field\" data-field=\"docproperty\">");
            emit_sf_content(sf, fallback, buf);
            buf.push_raw("</span>");
        }
        SemanticFieldRef::MergeField {
            name,
            fallback_text,
        } => {
            let fallback = fallback_text.as_deref().unwrap_or(name);
            buf.push_raw("<span class=\"rtf-semantic-field\" data-field=\"mergefield\">");
            emit_sf_content(sf, fallback, buf);
            buf.push_raw("</span>");
        }
    }
}

/// Emit the visible content of a semantic field.
///
/// Prefers formatted `runs` when available; falls back to plain escaped text.
fn emit_sf_content(sf: &SemanticField, fallback: &str, buf: &mut HtmlBuffer) {
    if !sf.runs.is_empty() {
        for run in &sf.runs {
            run_to_html(run, buf);
        }
    } else {
        buf.push_text(fallback);
    }
}

fn generated_block_marker_to_html(kind: &GeneratedBlockKind, buf: &mut HtmlBuffer) {
    if matches!(kind, GeneratedBlockKind::TableOfContents { .. }) {
        buf.push_raw("<nav class=\"rtf-generated-toc\"></nav>");
    }
}

/// Converts a bookmark anchor to an empty HTML `<a id="...">` element.
fn bookmark_anchor_to_html(anchor: &BookmarkAnchor, buf: &mut HtmlBuffer) {
    let escaped_id = escape_attribute(&anchor.name);
    buf.push_raw(&format!("<a id=\"{escaped_id}\"></a>"));
}

/// Converts a hyperlink to HTML.
///
/// Emits an `<a href="...">` element with `rtf-link` class.
/// External URLs are sanitized to reject dangerous schemes (javascript:, data:, vbscript:).
/// Internal bookmarks link to `#name` and are always safe.
/// If the URL is unsafe, the hyperlink content is emitted as plain text.
fn hyperlink_to_html(hyperlink: &Hyperlink, buf: &mut HtmlBuffer) {
    match &hyperlink.target {
        HyperlinkTarget::ExternalUrl(url) => {
            let normalized_url = url.trim();
            if !is_safe_url(normalized_url) {
                // Unsafe URL: emit content as plain text (security fallback)
                for run in &hyperlink.runs {
                    run_to_html(run, buf);
                }
                return;
            }
            let escaped_url = escape_attribute(normalized_url);
            buf.push_raw(&format!("<a href=\"{}\" class=\"rtf-link\">", escaped_url));
            for run in &hyperlink.runs {
                run_to_html(run, buf);
            }
            buf.push_raw("</a>");
        }
        HyperlinkTarget::InternalBookmark(name) => {
            // Internal anchors are always safe — no scheme injection possible
            let escaped_name = escape_attribute(name);
            buf.push_raw(&format!(
                "<a href=\"#{}\" class=\"rtf-link\">",
                escaped_name
            ));
            for run in &hyperlink.runs {
                run_to_html(run, buf);
            }
            buf.push_raw("</a>");
        }
    }
}

/// Converts a run to HTML with semantic tags.
///
/// Uses the stable nesting order:
/// `span[style] -> strong -> em -> span.rtf-u -> span.rtf-s -> span.rtf-sc -> span.rtf-ac`
/// Per the spec:
/// - font/color/size -> `<span style="...">`
/// - bold -> `<strong>`
/// - italic -> `<em>`
/// - underline -> `<span class="rtf-u">`
/// - strikethrough -> `<span class="rtf-s">`
/// - small caps -> `<span class="rtf-sc">`
/// - all caps -> `<span class="rtf-ac">`
pub fn run_to_html(run: &Run, buf: &mut HtmlBuffer) {
    // Handle empty runs
    if run.text.is_empty() {
        return;
    }

    // Build style string for font properties (deterministic order)
    let style = build_run_style(run);
    let has_style = !style.is_empty();

    // Open tags in stable order: span[style] -> strong -> em -> span.rtf-u -> span.rtf-s -> span.rtf-sc -> span.rtf-ac
    if has_style {
        buf.push_open_tag("span", &[("style", style.as_str())]);
    }
    if run.bold {
        buf.push_raw("<strong>");
    }
    if run.italic {
        buf.push_raw("<em>");
    }
    if run.underline {
        buf.push_raw("<span class=\"rtf-u\">");
    }
    if run.strikethrough {
        buf.push_raw("<span class=\"rtf-s\">");
    }
    if run.small_caps {
        buf.push_raw("<span class=\"rtf-sc\">");
    }
    if run.all_caps {
        buf.push_raw("<span class=\"rtf-ac\">");
    }

    // Emit escaped text content
    buf.push_text(&run.text);

    // Close tags in reverse order: span.rtf-ac -> span.rtf-sc -> span.rtf-s -> span.rtf-u -> em -> strong -> span[style]
    if run.all_caps {
        buf.push_raw("</span>");
    }
    if run.small_caps {
        buf.push_raw("</span>");
    }
    if run.strikethrough {
        buf.push_raw("</span>");
    }
    if run.underline {
        buf.push_raw("</span>");
    }
    if run.italic {
        buf.push_raw("</em>");
    }
    if run.bold {
        buf.push_raw("</strong>");
    }
    if has_style {
        buf.push_raw("</span>");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_paragraph() {
        let para = Paragraph::new();
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), r#"<p class="rtf-p"></p>"#);
    }

    #[test]
    fn simple_paragraph() {
        let para = Paragraph::from_runs(vec![Run::new("Hello")]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), r#"<p class="rtf-p">Hello</p>"#);
    }

    #[test]
    fn paragraph_with_bold() {
        let mut run = Run::new("bold");
        run.bold = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><strong>bold</strong></p>"#
        );
    }

    #[test]
    fn paragraph_with_italic() {
        let mut run = Run::new("italic");
        run.italic = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), r#"<p class="rtf-p"><em>italic</em></p>"#);
    }

    #[test]
    fn paragraph_with_underline() {
        let mut run = Run::new("underline");
        run.underline = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span class="rtf-u">underline</span></p>"#
        );
    }

    #[test]
    fn paragraph_with_strikethrough() {
        let mut run = Run::new("strike");
        run.strikethrough = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span class="rtf-s">strike</span></p>"#
        );
    }

    #[test]
    fn paragraph_with_small_caps() {
        let mut run = Run::new("small caps");
        run.small_caps = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span class="rtf-sc">small caps</span></p>"#
        );
    }

    #[test]
    fn paragraph_with_all_caps() {
        let mut run = Run::new("all caps");
        run.all_caps = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span class="rtf-ac">all caps</span></p>"#
        );
    }

    #[test]
    fn paragraph_with_nested_formatting() {
        let mut run = Run::new("bold italic underline");
        run.bold = true;
        run.italic = true;
        run.underline = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Nesting order: strong -> em -> span.rtf-u
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><strong><em><span class="rtf-u">bold italic underline</span></em></strong></p>"#
        );
    }

    #[test]
    fn paragraph_with_extended_nested_formatting() {
        let mut run = Run::new("all text styles");
        run.bold = true;
        run.italic = true;
        run.underline = true;
        run.strikethrough = true;
        run.small_caps = true;
        run.all_caps = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><strong><em><span class="rtf-u"><span class="rtf-s"><span class="rtf-sc"><span class="rtf-ac">all text styles</span></span></span></span></em></strong></p>"#
        );
    }

    #[test]
    fn paragraph_with_center_alignment() {
        let para = Paragraph {
            alignment: Alignment::Center,
            inlines: vec![Inline::Run(Run::new("centered"))],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p rtf-align-center">centered</p>"#
        );
    }

    #[test]
    fn paragraph_with_right_alignment() {
        let para = Paragraph {
            alignment: Alignment::Right,
            inlines: vec![Inline::Run(Run::new("right"))],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p rtf-align-right">right</p>"#
        );
    }

    #[test]
    fn paragraph_with_justify_alignment() {
        let para = Paragraph {
            alignment: Alignment::Justify,
            inlines: vec![Inline::Run(Run::new("justified"))],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p rtf-align-justify">justified</p>"#
        );
    }

    #[test]
    fn paragraph_with_left_alignment_no_class() {
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Run(Run::new("left"))],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Left is default, only rtf-p class should be emitted
        assert_eq!(buf.as_str(), r#"<p class="rtf-p">left</p>"#);
    }

    #[test]
    fn text_escaping() {
        let para = Paragraph::from_runs(vec![Run::new("<script>alert('xss')</script>")]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Text should be escaped
        let result = buf.as_str();
        assert!(result.contains("&lt;script&gt;"));
        assert!(result.contains("&lt;/script&gt;"));
        assert!(result.contains("&#39;xss&#39;"));
    }

    #[test]
    fn consecutive_runs_same_formatting() {
        let run1 = Run::new("Hello ");
        let run2 = Run::new("World");
        let para = Paragraph::from_runs(vec![run1, run2]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Runs are emitted separately (no merging at paragraph level)
        assert_eq!(buf.as_str(), r#"<p class="rtf-p">Hello World</p>"#);
    }

    #[test]
    fn consecutive_runs_different_formatting() {
        let run1 = Run::new("Hello ");
        let mut run2 = Run::new("World");
        run2.bold = true;
        let para = Paragraph::from_runs(vec![run1, run2]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Different formatting - separate elements
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p">Hello <strong>World</strong></p>"#
        );
    }

    #[test]
    fn mixed_formatting_runs() {
        let run1 = Run::new("normal ");
        let mut run2 = Run::new("bold ");
        run2.bold = true;
        let mut run3 = Run::new("italic");
        run3.italic = true;
        let para = Paragraph::from_runs(vec![run1, run2, run3]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p">normal <strong>bold </strong><em>italic</em></p>"#
        );
    }

    #[test]
    fn consecutive_bold_runs() {
        let mut run1 = Run::new("Hello ");
        run1.bold = true;
        let mut run2 = Run::new("World");
        run2.bold = true;
        let para = Paragraph::from_runs(vec![run1, run2]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Both runs are bold - emitted as separate strong elements
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><strong>Hello </strong><strong>World</strong></p>"#
        );
    }

    // =========================================================================
    // Hyperlink tests
    // =========================================================================

    #[test]
    fn simple_hyperlink() {
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![Run::new("Example Site")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><a href="https://example.com" class="rtf-link">Example Site</a></p>"#
        );
    }

    #[test]
    fn hyperlink_with_formatted_text() {
        let mut bold_run = Run::new("Bold");
        bold_run.bold = true;
        let mut italic_run = Run::new("Italic");
        italic_run.italic = true;
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![bold_run, italic_run],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><a href="https://example.com" class="rtf-link"><strong>Bold</strong><em>Italic</em></a></p>"#
        );
    }

    #[test]
    fn hyperlink_mixed_with_text() {
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![Run::new("link")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![
                Inline::Run(Run::new("Visit ")),
                Inline::Hyperlink(hyperlink),
                Inline::Run(Run::new(" for more.")),
            ],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p">Visit <a href="https://example.com" class="rtf-link">link</a> for more.</p>"#
        );
    }

    #[test]
    fn hyperlink_javascript_blocked() {
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("javascript:alert('xss')".to_string()),
            runs: vec![Run::new("Evil Link")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // JavaScript URL should be blocked - content emitted as plain text
        assert_eq!(buf.as_str(), r#"<p class="rtf-p">Evil Link</p>"#);
    }

    #[test]
    fn hyperlink_data_uri_blocked() {
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl(
                "data:text/html,<script>alert('xss')</script>".to_string(),
            ),
            runs: vec![Run::new("Data Link")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Data URL should be blocked - content emitted as plain text
        assert_eq!(buf.as_str(), r#"<p class="rtf-p">Data Link</p>"#);
    }

    #[test]
    fn hyperlink_mailto_allowed() {
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("mailto:test@example.com".to_string()),
            runs: vec![Run::new("Email Us")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // mailto: should be allowed
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><a href="mailto:test@example.com" class="rtf-link">Email Us</a></p>"#
        );
    }

    #[test]
    fn hyperlink_url_escaping() {
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl(
                "https://example.com/search?q=hello&lang=en".to_string(),
            ),
            runs: vec![Run::new("Search")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // URL should be escaped in href attribute
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><a href="https://example.com/search?q=hello&amp;lang=en" class="rtf-link">Search</a></p>"#
        );
    }

    // =========================================================================
    // URL safety tests
    // =========================================================================

    #[test]
    fn safe_url_http() {
        assert!(is_safe_url("http://example.com"));
    }

    #[test]
    fn safe_url_https() {
        assert!(is_safe_url("https://example.com"));
    }

    #[test]
    fn safe_url_mailto() {
        assert!(is_safe_url("mailto:test@example.com"));
    }

    #[test]
    fn unsafe_url_relative() {
        assert!(!is_safe_url("/page"));
        assert!(!is_safe_url("page.html"));
    }

    #[test]
    fn unsafe_url_javascript() {
        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url("JAVASCRIPT:alert(1)"));
        assert!(!is_safe_url("JavaScript:alert(1)"));
    }

    #[test]
    fn unsafe_url_data() {
        assert!(!is_safe_url("data:text/html,<script>"));
        assert!(!is_safe_url("DATA:text/html,<script>"));
    }

    #[test]
    fn unsafe_url_vbscript() {
        assert!(!is_safe_url("vbscript:msgbox(1)"));
        assert!(!is_safe_url("VBSCRIPT:msgbox(1)"));
    }

    #[test]
    fn unsafe_url_with_leading_space() {
        assert!(!is_safe_url(" javascript:alert(1)"));
    }

    #[test]
    fn unsafe_url_ftp() {
        assert!(!is_safe_url("ftp://example.com/file"));
    }

    // =========================================================================
    // Font/Color/Size style tests
    // =========================================================================

    #[test]
    fn run_with_font_family_only() {
        let mut run = Run::new("styled text");
        run.font_family = Some("Arial".to_string());
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="font-family: &quot;Arial&quot;;">styled text</span></p>"#
        );
    }

    #[test]
    fn run_with_font_size_only() {
        let mut run = Run::new("sized text");
        run.font_size = Some(14.0);
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="font-size: 14pt;">sized text</span></p>"#
        );
    }

    #[test]
    fn run_with_color_only() {
        let mut run = Run::new("colored text");
        run.color = Some(Color::new(255, 0, 0));
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="color: #ff0000;">colored text</span></p>"#
        );
    }

    #[test]
    fn run_with_all_font_properties() {
        let mut run = Run::new("fully styled");
        run.font_family = Some("Helvetica".to_string());
        run.font_size = Some(12.0);
        run.color = Some(Color::new(0, 128, 255));
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Deterministic order: font-family, font-size, color
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="font-family: &quot;Helvetica&quot;; font-size: 12pt; color: #0080ff;">fully styled</span></p>"#
        );
    }

    #[test]
    fn run_with_bold_and_font() {
        let mut run = Run::new("bold styled");
        run.bold = true;
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(16.0);
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Nesting order: span[style] -> strong
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="font-family: &quot;Arial&quot;; font-size: 16pt;"><strong>bold styled</strong></span></p>"#
        );
    }

    #[test]
    fn run_with_italic_and_color() {
        let mut run = Run::new("italic colored");
        run.italic = true;
        run.color = Some(Color::new(128, 0, 128));
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="color: #800080;"><em>italic colored</em></span></p>"#
        );
    }

    #[test]
    fn run_with_all_formatting() {
        let mut run = Run::new("everything");
        run.bold = true;
        run.italic = true;
        run.underline = true;
        run.font_family = Some("Times New Roman".to_string());
        run.font_size = Some(18.0);
        run.color = Some(Color::new(0, 100, 0));
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Nesting order: span[style] -> strong -> em -> span.rtf-u
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="font-family: &quot;Times New Roman&quot;; font-size: 18pt; color: #006400;"><strong><em><span class="rtf-u">everything</span></em></strong></span></p>"#
        );
    }

    #[test]
    fn hyperlink_with_font_styling() {
        let mut run = Run::new("styled link");
        run.font_family = Some("Arial".to_string());
        run.color = Some(Color::new(0, 0, 255));
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![run],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><a href="https://example.com" class="rtf-link"><span style="font-family: &quot;Arial&quot;; color: #0000ff;">styled link</span></a></p>"#
        );
    }

    #[test]
    fn hyperlink_with_bold_and_font() {
        let mut run = Run::new("bold link");
        run.bold = true;
        run.font_family = Some("Verdana".to_string());
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![run],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><a href="https://example.com" class="rtf-link"><span style="font-family: &quot;Verdana&quot;;"><strong>bold link</strong></span></a></p>"#
        );
    }

    // =========================================================================
    // Font family sanitization security tests
    // =========================================================================

    #[test]
    fn font_family_injection_attack() {
        // Try to break out of CSS context
        let mut run = Run::new("text");
        run.font_family = Some("Arial\"; } body { display: none; /*".to_string());
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        let result = buf.as_str();
        // The font-family value inside the style attribute should be sanitized
        // The sanitized font name should be "Arial body  display none" (quotes, semicolons, braces stripped)
        // Dangerous characters should be stripped from the font-family value
        assert!(!result.contains("}"));
        assert!(!result.contains("{"));
        // The output should contain the sanitized font name parts
        assert!(result.contains("Arial"));
    }

    #[test]
    fn font_family_xss_attempt() {
        // Try to inject script tag via font name
        let mut run = Run::new("text");
        run.font_family = Some("</style><script>alert('xss')</script>".to_string());
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        let result = buf.as_str();
        // The font-family value should be sanitized - dangerous chars stripped
        // After sanitization, only "style script alert xss script" remains (no HTML chars)
        // Check that no style attribute contains dangerous content
        if result.contains("style=\"") {
            let style_value = result
                .split("style=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            // HTML characters should be stripped from the font-family value
            assert!(!style_value.contains("<"));
            assert!(!style_value.contains(">"));
            assert!(!style_value.contains("\""));
            assert!(!style_value.contains("'"));
        }
        // Either way, the output should be safe
        assert!(result.contains("text"));
    }

    #[test]
    fn font_family_with_backslash() {
        let mut run = Run::new("text");
        run.font_family = Some("Font\\Name".to_string());
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        let result = buf.as_str();
        // Backslash should be stripped
        assert!(!result.contains("\\"));
    }

    #[test]
    fn font_family_with_newline() {
        let mut run = Run::new("text");
        run.font_family = Some("Font\nName".to_string());
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        let result = buf.as_str();
        // Newline should be stripped
        assert!(!result.contains("\n"));
    }

    #[test]
    fn font_family_empty_after_sanitization() {
        // Font name with only dangerous characters
        let mut run = Run::new("text");
        run.font_family = Some("\"';{}<>".to_string());
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        let result = buf.as_str();
        // No style span should be emitted if font name is empty after sanitization
        assert!(!result.contains("style="));
        assert_eq!(result, r#"<p class="rtf-p">text</p>"#);
    }

    #[test]
    fn font_family_safe_characters_preserved() {
        // Test that safe characters are preserved
        let mut run = Run::new("text");
        run.font_family = Some("Open-Sans_2024".to_string());
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        let result = buf.as_str();
        // Hyphens, underscores, and numbers should be preserved
        assert!(result.contains("Open-Sans_2024"));
    }

    // =========================================================================
    // Determinism tests
    // =========================================================================

    #[test]
    fn style_output_is_deterministic() {
        let mut run = Run::new("test");
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(12.0);
        run.color = Some(Color::new(255, 0, 0));

        // Generate style string multiple times
        let style1 = build_run_style(&run);
        let style2 = build_run_style(&run);
        let style3 = build_run_style(&run);

        // All should be identical
        assert_eq!(style1, style2);
        assert_eq!(style2, style3);
        // And in the expected order
        assert_eq!(
            style1,
            "font-family: \"Arial\"; font-size: 12pt; color: #ff0000;"
        );
    }

    #[test]
    fn html_output_is_deterministic() {
        let mut run = Run::new("test");
        run.bold = true;
        run.font_family = Some("Helvetica".to_string());
        run.color = Some(Color::new(0, 128, 255));

        let para = Paragraph::from_runs(vec![run]);

        // Generate HTML multiple times
        let mut buf1 = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf1);

        let mut buf2 = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf2);

        // Output should be identical
        assert_eq!(buf1.as_str(), buf2.as_str());
    }

    // =========================================================================
    // Color helper tests
    // =========================================================================

    #[test]
    fn color_to_hex_red() {
        assert_eq!(
            color_to_hex(&Color::new(255, 0, 0)),
            format!("#{}", "ff0000")
        );
    }

    #[test]
    fn color_to_hex_green() {
        assert_eq!(
            color_to_hex(&Color::new(0, 255, 0)),
            format!("#{}", "00ff00")
        );
    }

    #[test]
    fn color_to_hex_blue() {
        assert_eq!(
            color_to_hex(&Color::new(0, 0, 255)),
            format!("#{}", "0000ff")
        );
    }

    #[test]
    fn color_to_hex_mixed() {
        assert_eq!(
            color_to_hex(&Color::new(123, 45, 67)),
            format!("#{}", "7b2d43")
        );
    }

    #[test]
    fn color_to_hex_black() {
        assert_eq!(color_to_hex(&Color::new(0, 0, 0)), format!("#{}", "000000"));
    }

    #[test]
    fn color_to_hex_white() {
        assert_eq!(
            color_to_hex(&Color::new(255, 255, 255)),
            format!("#{}", "ffffff")
        );
    }

    // =========================================================================
    // Background color tests
    // =========================================================================

    #[test]
    fn run_with_background_color_only() {
        let mut run = Run::new("highlighted text");
        run.background_color = Some(Color::new(255, 255, 0)); // Yellow
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="background-color: #ffff00;">highlighted text</span></p>"#
        );
    }

    #[test]
    fn run_without_background_color_no_style() {
        let mut run = Run::new("normal text");
        run.color = Some(Color::new(255, 0, 0)); // Only foreground color
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Should NOT contain background-color style
        let result = buf.as_str();
        assert!(!result.contains("background-color"));
        assert!(result.contains("color: #ff0000"));
    }

    #[test]
    fn run_with_foreground_and_background_color() {
        let mut run = Run::new("colored text");
        run.color = Some(Color::new(255, 0, 0)); // Red foreground
        run.background_color = Some(Color::new(255, 255, 0)); // Yellow background
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Both colors should be present in deterministic order: color, then background-color
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="color: #ff0000; background-color: #ffff00;">colored text</span></p>"#
        );
    }

    #[test]
    fn background_color_deterministic_style_order() {
        let mut run = Run::new("fully styled");
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(12.0);
        run.color = Some(Color::new(255, 0, 0));
        run.background_color = Some(Color::new(0, 0, 255)); // Blue background

        // Generate style string multiple times
        let style1 = build_run_style(&run);
        let style2 = build_run_style(&run);
        let style3 = build_run_style(&run);

        // All should be identical
        assert_eq!(style1, style2);
        assert_eq!(style2, style3);
        // And in the expected order: font-family, font-size, color, background-color
        assert_eq!(
            style1,
            "font-family: \"Arial\"; font-size: 12pt; color: #ff0000; background-color: #0000ff;"
        );
    }

    #[test]
    fn background_color_with_all_properties() {
        let mut run = Run::new("everything");
        run.bold = true;
        run.italic = true;
        run.underline = true;
        run.font_family = Some("Times New Roman".to_string());
        run.font_size = Some(18.0);
        run.color = Some(Color::new(0, 100, 0));
        run.background_color = Some(Color::new(255, 200, 100));
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Nesting order: span[style] -> strong -> em -> span.rtf-u
        // Style order: font-family, font-size, color, background-color
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><span style="font-family: &quot;Times New Roman&quot;; font-size: 18pt; color: #006400; background-color: #ffc864;"><strong><em><span class="rtf-u">everything</span></em></strong></span></p>"#
        );
    }

    #[test]
    fn hyperlink_with_background_color() {
        let mut run = Run::new("highlighted link");
        run.background_color = Some(Color::new(255, 255, 0));
        let hyperlink = Hyperlink {
            target: HyperlinkTarget::ExternalUrl("https://example.com".to_string()),
            runs: vec![run],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p"><a href="https://example.com" class="rtf-link"><span style="background-color: #ffff00;">highlighted link</span></a></p>"#
        );
    }

    // =========================================================================
    // Paragraph shading tests
    // =========================================================================

    #[test]
    fn paragraph_with_shading() {
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Run(Run::new("shaded paragraph"))],
            shading: Some(Shading::solid(Color::new(255, 255, 0))), // Yellow
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p" style="background-color: #ffff00;">shaded paragraph</p>"#
        );
    }

    #[test]
    fn paragraph_with_shading_and_alignment() {
        let para = Paragraph {
            alignment: Alignment::Center,
            inlines: vec![Inline::Run(Run::new("centered and shaded"))],
            shading: Some(Shading::solid(Color::new(0, 128, 255))), // Blue
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Attributes in order: class, style
        assert_eq!(
            buf.as_str(),
            r#"<p class="rtf-p rtf-align-center" style="background-color: #0080ff;">centered and shaded</p>"#
        );
    }

    #[test]
    fn paragraph_without_shading_no_style() {
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Run(Run::new("normal paragraph"))],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Should NOT have style attribute
        let result = buf.as_str();
        assert!(!result.contains("style="));
        assert_eq!(result, r#"<p class="rtf-p">normal paragraph</p>"#);
    }

    #[test]
    fn semantic_ref_renders_as_internal_link() {
        use rtfkit_core::SemanticField;
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::SemanticField(SemanticField::new(SemanticFieldRef::Ref {
                target: "sec_intro".to_string(),
                fallback_text: Some("Introduction".to_string()),
            }))],
            shading: None,
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(
            buf.as_str(),
            r##"<p class="rtf-p"><a href="#sec_intro" class="rtf-link rtf-semantic-field" data-field="ref">Introduction</a></p>"##
        );
    }

    #[test]
    fn paragraph_shading_empty_fill_color_no_style() {
        // Shading with no fill_color should not emit style
        let shading = Shading::new(); // Empty shading
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Run(Run::new("text"))],
            shading: Some(shading),
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Should NOT have style attribute
        let result = buf.as_str();
        assert!(!result.contains("style="));
        assert_eq!(result, r#"<p class="rtf-p">text</p>"#);
    }

    #[test]
    fn paragraph_shading_deterministic() {
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Run(Run::new("test"))],
            shading: Some(Shading::solid(Color::new(255, 0, 128))),
        };

        // Generate HTML multiple times
        let mut buf1 = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf1);

        let mut buf2 = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf2);

        // Output should be identical
        assert_eq!(buf1.as_str(), buf2.as_str());
        assert!(buf1.as_str().contains("background-color: #ff0080"));
    }

    #[test]
    fn build_paragraph_style_helper() {
        use rtfkit_core::ShadingPattern;

        // Test the helper function directly
        let shading = Shading::solid(Color::new(0, 255, 0));
        let style = build_paragraph_style(Some(&shading));
        assert_eq!(style, "background-color: #00ff00;");

        let mut percent_shading = Shading::new();
        percent_shading.fill_color = Some(Color::new(255, 255, 255));
        percent_shading.pattern_color = Some(Color::new(0, 0, 0));
        percent_shading.pattern = Some(ShadingPattern::Percent50);
        let style = build_paragraph_style(Some(&percent_shading));
        assert_eq!(style, "background-color: #808080;");

        // Empty shading
        let empty_shading = Shading::new();
        let style = build_paragraph_style(Some(&empty_shading));
        assert_eq!(style, "");

        // None
        let style = build_paragraph_style(None);
        assert_eq!(style, "");
    }

    // =========================================================================
    // Pattern degradation tests (Slice B)
    // =========================================================================

    #[test]
    fn paragraph_with_percent_pattern_shading_uses_blended_fill() {
        use rtfkit_core::ShadingPattern;

        // Create a patterned shading (25% pattern with black on white)
        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 255)); // White background
        shading.pattern_color = Some(Color::new(0, 0, 0)); // Black foreground
        shading.pattern = Some(ShadingPattern::Percent25);

        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Run(Run::new("patterned text"))],
            shading: Some(shading),
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);

        // Percent pattern should be approximated as blended fill.
        let result = buf.as_str();
        assert!(result.contains("background-color: #bfbfbf"));
        assert_eq!(
            result,
            r#"<p class="rtf-p" style="background-color: #bfbfbf;">patterned text</p>"#
        );
    }

    #[test]
    fn paragraph_with_horz_stripe_pattern_degrades_to_flat_fill() {
        use rtfkit_core::ShadingPattern;

        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(200, 200, 200)); // Light gray
        shading.pattern_color = Some(Color::new(100, 100, 100)); // Dark gray
        shading.pattern = Some(ShadingPattern::HorzStripe);

        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Run(Run::new("striped text"))],
            shading: Some(shading),
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);

        // Only fill_color should be emitted
        let result = buf.as_str();
        assert!(result.contains("background-color: #c8c8c8"));
        assert!(!result.contains(&format!("#{}", "646464"))); // Pattern color not emitted
    }

    #[test]
    fn paragraph_with_diag_cross_pattern_degrades_to_flat_fill() {
        use rtfkit_core::ShadingPattern;

        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 0)); // Yellow
        shading.pattern_color = Some(Color::new(255, 0, 0)); // Red
        shading.pattern = Some(ShadingPattern::DiagCross);

        let para = Paragraph {
            alignment: Alignment::Center,
            inlines: vec![Inline::Run(Run::new("crosshatch"))],
            shading: Some(shading),
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);

        // Only fill_color should be emitted, pattern ignored
        let result = buf.as_str();
        assert!(result.contains("background-color: #ffff00"));
        assert!(!result.contains(&format!("#{}", "ff0000"))); // Pattern color not emitted
    }
}
