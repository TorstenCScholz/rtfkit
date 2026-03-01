//! Field and Hyperlink Tests
//!
//! Tests for RTF field/hyperlink handling including:
//! - Supported URL schemes
//! - Malformed/non-hyperlink field degradation
//! - Nested fields

use crate::rtf::parse;
use crate::{Block, GeneratedBlockKind, HyperlinkTarget, Inline, PageFieldRef, Warning};

// =============================================================================
// Simple Hyperlink Tests
// =============================================================================

#[test]
fn test_simple_hyperlink() {
    let input = include_str!("../../../../../fixtures/hyperlink_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have a hyperlink
    let has_hyperlink = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .any(|i| matches!(i, Inline::Hyperlink(_)));

    assert!(has_hyperlink, "Expected a hyperlink in the document");
}

#[test]
fn test_hyperlink_url_extraction() {
    let input = include_str!("../../../../../fixtures/hyperlink_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // Target should be an external URL
        assert!(matches!(&hlink.target, HyperlinkTarget::ExternalUrl(url) if !url.is_empty()));
    }
}

// =============================================================================
// Formatted Hyperlink Tests
// =============================================================================

#[test]
fn test_hyperlink_with_formatting() {
    let input = include_str!("../../../../../fixtures/hyperlink_formatted.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Find the hyperlink
    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // Should have runs with formatting
        assert!(!hlink.runs.is_empty());
    }
}

// =============================================================================
// Multiple Hyperlinks Tests
// =============================================================================

#[test]
fn test_multiple_hyperlinks() {
    let input = include_str!("../../../../../fixtures/hyperlink_multiple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Count hyperlinks
    let hyperlink_count = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .filter(|i| matches!(i, Inline::Hyperlink(_)))
        .count();

    assert!(hyperlink_count >= 2, "Expected at least 2 hyperlinks");
}

// =============================================================================
// Hyperlink in Table Tests
// =============================================================================

#[test]
fn test_hyperlink_in_table() {
    let input = include_str!("../../../../../fixtures/hyperlink_in_table.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have a table
    assert!(doc.blocks.iter().any(|b| matches!(b, Block::TableBlock(_))));
}

// =============================================================================
// URL Scheme Tests
// =============================================================================

#[test]
fn test_http_url_scheme() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        assert!(
            matches!(&hlink.target, HyperlinkTarget::ExternalUrl(url) if url.starts_with("http://") || url.starts_with("https://"))
        );
    }
}

#[test]
fn test_https_url_scheme() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        assert!(
            matches!(&hlink.target, HyperlinkTarget::ExternalUrl(url) if url.starts_with("https://"))
        );
    }
}

#[test]
fn test_mailto_url_scheme() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "mailto:test@example.com"}{\fldrslt Email}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        assert!(
            matches!(&hlink.target, HyperlinkTarget::ExternalUrl(url) if url.starts_with("mailto:"))
        );
    }
}

// =============================================================================
// Malformed Field Tests
// =============================================================================

#[test]
fn test_missing_fldrslt() {
    let input = include_str!("../../../../../fixtures/hyperlink_missing_fldrslt.rtf");
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

#[test]
fn test_unsupported_field_type() {
    let input = include_str!("../../../../../fixtures/hyperlink_unsupported_field.rtf");
    let result = parse(input);

    // Should parse gracefully, ignoring unsupported field
    assert!(result.is_ok());
}

#[test]
fn test_malformed_field_instruction() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst INVALID}{\fldrslt Text}}}"#;
    let result = parse(input);

    // Should parse, treating as regular text
    assert!(result.is_ok());
}

// =============================================================================
// Nested Field Tests
// =============================================================================

#[test]
fn test_nested_fields() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt {\field{\*\fldinst HYPERLINK "http://nested.com"}{\fldrslt Nested}}}}}"#;
    let result = parse(input);

    // Should handle nested fields
    assert!(result.is_ok());
}

// =============================================================================
// Field Result Tests
// =============================================================================

#[test]
fn test_field_result_text() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt Click Here}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // Should have the display text
        let text: String = hlink.runs.iter().map(|r| r.text.as_str()).collect();

        assert!(text.contains("Click") || text.contains("Here"));
    }
}

#[test]
fn test_field_result_with_formatting() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt \b Bold Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // Should have bold run
        assert!(hlink.runs.iter().any(|r| r.bold));
    }
}

