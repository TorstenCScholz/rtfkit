//! Style profile and token type definitions.
//!
//! This module defines the canonical token model for cross-format styling.
//! All token definitions are owned by this crate to ensure single source of truth.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::fmt;

/// Main style profile containing all token families.
///
/// A `StyleProfile` represents a complete visual theme that can be applied
/// across different output formats (HTML, PDF via Typst).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StyleProfile {
    /// Profile identifier
    pub name: StyleProfileName,
    /// Color tokens for text, surfaces, borders, and links
    pub colors: ColorTokens,
    /// Typography tokens for fonts, sizes, line heights, and weights
    pub typography: TypographyTokens,
    /// Spacing tokens for consistent whitespace
    pub spacing: SpacingTokens,
    /// Layout tokens for page dimensions
    pub layout: LayoutTokens,
    /// Component-specific tokens for tables, lists, headings
    pub components: ComponentTokens,
}

/// Profile identifier.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StyleProfileName {
    /// Classic conservative style, close to current behavior
    Classic,
    /// Professional report style with strong hierarchy (DEFAULT)
    #[default]
    Report,
    /// Dense compact style for enterprise output
    Compact,
    /// Custom profile with user-defined name
    Custom(String),
}

impl fmt::Display for StyleProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StyleProfileName::Classic => write!(f, "classic"),
            StyleProfileName::Report => write!(f, "report"),
            StyleProfileName::Compact => write!(f, "compact"),
            StyleProfileName::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

// ============================================================================
// Color Tokens
// ============================================================================

/// A hexadecimal color value in #RRGGBB format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColorHex(String);

impl ColorHex {
    /// Creates a new ColorHex from a string.
    ///
    /// The string must be in #RRGGBB format (7 characters including the # prefix).
    pub fn new(value: String) -> Result<Self, ColorHexError> {
        if value.len() != 7 {
            return Err(ColorHexError::InvalidLength {
                actual: value.len(),
                expected: 7,
            });
        }
        if !value.starts_with('#') {
            return Err(ColorHexError::MissingHashPrefix);
        }
        // Validate hex digits
        for (i, c) in value[1..].chars().enumerate() {
            if !c.is_ascii_hexdigit() {
                return Err(ColorHexError::InvalidHexDigit {
                    position: i + 1,
                    character: c,
                });
            }
        }
        Ok(ColorHex(value))
    }

    /// Creates a ColorHex without validation.
    ///
    /// # Safety
    /// The caller must ensure the value is valid #RRGGBB format.
    pub unsafe fn new_unchecked(value: String) -> Self {
        debug_assert!(value.len() == 7 && value.starts_with('#'));
        ColorHex(value)
    }

    /// Returns the color as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the color without the # prefix.
    pub fn without_hash(&self) -> &str {
        &self.0[1..]
    }
}

impl fmt::Display for ColorHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for ColorHex {
    fn default() -> Self {
        ColorHex(String::from("#000000"))
    }
}

/// Error type for ColorHex validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ColorHexError {
    #[error("invalid color length: expected 7 characters, got {actual}")]
    InvalidLength { actual: usize, expected: usize },
    #[error("color must start with '#'")]
    MissingHashPrefix,
    #[error("invalid hex digit '{character}' at position {position}")]
    InvalidHexDigit { position: usize, character: char },
}

/// Color tokens for text, surfaces, borders, and links.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColorTokens {
    /// Primary text color
    pub text_primary: ColorHex,
    /// Muted/deemphasized text color
    pub text_muted: ColorHex,
    /// Page background color
    pub surface_page: ColorHex,
    /// Table header background color
    pub surface_table_header: ColorHex,
    /// Table stripe (alternating row) background color
    pub surface_table_stripe: ColorHex,
    /// Default border color
    pub border_default: ColorHex,
    /// Link color
    pub link_default: ColorHex,
    /// Link hover/focus color
    pub link_hover: ColorHex,
}

