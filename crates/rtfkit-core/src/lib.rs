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
pub use interpreter::{
    Interpreter, ParagraphListRef, ParsedListDefinition, ParsedListLevel, ParsedListOverride,
    RtfEvent, StyleState, Token,
};

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

// =============================================================================
// Table Cell Merge and Alignment Types for Phase 5
// =============================================================================

/// Cell merge semantics for horizontal and vertical merges.
///
/// RTF uses `\clmgf` (merge start) and `\clmrg` (merge continuation) for horizontal merges,
/// and `\clvmgf`/`\clvmrg` for vertical merges.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellMerge {
    /// No merge applied (default)
    #[default]
    None,
    /// Start of a horizontal merge spanning N cells (including this one)
    HorizontalStart { span: u16 },
    /// Continuation of a horizontal merge (content belongs to previous cell)
    HorizontalContinue,
    /// Start of a vertical merge (top cell of merged region)
    VerticalStart,
    /// Continuation of a vertical merge (content belongs to cell above)
    VerticalContinue,
}

/// Vertical alignment of content within a table cell.
///
/// RTF controls: `\clvertalt` (top), `\clvertalc` (center), `\clvertalb` (bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellVerticalAlign {
    /// Align content to top of cell
    #[default]
    Top,
    /// Center content vertically in cell
    Center,
    /// Align content to bottom of cell
    Bottom,
}

/// Row-level alignment for table rows.
///
/// RTF controls: `\trql` (left), `\trqc` (center), `\trqr` (right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowAlignment {
    /// Left-aligned row
    #[default]
    Left,
    /// Centered row
    Center,
    /// Right-aligned row
    Right,
}

/// Table row properties.
///
/// Captures row-level formatting from RTF controls like `\trql`, `\trqr`, `\trqc`, `\trleft`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RowProps {
    /// Row alignment (left/center/right)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<RowAlignment>,
    /// Left indent in twips (from `\trleft`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_indent: Option<i32>,
}

/// Table-level properties.
///
/// Placeholder for future table-level formatting (borders, spacing, etc.)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableProps {
    // Future: borders, spacing, width preferences, etc.
    // Keeping sparse for now per PHASE5.md guidelines
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
///     font_family: Some("Arial".to_string()),
///     font_size: Some(12.0),
///     color: None,
///     background_color: None,
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
    /// Font family name (if specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    /// Font size in points (if specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// Text color (if specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Background/highlight color (if specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<Color>,
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
            font_family: None,
            font_size: None,
            color: None,
            background_color: None,
        }
    }
}

/// An inline element within a paragraph.
///
/// `Inline` represents content that flows within a paragraph.
/// It can be either a plain text run with formatting, or a hyperlink
/// wrapping one or more formatted runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    /// A plain text run with formatting.
    Run(Run),
    /// A hyperlink wrapping one or more formatted runs.
    Hyperlink(Hyperlink),
}

/// A hyperlink with a target URL and visible content.
///
/// The hyperlink wraps one or more `Run` elements that represent
/// the visible, clickable text of the link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hyperlink {
    /// Target URL
    pub url: String,
    /// Visible content (formatted runs)
    pub runs: Vec<Run>,
}

/// A paragraph containing one or more inline elements.
///
/// A `Paragraph` represents a block of text that is separated from other
/// blocks by paragraph breaks. It contains a sequence of `Inline` elements
/// (runs or hyperlinks) and has an associated alignment.
///
/// # Example
/// ```
/// use rtfkit_core::{Paragraph, Run, Inline, Alignment};
/// let para = Paragraph {
///     alignment: Alignment::Left,
///     inlines: vec![Inline::Run(Run::new("Hello, World!"))],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Paragraph {
    /// Horizontal alignment of the paragraph
    #[serde(default)]
    pub alignment: Alignment,
    /// The inline elements that make up this paragraph
    pub inlines: Vec<Inline>,
}

impl Paragraph {
    /// Creates a new empty paragraph with default (left) alignment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a paragraph from a vector of runs, wrapping each in Inline::Run.
    pub fn from_runs(runs: Vec<Run>) -> Self {
        Self {
            alignment: Alignment::default(),
            inlines: runs.into_iter().map(Inline::Run).collect(),
        }
    }

    /// Creates a paragraph from a vector of inline elements.
    pub fn from_inlines(inlines: Vec<Inline>) -> Self {
        Self {
            alignment: Alignment::default(),
            inlines,
        }
    }
}

// =============================================================================
// List Types for Phase 3
// =============================================================================

/// Unique identifier for a list within a document.
///
/// List IDs are assigned during interpretation and used to group
/// list items that belong to the same logical list.
pub type ListId = u32;

/// The kind of list numbering.
///
/// Represents the numbering style for a list or list level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListKind {
    /// Bullet list with glyph markers
    #[default]
    Bullet,
    /// Decimal numbered list (1, 2, 3, ...)
    OrderedDecimal,
    /// Mixed or inconsistent list kind (fallback for ambiguous sources)
    Mixed,
}

/// A single item within a list.
///
/// Each item has a nesting level and contains blocks (typically paragraphs).
/// The level is 0-indexed, where 0 is the top level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    /// Nesting level (0-8, clamped for DOCX compatibility)
    pub level: u8,
    /// Content blocks for this item (typically one Paragraph in Phase 3)
    pub blocks: Vec<Block>,
}

