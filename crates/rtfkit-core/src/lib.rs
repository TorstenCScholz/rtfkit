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
// RTF Module (Modular Architecture)
// =============================================================================

pub mod rtf;

// Re-export the main API functions for convenience
pub use rtf::{RtfParser, parse, parse_with_limits};

// =============================================================================
// Report Module
// =============================================================================

pub mod report;

// Re-export key types from report module for convenience
pub use report::{Report, Stats, Warning, WarningSeverity};

// =============================================================================
// Shared Rendering Helpers
// =============================================================================

pub mod shading_render;

pub use shading_render::{
    ShadingRenderPolicy, percent_pattern_density, resolve_shading_fill_color,
};

// =============================================================================
// IR Types for RTF Conversion
// =============================================================================

/// RGB color representation for text formatting.
///
/// Each component is stored as a `u8` value (0-255).
/// This type is used to represent foreground text colors in the IR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Creates a Color from a hex string (e.g., #RRGGBB or RRGGBB).
    ///
    /// Returns None if the string is not a valid 6-character hex color.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    /// Apply a tint to this color (mix with white).
    ///
    /// Tint value is in the range 0-255 where:
    /// - 0 = no change (original color)
    /// - 255 = fully tinted (white)
    ///
    /// This matches RTF/Office theme tint semantics.
    pub fn tint(&self, tint_value: u8) -> Self {
        if tint_value == 0 {
            return self.clone();
        }
        let factor = tint_value as f32 / 255.0;
        Self {
            r: (self.r as f32 + (255.0 - self.r as f32) * factor).round() as u8,
            g: (self.g as f32 + (255.0 - self.g as f32) * factor).round() as u8,
            b: (self.b as f32 + (255.0 - self.b as f32) * factor).round() as u8,
        }
    }

    /// Apply a shade to this color (mix with black).
    ///
    /// Shade value is in the range 0-255 where:
    /// - 0 = no change (original color)
    /// - 255 = fully shaded (black)
    ///
    /// This matches RTF/Office theme shade semantics.
    pub fn shade(&self, shade_value: u8) -> Self {
        if shade_value == 0 {
            return self.clone();
        }
        let factor = shade_value as f32 / 255.0;
        Self {
            r: (self.r as f32 * (1.0 - factor)).round() as u8,
            g: (self.g as f32 * (1.0 - factor)).round() as u8,
            b: (self.b as f32 * (1.0 - factor)).round() as u8,
        }
    }
}

// =============================================================================
// Theme Color Types for RTF Theme Color Resolution
// =============================================================================

/// Theme color indices as defined in RTF/Office themes.
///
/// These indices correspond to the standard Office theme color slots.
/// RTF uses `\themecolorN` where N maps to these theme color types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeColor {
    /// Light 1 (typically white or very light)
    Light1 = 0,
    /// Dark 1 (typically black or very dark)
    Dark1 = 1,
    /// Light 2 (secondary light color)
    Light2 = 2,
    /// Dark 2 (secondary dark color)
    Dark2 = 3,
    /// Accent 1
    Accent1 = 4,
    /// Accent 2
    Accent2 = 5,
    /// Accent 3
    Accent3 = 6,
    /// Accent 4
    Accent4 = 7,
    /// Accent 5
    Accent5 = 8,
    /// Accent 6
    Accent6 = 9,
    /// Hyperlink color
    Hyperlink = 10,
    /// Followed hyperlink color
    FollowedHyperlink = 11,
}

impl ThemeColor {
    /// Convert from RTF theme color index to ThemeColor enum.
    ///
    /// RTF uses indices 0-11 for theme colors.
    /// Returns None for invalid indices.
    pub fn from_index(index: i32) -> Option<Self> {
        match index {
            0 => Some(ThemeColor::Light1),
            1 => Some(ThemeColor::Dark1),
            2 => Some(ThemeColor::Light2),
            3 => Some(ThemeColor::Dark2),
            4 => Some(ThemeColor::Accent1),
            5 => Some(ThemeColor::Accent2),
            6 => Some(ThemeColor::Accent3),
            7 => Some(ThemeColor::Accent4),
            8 => Some(ThemeColor::Accent5),
            9 => Some(ThemeColor::Accent6),
            10 => Some(ThemeColor::Hyperlink),
            11 => Some(ThemeColor::FollowedHyperlink),
            _ => None,
        }
    }

