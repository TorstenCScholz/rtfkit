//! Paragraph and run mapping from IR to Typst source.
//!
//! This module provides functions to convert rtfkit-core `Paragraph` types
//! to Typst markup source code.
//!
//! ## Typst Markup Mapping
//!
//! | IR | Typst |
//! |---|---|
//! | `Run.bold` | `*text*` |
//! | `Run.italic` | `_text_` |
//! | `Run.underline` | `#underline[text]` |
//! | `Run.font_family` | `#text(font: "Name")[...]` |
//! | `Run.font_size` | `#text(size: Npt)[...]` |
//! | `Run.color` | `#text(fill: rgb(r, g, b))[...]` |
//! | `Alignment::Left` | (default, no directive) |
//! | `Alignment::Center` | `#align(center)[...]` |
//! | `Alignment::Right` | `#align(right)[...]` |
//! | `Alignment::Justify` | `#align(justify)[...]` |
//!
//! ## Special Character Escaping
//!
//! The following characters are escaped in text content:
//! - `[` and `]` - Typst content markers
//! - `*` and `_` - Typst emphasis markers
//! - `@` - Typst reference marker
//! - `#` - Typst directive marker
//! - `\` - Typst escape character
//! - `$` - Typst math mode marker
//! - `~` - Typst non-breaking space

use rtfkit_core::{Alignment, Color, Hyperlink, Inline, Paragraph, Run};

use super::MappingWarning;

/// Result of mapping a paragraph to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated during mapping.
    pub warnings: Vec<MappingWarning>,
}

/// Map a rtfkit-core Paragraph to Typst source code.
///
/// # Arguments
///
/// * `paragraph` - The source paragraph from rtfkit-core.
///
/// # Returns
///
/// A `ParagraphOutput` containing the Typst source and any warnings.
///
/// # Determinism
///
/// This function is deterministic: the same input always produces the same output.
pub fn map_paragraph(paragraph: &Paragraph) -> ParagraphOutput {
    let mut warnings = Vec::new();

    // Map inlines to text content
    let content = map_inlines(&paragraph.inlines, &mut warnings);

    // Apply alignment if needed
    let typst_source = if content.is_empty() {
        // Empty paragraph - emit empty line
        String::new()
    } else {
        match paragraph.alignment {
            Alignment::Left => content,
            Alignment::Center => format!("#align(center)[{}]", content),
            Alignment::Right => format!("#align(right)[{}]", content),
            Alignment::Justify => format!("#align(justify)[{}]", content),
        }
    };

    ParagraphOutput {
        typst_source,
        warnings,
    }
}

/// Map inlines to Typst text content.
///
/// Handles both runs and hyperlinks, preserving formatting.
/// Adjacent runs with identical formatting are merged for cleaner output.
fn map_inlines(inlines: &[Inline], warnings: &mut Vec<MappingWarning>) -> String {
    if inlines.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut pending_runs: Vec<Run> = Vec::new();

    for inline in inlines {
        match inline {
            Inline::Run(run) => {
                // Accumulate runs for potential merging
                pending_runs.push(run.clone());
            }
            Inline::Hyperlink(hyperlink) => {
                // Flush any pending runs before the hyperlink
                if !pending_runs.is_empty() {
                    result.push_str(&map_runs(&pending_runs, warnings));
                    pending_runs.clear();
                }
                result.push_str(&map_hyperlink(hyperlink, warnings));
            }
        }
    }

    // Flush any remaining pending runs
    if !pending_runs.is_empty() {
        result.push_str(&map_runs(&pending_runs, warnings));
    }

    result
}

/// Map a hyperlink to Typst `#link()` syntax.
///
/// Generates `#link("url")[content]` with proper formatting for runs.
fn map_hyperlink(hyperlink: &Hyperlink, warnings: &mut Vec<MappingWarning>) -> String {
    // Map the runs inside the hyperlink
    let content = map_runs(&hyperlink.runs, warnings);

    if content.is_empty() {
        return String::new();
    }

    // Escape the URL for Typst string literal
    let escaped_url = escape_typst_string(&hyperlink.url);

    // Generate #link("url")[content]
    format!("#link(\"{}\")[{}]", escaped_url, content)
}

