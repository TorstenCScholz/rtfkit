use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Error Module
// =============================================================================

pub mod error;

// Re-export key error types for convenience
pub use error::{ConversionError, ParseError, ReportError};

// =============================================================================
// Limits Module
// =============================================================================

pub mod limits;

// Re-export limits types for convenience
pub use limits::ParserLimits;

// =============================================================================
// Interpreter Module
// =============================================================================

pub mod interpreter;

// Re-export key types from interpreter module for convenience
pub use interpreter::{Interpreter, RtfEvent, StyleState, Token};

// =============================================================================
// Report Module
// =============================================================================

pub mod report;

// Re-export key types from report module for convenience
pub use report::{Report, Stats, Warning, WarningSeverity};

// =============================================================================
// IR Types for RTF Conversion
// =============================================================================

/// RGB color representation for text formatting.
///
/// Each component is stored as a `u8` value (0-255).
/// This type is used to represent foreground text colors in the IR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

impl Color {
    /// Creates a new Color from RGB components.
    ///
    /// # Example
    /// ```
    /// use rtfkit_core::Color;
    /// let red = Color::new(255, 0, 0);
    /// ```
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// Text alignment options for paragraphs.
///
/// Represents the horizontal alignment of text within a paragraph.
/// The default alignment is `Left`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    /// Left-aligned text (default)
    #[default]
    Left,
    /// Centered text
    Center,
    /// Right-aligned text
    Right,
    /// Justified text (aligned on both left and right edges)
    Justify,
}

/// A contiguous run of text with uniform formatting.
///
/// A `Run` represents a segment of text within a paragraph that shares
/// the same formatting attributes (bold, italic, underline, font size, color).
///
/// # Example
/// ```
/// use rtfkit_core::Run;
/// let run = Run {
///     text: "Hello, World!".to_string(),
///     bold: true,
///     italic: false,
///     underline: false,
///     font_size: Some(12.0),
///     color: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    /// The text content of this run
    pub text: String,
    /// Whether the text is bold
    #[serde(default)]
    pub bold: bool,
    /// Whether the text is italic
    #[serde(default)]
    pub italic: bool,
    /// Whether the text is underlined
    #[serde(default)]
    pub underline: bool,
    /// Font size in points (if specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// Text color (if specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

impl Run {
    /// Creates a new Run with the given text and default formatting.
    ///
    /// # Example
    /// ```
    /// use rtfkit_core::Run;
    /// let run = Run::new("Hello");
    /// ```
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            underline: false,
            font_size: None,
            color: None,
        }
    }
}

/// A paragraph containing one or more text runs.
///
/// A `Paragraph` represents a block of text that is separated from other
/// blocks by paragraph breaks. It contains a sequence of `Run` objects
/// and has an associated alignment.
///
/// # Example
/// ```
/// use rtfkit_core::{Paragraph, Run, Alignment};
/// let para = Paragraph {
///     alignment: Alignment::Left,
///     runs: vec![Run::new("Hello, World!")],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Paragraph {
    /// Horizontal alignment of the paragraph
    #[serde(default)]
    pub alignment: Alignment,
    /// The text runs that make up this paragraph
    pub runs: Vec<Run>,
}

impl Paragraph {
    /// Creates a new empty paragraph with default (left) alignment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a paragraph from a vector of runs.
    pub fn from_runs(runs: Vec<Run>) -> Self {
        Self {
            alignment: Alignment::default(),
            runs,
        }
    }
}

/// A block-level element in the document.
///
/// `Block` represents the top-level structural elements of a document.
/// Currently, only paragraphs are supported, but the enum structure
/// allows for future expansion (e.g., tables, lists, images).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    /// A paragraph block containing text
    Paragraph(Paragraph),
}

/// The root document structure containing all content.
///
/// `Document` is the top-level IR type that represents a complete
/// parsed RTF document. It contains a sequence of block-level elements.
///
/// # Example
/// ```
/// use rtfkit_core::{Document, Block, Paragraph, Run};
/// let doc = Document {
///     blocks: vec![
///         Block::Paragraph(Paragraph::from_runs(vec![Run::new("Hello")])),
///     ],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Document {
    /// The block-level elements that make up the document
    pub blocks: Vec<Block>,
}

impl Document {
    /// Creates a new empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a document from a vector of blocks.
    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }
}

// =============================================================================
// Text Analysis (existing functionality)
// =============================================================================

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TextStats {
    pub lines: usize,
    pub words: usize,
    pub chars: usize,
    pub bytes: usize,
    pub most_common_word: Option<String>,
    pub unique_words: usize,
}

pub fn analyze(input: &str) -> TextStats {
    let lines = input.lines().count();
    let chars = input.chars().count();
    let bytes = input.len();

    let words_vec: Vec<&str> = input.split_whitespace().collect();
    let words = words_vec.len();

    let mut freq: HashMap<String, usize> = HashMap::new();
    for word in &words_vec {
        let lower = word.to_lowercase();
        *freq.entry(lower).or_insert(0) += 1;
    }

    let unique_words = freq.len();

    let most_common_word = freq
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(word, _)| word);

    TextStats {
        lines,
        words,
        chars,
        bytes,
        most_common_word,
        unique_words,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let stats = analyze("");
        assert_eq!(stats.lines, 0);
        assert_eq!(stats.words, 0);
        assert_eq!(stats.chars, 0);
        assert_eq!(stats.bytes, 0);
        assert_eq!(stats.most_common_word, None);
        assert_eq!(stats.unique_words, 0);
    }

    #[test]
    fn simple_text() {
        let stats = analyze("hello world");
        assert_eq!(stats.lines, 1);
        assert_eq!(stats.words, 2);
        assert_eq!(stats.unique_words, 2);
    }

    #[test]
    fn repeated_words() {
        let stats = analyze("the the the cat");
        assert_eq!(stats.most_common_word, Some("the".to_string()));
        assert_eq!(stats.unique_words, 2);
    }
}