    /// Get the default Office theme color for this theme color slot.
    ///
    /// These are the standard colors from the default Office theme.
    pub fn default_color(&self) -> Color {
        match self {
            ThemeColor::Light1 => Color::from_hex("FFFFFF").unwrap(), // White
            ThemeColor::Dark1 => Color::from_hex("000000").unwrap(),  // Black
            ThemeColor::Light2 => Color::from_hex("E7E6E6").unwrap(), // Light gray
            ThemeColor::Dark2 => Color::from_hex("44546A").unwrap(),  // Dark blue-gray
            ThemeColor::Accent1 => Color::from_hex("4472C4").unwrap(), // Blue
            ThemeColor::Accent2 => Color::from_hex("ED7D31").unwrap(), // Orange
            ThemeColor::Accent3 => Color::from_hex("A5A5A5").unwrap(), // Gray
            ThemeColor::Accent4 => Color::from_hex("FFC000").unwrap(), // Gold
            ThemeColor::Accent5 => Color::from_hex("5B9BD5").unwrap(), // Light blue
            ThemeColor::Accent6 => Color::from_hex("70AD47").unwrap(), // Green
            ThemeColor::Hyperlink => Color::from_hex("0563C1").unwrap(), // Hyperlink blue
            ThemeColor::FollowedHyperlink => Color::from_hex("954F72").unwrap(), // Purple
        }
    }
}

/// A color entry in the color table that may include theme color metadata.
///
/// RTF color tables can contain theme color references in addition to
/// explicit RGB values. This type captures both possibilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorEntry {
    /// The resolved RGB color (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rgb: Option<Color>,
    /// Theme color reference (if this is a theme color)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<ThemeColor>,
    /// Theme tint value (0-255, lightens the color)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_tint: Option<u8>,
    /// Theme shade value (0-255, darkens the color)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_shade: Option<u8>,
}

impl ColorEntry {
    /// Creates a new explicit RGB color entry.
    pub fn rgb(color: Color) -> Self {
        Self {
            rgb: Some(color),
            theme_color: None,
            theme_tint: None,
            theme_shade: None,
        }
    }

    /// Creates a new theme color entry.
    pub fn theme(theme_color: ThemeColor, tint: Option<u8>, shade: Option<u8>) -> Self {
        Self {
            rgb: None,
            theme_color: Some(theme_color),
            theme_tint: tint,
            theme_shade: shade,
        }
    }

    /// Creates an auto/default color entry (no color).
    pub fn auto_color() -> Self {
        Self {
            rgb: None,
            theme_color: None,
            theme_tint: None,
            theme_shade: None,
        }
    }

    /// Resolve this color entry to a concrete RGB color.
    ///
    /// Resolution order:
    /// 1. Explicit RGB if present
    /// 2. Resolved theme color (with tint/shade applied)
    /// 3. None (cosmetic loss)
    pub fn resolve(&self) -> Option<Color> {
        // 1. Explicit RGB takes precedence
        if let Some(rgb) = &self.rgb {
            return Some(rgb.clone());
        }

        // 2. Resolve theme color
        if let Some(theme_color) = &self.theme_color {
            let mut color = theme_color.default_color();

            // Apply shade first (darken), then tint (lighten)
            // Note: In practice, only one should be set, but we handle both
            if let Some(shade) = self.theme_shade {
                color = color.shade(shade);
            }
            if let Some(tint) = self.theme_tint {
                color = color.tint(tint);
            }

            return Some(color);
        }

        // 3. No color available
        None
    }
}

