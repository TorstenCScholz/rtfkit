//! Deterministic serialization helpers for style profiles.
//!
//! This module provides functions to convert style profiles into
//! format-specific output (CSS variables, Typst preamble).
//!
//! All output is deterministic: the same input always produces the same output.

use crate::profile::{StyleProfile, StyleProfileName, TableStripeMode};

/// Generate CSS variables block from a style profile.
///
/// The output uses `--rtfkit-` prefix for all variables and maintains
/// a fixed ordering of properties for deterministic output.
///
/// # Example output
///
/// ```css
/// :root {
///   /* Colors */
///   --rtfkit-color-text-primary: #000000;
///   --rtfkit-color-text-muted: #666666;
///   ...
/// }
/// ```
pub fn to_css_variables(profile: &StyleProfile) -> String {
    let mut output = String::new();

    output.push_str(":root {\n");

    // Colors section
    output.push_str("  /* Colors */\n");
    output.push_str(&format!(
        "  --rtfkit-color-text-primary: {};\n",
        profile.colors.text_primary
    ));
    output.push_str(&format!(
        "  --rtfkit-color-text-muted: {};\n",
        profile.colors.text_muted
    ));
    output.push_str(&format!(
        "  --rtfkit-color-surface-page: {};\n",
        profile.colors.surface_page
    ));
    output.push_str(&format!(
        "  --rtfkit-color-surface-table-header: {};\n",
        profile.colors.surface_table_header
    ));
    output.push_str(&format!(
        "  --rtfkit-color-surface-table-stripe: {};\n",
        profile.colors.surface_table_stripe
    ));
    output.push_str(&format!(
        "  --rtfkit-color-border-default: {};\n",
        profile.colors.border_default
    ));
    output.push_str(&format!(
        "  --rtfkit-color-link-default: {};\n",
        profile.colors.link_default
    ));
    output.push_str(&format!(
        "  --rtfkit-color-link-hover: {};\n",
        profile.colors.link_hover
    ));

    // Typography section
    output.push_str("\n  /* Typography */\n");
    output.push_str(&format!(
        "  --rtfkit-font-body: {};\n",
        escape_css_string(&profile.typography.font_body)
    ));
    output.push_str(&format!(
        "  --rtfkit-font-heading: {};\n",
        escape_css_string(&profile.typography.font_heading)
    ));
    output.push_str(&format!(
        "  --rtfkit-font-mono: {};\n",
        escape_css_string(&profile.typography.font_mono)
    ));
    output.push_str(&format!(
        "  --rtfkit-size-body: {}pt;\n",
        profile.typography.size_body
    ));
    output.push_str(&format!(
        "  --rtfkit-size-small: {}pt;\n",
        profile.typography.size_small
    ));
    output.push_str(&format!(
        "  --rtfkit-size-h1: {}pt;\n",
        profile.typography.size_h1
    ));
    output.push_str(&format!(
        "  --rtfkit-size-h2: {}pt;\n",
        profile.typography.size_h2
    ));
    output.push_str(&format!(
        "  --rtfkit-size-h3: {}pt;\n",
        profile.typography.size_h3
    ));
    output.push_str(&format!(
        "  --rtfkit-line-height-body: {};\n",
        profile.typography.line_height_body
    ));
    output.push_str(&format!(
        "  --rtfkit-line-height-heading: {};\n",
        profile.typography.line_height_heading
    ));
    output.push_str(&format!(
        "  --rtfkit-weight-regular: {};\n",
        profile.typography.weight_regular
    ));
    output.push_str(&format!(
        "  --rtfkit-weight-semibold: {};\n",
        profile.typography.weight_semibold
    ));
    output.push_str(&format!(
        "  --rtfkit-weight-bold: {};\n",
        profile.typography.weight_bold
    ));

    // Spacing section
    output.push_str("\n  /* Spacing */\n");
    output.push_str(&format!(
        "  --rtfkit-space-xs: {}pt;\n",
        profile.spacing.space_xs
    ));
    output.push_str(&format!(
        "  --rtfkit-space-sm: {}pt;\n",
        profile.spacing.space_sm
    ));
    output.push_str(&format!(
        "  --rtfkit-space-md: {}pt;\n",
        profile.spacing.space_md
    ));
    output.push_str(&format!(
        "  --rtfkit-space-lg: {}pt;\n",
        profile.spacing.space_lg
    ));
    output.push_str(&format!(
        "  --rtfkit-space-xl: {}pt;\n",
        profile.spacing.space_xl
    ));
    output.push_str(&format!(
        "  --rtfkit-paragraph-gap: {}pt;\n",
        profile.spacing.paragraph_gap
    ));
    output.push_str(&format!(
        "  --rtfkit-list-item-gap: {}pt;\n",
        profile.spacing.list_item_gap
    ));
    output.push_str(&format!(
        "  --rtfkit-table-cell-padding-x: {}pt;\n",
        profile.spacing.table_cell_padding_x
    ));
    output.push_str(&format!(
        "  --rtfkit-table-cell-padding-y: {}pt;\n",
        profile.spacing.table_cell_padding_y
    ));

    // Layout section
    output.push_str("\n  /* Layout */\n");
    output.push_str(&format!(
        "  --rtfkit-content-max-width: {}mm;\n",
        profile.layout.content_max_width_mm
    ));
    output.push_str(&format!(
        "  --rtfkit-page-margin-top: {}mm;\n",
        profile.layout.page_margin_top_mm
    ));
    output.push_str(&format!(
        "  --rtfkit-page-margin-bottom: {}mm;\n",
        profile.layout.page_margin_bottom_mm
    ));
    output.push_str(&format!(
        "  --rtfkit-page-margin-left: {}mm;\n",
        profile.layout.page_margin_left_mm
    ));
    output.push_str(&format!(
        "  --rtfkit-page-margin-right: {}mm;\n",
        profile.layout.page_margin_right_mm
    ));

    // Components section
    output.push_str("\n  /* Components */\n");
    output.push_str(&format!(
        "  --rtfkit-table-border-width: {}pt;\n",
        profile.components.table.border_width
    ));
    output.push_str(&format!(
        "  --rtfkit-table-stripe-mode: {};\n",
        match profile.components.table.stripe_mode {
            TableStripeMode::None => "none",
            TableStripeMode::AlternateRows => "alternate-rows",
        }
    ));
    output.push_str(&format!(
        "  --rtfkit-table-header-emphasis: {};\n",
        profile.components.table.header_emphasis
    ));
    output.push_str(&format!(
        "  --rtfkit-table-stripe-fill: {};\n",
        match profile.components.table.stripe_mode {
            TableStripeMode::None => "transparent",
            TableStripeMode::AlternateRows => profile.colors.surface_table_stripe.as_str(),
        }
    ));
    output.push_str(&format!(
        "  --rtfkit-table-header-font-weight: {};\n",
        if profile.components.table.header_emphasis {
            profile.typography.weight_semibold
        } else {
            profile.typography.weight_regular
        }
    ));
    output.push_str(&format!(
        "  --rtfkit-list-indentation-step: {}pt;\n",
        profile.components.list.indentation_step
    ));
    output.push_str(&format!(
        "  --rtfkit-list-marker-gap: {}pt;\n",
        profile.components.list.marker_gap
    ));
    output.push_str(&format!(
        "  --rtfkit-heading-spacing-above-h1: {}pt;\n",
        profile.components.heading.spacing_above_h1
    ));
    output.push_str(&format!(
        "  --rtfkit-heading-spacing-below-h1: {}pt;\n",
        profile.components.heading.spacing_below_h1
    ));
    output.push_str(&format!(
        "  --rtfkit-heading-spacing-above-h2: {}pt;\n",
        profile.components.heading.spacing_above_h2
    ));
    output.push_str(&format!(
        "  --rtfkit-heading-spacing-below-h2: {}pt;\n",
        profile.components.heading.spacing_below_h2
    ));
    output.push_str(&format!(
        "  --rtfkit-heading-spacing-above-h3: {}pt;\n",
        profile.components.heading.spacing_above_h3
    ));
    output.push_str(&format!(
        "  --rtfkit-heading-spacing-below-h3: {}pt;\n",
        profile.components.heading.spacing_below_h3
    ));

    output.push_str("}\n");

    output
}