impl Default for ColorTokens {
    fn default() -> Self {
        ColorTokens {
            text_primary: ColorHex::new(String::from("#000000")).unwrap(),
            text_muted: ColorHex::new(String::from("#666666")).unwrap(),
            surface_page: ColorHex::new(String::from("#FFFFFF")).unwrap(),
            surface_table_header: ColorHex::new(String::from("#F0F0F0")).unwrap(),
            surface_table_stripe: ColorHex::new(String::from("#F8F8F8")).unwrap(),
            border_default: ColorHex::new(String::from("#CCCCCC")).unwrap(),
            link_default: ColorHex::new(String::from("#0066CC")).unwrap(),
            link_hover: ColorHex::new(String::from("#004499")).unwrap(),
        }
    }
}

// ============================================================================
// Typography Tokens
// ============================================================================

/// Typography tokens for fonts, sizes, line heights, and weights.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TypographyTokens {
    /// Body text font family
    pub font_body: String,
    /// Heading font family
    pub font_heading: String,
    /// Monospace/code font family
    pub font_mono: String,
    /// Body text size in points
    pub size_body: f32,
    /// Small text size in points
    pub size_small: f32,
    /// Heading 1 size in points
    pub size_h1: f32,
    /// Heading 2 size in points
    pub size_h2: f32,
    /// Heading 3 size in points
    pub size_h3: f32,
    /// Body text line height (unitless ratio)
    pub line_height_body: f32,
    /// Heading line height (unitless ratio)
    pub line_height_heading: f32,
    /// Regular font weight (100-900 scale)
    pub weight_regular: u16,
    /// Semibold font weight (100-900 scale)
    pub weight_semibold: u16,
    /// Bold font weight (100-900 scale)
    pub weight_bold: u16,
}

impl Default for TypographyTokens {
    fn default() -> Self {
        TypographyTokens {
            font_body: String::from("Georgia, serif"),
            font_heading: String::from("Arial, sans-serif"),
            font_mono: String::from("Consolas, monospace"),
            size_body: 11.0,
            size_small: 9.0,
            size_h1: 24.0,
            size_h2: 18.0,
            size_h3: 14.0,
            line_height_body: 1.5,
            line_height_heading: 1.3,
            weight_regular: 400,
            weight_semibold: 600,
            weight_bold: 700,
        }
    }
}

// ============================================================================
// Spacing Tokens
// ============================================================================

/// Spacing tokens for consistent whitespace.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpacingTokens {
    /// Extra small spacing in points
    pub space_xs: f32,
    /// Small spacing in points
    pub space_sm: f32,
    /// Medium spacing in points
    pub space_md: f32,
    /// Large spacing in points
    pub space_lg: f32,
    /// Extra large spacing in points
    pub space_xl: f32,
    /// Gap between paragraphs in points
    pub paragraph_gap: f32,
    /// Gap between list items in points
    pub list_item_gap: f32,
    /// Horizontal padding for table cells in points
    pub table_cell_padding_x: f32,
    /// Vertical padding for table cells in points
    pub table_cell_padding_y: f32,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        SpacingTokens {
            space_xs: 2.0,
            space_sm: 4.0,
            space_md: 8.0,
            space_lg: 16.0,
            space_xl: 24.0,
            paragraph_gap: 10.0,
            list_item_gap: 4.0,
            table_cell_padding_x: 6.0,
            table_cell_padding_y: 4.0,
        }
    }
}

// ============================================================================
// Layout Tokens
// ============================================================================

/// Layout tokens for page dimensions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LayoutTokens {
    /// Maximum content width in millimeters
    pub content_max_width_mm: f32,
    /// Top page margin in millimeters
    pub page_margin_top_mm: f32,
    /// Bottom page margin in millimeters
    pub page_margin_bottom_mm: f32,
    /// Left page margin in millimeters
    pub page_margin_left_mm: f32,
    /// Right page margin in millimeters
    pub page_margin_right_mm: f32,
}

impl Default for LayoutTokens {
    fn default() -> Self {
        LayoutTokens {
            content_max_width_mm: 170.0,
            page_margin_top_mm: 25.0,
            page_margin_bottom_mm: 25.0,
            page_margin_left_mm: 20.0,
            page_margin_right_mm: 20.0,
        }
    }
}