impl Default for ColorEntry {
    fn default() -> Self {
        Self::auto_color()
    }
}

// =============================================================================
// Shading Types for Block/Cell Background Patterns
// =============================================================================

/// Pattern types for shading.
///
/// Represents the various fill patterns supported by RTF/DOCX for cell and
/// paragraph backgrounds. The pattern determines how the fill color and
/// pattern color are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadingPattern {
    /// No pattern (transparent/clear)
    #[default]
    Clear,
    /// Solid fill with fill_color only
    Solid,
    /// Horizontal stripes using pattern_color on fill_color
    HorzStripe,
    /// Vertical stripes using pattern_color on fill_color
    VertStripe,
    /// Diagonal stripes (forward slash direction)
    DiagStripe,
    /// Reverse diagonal stripes (backslash direction)
    ReverseDiagStripe,
    /// Horizontal crosshatch pattern
    HorzCross,
    /// Diagonal crosshatch pattern
    DiagCross,
    /// Light percentage patterns (5-95% density)
    Percent5,
    Percent10,
    Percent20,
    Percent25,
    Percent30,
    Percent40,
    Percent50,
    Percent60,
    Percent70,
    Percent75,
    Percent80,
    Percent90,
}

/// A reusable shading object for cell and paragraph backgrounds.
///
/// Shading supports both solid fills and pattern-based fills. The pattern
/// determines how `fill_color` and `pattern_color` are combined.
///
/// - For `Solid` pattern: only `fill_color` is used
/// - For pattern patterns: `fill_color` is the background, `pattern_color` is the foreground
/// - For `Clear`: no colors are needed (transparent)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shading {
    /// The background/fill color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<Color>,
    /// The pattern foreground color (used with patterned fills)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_color: Option<Color>,
    /// The pattern type (defaults to Solid if fill_color is set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<ShadingPattern>,
}

impl Shading {
    /// Creates a new empty shading (transparent).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a solid fill shading with the given color.
    pub fn solid(color: Color) -> Self {
        Self {
            fill_color: Some(color),
            pattern_color: None,
            pattern: Some(ShadingPattern::Solid),
        }
    }

    /// Creates a patterned shading with fill and pattern colors.
    pub fn with_pattern(fill_color: Color, pattern_color: Color, pattern: ShadingPattern) -> Self {
        Self {
            fill_color: Some(fill_color),
            pattern_color: Some(pattern_color),
            pattern: Some(pattern),
        }
    }
}

// =============================================================================
// Border Types for Table Cells, Rows, and Tables
// =============================================================================

/// Border line style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    /// No border (explicitly suppressed).
    None,
    /// Single solid line.
    Single,
    /// Double line.
    Double,
    /// Dotted line.
    Dotted,
    /// Dashed line.
    Dashed,
}

/// A single border side with style, width, and optional color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Border {
    /// The line style of this border.
    pub style: BorderStyle,
    /// Width in half-points (RTF `\brdrwN` native unit). `None` → writer default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_half_pts: Option<u32>,
    /// Border color. `None` → automatic/black.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

/// Up to six named border sides for a cell, row, or table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderSet {
    /// Top border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<Border>,
    /// Left border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<Border>,
    /// Bottom border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<Border>,
    /// Right border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<Border>,
    /// Inside-horizontal rule (between rows, row-level only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inside_h: Option<Border>,
    /// Inside-vertical rule (between columns, row-level only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inside_v: Option<Border>,
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
    /// Row-level shading (default for cells in this row)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shading: Option<Shading>,
    /// Row-level border defaults (outer edges and inside rules).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borders: Option<BorderSet>,
}

