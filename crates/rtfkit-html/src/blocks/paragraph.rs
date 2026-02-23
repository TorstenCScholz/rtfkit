//! Paragraph block HTML emission.
//!
//! This module handles converting IR paragraphs to HTML with proper
//! semantic tags and run merging.

use rtfkit_core::{Hyperlink, Inline, Paragraph, Run};

use crate::escape::escape_attribute;
use crate::serialize::HtmlBuffer;
use crate::style::alignment_class;
#[cfg(test)]
use rtfkit_core::Alignment;

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
pub fn paragraph_to_html(para: &Paragraph, buf: &mut HtmlBuffer) {
    // Build class attribute - always include rtf-p, add alignment if non-default
    let mut classes: Vec<&'static str> = vec!["rtf-p"];
    if let Some(align_class) = alignment_class(para.alignment) {
        classes.push(align_class);
    }
    let class_str = classes.join(" ");
    let attrs: Vec<(&str, &str)> = vec![("class", &class_str)];

    buf.push_open_tag("p", &attrs);

    // Emit inlines in order, handling hyperlinks specially
    for inline in &para.inlines {
        match inline {
            Inline::Run(run) => {
                run_to_html(run, buf);
            }
            Inline::Hyperlink(hyperlink) => {
                hyperlink_to_html(hyperlink, buf);
            }
        }
    }

    buf.push_close_tag("p");
}

/// Converts a hyperlink to HTML.
///
/// Emits an `<a href="...">` element with `rtf-link` class.
/// URLs are sanitized to reject dangerous schemes (javascript:, data:, vbscript:).
/// If the URL is unsafe, the hyperlink content is emitted as plain text.
fn hyperlink_to_html(hyperlink: &Hyperlink, buf: &mut HtmlBuffer) {
    let normalized_url = hyperlink.url.trim();

    // Check if URL is safe
    if !is_safe_url(normalized_url) {
        // Unsafe URL: emit content as plain text (security fallback)
        for run in &hyperlink.runs {
            run_to_html(run, buf);
        }
        return;
    }

    // Safe URL: emit proper hyperlink
    let escaped_url = escape_attribute(normalized_url);
    buf.push_raw(&format!("<a href=\"{}\" class=\"rtf-link\">", escaped_url));

    // Emit runs inside the hyperlink
    for run in &hyperlink.runs {
        run_to_html(run, buf);
    }

    buf.push_raw("</a>");
}

/// Converts a run to HTML with semantic tags.
///
/// Uses the stable nesting order: `strong -> em -> span.rtf-u`
/// Per the spec:
/// - bold -> `<strong>`
/// - italic -> `<em>`
/// - underline -> `<span class="rtf-u">`
pub fn run_to_html(run: &Run, buf: &mut HtmlBuffer) {
    // Handle empty runs
    if run.text.is_empty() {
        return;
    }

    // Open tags in stable order: strong -> em -> span.rtf-u
    if run.bold {
        buf.push_raw("<strong>");
    }
    if run.italic {
        buf.push_raw("<em>");
    }
    if run.underline {
        buf.push_raw("<span class=\"rtf-u\">");
    }

    // Emit escaped text content
    buf.push_text(&run.text);

    // Close tags in reverse order: span.rtf-u -> em -> strong
    if run.underline {
        buf.push_raw("</span>");
    }
    if run.italic {
        buf.push_raw("</em>");
    }
    if run.bold {
        buf.push_raw("</strong>");
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
    fn paragraph_with_center_alignment() {
        let para = Paragraph {
            alignment: Alignment::Center,
            inlines: vec![Inline::Run(Run::new("centered"))],
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
            url: "https://example.com".to_string(),
            runs: vec![Run::new("Example Site")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
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
            url: "https://example.com".to_string(),
            runs: vec![bold_run, italic_run],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
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
            url: "https://example.com".to_string(),
            runs: vec![Run::new("link")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![
                Inline::Run(Run::new("Visit ")),
                Inline::Hyperlink(hyperlink),
                Inline::Run(Run::new(" for more.")),
            ],
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
            url: "javascript:alert('xss')".to_string(),
            runs: vec![Run::new("Evil Link")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // JavaScript URL should be blocked - content emitted as plain text
        assert_eq!(buf.as_str(), r#"<p class="rtf-p">Evil Link</p>"#);
    }

    #[test]
    fn hyperlink_data_uri_blocked() {
        let hyperlink = Hyperlink {
            url: "data:text/html,<script>alert('xss')</script>".to_string(),
            runs: vec![Run::new("Data Link")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Data URL should be blocked - content emitted as plain text
        assert_eq!(buf.as_str(), r#"<p class="rtf-p">Data Link</p>"#);
    }

    #[test]
    fn hyperlink_mailto_allowed() {
        let hyperlink = Hyperlink {
            url: "mailto:test@example.com".to_string(),
            runs: vec![Run::new("Email Us")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
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
            url: "https://example.com/search?q=hello&lang=en".to_string(),
            runs: vec![Run::new("Search")],
        };
        let para = Paragraph {
            alignment: Alignment::Left,
            inlines: vec![Inline::Hyperlink(hyperlink)],
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
}
