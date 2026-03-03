//! Shared heading inference helpers for TOC and renderers.
//!
//! These helpers provide a single, deterministic heading-candidate contract so
//! parser/finalizer and backend mappers do not drift over time.

use crate::{Inline, Paragraph};

/// Configuration for heading inference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadingInferenceOptions {
    /// Require the first visible character to be ASCII digit/uppercase.
    pub require_leading_marker: bool,
    /// Allow bookmark-anchor-bearing paragraphs to be inferred as headings.
    pub allow_bookmark_anchors: bool,
    /// Minimum detected font size (pt) required to infer heading.
    pub min_font_size_pt: f32,
    /// Maximum visible text length to still be considered a heading.
    pub max_text_len: usize,
}

impl Default for HeadingInferenceOptions {
    fn default() -> Self {
        Self {
            require_leading_marker: true,
            allow_bookmark_anchors: false,
            min_font_size_pt: 13.0,
            max_text_len: 140,
        }
    }
}

/// Extract plain visible heading text from runs and hyperlink runs.
pub fn extract_heading_plain_text(paragraph: &Paragraph) -> String {
    let mut text = String::new();
    for inline in &paragraph.inlines {
        match inline {
            Inline::Run(run) => text.push_str(&run.text),
            Inline::Hyperlink(link) => {
                for run in &link.runs {
                    text.push_str(&run.text);
                }
            }
            Inline::BookmarkAnchor(_)
            | Inline::NoteRef(_)
            | Inline::PageField(_)
            | Inline::GeneratedBlockMarker(_) => {}
        }
    }
    text
}

/// Infer heading level from paragraph content and formatting.
///
/// Current policy only emits level `1` for inferred headings.
pub fn infer_heading_level(paragraph: &Paragraph) -> Option<u8> {
    infer_heading_level_with_options(paragraph, HeadingInferenceOptions::default())
}

/// Infer heading level with explicit policy options.
pub fn infer_heading_level_with_options(
    paragraph: &Paragraph,
    options: HeadingInferenceOptions,
) -> Option<u8> {
    let mut has_bold = false;
    let mut max_size = 0.0_f32;
    let mut has_bookmark_anchor = false;

    for inline in &paragraph.inlines {
        match inline {
            Inline::Run(run) => {
                has_bold |= run.bold;
                if let Some(size) = run.font_size {
                    max_size = max_size.max(size);
                }
            }
            Inline::Hyperlink(link) => {
                for run in &link.runs {
                    has_bold |= run.bold;
                    if let Some(size) = run.font_size {
                        max_size = max_size.max(size);
                    }
                }
            }
            Inline::BookmarkAnchor(_) => {
                has_bookmark_anchor = true;
            }
            Inline::NoteRef(_) | Inline::PageField(_) | Inline::GeneratedBlockMarker(_) => {}
        }
    }

    if !options.allow_bookmark_anchors && has_bookmark_anchor {
        return None;
    }

    let text = extract_heading_plain_text(paragraph);
    let trimmed = text.trim();

    if trimmed.is_empty() || trimmed.len() > options.max_text_len {
        return None;
    }
    if !(has_bold && max_size >= options.min_font_size_pt) {
        return None;
    }
    if options.require_leading_marker
        && !trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
    {
        return None;
    }

    Some(1)
}

/// Predicate helper used by TOC fallback heuristics.
pub fn paragraph_looks_like_heading(paragraph: &Paragraph) -> bool {
    infer_heading_level(paragraph).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BookmarkAnchor, Hyperlink, HyperlinkTarget, Paragraph, Run};

    #[test]
    fn extracts_plain_text_from_runs_and_hyperlinks() {
        let paragraph = Paragraph::from_inlines(vec![
            Inline::Run(Run::new("2. ")),
            Inline::Hyperlink(Hyperlink {
                target: HyperlinkTarget::ExternalUrl("https://example.com".into()),
                runs: vec![Run::new("Operational"), Run::new(" Performance")],
            }),
        ]);
        assert_eq!(
            extract_heading_plain_text(&paragraph),
            "2. Operational Performance"
        );
    }

    #[test]
    fn infers_heading_for_numbered_bold_large_text() {
        let mut run = Run::new("2. Operational Performance");
        run.bold = true;
        run.font_size = Some(13.0);
        let paragraph = Paragraph::from_runs(vec![run]);
        assert_eq!(infer_heading_level(&paragraph), Some(1));
    }

    #[test]
    fn rejects_heading_with_bookmark_anchor_by_default() {
        let mut run = Run::new("2. Operational Performance");
        run.bold = true;
        run.font_size = Some(13.0);
        let paragraph = Paragraph::from_inlines(vec![
            Inline::BookmarkAnchor(BookmarkAnchor {
                name: "section-2".into(),
            }),
            Inline::Run(run),
        ]);
        assert_eq!(infer_heading_level(&paragraph), None);
    }

    #[test]
    fn can_allow_bookmark_anchor_via_options() {
        let mut run = Run::new("2. Operational Performance");
        run.bold = true;
        run.font_size = Some(13.0);
        let paragraph = Paragraph::from_inlines(vec![
            Inline::BookmarkAnchor(BookmarkAnchor {
                name: "section-2".into(),
            }),
            Inline::Run(run),
        ]);
        let options = HeadingInferenceOptions {
            allow_bookmark_anchors: true,
            ..HeadingInferenceOptions::default()
        };
        assert_eq!(
            infer_heading_level_with_options(&paragraph, options),
            Some(1)
        );
    }
}