/// Generate Typst style preamble from a style profile.
///
/// The output sets document defaults, heading styles, and list/table rules.
/// All values are in Typst-compatible format.
///
/// # Example output
///
/// ```typst
/// // rtfkit style profile: report
/// ...
/// ```
pub fn to_typst_preamble(profile: &StyleProfile) -> String {
    let mut output = String::new();

    // Header comment
    let profile_name = match &profile.name {
        StyleProfileName::Classic => "classic",
        StyleProfileName::Report => "report",
        StyleProfileName::Compact => "compact",
        StyleProfileName::Custom(name) => name,
    };
    output.push_str(&format!(
        "// rtfkit style profile: {}\n\n",
        escape_typst_comment(profile_name)
    ));

    // Text defaults
    output.push_str("#set text(\n");
    output.push_str(&format!(
        "  font: (\"{}\"),\n",
        escape_typst_string(&profile.typography.font_body)
    ));
    output.push_str(&format!("  size: {}pt,\n", profile.typography.size_body));
    output.push_str(&format!(
        "  fill: rgb(\"{}\"),\n",
        profile.colors.text_primary.without_hash()
    ));
    output.push_str("  lang: \"en\",\n");
    output.push_str(")\n\n");

    // Paragraph spacing
    output.push_str("#set par(\n");
    output.push_str(&format!(
        "  leading: {}em,\n",
        profile.typography.line_height_body
    ));
    output.push_str(&format!(
        "  spacing: {}pt,\n",
        profile.spacing.paragraph_gap
    ));
    output.push_str(")\n\n");

    // Heading styles
    output.push_str("// Heading styles\n");
    output.push_str("#show heading.where(level: 1): it => {\n");
    output.push_str(&format!(
        "  set text(font: (\"{}\"), size: {}pt, weight: {})\n",
        escape_typst_string(&profile.typography.font_heading),
        profile.typography.size_h1,
        profile.typography.weight_bold
    ));
    output.push_str(&format!(
        "  set block(above: {}pt, below: {}pt)\n",
        profile.components.heading.spacing_above_h1, profile.components.heading.spacing_below_h1
    ));
    output.push_str("  it\n");
    output.push_str("}\n\n");

    output.push_str("#show heading.where(level: 2): it => {\n");
    output.push_str(&format!(
        "  set text(font: (\"{}\"), size: {}pt, weight: {})\n",
        escape_typst_string(&profile.typography.font_heading),
        profile.typography.size_h2,
        profile.typography.weight_semibold
    ));
    output.push_str(&format!(
        "  set block(above: {}pt, below: {}pt)\n",
        profile.components.heading.spacing_above_h2, profile.components.heading.spacing_below_h2
    ));
    output.push_str("  it\n");
    output.push_str("}\n\n");

    output.push_str("#show heading.where(level: 3): it => {\n");
    output.push_str(&format!(
        "  set text(font: (\"{}\"), size: {}pt, weight: {})\n",
        escape_typst_string(&profile.typography.font_heading),
        profile.typography.size_h3,
        profile.typography.weight_semibold
    ));
    output.push_str(&format!(
        "  set block(above: {}pt, below: {}pt)\n",
        profile.components.heading.spacing_above_h3, profile.components.heading.spacing_below_h3
    ));
    output.push_str("  it\n");
    output.push_str("}\n\n");

    // Link styles
    output.push_str("// Link styles\n");
    output.push_str("#show link: set text(fill: rgb(\"");
    output.push_str(profile.colors.link_default.without_hash());
    output.push_str("\"))\n\n");

    // Table styles
    output.push_str("// Table styles\n");
    output.push_str("#set table(\n");
    output.push_str(&format!(
        "  stroke: {}pt + rgb(\"{}\"),\n",
        profile.components.table.border_width,
        profile.colors.border_default.without_hash()
    ));
    output.push_str(&format!(
        "  inset: (x: {}pt, y: {}pt),\n",
        profile.spacing.table_cell_padding_x, profile.spacing.table_cell_padding_y
    ));
    if profile.components.table.header_emphasis
        || profile.components.table.stripe_mode == TableStripeMode::AlternateRows
    {
        output.push_str("  fill: (x, y) => {\n");
        match (
            profile.components.table.header_emphasis,
            profile.components.table.stripe_mode,
        ) {
            (true, TableStripeMode::AlternateRows) => {
                output.push_str("    if y == 0 {\n");
                output.push_str(&format!(
                    "      rgb(\"{}\")\n",
                    profile.colors.surface_table_header.without_hash()
                ));
                output.push_str("    } else if calc.rem(y, 2) == 1 {\n");
                output.push_str(&format!(
                    "      rgb(\"{}\")\n",
                    profile.colors.surface_table_stripe.without_hash()
                ));
                output.push_str("    } else {\n");
                output.push_str("      none\n");
                output.push_str("    }\n");
            }
            (true, TableStripeMode::None) => {
                output.push_str("    if y == 0 {\n");
                output.push_str(&format!(
                    "      rgb(\"{}\")\n",
                    profile.colors.surface_table_header.without_hash()
                ));
                output.push_str("    } else {\n");
                output.push_str("      none\n");
                output.push_str("    }\n");
            }
            (false, TableStripeMode::AlternateRows) => {
                output.push_str("    if calc.rem(y, 2) == 1 {\n");
                output.push_str(&format!(
                    "      rgb(\"{}\")\n",
                    profile.colors.surface_table_stripe.without_hash()
                ));
                output.push_str("    } else {\n");
                output.push_str("      none\n");
                output.push_str("    }\n");
            }
            (false, TableStripeMode::None) => {}
        }
        output.push_str("  },\n");
    }
    output.push_str(")\n\n");

    // Table header emphasis
    if profile.components.table.header_emphasis {
        output.push_str("// Table header emphasis\n");
        output.push_str(&format!(
            "#show table.cell.where(y: 0): set text(weight: {})\n\n",
            profile.typography.weight_semibold
        ));
    }

    // List styles
    output.push_str("// List styles\n");
    output.push_str("#set list(\n");
    output.push_str(&format!(
        "  indent: {}pt,\n",
        profile.components.list.indentation_step
    ));
    output.push_str(&format!(
        "  body-indent: {}pt,\n",
        profile.components.list.marker_gap
    ));
    output.push_str(&format!(
        "  spacing: {}pt,\n",
        profile.spacing.list_item_gap
    ));
    output.push_str(")\n\n");

    // Enum styles
    output.push_str("#set enum(\n");
    output.push_str(&format!(
        "  indent: {}pt,\n",
        profile.components.list.indentation_step
    ));
    output.push_str(&format!(
        "  body-indent: {}pt,\n",
        profile.components.list.marker_gap
    ));
    output.push_str(&format!(
        "  spacing: {}pt,\n",
        profile.spacing.list_item_gap
    ));
    output.push_str(")\n");

    output
}

