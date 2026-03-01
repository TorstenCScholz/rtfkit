//! CSS class and style mapping utilities.
//!
//! This module provides helpers for mapping IR formatting attributes
//! to CSS classes and inline styles.

use rtfkit_core::{Alignment, CellVerticalAlign};
use rtfkit_style_tokens::{StyleProfile, StyleProfileName, builtins, serialize::to_css_variables};

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

/// Resolves a style profile name to a concrete style profile.
///
/// Built-in profiles (Classic, Report, Compact) are resolved to their
/// predefined configurations. Custom profile names fall back to Report
/// with a warning (future: load from file via --style-file).
///
/// # Example
///
/// ```rust
/// use rtfkit_html::style::resolve_style_profile;
/// use rtfkit_style_tokens::StyleProfileName;
///
/// let profile = resolve_style_profile(&StyleProfileName::Report);
/// assert_eq!(profile.name, StyleProfileName::Report);
/// ```
pub fn resolve_style_profile(name: &StyleProfileName) -> StyleProfile {
    builtins::resolve_profile(name)
}

/// Generates a complete CSS stylesheet for a given style profile.
///
/// The output includes:
/// 1. CSS custom properties (variables) from the style profile
/// 2. Semantic class rules that use those variables
/// 3. Print media query for PDF/print output
///
/// The output is deterministic: the same profile always produces identical CSS.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::style::stylesheet_for_profile;
/// use rtfkit_style_tokens::StyleProfileName;
///
/// let css = stylesheet_for_profile(&StyleProfileName::Report);
/// assert!(css.contains("--rtfkit-color-text-primary"));
/// assert!(css.contains(".rtf-doc"));
/// ```
pub fn stylesheet_for_profile(name: &StyleProfileName) -> String {
    let profile = resolve_style_profile(name);
    generate_stylesheet(&profile)
}

/// Generates a complete CSS stylesheet from a resolved style profile.
///
/// This is an internal function that combines CSS variables from the profile
/// with the semantic class rules.
fn generate_stylesheet(profile: &StyleProfile) -> String {
    let mut css = String::new();

    // 1. Generate CSS variables from the profile
    css.push_str(&to_css_variables(profile));
    css.push('\n');

    // 2. Add semantic class rules that use the variables
    // These use the rtfkit- prefixed variables from the profile
    css.push_str(semantic_css_rules());

    css
}

/// Returns the semantic CSS class rules.
///
/// These rules use the CSS custom properties defined in the :root block
/// to provide consistent styling for RTF-derived content.
fn semantic_css_rules() -> &'static str {
    r#".rtf-doc{font-family:var(--rtfkit-font-body);font-size:var(--rtfkit-size-body);color:var(--rtfkit-color-text-primary);background:var(--rtfkit-color-surface-page);line-height:var(--rtfkit-line-height-body)}
.rtf-content{max-width:var(--rtfkit-content-max-width);margin:0 auto;padding:var(--rtfkit-space-lg)}
.rtf-p{margin:0 0 var(--rtfkit-paragraph-gap)}
.rtf-link{color:var(--rtfkit-color-link-default);text-decoration:none}
.rtf-link:hover{color:var(--rtfkit-color-link-hover);text-decoration:underline}
.rtf-table{width:100%;border-collapse:collapse;margin:var(--rtfkit-space-md) 0}
.rtf-table tr:first-child{background:var(--rtfkit-color-surface-table-header);font-weight:var(--rtfkit-table-header-font-weight)}
.rtf-table tr:nth-child(even){background:var(--rtfkit-table-stripe-fill)}
.rtf-table th,.rtf-table td{padding:var(--rtfkit-table-cell-padding-y) var(--rtfkit-table-cell-padding-x);border:var(--rtfkit-table-border-width) solid var(--rtfkit-color-border-default)}
.rtf-list{margin:var(--rtfkit-space-md) 0;padding-left:var(--rtfkit-list-indentation-step)}
.rtf-list li{padding-left:var(--rtfkit-list-marker-gap);margin-bottom:var(--rtfkit-list-item-gap)}
.rtf-list li:last-child{margin-bottom:0}
h1{font-family:var(--rtfkit-font-heading);font-size:var(--rtfkit-size-h1);line-height:var(--rtfkit-line-height-heading);font-weight:var(--rtfkit-weight-bold);margin:var(--rtfkit-heading-spacing-above-h1) 0 var(--rtfkit-heading-spacing-below-h1)}
h2{font-family:var(--rtfkit-font-heading);font-size:var(--rtfkit-size-h2);line-height:var(--rtfkit-line-height-heading);font-weight:var(--rtfkit-weight-semibold);margin:var(--rtfkit-heading-spacing-above-h2) 0 var(--rtfkit-heading-spacing-below-h2)}
h3{font-family:var(--rtfkit-font-heading);font-size:var(--rtfkit-size-h3);line-height:var(--rtfkit-line-height-heading);font-weight:var(--rtfkit-weight-semibold);margin:var(--rtfkit-heading-spacing-above-h3) 0 var(--rtfkit-heading-spacing-below-h3)}
.rtf-align-left{text-align:left}
.rtf-align-center{text-align:center}
.rtf-align-right{text-align:right}
.rtf-align-justify{text-align:justify}
.rtf-u{text-decoration:underline}.rtf-s{text-decoration:line-through}.rtf-sc{font-variant:small-caps}.rtf-ac{text-transform:uppercase}
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

