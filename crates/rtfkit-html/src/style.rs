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

/// Returns the default tokenized CSS stylesheet.
///
/// This provides styling for HTML output using CSS custom properties (tokens)
/// for consistent theming and maintainability per the PHASE_CSS_POLISH spec.
pub fn default_stylesheet() -> &'static str {
    r#":root{--rtf-font-body:system-ui,-apple-system,"Segoe UI",Roboto,"Helvetica Neue",Arial,sans-serif;--rtf-font-mono:"SF Mono",Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace;--rtf-color-text:#1a1a1a;--rtf-color-text-muted:#5a5a5a;--rtf-color-background:#fff;--rtf-color-link:#06c;--rtf-color-link-hover:#049;--rtf-color-border:#d0d0d0;--rtf-color-border-light:#e8e8e8;--rtf-color-table-header-bg:#f5f5f5;--rtf-color-table-stripe-bg:#fafafa;--rtf-space-xs:0.25rem;--rtf-space-sm:0.5rem;--rtf-space-md:1rem;--rtf-space-lg:1.5rem;--rtf-space-xl:2rem;--rtf-table-cell-padding:0.5rem 0.75rem;--rtf-table-border-width:1px}
.rtf-doc{font-family:var(--rtf-font-body);color:var(--rtf-color-text);background:var(--rtf-color-background);line-height:1.6}
.rtf-content{max-width:800px;margin:0 auto;padding:var(--rtf-space-lg)}
.rtf-p{margin:0 0 var(--rtf-space-md)}
.rtf-link{color:var(--rtf-color-link);text-decoration:none}
.rtf-link:hover{color:var(--rtf-color-link-hover);text-decoration:underline}
.rtf-table{width:100%;border-collapse:collapse;margin:var(--rtf-space-md) 0}
.rtf-table tr:first-child{background:var(--rtf-color-table-header-bg)}
.rtf-table tr:nth-child(even){background:var(--rtf-color-table-stripe-bg)}
.rtf-table th,.rtf-table td{padding:var(--rtf-table-cell-padding);border:var(--rtf-table-border-width) solid var(--rtf-color-border)}
.rtf-list{margin:var(--rtf-space-md) 0;padding-left:var(--rtf-space-lg)}
.rtf-align-left{text-align:left}
.rtf-align-center{text-align:center}
.rtf-align-right{text-align:right}
.rtf-align-justify{text-align:justify}
.rtf-u{text-decoration:underline}
.rtf-valign-top{vertical-align:top}
.rtf-valign-middle{vertical-align:middle}
.rtf-valign-bottom{vertical-align:bottom}
@media print{@page{margin:2cm;size:A4 portrait}
.rtf-doc{color:#000;background:#fff}
.rtf-link{color:#000;text-decoration:underline}
.rtf-link[href]::after{content:" (" attr(href) ")";font-size:0.85em;color:#5a5a5a}
.rtf-table tr{break-inside:avoid}
.rtf-p{orphans:3;widows:3}
.rtf-table{break-before:auto;break-after:auto;max-width:100%}
h1,h2,h3{break-after:avoid}}"#
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
        assert_eq!(
            alignment_class(Alignment::Justify),
            Some("rtf-align-justify")
        );
    }

    #[test]
    fn test_valign_class() {
        assert_eq!(valign_class(CellVerticalAlign::Top), None);
        assert_eq!(
            valign_class(CellVerticalAlign::Center),
            Some("rtf-valign-middle")
        );
        assert_eq!(
            valign_class(CellVerticalAlign::Bottom),
            Some("rtf-valign-bottom")
        );
    }

    #[test]
    fn test_default_stylesheet() {
        let css = default_stylesheet();
        // Check for CSS custom properties
        assert!(css.contains("--rtf-font-body"));
        assert!(css.contains("--rtf-color-text"));
        assert!(css.contains("--rtf-space-md"));
        // Check for new classes
        assert!(css.contains(".rtf-doc"));
        assert!(css.contains(".rtf-content"));
        assert!(css.contains(".rtf-p"));
        assert!(css.contains(".rtf-link"));
        assert!(css.contains(".rtf-list"));
        // Check for existing classes still present
        assert!(css.contains(".rtf-align-center"));
        assert!(css.contains(".rtf-u"));
        assert!(css.contains(".rtf-table"));
        assert!(css.contains(".rtf-valign-top"));
    }

    #[test]
    fn test_default_stylesheet_print_media() {
        let css = default_stylesheet();
        // Check for print media query
        assert!(
            css.contains("@media print"),
            "Print media query should be present"
        );
        // Check for @page rule
        assert!(
            css.contains("@page{margin:2cm;size:A4 portrait}"),
            "@page rule should be present"
        );
        // Check for print-specific color adjustments
        assert!(
            css.contains(".rtf-doc{color:#000;background:#fff}"),
            "Print color adjustments should be present"
        );
        // Check for link handling in print
        assert!(
            css.contains(".rtf-link[href]::after"),
            "Link href after pseudo-element should be present"
        );
        // Check for table row break prevention
        assert!(
            css.contains(".rtf-table tr{break-inside:avoid}"),
            "Table row break prevention should be present"
        );
        // Check for widow/orphan control
        assert!(
            css.contains("orphans:3"),
            "Orphans control should be present"
        );
        assert!(css.contains("widows:3"), "Widows control should be present");
        // Check for page break hints
        assert!(
            css.contains("break-before:auto"),
            "Page break hints should be present"
        );
        assert!(
            css.contains("break-after:avoid"),
            "Heading break-after should be present"
        );
    }

    /// Test that default CSS serialization is deterministic.
    /// Calling default_stylesheet() multiple times should return identical output.
    #[test]
    fn test_default_stylesheet_is_deterministic() {
        let css1 = default_stylesheet();
        let css2 = default_stylesheet();
        assert_eq!(
            css1, css2,
            "default_stylesheet() should return identical output on each call"
        );
    }

    /// Test that CSS contains all expected CSS custom property tokens.
    #[test]
    fn test_default_stylesheet_contains_expected_tokens() {
        let css = default_stylesheet();

        // Check for CSS custom properties (tokens)
        assert!(
            css.contains("--rtf-font-body"),
            "CSS should contain --rtf-font-body token"
        );
        assert!(
            css.contains("--rtf-font-mono"),
            "CSS should contain --rtf-font-mono token"
        );
        assert!(
            css.contains("--rtf-color-text"),
            "CSS should contain --rtf-color-text token"
        );
        assert!(
            css.contains("--rtf-color-text-muted"),
            "CSS should contain --rtf-color-text-muted token"
        );
        assert!(
            css.contains("--rtf-color-background"),
            "CSS should contain --rtf-color-background token"
        );
        assert!(
            css.contains("--rtf-color-link"),
            "CSS should contain --rtf-color-link token"
        );
        assert!(
            css.contains("--rtf-color-link-hover"),
            "CSS should contain --rtf-color-link-hover token"
        );
        assert!(
            css.contains("--rtf-color-border"),
            "CSS should contain --rtf-color-border token"
        );
        assert!(
            css.contains("--rtf-color-border-light"),
            "CSS should contain --rtf-color-border-light token"
        );
        assert!(
            css.contains("--rtf-color-table-header-bg"),
            "CSS should contain --rtf-color-table-header-bg token"
        );
        assert!(
            css.contains("--rtf-color-table-stripe-bg"),
            "CSS should contain --rtf-color-table-stripe-bg token"
        );

        // Check for spacing tokens
        assert!(
            css.contains("--rtf-space-xs"),
            "CSS should contain --rtf-space-xs token"
        );
        assert!(
            css.contains("--rtf-space-sm"),
            "CSS should contain --rtf-space-sm token"
        );
        assert!(
            css.contains("--rtf-space-md"),
            "CSS should contain --rtf-space-md token"
        );
        assert!(
            css.contains("--rtf-space-lg"),
            "CSS should contain --rtf-space-lg token"
        );
        assert!(
            css.contains("--rtf-space-xl"),
            "CSS should contain --rtf-space-xl token"
        );

        // Check for table-specific tokens
        assert!(
            css.contains("--rtf-table-cell-padding"),
            "CSS should contain --rtf-table-cell-padding token"
        );
        assert!(
            css.contains("--rtf-table-border-width"),
            "CSS should contain --rtf-table-border-width token"
        );
    }
}