/// Table-level properties.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableProps {
    /// Table-level shading (default for all cells in table)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shading: Option<Shading>,
    /// Table-level border defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borders: Option<BorderSet>,
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
///     shading: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Paragraph {
    /// Horizontal alignment of the paragraph
    #[serde(default)]
    pub alignment: Alignment,
    /// The inline elements that make up this paragraph
    pub inlines: Vec<Inline>,
    /// Paragraph-level shading (background)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shading: Option<Shading>,
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
            shading: None,
        }
    }

    /// Creates a paragraph from a vector of inline elements.
    pub fn from_inlines(inlines: Vec<Inline>) -> Self {
        Self {
            alignment: Alignment::default(),
            inlines,
            shading: None,
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
///                     shading: None,
///                     borders: None,
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

    /// Cell-level shading (background)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shading: Option<Shading>,
    /// Per-side cell borders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borders: Option<BorderSet>,
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
            shading: None,
            borders: None,
        }
    }

    /// Creates a cell from a single paragraph.
    pub fn from_paragraph(paragraph: Paragraph) -> Self {
        Self {
            blocks: vec![Block::Paragraph(paragraph)],
            width_twips: None,
            merge: None,
            v_align: None,
            shading: None,
            borders: None,
        }
    }

    /// Creates a cell from a paragraph with width.
    pub fn from_paragraph_with_width(paragraph: Paragraph, width_twips: i32) -> Self {
        Self {
            blocks: vec![Block::Paragraph(paragraph)],
            width_twips: Some(width_twips),
            merge: None,
            v_align: None,
            shading: None,
            borders: None,
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

// =============================================================================
// Image Types for Phase 6 (Images)
// =============================================================================

/// Image format for embedded pictures.
///
/// Represents the supported image formats for RTF `\pict` groups.
/// RTF supports various image formats, but we currently only support
/// the most common ones: PNG and JPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// PNG image format
    Png,
    /// JPEG image format
    Jpeg,
}

/// An embedded image block.
///
/// Represents an image embedded in the document via RTF `\pict` groups.
/// The image data is stored as raw bytes in the format specified by `format`.
/// Dimensions are stored in twips (1/20th of a point) when available.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageBlock {
    /// The image format (PNG or JPEG)
    pub format: ImageFormat,
    /// Raw image data bytes
    pub data: Vec<u8>,
    /// Image width in twips (1/20th point), if specified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_twips: Option<i32>,
    /// Image height in twips (1/20th point), if specified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_twips: Option<i32>,
}

impl ImageBlock {
    /// Creates a new image block with the given format and data.
    pub fn new(format: ImageFormat, data: Vec<u8>) -> Self {
        Self {
            format,
            data,
            width_twips: None,
            height_twips: None,
        }
    }

    /// Creates an image block with dimensions.
    pub fn with_dimensions(
        format: ImageFormat,
        data: Vec<u8>,
        width_twips: i32,
        height_twips: i32,
    ) -> Self {
        Self {
            format,
            data,
            width_twips: Some(width_twips),
            height_twips: Some(height_twips),
        }
    }
}