/// Returns the default tokenized CSS stylesheet.
///
/// This provides styling for HTML output using CSS custom properties (tokens)
/// for consistent theming and maintainability per the PHASE_CSS_POLISH spec.
///
/// Note: This function returns a stylesheet for the Report profile.
/// For other profiles, use [`stylesheet_for_profile`].
pub fn default_stylesheet() -> String {
    stylesheet_for_profile(&StyleProfileName::Report)
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
        // Check for CSS custom properties (now using rtfkit- prefix)
        assert!(css.contains("--rtfkit-font-body"));
        assert!(css.contains("--rtfkit-color-text-primary"));
        assert!(css.contains("--rtfkit-space-md"));
        // Check for new classes
        assert!(css.contains(".rtf-doc"));
        assert!(css.contains(".rtf-content"));
        assert!(css.contains(".rtf-p"));
        assert!(css.contains(".rtf-link"));
        assert!(css.contains(".rtf-list"));
        // Check for existing classes still present
        assert!(css.contains(".rtf-align-center"));
        assert!(css.contains(".rtf-u"));
        assert!(css.contains(".rtf-s"));
        assert!(css.contains(".rtf-sc"));
        assert!(css.contains(".rtf-ac"));
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

        // Check for CSS custom properties (tokens) with rtfkit- prefix
        assert!(
            css.contains("--rtfkit-font-body"),
            "CSS should contain --rtfkit-font-body token"
        );
        assert!(
            css.contains("--rtfkit-font-mono"),
            "CSS should contain --rtfkit-font-mono token"
        );
        assert!(
            css.contains("--rtfkit-color-text-primary"),
            "CSS should contain --rtfkit-color-text-primary token"
        );
        assert!(
            css.contains("--rtfkit-color-text-muted"),
            "CSS should contain --rtfkit-color-text-muted token"
        );
        assert!(
            css.contains("--rtfkit-color-surface-page"),
            "CSS should contain --rtfkit-color-surface-page token"
        );
        assert!(
            css.contains("--rtfkit-color-link-default"),
            "CSS should contain --rtfkit-color-link-default token"
        );
        assert!(
            css.contains("--rtfkit-color-link-hover"),
            "CSS should contain --rtfkit-color-link-hover token"
        );
        assert!(
            css.contains("--rtfkit-color-border-default"),
            "CSS should contain --rtfkit-color-border-default token"
        );
        assert!(
            css.contains("--rtfkit-color-surface-table-header"),
            "CSS should contain --rtfkit-color-surface-table-header token"
        );
        assert!(
            css.contains("--rtfkit-color-surface-table-stripe"),
            "CSS should contain --rtfkit-color-surface-table-stripe token"
        );

        // Check for spacing tokens
        assert!(
            css.contains("--rtfkit-space-xs"),
            "CSS should contain --rtfkit-space-xs token"
        );
        assert!(
            css.contains("--rtfkit-space-sm"),
            "CSS should contain --rtfkit-space-sm token"
        );
        assert!(
            css.contains("--rtfkit-space-md"),
            "CSS should contain --rtfkit-space-md token"
        );
        assert!(
            css.contains("--rtfkit-space-lg"),
            "CSS should contain --rtfkit-space-lg token"
        );
        assert!(
            css.contains("--rtfkit-space-xl"),
            "CSS should contain --rtfkit-space-xl token"
        );

        // Check for table-specific tokens
        assert!(
            css.contains("--rtfkit-table-cell-padding-x"),
            "CSS should contain --rtfkit-table-cell-padding-x token"
        );
        assert!(
            css.contains("--rtfkit-table-cell-padding-y"),
            "CSS should contain --rtfkit-table-cell-padding-y token"
        );
        assert!(
            css.contains("--rtfkit-table-border-width"),
            "CSS should contain --rtfkit-table-border-width token"
        );
    }

    /// Test that resolve_style_profile returns correct profiles.
    #[test]
    fn test_resolve_style_profile() {
        let classic = resolve_style_profile(&StyleProfileName::Classic);
        assert_eq!(classic.name, StyleProfileName::Classic);

        let report = resolve_style_profile(&StyleProfileName::Report);
        assert_eq!(report.name, StyleProfileName::Report);

        let compact = resolve_style_profile(&StyleProfileName::Compact);
        assert_eq!(compact.name, StyleProfileName::Compact);
    }

    /// Test that custom profile names fall back to Report.
    #[test]
    fn test_resolve_custom_profile_falls_back_to_report() {
        let custom = resolve_style_profile(&StyleProfileName::Custom(String::from("my-theme")));
        // Custom profiles fall back to report for MVP
        assert_eq!(custom.name, StyleProfileName::Report);
    }

    /// Test that stylesheet_for_profile produces different output for different profiles.
    #[test]
    fn test_stylesheet_for_different_profiles() {
        let classic_css = stylesheet_for_profile(&StyleProfileName::Classic);
        let report_css = stylesheet_for_profile(&StyleProfileName::Report);
        let compact_css = stylesheet_for_profile(&StyleProfileName::Compact);

        // All should contain the semantic classes
        assert!(classic_css.contains(".rtf-doc"));
        assert!(report_css.contains(".rtf-doc"));
        assert!(compact_css.contains(".rtf-doc"));

        // All profiles use Libertinus Serif as the body font
        assert!(classic_css.contains("Libertinus Serif"));
        assert!(report_css.contains("Libertinus Serif"));

        // Compact has smaller body size (9pt vs 11pt)
        assert!(compact_css.contains("--rtfkit-size-body: 9pt;"));
        assert!(report_css.contains("--rtfkit-size-body: 11pt;"));

        // Classic has larger body size (12pt)
        assert!(classic_css.contains("--rtfkit-size-body: 12pt;"));
    }

    /// Test that stylesheet generation is deterministic.
    #[test]
    fn test_stylesheet_is_deterministic() {
        let css1 = stylesheet_for_profile(&StyleProfileName::Report);
        let css2 = stylesheet_for_profile(&StyleProfileName::Report);
        assert_eq!(css1, css2, "Stylesheet generation should be deterministic");
    }
}