// ============================================================================
// Component Tokens
// ============================================================================

/// Component-specific tokens for tables, lists, headings.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ComponentTokens {
    /// Table component tokens
    pub table: TableComponentTokens,
    /// List component tokens
    pub list: ListComponentTokens,
    /// Heading component tokens
    pub heading: HeadingComponentTokens,
}

/// Table striping mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TableStripeMode {
    /// No striping
    #[default]
    None,
    /// Alternate row striping
    AlternateRows,
}

/// Table component tokens.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TableComponentTokens {
    /// Border width in points
    pub border_width: f32,
    /// Row striping mode
    pub stripe_mode: TableStripeMode,
    /// Whether to emphasize the header row
    pub header_emphasis: bool,
}

impl Default for TableComponentTokens {
    fn default() -> Self {
        TableComponentTokens {
            border_width: 0.5,
            stripe_mode: TableStripeMode::None,
            header_emphasis: true,
        }
    }
}

/// List component tokens.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ListComponentTokens {
    /// Indentation step per level in points
    pub indentation_step: f32,
    /// Gap between marker and content in points
    pub marker_gap: f32,
}

impl Default for ListComponentTokens {
    fn default() -> Self {
        ListComponentTokens {
            indentation_step: 18.0,
            marker_gap: 6.0,
        }
    }
}

/// Heading component tokens.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeadingComponentTokens {
    /// Spacing above H1 in points
    pub spacing_above_h1: f32,
    /// Spacing below H1 in points
    pub spacing_below_h1: f32,
    /// Spacing above H2 in points
    pub spacing_above_h2: f32,
    /// Spacing below H2 in points
    pub spacing_below_h2: f32,
    /// Spacing above H3 in points
    pub spacing_above_h3: f32,
    /// Spacing below H3 in points
    pub spacing_below_h3: f32,
}

impl Default for HeadingComponentTokens {
    fn default() -> Self {
        HeadingComponentTokens {
            spacing_above_h1: 24.0,
            spacing_below_h1: 12.0,
            spacing_above_h2: 18.0,
            spacing_below_h2: 10.0,
            spacing_above_h3: 14.0,
            spacing_below_h3: 8.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_hex_valid() {
        let color = ColorHex::new(String::from("#FF00FF")).unwrap();
        assert_eq!(color.as_str(), "#FF00FF");
        assert_eq!(color.without_hash(), "FF00FF");
    }

    #[test]
    fn color_hex_invalid_length() {
        let result = ColorHex::new(String::from("#FFF"));
        assert!(matches!(result, Err(ColorHexError::InvalidLength { .. })));
    }

    #[test]
    fn color_hex_missing_hash() {
        // Use a 7-character string without hash to test MissingHashPrefix
        let result = ColorHex::new(String::from("XFF00FF"));
        assert!(matches!(result, Err(ColorHexError::MissingHashPrefix)));
    }

    #[test]
    fn color_hex_invalid_hex_digit() {
        let result = ColorHex::new(String::from("#FF0GFF"));
        assert!(matches!(result, Err(ColorHexError::InvalidHexDigit { .. })));
    }

    #[test]
    fn style_profile_name_display() {
        assert_eq!(StyleProfileName::Classic.to_string(), "classic");
        assert_eq!(StyleProfileName::Report.to_string(), "report");
        assert_eq!(StyleProfileName::Compact.to_string(), "compact");
        assert_eq!(
            StyleProfileName::Custom(String::from("my-theme")).to_string(),
            "custom:my-theme"
        );
    }

    #[test]
    fn default_tokens_are_valid() {
        let colors = ColorTokens::default();
        assert!(ColorHex::new(colors.text_primary.as_str().to_string()).is_ok());

        let typography = TypographyTokens::default();
        assert!(typography.size_body > 0.0);
        assert!(typography.line_height_body >= 1.0);
        assert!((100..=900).contains(&typography.weight_regular));

        let spacing = SpacingTokens::default();
        assert!(spacing.space_xs >= 0.0);

        let layout = LayoutTokens::default();
        assert!(layout.page_margin_top_mm >= 0.0);
    }
}