// =============================================================================
// Field Instruction Parsing Tests
// =============================================================================

#[test]
fn test_hyperlink_with_quotes() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com/path?query=1"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // URL should include the path
        assert!(matches!(&hlink.target, HyperlinkTarget::ExternalUrl(url) if url.contains("path")));
    }
}

#[test]
fn test_hyperlink_without_quotes() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK http://example.com}{\fldrslt Link}}}"#;
    let result = parse(input);

    // Should still parse
    assert!(result.is_ok());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_field_result() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt }}}"#;
    let result = parse(input);

    // Should parse, even with empty result
    assert!(result.is_ok());
}

#[test]
fn test_field_without_instruction() {
    let input = r#"{\rtf1\ansi {\field{\fldrslt Just Text}}}"#;
    let result = parse(input);

    // Should parse, treating as regular text
    assert!(result.is_ok());
}

#[test]
fn test_hyperlink_with_special_characters() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "http://example.com/path?name=Test&value=123"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Inline::Hyperlink(hlink)) = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find(|i| matches!(i, Inline::Hyperlink(_)))
    {
        // URL should preserve special characters
        assert!(
            matches!(&hlink.target, HyperlinkTarget::ExternalUrl(url) if url.contains('?') || url.contains('&'))
        );
    }
}

// =============================================================================
// Non-Hyperlink Field Tests
// =============================================================================

#[test]
fn test_page_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst PAGE}{\fldrslt 1}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    let field = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find_map(|inline| {
            if let Inline::PageField(field) = inline {
                Some(field)
            } else {
                None
            }
        });

    assert!(
        matches!(field, Some(PageFieldRef::CurrentPage { .. })),
        "Expected semantic PAGE field, got {field:?}"
    );
}

#[test]
fn test_numpages_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst NUMPAGES}{\fldrslt 10}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    let field = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find_map(|inline| {
            if let Inline::PageField(field) = inline {
                Some(field)
            } else {
                None
            }
        });

    assert!(
        matches!(field, Some(PageFieldRef::TotalPages { .. })),
        "Expected semantic NUMPAGES field, got {field:?}"
    );
}

#[test]
fn test_pageref_field_parses_with_target_and_fallback() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst PAGEREF sec_exec_summary}{\fldrslt 7}}}"#;
    let result = parse(input);
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();
    let field = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find_map(|inline| {
            if let Inline::PageField(field) = inline {
                Some(field)
            } else {
                None
            }
        });

    assert!(
        matches!(
            field,
            Some(PageFieldRef::PageRef { target, fallback_text, .. })
            if target == "sec_exec_summary" && fallback_text.as_deref() == Some("7")
        ),
        "Expected PAGEREF with target + fallback, got {field:?}"
    );
}

#[test]
fn test_toc_field_emits_generated_block_marker_and_page_management() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst TOC \o "1-2" \h}{\fldrslt old toc}}}"#;
    let result = parse(input);
    assert!(result.is_ok());
    let (doc, _report) = result.unwrap();

    let page_management = doc
        .page_management
        .as_ref()
        .expect("Expected page management metadata");
    assert_eq!(page_management.generated_blocks.len(), 1);
    assert!(
        matches!(
            page_management.generated_blocks[0].kind,
            GeneratedBlockKind::TableOfContents { .. }
        ),
        "Expected generated TOC block"
    );
}

#[test]
fn test_date_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst DATE}{\fldrslt 2024-01-01}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_time_field() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst TIME}{\fldrslt 12:00}}}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Internal Hyperlink (Bookmark Reference) Tests
// =============================================================================

#[test]
fn test_internal_hyperlink_parses_to_internal_bookmark_target() {
    let input =
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK \l "section1"}{\fldrslt Go to section}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    let hlink = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find_map(|i| {
            if let Inline::Hyperlink(h) = i {
                Some(h)
            } else {
                None
            }
        });

    assert!(hlink.is_some(), "Expected a hyperlink in the document");
    let hlink = hlink.unwrap();
    assert!(
        matches!(&hlink.target, HyperlinkTarget::InternalBookmark(name) if name == "section1"),
        "Expected InternalBookmark(\"section1\"), got {:?}",
        hlink.target
    );
}

