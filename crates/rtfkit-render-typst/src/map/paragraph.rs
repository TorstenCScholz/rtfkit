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

use rtfkit_core::{Alignment, Paragraph, Run};

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

    // Map runs to text content
    let content = map_runs(&paragraph.runs, &mut warnings);

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

/// Map runs to Typst text content.
///
/// Adjacent runs with identical formatting are merged for cleaner output.
fn map_runs(runs: &[Run], warnings: &mut Vec<MappingWarning>) -> String {
    if runs.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut current_text = String::new();
    let mut current_bold = false;
    let mut current_italic = false;
    let mut current_underline = false;

    for (i, run) in runs.iter().enumerate() {
        // Check for dropped formatting
        if run.font_size.is_some() {
            warnings.push(MappingWarning::FontSizeDropped);
        }
        if run.color.is_some() {
            warnings.push(MappingWarning::ColorDropped);
        }

        let same_formatting = i > 0
            && current_bold == run.bold
            && current_italic == run.italic
            && current_underline == run.underline;

        if i == 0 {
            current_text = escape_typst_text(&run.text);
            current_bold = run.bold;
            current_italic = run.italic;
            current_underline = run.underline;
        } else if same_formatting {
            // Same formatting - merge
            current_text.push_str(&escape_typst_text(&run.text));
        } else {
            // Different formatting - push current and start new
            result.push_str(&format_run(
                &current_text,
                current_bold,
                current_italic,
                current_underline,
            ));
            current_text = escape_typst_text(&run.text);
            current_bold = run.bold;
            current_italic = run.italic;
            current_underline = run.underline;
        }
    }

    // Don't forget the last run
    if !current_text.is_empty() {
        result.push_str(&format_run(
            &current_text,
            current_bold,
            current_italic,
            current_underline,
        ));
    }

    result
}

/// Format a single run with the given formatting.
fn format_run(text: &str, bold: bool, italic: bool, underline: bool) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut result = text.to_string();

    // Apply formatting in order: underline, italic, bold
    // This ensures proper nesting in Typst
    if underline {
        result = format!("#underline[{}]", result);
    }

    if italic {
        result = format!("_{}_", result);
    }

    if bold {
        result = format!("*{}*", result);
    }

    result
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
    fn test_font_size_dropped_warning() {
        let mut run = Run::new("text");
        run.font_size = Some(14.0);

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert!(
            output
                .warnings
                .contains(&crate::map::MappingWarning::FontSizeDropped)
        );
    }

    #[test]
    fn test_color_dropped_warning() {
        let mut run = Run::new("text");
        run.color = Some(rtfkit_core::Color::new(255, 0, 0));

        let paragraph = Paragraph::from_runs(vec![run]);
        let output = map_paragraph(&paragraph);

        assert!(
            output
                .warnings
                .contains(&crate::map::MappingWarning::ColorDropped)
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
}