impl ListItem {
    /// Creates a new list item at the given level.
    pub fn new(level: u8) -> Self {
        Self {
            level: level.min(8), // Clamp to DOCX max
            blocks: Vec::new(),
        }
    }

    /// Creates a list item from a paragraph at the given level.
    pub fn from_paragraph(level: u8, paragraph: Paragraph) -> Self {
        Self {
            level: level.min(8),
            blocks: vec![Block::Paragraph(paragraph)],
        }
    }
}

/// A list containing one or more items.
///
/// Lists are block-level elements that contain numbered or bulleted items.
/// All items in a ListBlock share the same list_id and kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListBlock {
    /// Unique identifier for this list
    pub list_id: ListId,
    /// The kind of list (bullet, ordered, or mixed)
    pub kind: ListKind,
    /// The items in this list
    pub items: Vec<ListItem>,
}

impl ListBlock {
    /// Creates a new empty list with the given ID and kind.
    pub fn new(list_id: ListId, kind: ListKind) -> Self {
        Self {
            list_id,
            kind,
            items: Vec::new(),
        }
    }

    /// Adds an item to the list.
    pub fn add_item(&mut self, item: ListItem) {
        self.items.push(item);
    }

    /// Returns true if the list has no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// =============================================================================
// Table Types for Phase 4
// =============================================================================

/// A table containing one or more rows.
///
/// Tables are block-level elements that contain rows of cells.
/// Each row contains cells that can hold block content.
///
/// # Example
/// ```
/// use rtfkit_core::{TableBlock, TableRow, TableCell, Paragraph, Run, Block};
/// let table = TableBlock {
///     rows: vec![
///         TableRow {
///             cells: vec![
///                 TableCell {
///                     blocks: vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new("Cell 1")]))],
///                     width_twips: Some(1440), // 1 inch
///                     merge: None,
///                     v_align: None,
///                 },
///             ],
///             row_props: None,
///         },
///     ],
///     table_props: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableBlock {
    /// The rows that make up this table
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<TableRow>,

    /// Table-level properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_props: Option<TableProps>,
}

impl TableBlock {
    /// Creates a new empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a table from a vector of rows.
    pub fn from_rows(rows: Vec<TableRow>) -> Self {
        Self {
            rows,
            table_props: None,
        }
    }

    /// Adds a row to the table.
    pub fn add_row(&mut self, row: TableRow) {
        self.rows.push(row);
    }

    /// Returns true if the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the number of rows in the table.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

/// A row within a table.
///
/// Each row contains a sequence of cells. The number of cells
/// may vary between rows in malformed RTF, though well-formed
/// tables have consistent column counts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableRow {
    /// The cells in this row
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<TableCell>,

    /// Row-level properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_props: Option<RowProps>,
}

impl TableRow {
    /// Creates a new empty row.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a row from a vector of cells.
    pub fn from_cells(cells: Vec<TableCell>) -> Self {
        Self {
            cells,
            row_props: None,
        }
    }

    /// Adds a cell to the row.
    pub fn add_cell(&mut self, cell: TableCell) {
        self.cells.push(cell);
    }

    /// Returns true if the row has no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Returns the number of cells in this row.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

/// A cell within a table row.
///
/// Cells contain block-level content (paragraphs and lists in Phase 4).
/// Width is stored as computed cell width in twips (1/20th of a point).
/// A `None` value indicates width was not specified or could not be determined.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableCell {
    /// Block content within this cell (Paragraph and ListBlock in Phase 4)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,

    /// Computed cell width in twips (1/20th point)
    /// None if width not specified or determinable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_twips: Option<i32>,

    /// Merge semantics for this cell
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge: Option<CellMerge>,

    /// Vertical alignment of cell content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v_align: Option<CellVerticalAlign>,
}

impl TableCell {
    /// Creates a new empty cell.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a cell from blocks with optional width.
    pub fn from_blocks(blocks: Vec<Block>, width_twips: Option<i32>) -> Self {
        Self {
            blocks,
            width_twips,
            merge: None,
            v_align: None,
        }
    }

    /// Creates a cell from a single paragraph.
    pub fn from_paragraph(paragraph: Paragraph) -> Self {
        Self {
            blocks: vec![Block::Paragraph(paragraph)],
            width_twips: None,
            merge: None,
            v_align: None,
        }
    }

    /// Creates a cell from a paragraph with width.
    pub fn from_paragraph_with_width(paragraph: Paragraph, width_twips: i32) -> Self {
        Self {
            blocks: vec![Block::Paragraph(paragraph)],
            width_twips: Some(width_twips),
            merge: None,
            v_align: None,
        }
    }

    /// Adds a block to the cell.
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Returns true if the cell has no content.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Sets the cell width in twips.
    pub fn with_width(mut self, width_twips: i32) -> Self {
        self.width_twips = Some(width_twips);
        self
    }
}

/// A block-level element in the document.
///
/// `Block` represents the top-level structural elements of a document.
/// Currently supports paragraphs, lists, and tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    /// A paragraph block containing text
    Paragraph(Paragraph),
    /// A list block containing items
    ListBlock(ListBlock),
    /// A table block containing rows and cells
    TableBlock(TableBlock),
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