#[test]
fn test_hyperlink_with_switch_before_target_url_is_parsed() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK \o "tooltip" "https://example.com"}{\fldrslt Link}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();
    let hlink = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .find_map(|i| {
            if let Inline::Hyperlink(h) = i {
                Some(h)
            } else {
                None
            }
        });

    assert!(
        hlink.is_some(),
        "Expected hyperlink for switch-based fldinst"
    );
    let hlink = hlink.unwrap();
    assert!(
        matches!(&hlink.target, HyperlinkTarget::ExternalUrl(url) if url == "https://example.com")
    );
}

#[test]
fn test_bookmark_anchor_emitted_inline() {
    let input = r#"{\rtf1\ansi {\*\bkmkstart mybookmark}{\*\bkmkend mybookmark}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    let has_anchor = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .any(|i| matches!(i, Inline::BookmarkAnchor(a) if a.name == "mybookmark"));

    assert!(has_anchor, "Expected BookmarkAnchor(\"mybookmark\") inline");
}

#[test]
fn test_bookmark_name_in_field_result_is_not_rendered_as_text() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt pre {\*\bkmkstart mark1}{\*\bkmkend mark1} post}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();
    let para = doc
        .blocks
        .iter()
        .find_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .expect("expected paragraph");

    // The bookmark name should not leak into visible text.
    let visible_text: String = para
        .inlines
        .iter()
        .flat_map(|inline| match inline {
            Inline::Run(run) => vec![run.text.as_str()],
            Inline::Hyperlink(link) => link.runs.iter().map(|r| r.text.as_str()).collect(),
            Inline::BookmarkAnchor(_) => Vec::new(),
            Inline::NoteRef(_) => Vec::new(),
            Inline::PageField(_) => Vec::new(),
            Inline::GeneratedBlockMarker(_) => Vec::new(),
        })
        .collect();
    assert!(
        !visible_text.contains("mark1"),
        "bookmark destination text leaked into output: {visible_text:?}"
    );

    // Anchor should still be represented as an inline bookmark anchor.
    assert!(
        para.inlines
            .iter()
            .any(|i| matches!(i, Inline::BookmarkAnchor(a) if a.name == "mark1")),
        "expected bookmark anchor inline"
    );
}

#[test]
fn test_mixed_external_internal_links() {
    let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt External}} {\field{\*\fldinst HYPERLINK \l "anchor1"}{\fldrslt Internal}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    let hyperlinks: Vec<_> = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .flat_map(|p| p.inlines.iter())
        .filter_map(|i| {
            if let Inline::Hyperlink(h) = i {
                Some(h)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(hyperlinks.len(), 2, "Expected 2 hyperlinks");

    let has_external = hyperlinks.iter().any(
        |h| matches!(&h.target, HyperlinkTarget::ExternalUrl(url) if url.starts_with("https://")),
    );
    let has_internal = hyperlinks
        .iter()
        .any(|h| matches!(&h.target, HyperlinkTarget::InternalBookmark(name) if name == "anchor1"));

    assert!(has_external, "Expected an ExternalUrl hyperlink");
    assert!(has_internal, "Expected an InternalBookmark hyperlink");
}

#[test]
fn test_unsupported_field_does_not_strict_fail_when_result_preserved() {
    // DATE field with fldrslt text: the result text is preserved, so strict mode should NOT trigger
    let input = r#"{\rtf1\ansi {\field{\*\fldinst DATE}{\fldrslt 2024-01-01}}}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (_doc, report) = result.unwrap();

    // Should not have any DroppedContent warnings (which trigger strict mode)
    let has_dropped_content = report
        .warnings
        .iter()
        .any(|w| matches!(w, Warning::DroppedContent { .. }));
    assert!(
        !has_dropped_content,
        "DATE field with result text should not emit DroppedContent"
    );
}

// =============================================================================
// Hyperlink Adjacent to Text Tests
// =============================================================================

#[test]
fn test_hyperlink_with_adjacent_text() {
    let input = r#"{\rtf1\ansi Before {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt Link}} After}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have text before and after hyperlink
        let has_before = para
            .inlines
            .iter()
            .any(|i| matches!(i, Inline::Run(r) if r.text.contains("Before")));
        let has_after = para
            .inlines
            .iter()
            .any(|i| matches!(i, Inline::Run(r) if r.text.contains("After")));

        assert!(
            has_before || has_after,
            "Expected text adjacent to hyperlink"
        );
    }
}
