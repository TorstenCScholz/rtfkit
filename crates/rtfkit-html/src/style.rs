//! CSS class and style mapping utilities.
//!
//! This module provides helpers for mapping IR formatting attributes
//! to CSS classes and inline styles.

use rtfkit_core::{Alignment, CellVerticalAlign};

/// Converts an alignment value to a CSS text-align value.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::style::alignment_to_css;
/// use rtfkit_core::Alignment;
///
/// assert_eq!(alignment_to_css(Alignment::Left), "left");
/// assert_eq!(alignment_to_css(Alignment::Center), "center");
/// assert_eq!(alignment_to_css(Alignment::Right), "right");
/// assert_eq!(alignment_to_css(Alignment::Justify), "justify");
/// ```
pub fn alignment_to_css(align: Alignment) -> &'static str {
    match align {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
    }
}

/// Converts an alignment value to a CSS class name.
///
/// Returns `None` for default alignment (Left), otherwise returns the class name.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::style::alignment_class;
/// use rtfkit_core::Alignment;
///
/// assert_eq!(alignment_class(Alignment::Left), None);
/// assert_eq!(alignment_class(Alignment::Center), Some("rtf-align-center"));
/// assert_eq!(alignment_class(Alignment::Right), Some("rtf-align-right"));
/// assert_eq!(alignment_class(Alignment::Justify), Some("rtf-align-justify"));
/// ```
pub fn alignment_class(align: Alignment) -> Option<&'static str> {
    match align {
        Alignment::Left => None, // Default, no class needed
        Alignment::Center => Some("rtf-align-center"),
        Alignment::Right => Some("rtf-align-right"),
        Alignment::Justify => Some("rtf-align-justify"),
    }
}

/// Converts a vertical alignment value to a CSS class name.
///
/// Returns `None` for default alignment (Top), otherwise returns the class name.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::style::valign_class;
/// use rtfkit_core::CellVerticalAlign;
///
/// assert_eq!(valign_class(CellVerticalAlign::Top), None);
/// assert_eq!(valign_class(CellVerticalAlign::Center), Some("rtf-valign-middle"));
/// assert_eq!(valign_class(CellVerticalAlign::Bottom), Some("rtf-valign-bottom"));
/// ```
pub fn valign_class(align: CellVerticalAlign) -> Option<&'static str> {
    match align {
        CellVerticalAlign::Top => None, // Default, no class needed
        CellVerticalAlign::Center => Some("rtf-valign-middle"),
        CellVerticalAlign::Bottom => Some("rtf-valign-bottom"),
    }
}

/// Returns the default minimal CSS stylesheet.
///
/// This provides basic styling for HTML output per the PHASE_HTML spec.
pub fn default_stylesheet() -> &'static str {
    r#".rtf-align-left{text-align:left}
.rtf-align-center{text-align:center}
.rtf-align-right{text-align:right}
.rtf-align-justify{text-align:justify}
.rtf-u{text-decoration:underline}
.rtf-table{border-collapse:collapse}
.rtf-valign-top{vertical-align:top}
.rtf-valign-middle{vertical-align:middle}
.rtf-valign-bottom{vertical-align:bottom}"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_to_css() {
        assert_eq!(alignment_to_css(Alignment::Left), "left");
        assert_eq!(alignment_to_css(Alignment::Center), "center");
        assert_eq!(alignment_to_css(Alignment::Right), "right");
        assert_eq!(alignment_to_css(Alignment::Justify), "justify");
    }

    #[test]
    fn test_alignment_class() {
        assert_eq!(alignment_class(Alignment::Left), None);
        assert_eq!(alignment_class(Alignment::Center), Some("rtf-align-center"));
        assert_eq!(alignment_class(Alignment::Right), Some("rtf-align-right"));
        assert_eq!(alignment_class(Alignment::Justify), Some("rtf-align-justify"));
    }

    #[test]
    fn test_valign_class() {
        assert_eq!(valign_class(CellVerticalAlign::Top), None);
        assert_eq!(valign_class(CellVerticalAlign::Center), Some("rtf-valign-middle"));
        assert_eq!(valign_class(CellVerticalAlign::Bottom), Some("rtf-valign-bottom"));
    }

    #[test]
    fn test_default_stylesheet() {
        let css = default_stylesheet();
        assert!(css.contains(".rtf-align-center"));
        assert!(css.contains(".rtf-u"));
        assert!(css.contains(".rtf-table"));
        assert!(css.contains(".rtf-valign-top"));
    }
}