/// Escape a string for use in a Typst string literal.
///
/// Escapes: " \ and newlines
fn escape_typst_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }

    result
}

/// Style key for comparing runs during merging.
///
/// Two runs can be merged only if all style fields match.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    font_family: Option<String>,
    // Canonicalized to half-points for stable equality and output.
    font_size_half_points: Option<i32>,
    color: Option<ColorKey>,
}

/// Color comparison key that implements Eq for hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ColorKey {
    r: u8,
    g: u8,
    b: u8,
}

impl From<&Color> for ColorKey {
    fn from(color: &Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}

impl RunStyle {
    /// Extract style key from a Run for comparison.
    fn from_run(run: &Run) -> Self {
        Self {
            bold: run.bold,
            italic: run.italic,
            underline: run.underline,
            font_family: run.font_family.clone(),
            font_size_half_points: run.font_size.and_then(points_to_half_points),
            color: run.color.as_ref().map(ColorKey::from),
        }
    }

    /// Check if this style needs a `#text(...)` wrapper.
    fn needs_text_wrapper(&self) -> bool {
        self.font_family.is_some() || self.font_size_half_points.is_some() || self.color.is_some()
    }
}

fn points_to_half_points(points: f32) -> Option<i32> {
    if points.is_finite() && points > 0.0 {
        Some((points * 2.0).round() as i32)
    } else {
        None
    }
}

fn format_half_points(half_points: i32) -> String {
    if half_points % 2 == 0 {
        format!("{}", half_points / 2)
    } else {
        format!("{}.5", half_points / 2)
    }
}

/// Map runs to Typst text content.
///
/// Adjacent runs with identical formatting are merged for cleaner output.
fn map_runs(runs: &[Run], _warnings: &mut Vec<MappingWarning>) -> String {
    if runs.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut current_text = String::new();
    let mut current_style: Option<RunStyle> = None;

    for run in runs.iter() {
        let style = RunStyle::from_run(run);

        let same_formatting = current_style.as_ref() == Some(&style);

        if current_style.is_none() {
            // First run
            current_text = escape_typst_text(&run.text);
            current_style = Some(style);
        } else if same_formatting {
            // Same formatting - merge
            current_text.push_str(&escape_typst_text(&run.text));
        } else {
            // Different formatting - push current and start new
            if let Some(style) = current_style.take() {
                result.push_str(&format_run(&current_text, &style));
            }
            current_text = escape_typst_text(&run.text);
            current_style = Some(style);
        }
    }

    // Don't forget the last run
    if !current_text.is_empty() {
        if let Some(style) = current_style {
            result.push_str(&format_run(&current_text, &style));
        }
    }

    result
}

/// Format a single run with the given formatting.
///
/// Applies formatting in order: #text(...) wrapper, then underline, italic, bold.
/// This ensures proper nesting in Typst.
fn format_run(text: &str, style: &RunStyle) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut result = text.to_string();

    // Apply #text(...) wrapper if font/size/color styling is needed
    if style.needs_text_wrapper() {
        result = format_text_wrapper(&result, style);
    }

    // Apply underline (Typst function call)
    if style.underline {
        result = format!("#underline[{}]", result);
    }

    // Apply italic (Typst emphasis)
    if style.italic {
        result = format!("_{}_", result);
    }

    // Apply bold (Typst strong emphasis)
    if style.bold {
        result = format!("*{}*", result);
    }

    result
}

/// Format a `#text(...)` wrapper for font/size/color styling.
fn format_text_wrapper(content: &str, style: &RunStyle) -> String {
    let mut params = Vec::new();

    // Add font family parameter
    if let Some(ref font) = style.font_family {
        let escaped_font = escape_typst_string(font);
        params.push(format!("font: \"{}\"", escaped_font));
    }

    // Add font size parameter (convert points to Typst length)
    if let Some(size_hp) = style.font_size_half_points {
        params.push(format!("size: {}pt", format_half_points(size_hp)));
    }

    // Add fill color parameter
    if let Some(ref color) = style.color {
        params.push(format!("fill: rgb({}, {}, {})", color.r, color.g, color.b));
    }

    if params.is_empty() {
        return content.to_string();
    }

    format!("#text({})[{}]", params.join(", "), content)
}