/// A block-level element in the document.
///
/// `Block` represents the top-level structural elements of a document.
/// Currently supports paragraphs, lists, tables, and images.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    /// A paragraph block containing text
    Paragraph(Paragraph),
    /// A list block containing items
    ListBlock(ListBlock),
    /// A table block containing rows and cells
    TableBlock(TableBlock),
    /// An image block containing embedded picture data
    ImageBlock(ImageBlock),
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

    // ==========================================================================
    // Theme Color Resolution Tests (Slice C)
    // ==========================================================================

    #[test]
    fn test_theme_color_from_index_valid() {
        assert_eq!(ThemeColor::from_index(0), Some(ThemeColor::Light1));
        assert_eq!(ThemeColor::from_index(1), Some(ThemeColor::Dark1));
        assert_eq!(ThemeColor::from_index(2), Some(ThemeColor::Light2));
        assert_eq!(ThemeColor::from_index(3), Some(ThemeColor::Dark2));
        assert_eq!(ThemeColor::from_index(4), Some(ThemeColor::Accent1));
        assert_eq!(ThemeColor::from_index(5), Some(ThemeColor::Accent2));
        assert_eq!(ThemeColor::from_index(6), Some(ThemeColor::Accent3));
        assert_eq!(ThemeColor::from_index(7), Some(ThemeColor::Accent4));
        assert_eq!(ThemeColor::from_index(8), Some(ThemeColor::Accent5));
        assert_eq!(ThemeColor::from_index(9), Some(ThemeColor::Accent6));
        assert_eq!(ThemeColor::from_index(10), Some(ThemeColor::Hyperlink));
        assert_eq!(
            ThemeColor::from_index(11),
            Some(ThemeColor::FollowedHyperlink)
        );
    }

    #[test]
    fn test_theme_color_from_index_invalid() {
        assert_eq!(ThemeColor::from_index(-1), None);
        assert_eq!(ThemeColor::from_index(12), None);
        assert_eq!(ThemeColor::from_index(100), None);
    }

    #[test]
    fn test_theme_color_default_colors() {
        // Light1 = White
        let lt1 = ThemeColor::Light1.default_color();
        assert_eq!(lt1.r, 255);
        assert_eq!(lt1.g, 255);
        assert_eq!(lt1.b, 255);

        // Dark1 = Black
        let dk1 = ThemeColor::Dark1.default_color();
        assert_eq!(dk1.r, 0);
        assert_eq!(dk1.g, 0);
        assert_eq!(dk1.b, 0);

        // Hyperlink = #0563C1
        let hlink = ThemeColor::Hyperlink.default_color();
        assert_eq!(hlink.r, 0x05);
        assert_eq!(hlink.g, 0x63);
        assert_eq!(hlink.b, 0xC1);

        // FollowedHyperlink = #954F72
        let fol_hlink = ThemeColor::FollowedHyperlink.default_color();
        assert_eq!(fol_hlink.r, 0x95);
        assert_eq!(fol_hlink.g, 0x4F);
        assert_eq!(fol_hlink.b, 0x72);
    }

    #[test]
    fn test_color_tint_no_change() {
        let color = Color::new(128, 128, 128);
        let tinted = color.tint(0);
        assert_eq!(tinted.r, 128);
        assert_eq!(tinted.g, 128);
        assert_eq!(tinted.b, 128);
    }

    #[test]
    fn test_color_tint_full() {
        let color = Color::new(0, 0, 0);
        let tinted = color.tint(255);
        // Full tint should produce white
        assert_eq!(tinted.r, 255);
        assert_eq!(tinted.g, 255);
        assert_eq!(tinted.b, 255);
    }

    #[test]
    fn test_color_tint_partial() {
        let color = Color::new(0, 0, 0);
        // Tint of 128 (50%) should produce (128, 128, 128)
        let tinted = color.tint(128);
        assert_eq!(tinted.r, 128);
        assert_eq!(tinted.g, 128);
        assert_eq!(tinted.b, 128);
    }

    #[test]
    fn test_color_shade_no_change() {
        let color = Color::new(128, 128, 128);
        let shaded = color.shade(0);
        assert_eq!(shaded.r, 128);
        assert_eq!(shaded.g, 128);
        assert_eq!(shaded.b, 128);
    }

    #[test]
    fn test_color_shade_full() {
        let color = Color::new(255, 255, 255);
        let shaded = color.shade(255);
        // Full shade should produce black
        assert_eq!(shaded.r, 0);
        assert_eq!(shaded.g, 0);
        assert_eq!(shaded.b, 0);
    }

    #[test]
    fn test_color_shade_partial() {
        let color = Color::new(100, 100, 100);
        // Shade of 128 (50%) should produce (50, 50, 50)
        let shaded = color.shade(128);
        assert_eq!(shaded.r, 50);
        assert_eq!(shaded.g, 50);
        assert_eq!(shaded.b, 50);
    }

    #[test]
    fn test_color_entry_rgb_resolution() {
        let entry = ColorEntry::rgb(Color::new(255, 0, 0));
        let resolved = entry.resolve();
        assert!(resolved.is_some());
        let color = resolved.unwrap();
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 0);
        assert_eq!(color.b, 0);
    }

    #[test]
    fn test_color_entry_theme_resolution() {
        let entry = ColorEntry::theme(ThemeColor::Hyperlink, None, None);
        let resolved = entry.resolve();
        assert!(resolved.is_some());
        let color = resolved.unwrap();
        // Hyperlink default is #0563C1
        assert_eq!(color.r, 0x05);
        assert_eq!(color.g, 0x63);
        assert_eq!(color.b, 0xC1);
    }

    #[test]
    fn test_color_entry_theme_with_tint() {
        // Dark1 (black) with 50% tint should be gray (128, 128, 128)
        let entry = ColorEntry::theme(ThemeColor::Dark1, Some(128), None);
        let resolved = entry.resolve();
        assert!(resolved.is_some());
        let color = resolved.unwrap();
        assert_eq!(color.r, 128);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 128);
    }

    #[test]
    fn test_color_entry_theme_with_shade() {
        // Light1 (white) with shade 128 produces gray
        // shade(128) means factor = 128/255 ≈ 0.502
        // 255 * (1 - 0.502) ≈ 127
        let entry = ColorEntry::theme(ThemeColor::Light1, None, Some(128));
        let resolved = entry.resolve();
        assert!(resolved.is_some());
        let color = resolved.unwrap();
        assert_eq!(color.r, 127);
        assert_eq!(color.g, 127);
        assert_eq!(color.b, 127);
    }

    #[test]
    fn test_color_entry_rgb_takes_precedence_over_theme() {
        // When both RGB and theme are present, RGB should win
        let entry = ColorEntry {
            rgb: Some(Color::new(255, 0, 0)),
            theme_color: Some(ThemeColor::Hyperlink),
            theme_tint: None,
            theme_shade: None,
        };
        let resolved = entry.resolve();
        assert!(resolved.is_some());
        let color = resolved.unwrap();
        // Should be the explicit RGB, not the theme color
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 0);
        assert_eq!(color.b, 0);
    }

    #[test]
    fn test_color_entry_auto_color_resolves_to_none() {
        let entry = ColorEntry::auto_color();
        let resolved = entry.resolve();
        assert!(resolved.is_none());
    }

    #[test]
    fn test_color_entry_default_is_auto() {
        let entry = ColorEntry::default();
        assert!(entry.rgb.is_none());
        assert!(entry.theme_color.is_none());
        assert!(entry.theme_tint.is_none());
        assert!(entry.theme_shade.is_none());
    }

    #[test]
    fn test_deterministic_fallback_chain() {
        // Test the fallback chain: RGB > theme > None

        // 1. Explicit RGB wins
        let entry_rgb = ColorEntry {
            rgb: Some(Color::new(100, 100, 100)),
            theme_color: Some(ThemeColor::Dark1),
            theme_tint: None,
            theme_shade: None,
        };
        let resolved_rgb = entry_rgb.resolve().unwrap();
        assert_eq!(resolved_rgb.r, 100);

        // 2. Theme color when no RGB
        let entry_theme = ColorEntry {
            rgb: None,
            theme_color: Some(ThemeColor::Dark1),
            theme_tint: None,
            theme_shade: None,
        };
        let resolved_theme = entry_theme.resolve().unwrap();
        assert_eq!(resolved_theme.r, 0); // Dark1 is black

        // 3. None when neither RGB nor theme
        let entry_none = ColorEntry::auto_color();
        assert!(entry_none.resolve().is_none());
    }

    #[test]
    fn test_no_hard_failure_for_missing_theme_data() {
        // Invalid theme color index should not cause hard failure
        assert!(ThemeColor::from_index(999).is_none());

        // ColorEntry with missing theme data should resolve to None
        let entry = ColorEntry::auto_color();
        assert!(entry.resolve().is_none());
    }
}