/// Escape a string for use in CSS.
fn escape_css_string(s: &str) -> String {
    let sanitized: String = s
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '"' => String::from("\\\""),
            '\\' => String::from("\\\\"),
            _ => c.to_string(),
        })
        .collect();

    if sanitized.contains(' ') || sanitized.contains(',') || sanitized.contains('"') {
        format!("\"{}\"", sanitized)
    } else {
        sanitized
    }
}

/// Escape a string for use in Typst.
fn escape_typst_string(s: &str) -> String {
    let first_font = s.split(',').next().unwrap_or(s).trim();
    first_font
        .trim_matches('"')
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '"' => String::from("\\\""),
            '\\' => String::from("\\\\"),
            _ => c.to_string(),
        })
        .collect()
}

fn escape_typst_comment(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::{classic, compact, report};

    #[test]
    fn css_variables_deterministic() {
        let profile = report();
        let output1 = to_css_variables(&profile);
        let output2 = to_css_variables(&profile);
        assert_eq!(output1, output2);
    }

    #[test]
    fn typst_preamble_deterministic() {
        let profile = report();
        let output1 = to_typst_preamble(&profile);
        let output2 = to_typst_preamble(&profile);
        assert_eq!(output1, output2);
    }

    #[test]
    fn css_variables_has_rtfkit_prefix() {
        let profile = report();
        let output = to_css_variables(&profile);
        assert!(output.contains("--rtfkit-"));
        assert!(output.contains("--rtfkit-color-text-primary"));
        assert!(output.contains("--rtfkit-size-body"));
    }

    #[test]
    fn css_variables_has_root_selector() {
        let profile = report();
        let output = to_css_variables(&profile);
        assert!(output.starts_with(":root {\n"));
        assert!(output.ends_with("}\n"));
    }

    #[test]
    fn typst_preamble_does_not_set_page_geometry() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(!output.contains("#set page("));
    }

    #[test]
    fn typst_preamble_has_text_setup() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(output.contains("#set text("));
        assert!(output.contains("font:"));
        assert!(output.contains("size:"));
    }

    #[test]
    fn typst_preamble_has_heading_styles() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(output.contains("heading.where(level: 1)"));
        assert!(output.contains("heading.where(level: 2)"));
        assert!(output.contains("heading.where(level: 3)"));
    }

    #[test]
    fn typst_preamble_has_table_styles() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(output.contains("#set table("));
        assert!(output.contains("stroke:"));
        assert!(output.contains("inset:"));
    }

    #[test]
    fn typst_preamble_has_list_styles() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(output.contains("#set list("));
        assert!(output.contains("#set enum("));
    }

    #[test]
    fn css_variables_classic_profile() {
        let profile = classic();
        let output = to_css_variables(&profile);
        assert!(output.contains("--rtfkit-color-text-primary: #000000;"));
        assert!(output.contains("--rtfkit-size-body: 12pt;"));
    }

    #[test]
    fn css_variables_report_profile() {
        let profile = report();
        let output = to_css_variables(&profile);
        assert!(output.contains("--rtfkit-color-text-primary: #1A1A1A;"));
        assert!(output.contains("--rtfkit-size-body: 11pt;"));
    }

    #[test]
    fn css_variables_compact_profile() {
        let profile = compact();
        let output = to_css_variables(&profile);
        assert!(output.contains("--rtfkit-size-body: 9pt;"));
    }

    #[test]
    fn css_variables_table_striping_report() {
        let profile = report();
        let output = to_css_variables(&profile);
        assert!(output.contains("--rtfkit-table-stripe-mode: alternate-rows;"));
        assert!(output.contains("--rtfkit-table-stripe-fill: #F4F6F8;"));
    }

    #[test]
    fn css_variables_table_striping_classic() {
        let profile = classic();
        let output = to_css_variables(&profile);
        assert!(output.contains("--rtfkit-table-stripe-mode: none;"));
        assert!(output.contains("--rtfkit-table-stripe-fill: transparent;"));
    }

    #[test]
    fn typst_preamble_includes_profile_name() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(output.contains("// rtfkit style profile: report"));
    }

    #[test]
    fn typst_preamble_table_striping_report() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(output.contains("fill: (x, y) => {"));
        assert!(output.contains("calc.rem(y, 2) == 1"));
    }

    #[test]
    fn typst_preamble_no_table_striping_classic() {
        let profile = classic();
        let output = to_typst_preamble(&profile);
        assert!(!output.contains("calc.rem(y, 2) == 1"));
    }

    #[test]
    fn typst_preamble_header_emphasis_targets_header_row() {
        let profile = report();
        let output = to_typst_preamble(&profile);
        assert!(output.contains("table.cell.where(y: 0)"));
        assert!(!output.contains("table.cell.where(x: 0)"));
    }

    #[test]
    fn escape_css_string_simple() {
        assert_eq!(escape_css_string("Arial"), "Arial");
    }

    #[test]
    fn escape_css_string_with_space() {
        assert_eq!(escape_css_string("Times New Roman"), "\"Times New Roman\"");
    }

    #[test]
    fn escape_typst_string_simple() {
        assert_eq!(escape_typst_string("Arial"), "Arial");
    }

    #[test]
    fn escape_typst_string_with_fallback() {
        assert_eq!(escape_typst_string("Georgia, serif"), "Georgia");
    }

    #[test]
    fn escape_typst_string_with_quotes() {
        assert_eq!(
            escape_typst_string("\"Times New Roman\", serif"),
            "Times New Roman"
        );
    }
}
