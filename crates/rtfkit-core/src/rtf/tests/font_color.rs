//! Font and Color Table Tests
//!
//! Tests for RTF font/color table handling including:
//! - `\fonttbl` parsing
//! - `\deff` default font
//! - `\f` font selection
//! - `\fs` font size
//! - `\colortbl` parsing
//! - `\cf` foreground color
//! - `\highlight` highlight color
//! - `\cb` background color

use crate::rtf::parse;
use crate::{Block, Color, Inline};

// =============================================================================
// Font Table Tests
// =============================================================================

#[test]
fn test_font_table_parsing() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0 Arial;}{\f1 Times New Roman;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_font_table_with_family() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0\fswiss Arial;}{\f1\froman Times New Roman;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_font_table_with_charset() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0\fswiss\fcharset0 Arial;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_font_table_complex() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0\fswiss\fcharset0 Arial;}{\f1\froman\fcharset0 Times New Roman;}{\f2\fmodern\fcharset0 Courier New;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Default Font Tests
// =============================================================================

#[test]
fn test_default_font_deff() {
    let input = include_str!("../../../../../fixtures/text_default_font_deff.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_deff_in_header() {
    let input = r#"{\rtf1\ansi\deff0 {\fonttbl{\f0 Arial;}}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Font Selection Tests
// =============================================================================

#[test]
fn test_font_selection() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0 Arial;}{\f1 Times;}}\f1 Times Text}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have a run with font family
        let has_font = para
            .inlines
            .iter()
            .filter_map(|i| {
                if let Inline::Run(r) = i {
                    Some(r)
                } else {
                    None
                }
            })
            .any(|r| r.font_family.is_some());

        assert!(has_font, "Expected font family to be set");
    }
}

#[test]
fn test_font_selection_from_fixture() {
    let input = include_str!("../../../../../fixtures/text_font_family.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Font Size Tests
// =============================================================================

#[test]
fn test_font_size() {
    let input = r#"{\rtf1\ansi \fs24 12pt Text}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // fs24 = 12pt (24 half-points)
        let has_size = para
            .inlines
            .iter()
            .filter_map(|i| {
                if let Inline::Run(r) = i {
                    Some(r)
                } else {
                    None
                }
            })
            .any(|r| r.font_size == Some(12.0));

        assert!(has_size, "Expected font size of 12pt");
    }
}

#[test]
fn test_font_size_from_fixture() {
    let input = include_str!("../../../../../fixtures/text_font_size.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_font_size_various() {
    let input = r#"{\rtf1\ansi \fs10 5pt\fs20 10pt\fs24 12pt\fs32 16pt\fs48 24pt}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        let sizes: Vec<Option<f32>> = para
            .inlines
            .iter()
            .filter_map(|i| {
                if let Inline::Run(r) = i {
                    Some(r.font_size)
                } else {
                    None
                }
            })
            .collect();

        // Should have various font sizes
        assert!(sizes.contains(&Some(5.0)));
        assert!(sizes.contains(&Some(10.0)));
        assert!(sizes.contains(&Some(12.0)));
    }
}

// =============================================================================
// Color Table Tests
// =============================================================================

#[test]
fn test_color_table_parsing() {
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;\red0\green255\blue0;\red0\green0\blue255;}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_color_table_empty_first() {
    // First entry (after ;) is auto color
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;}Text}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_color_table_rgb() {
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;}\cf1 Red Text}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have colored text
        let has_color = para
            .inlines
            .iter()
            .filter_map(|i| {
                if let Inline::Run(r) = i {
                    Some(r)
                } else {
                    None
                }
            })
            .any(|r| r.color.is_some());

        assert!(has_color, "Expected color to be set");
    }
}

// =============================================================================
// Foreground Color Tests
// =============================================================================

#[test]
fn test_foreground_color() {
    let input = include_str!("../../../../../fixtures/text_color.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_cf_color_selection() {
    let input =
        r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;\red0\green255\blue0;}\cf1 Red \cf2 Green}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have different colors
        let colors: Vec<Option<Color>> = para
            .inlines
            .iter()
            .filter_map(|i| {
                if let Inline::Run(r) = i {
                    Some(r.color.clone())
                } else {
                    None
                }
            })
            .collect();

        // At least one should have color
        assert!(colors.iter().any(|c| c.is_some()));
    }
}

// =============================================================================
// Highlight Color Tests
// =============================================================================

#[test]
fn test_highlight_color() {
    let input = include_str!("../../../../../fixtures/text_highlight.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_highlight_selection() {
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green255\blue0;}\highlight1 Highlighted}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first() {
        // Should have background color (highlight)
        let has_highlight = para
            .inlines
            .iter()
            .filter_map(|i| {
                if let Inline::Run(r) = i {
                    Some(r)
                } else {
                    None
                }
            })
            .any(|r| r.background_color.is_some());

        assert!(has_highlight, "Expected highlight/background color");
    }
}

// =============================================================================
// Background Color Tests
// =============================================================================

#[test]
fn test_background_color_cb() {
    let input = include_str!("../../../../../fixtures/text_background_cb.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_cb_background() {
    let input = r#"{\rtf1\ansi {\colortbl;\red200\green200\blue200;}\cb1 Gray Background}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Highlight vs Background Precedence Tests
// =============================================================================

#[test]
fn test_highlight_background_precedence() {
    let input = include_str!("../../../../../fixtures/text_highlight_background_precedence.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Combined Font and Color Tests
// =============================================================================

#[test]
fn test_font_color_combined() {
    let input = include_str!("../../../../../fixtures/text_font_color_combined.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_font_and_color_together() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0 Arial;}}{\colortbl;\red255\green0\blue0;}\f0\fs24\cf1 Red Arial 12pt}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::Paragraph(para)) = doc.blocks.first()
        && let Some(Inline::Run(run)) = para.inlines.first() {
            assert!(run.font_family.is_some());
            assert!(run.font_size.is_some());
            assert!(run.color.is_some());
        }
}

// =============================================================================
// Plain Reset Tests
// =============================================================================

#[test]
fn test_plain_reset_font() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0 Arial;}{\f1 Times;}}\f1\fs24 Styled\plain Default}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_plain_reset_color() {
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;}\cf1 Red\plain Default}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_plain_reset_from_fixture() {
    let input = include_str!("../../../../../fixtures/text_plain_reset.rtf");
    let result = parse(input);

    assert!(result.is_ok());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_font_table() {
    let input = r#"{\rtf1\ansi {\fonttbl}Text}"#;
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

#[test]
fn test_empty_color_table() {
    let input = r#"{\rtf1\ansi {\colortbl}Text}"#;
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

#[test]
fn test_invalid_color_index() {
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;}\cf999 Invalid Index}"#;
    let result = parse(input);

    // Should parse gracefully, ignoring invalid index
    assert!(result.is_ok());
}

#[test]
fn test_invalid_font_index() {
    let input = r#"{\rtf1\ansi {\fonttbl{\f0 Arial;}}\f999 Invalid Font}"#;
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());
}

#[test]
fn test_color_reset_cf0() {
    // \cf0 resets to default color
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green0\blue0;}\cf1 Red\cf0 Default}"#;
    let result = parse(input);

    assert!(result.is_ok());
}

#[test]
fn test_highlight_reset_highlight0() {
    // \highlight0 resets highlight
    let input = r#"{\rtf1\ansi {\colortbl;\red255\green255\blue0;}\highlight1 Highlighted\highlight0 No Highlight}"#;
    let result = parse(input);

    assert!(result.is_ok());
}