/// Escape special Typst characters in text content.
///
/// Characters that need escaping in Typst markup mode:
/// - `[` and `]` - Content markers
/// - `*` and `_` - Emphasis markers  
/// - `@` - Reference marker
/// - `#` - Directive marker
/// - `\` - Escape character
/// - `$` - Math mode
/// - `~` - Non-breaking space
pub fn escape_typst_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '[' => result.push_str("\\["),
            ']' => result.push_str("\\]"),
            '*' => result.push_str("\\*"),
            '_' => result.push_str("\\_"),
            '@' => result.push_str("\\@"),
            '#' => result.push_str("\\#"),
            '\\' => result.push_str("\\\\"),
            '$' => result.push_str("\\$"),
            '~' => result.push_str("\\~"),
            _ => result.push(c),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_empty_paragraph() {
        let paragraph = Paragraph::from_runs(vec![]);
        let output = map_paragraph(&paragraph);

        assert!(output.typst_source.is_empty());
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_simple_paragraph() {
        let paragraph = Paragraph::from_runs(vec![Run::new("Hello, World!")]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "Hello, World!");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_bold_run() {
        let mut run = Run::new("bold");
        run.bold = true;

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "*bold*");
    }

    #[test]
    fn test_map_italic_run() {
        let mut run = Run::new("italic");
        run.italic = true;

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "_italic_");
    }

    #[test]
    fn test_map_underline_run() {
        let mut run = Run::new("underline");
        run.underline = true;

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "#underline[underline]");
    }

    #[test]
    fn test_map_bold_italic_run() {
        let mut run = Run::new("bold italic");
        run.bold = true;
        run.italic = true;

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "*_bold italic_*");
    }

    #[test]
    fn test_map_bold_underline_run() {
        let mut run = Run::new("bold underline");
        run.bold = true;
        run.underline = true;

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "*#underline[bold underline]*");
    }

    #[test]
    fn test_map_all_formatting() {
        let mut run = Run::new("all formats");
        run.bold = true;
        run.italic = true;
        run.underline = true;

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "*_#underline[all formats]_*");
    }

    #[test]
    fn test_map_mixed_formatting() {
        let run1 = Run::new("normal ");
        let mut run2 = Run::new("bold");
        run2.bold = true;
        let mut run3 = Run::new(" italic");
        run3.italic = true;

        let paragraph = Paragraph::from_runs(vec![run1, run2, run3]);
        let output = map_paragraph(&paragraph);

        // Note: the space is part of the italic run, so it's inside the emphasis
        assert_eq!(output.typst_source, "normal *bold*_ italic_");
    }

    #[test]
    fn test_map_merged_runs() {
        let mut run1 = Run::new("Hello ");
        run1.bold = true;
        let mut run2 = Run::new("World");
        run2.bold = true;

        let paragraph = Paragraph::from_runs(vec![run1, run2]);
        let output = map_paragraph(&paragraph);

        // Should merge into single bold run
        assert_eq!(output.typst_source, "*Hello World*");
    }

    #[test]
    fn test_map_center_alignment() {
        let mut paragraph = Paragraph::from_runs(vec![Run::new("centered")]);
        paragraph.alignment = Alignment::Center;

        let output = map_paragraph(&paragraph);
        assert_eq!(output.typst_source, "#align(center)[centered]");
    }

    #[test]
    fn test_map_right_alignment() {
        let mut paragraph = Paragraph::from_runs(vec![Run::new("right")]);
        paragraph.alignment = Alignment::Right;

        let output = map_paragraph(&paragraph);
        assert_eq!(output.typst_source, "#align(right)[right]");
    }

    #[test]
    fn test_map_justify_alignment() {
        let mut paragraph = Paragraph::from_runs(vec![Run::new("justified")]);
        paragraph.alignment = Alignment::Justify;

        let output = map_paragraph(&paragraph);
        assert_eq!(output.typst_source, "#align(justify)[justified]");
    }

    #[test]
    fn test_escape_special_characters() {
        assert_eq!(escape_typst_text("a[b]c"), "a\\[b\\]c");
        assert_eq!(escape_typst_text("a*b_c"), "a\\*b\\_c");
        assert_eq!(escape_typst_text("a@b#c"), "a\\@b\\#c");
        assert_eq!(escape_typst_text("a\\b"), "a\\\\b");
        assert_eq!(escape_typst_text("a$b"), "a\\$b");
        assert_eq!(escape_typst_text("a~b"), "a\\~b");
    }

    #[test]
    fn test_escape_multiple_special_chars() {
        assert_eq!(
            escape_typst_text("use *bold* and _italic_"),
            "use \\*bold\\* and \\_italic\\_"
        );
    }

    #[test]
    fn test_font_size_output() {
        let mut run = Run::new("text");
        run.font_size = Some(14.0);

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "#text(size: 14pt)[text]");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_font_size_half_point_output() {
        let mut run = Run::new("text");
        run.font_size = Some(12.5);

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "#text(size: 12.5pt)[text]");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_non_positive_font_size_is_omitted() {
        let mut run = Run::new("text");
        run.font_size = Some(0.0);

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "text");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_color_output() {
        let mut run = Run::new("text");
        run.color = Some(Color::new(255, 0, 0));

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "#text(fill: rgb(255, 0, 0))[text]");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_font_family_output() {
        let mut run = Run::new("text");
        run.font_family = Some("Arial".to_string());

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(output.typst_source, "#text(font: \"Arial\")[text]");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_font_family_with_special_chars() {
        let mut run = Run::new("text");
        run.font_family = Some("Font \"Name\"".to_string());

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(
            output.typst_source,
            "#text(font: \"Font \\\"Name\\\"\")[text]"
        );
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_combined_font_and_size() {
        let mut run = Run::new("text");
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(12.0);

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(
            output.typst_source,
            "#text(font: \"Arial\", size: 12pt)[text]"
        );
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_combined_font_size_color() {
        let mut run = Run::new("text");
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(12.0);
        run.color = Some(Color::new(255, 0, 0));

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(
            output.typst_source,
            "#text(font: \"Arial\", size: 12pt, fill: rgb(255, 0, 0))[text]"
        );
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_bold_with_font_and_color() {
        let mut run = Run::new("text");
        run.bold = true;
        run.font_family = Some("Arial".to_string());
        run.color = Some(Color::new(255, 0, 0));

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert_eq!(
            output.typst_source,
            "*#text(font: \"Arial\", fill: rgb(255, 0, 0))[text]*"
        );
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_all_formatting_with_font_size_color() {
        let mut run = Run::new("text");
        run.bold = true;
        run.italic = true;
        run.underline = true;
        run.font_family = Some("Arial".to_string());
        run.font_size = Some(14.0);
        run.color = Some(Color::new(0, 128, 255));

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        // The #text(...) wrapper is inside #underline[], which is inside _italic_, which is inside *bold*
        assert_eq!(
            output.typst_source,
            "*_#underline[#text(font: \"Arial\", size: 14pt, fill: rgb(0, 128, 255))[text]]_*"
        );
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_run_merging_with_different_fonts() {
        let mut run1 = Run::new("Hello ");
        run1.font_family = Some("Arial".to_string());
        let mut run2 = Run::new("World");
        run2.font_family = Some("Times New Roman".to_string());

        let paragraph = Paragraph::from_runs(vec![run1, run2]);
        let output = map_paragraph(&paragraph);

        // Should NOT merge - different fonts
        assert_eq!(
            output.typst_source,
            "#text(font: \"Arial\")[Hello ]#text(font: \"Times New Roman\")[World]"
        );
    }

    #[test]
    fn test_run_merging_with_same_font() {
        let mut run1 = Run::new("Hello ");
        run1.font_family = Some("Arial".to_string());
        let mut run2 = Run::new("World");
        run2.font_family = Some("Arial".to_string());

        let paragraph = Paragraph::from_runs(vec![run1, run2]);
        let output = map_paragraph(&paragraph);

        // Should merge - same font
        assert_eq!(output.typst_source, "#text(font: \"Arial\")[Hello World]");
    }

    #[test]
    fn test_run_merging_with_different_colors() {
        let mut run1 = Run::new("Red ");
        run1.color = Some(Color::new(255, 0, 0));
        let mut run2 = Run::new("Blue");
        run2.color = Some(Color::new(0, 0, 255));

        let paragraph = Paragraph::from_runs(vec![run1, run2]);
        let output = map_paragraph(&paragraph);

        // Should NOT merge - different colors
        assert_eq!(
            output.typst_source,
            "#text(fill: rgb(255, 0, 0))[Red ]#text(fill: rgb(0, 0, 255))[Blue]"
        );
    }

    #[test]
    fn test_run_merging_with_different_sizes() {
        let mut run1 = Run::new("Small ");
        run1.font_size = Some(10.0);
        let mut run2 = Run::new("Large");
        run2.font_size = Some(20.0);

        let paragraph = Paragraph::from_runs(vec![run1, run2]);
        let output = map_paragraph(&paragraph);

        // Should NOT merge - different sizes
        assert_eq!(
            output.typst_source,
            "#text(size: 10pt)[Small ]#text(size: 20pt)[Large]"
        );
    }

    #[test]
    fn test_run_merging_all_style_fields_match() {
        let mut run1 = Run::new("Hello ");
        run1.bold = true;
        run1.font_family = Some("Arial".to_string());
        run1.font_size = Some(12.0);
        run1.color = Some(Color::new(255, 0, 0));

        let mut run2 = Run::new("World");
        run2.bold = true;
        run2.font_family = Some("Arial".to_string());
        run2.font_size = Some(12.0);
        run2.color = Some(Color::new(255, 0, 0));

        let paragraph = Paragraph::from_runs(vec![run1, run2]);
        let output = map_paragraph(&paragraph);

        // Should merge - all style fields match
        assert_eq!(
            output.typst_source,
            "*#text(font: \"Arial\", size: 12pt, fill: rgb(255, 0, 0))[Hello World]*"
        );
    }

    #[test]
    fn test_determinism() {
        let paragraph = Paragraph::from_runs(vec![
            Run::new("Hello "),
            {
                let mut r = Run::new("World");
                r.bold = true;
                r
            },
            Run::new("!"),
        ]);

        // Run multiple times to verify determinism
        let output1 = map_paragraph(&paragraph);
        let output2 = map_paragraph(&paragraph);
        let output3 = map_paragraph(&paragraph);

        assert_eq!(output1.typst_source, output2.typst_source);
        assert_eq!(output2.typst_source, output3.typst_source);
    }

    #[test]
    fn test_map_hyperlink_inline() {
        let paragraph = Paragraph::from_inlines(vec![Inline::Hyperlink(Hyperlink {
            url: "https://example.com".to_string(),
            runs: vec![Run::new("Example")],
        })]);

        let output = map_paragraph(&paragraph);
        assert_eq!(
            output.typst_source,
            "#link(\"https://example.com\")[Example]"
        );
    }

    #[test]
    fn test_map_hyperlink_with_formatted_runs() {
        let mut bold = Run::new("Bold");
        bold.bold = true;
        let mut italic = Run::new("Italic");
        italic.italic = true;

        let paragraph = Paragraph::from_inlines(vec![Inline::Hyperlink(Hyperlink {
            url: "https://example.com".to_string(),
            runs: vec![bold, Run::new(" "), italic],
        })]);

        let output = map_paragraph(&paragraph);
        assert_eq!(
            output.typst_source,
            "#link(\"https://example.com\")[*Bold* _Italic_]"
        );
    }

    #[test]
    fn test_hyperlink_url_is_escaped_for_typst_string_literal() {
        let paragraph = Paragraph::from_inlines(vec![Inline::Hyperlink(Hyperlink {
            url: "https://example.com?q=\\\"x\\\"".to_string(),
            runs: vec![Run::new("Example")],
        })]);

        let output = map_paragraph(&paragraph);
        assert_eq!(
            output.typst_source,
            "#link(\"https://example.com?q=\\\\\\\"x\\\\\\\"\")[Example]"
        );
    }
}
