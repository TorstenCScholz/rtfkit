//! Paragraph block HTML emission.
//!
//! This module handles converting IR paragraphs to HTML with proper
//! semantic tags and run merging.

use rtfkit_core::{Paragraph, Run};

#[cfg(test)]
use rtfkit_core::Alignment;
use crate::serialize::HtmlBuffer;
use crate::style::alignment_class;

/// Converts a paragraph to HTML.
///
/// Emits a `<p>` element with appropriate alignment class if needed.
/// Runs are merged when they have identical formatting to reduce noise.
pub fn paragraph_to_html(para: &Paragraph, buf: &mut HtmlBuffer) {
    // Merge adjacent runs with identical formatting
    let merged_runs = merge_runs(&para.runs);
    
    // Build class attribute if alignment is non-default
    let attrs: Vec<(&str, &str)> = alignment_class(para.alignment)
        .map(|class| vec![("class", class)])
        .unwrap_or_default();
    
    buf.push_open_tag("p", &attrs);
    
    // Emit merged runs
    for run in &merged_runs {
        run_to_html(run, buf);
    }
    
    buf.push_close_tag("p");
}

/// Merges adjacent runs with identical formatting.
///
/// This reduces noisy output by combining consecutive runs that have
/// the same bold, italic, underline, font_size, and color attributes.
fn merge_runs(runs: &[Run]) -> Vec<Run> {
    if runs.is_empty() {
        return Vec::new();
    }
    
    let mut result = Vec::new();
    let mut current = runs[0].clone();
    
    for run in runs.iter().skip(1) {
        // Check if formatting is identical
        if current.bold == run.bold
            && current.italic == run.italic
            && current.underline == run.underline
            && current.font_size == run.font_size
            && current.color == run.color
        {
            // Merge: append text to current run
            current.text.push_str(&run.text);
        } else {
            // Different formatting: push current and start new
            result.push(current);
            current = run.clone();
        }
    }
    
    // Don't forget the last run
    result.push(current);
    result
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
        assert_eq!(buf.as_str(), "<p></p>");
    }

    #[test]
    fn simple_paragraph() {
        let para = Paragraph::from_runs(vec![Run::new("Hello")]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p>Hello</p>");
    }

    #[test]
    fn paragraph_with_bold() {
        let mut run = Run::new("bold");
        run.bold = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p><strong>bold</strong></p>");
    }

    #[test]
    fn paragraph_with_italic() {
        let mut run = Run::new("italic");
        run.italic = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p><em>italic</em></p>");
    }

    #[test]
    fn paragraph_with_underline() {
        let mut run = Run::new("underline");
        run.underline = true;
        let para = Paragraph::from_runs(vec![run]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p><span class=\"rtf-u\">underline</span></p>");
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
        assert_eq!(buf.as_str(), "<p><strong><em><span class=\"rtf-u\">bold italic underline</span></em></strong></p>");
    }

    #[test]
    fn paragraph_with_center_alignment() {
        let para = Paragraph {
            alignment: Alignment::Center,
            runs: vec![Run::new("centered")],
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p class=\"rtf-align-center\">centered</p>");
    }

    #[test]
    fn paragraph_with_right_alignment() {
        let para = Paragraph {
            alignment: Alignment::Right,
            runs: vec![Run::new("right")],
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p class=\"rtf-align-right\">right</p>");
    }

    #[test]
    fn paragraph_with_justify_alignment() {
        let para = Paragraph {
            alignment: Alignment::Justify,
            runs: vec![Run::new("justified")],
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p class=\"rtf-align-justify\">justified</p>");
    }

    #[test]
    fn paragraph_with_left_alignment_no_class() {
        let para = Paragraph {
            alignment: Alignment::Left,
            runs: vec![Run::new("left")],
        };
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Left is default, no class should be emitted
        assert_eq!(buf.as_str(), "<p>left</p>");
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
    fn run_merging_identical_formatting() {
        let run1 = Run::new("Hello ");
        let run2 = Run::new("World");
        let para = Paragraph::from_runs(vec![run1, run2]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Should merge into single text node
        assert_eq!(buf.as_str(), "<p>Hello World</p>");
    }

    #[test]
    fn run_merging_different_formatting() {
        let run1 = Run::new("Hello ");
        let mut run2 = Run::new("World");
        run2.bold = true;
        let para = Paragraph::from_runs(vec![run1, run2]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Should NOT merge - different formatting
        assert_eq!(buf.as_str(), "<p>Hello <strong>World</strong></p>");
    }

    #[test]
    fn run_merging_mixed_formatting() {
        let run1 = Run::new("normal ");
        let mut run2 = Run::new("bold ");
        run2.bold = true;
        let mut run3 = Run::new("italic");
        run3.italic = true;
        let para = Paragraph::from_runs(vec![run1, run2, run3]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        assert_eq!(buf.as_str(), "<p>normal <strong>bold </strong><em>italic</em></p>");
    }

    #[test]
    fn run_merging_consecutive_bold() {
        let mut run1 = Run::new("Hello ");
        run1.bold = true;
        let mut run2 = Run::new("World");
        run2.bold = true;
        let para = Paragraph::from_runs(vec![run1, run2]);
        let mut buf = HtmlBuffer::new();
        paragraph_to_html(&para, &mut buf);
        // Should merge into single strong element
        assert_eq!(buf.as_str(), "<p><strong>Hello World</strong></p>");
    }

    #[test]
    fn merge_runs_empty() {
        let runs: Vec<Run> = Vec::new();
        let merged = merge_runs(&runs);
        assert!(merged.is_empty());
    }

    #[test]
    fn merge_runs_single() {
        let runs = vec![Run::new("single")];
        let merged = merge_runs(&runs);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "single");
    }

    #[test]
    fn merge_runs_all_same() {
        let runs = vec![Run::new("a"), Run::new("b"), Run::new("c")];
        let merged = merge_runs(&runs);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "abc");
    }

    #[test]
    fn merge_runs_alternating() {
        let mut run1 = Run::new("a");
        run1.bold = true;
        let run2 = Run::new("b");
        let mut run3 = Run::new("c");
        run3.bold = true;
        let runs = vec![run1, run2, run3];
        let merged = merge_runs(&runs);
        // run1 and run3 should NOT merge (run2 in between with different formatting)
        assert_eq!(merged.len(), 3);
    }
}
