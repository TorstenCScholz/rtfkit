//! Built-in style profiles.
//!
//! This module provides three pre-configured style profiles:
//! - **Classic**: Conservative style, close to current behavior
//! - **Report**: Professional style with strong hierarchy (DEFAULT)
//! - **Compact**: Dense style for enterprise output

use crate::profile::*;

/// Returns the default style profile (Report).
pub fn default_profile() -> StyleProfile {
    report()
}

/// Resolve a profile name to a concrete built-in profile.
///
/// For MVP, custom profile names fall back to `report`.
pub fn resolve_profile(name: &StyleProfileName) -> StyleProfile {
    match name {
        StyleProfileName::Classic => classic(),
        StyleProfileName::Report => report(),
        StyleProfileName::Compact => compact(),
        StyleProfileName::Custom(_) => report(),
    }
}

/// Creates the Classic style profile.
///
/// Classic is a conservative style that closely matches the current
/// rtfkit behavior. It uses neutral colors, standard fonts, and
/// moderate spacing.
pub fn classic() -> StyleProfile {
    StyleProfile {
        name: StyleProfileName::Classic,
        colors: ColorTokens {
            text_primary: ColorHex::new(String::from("#000000")).unwrap(),
            text_muted: ColorHex::new(String::from("#666666")).unwrap(),
            surface_page: ColorHex::new(String::from("#FFFFFF")).unwrap(),
            surface_table_header: ColorHex::new(String::from("#E8E8E8")).unwrap(),
            surface_table_stripe: ColorHex::new(String::from("#F5F5F5")).unwrap(),
            border_default: ColorHex::new(String::from("#000000")).unwrap(),
            link_default: ColorHex::new(String::from("#0000EE")).unwrap(),
            link_hover: ColorHex::new(String::from("#0000AA")).unwrap(),
        },
        typography: TypographyTokens {
            font_body: String::from("Libertinus Serif"),
            font_heading: String::from("Libertinus Serif"),
            font_mono: String::from("DejaVu Sans Mono"),
            size_body: 12.0,
            size_small: 10.0,
            size_h1: 20.0,
            size_h2: 16.0,
            size_h3: 14.0,
            line_height_body: 1.4,
            line_height_heading: 1.2,
            weight_regular: 400,
            weight_semibold: 600,
            weight_bold: 700,
        },
        spacing: SpacingTokens {
            space_xs: 2.0,
            space_sm: 4.0,
            space_md: 8.0,
            space_lg: 12.0,
            space_xl: 18.0,
            paragraph_gap: 8.0,
            list_item_gap: 4.0,
            table_cell_padding_x: 4.0,
            table_cell_padding_y: 2.0,
        },
        layout: LayoutTokens {
            content_max_width_mm: 170.0,
            page_margin_top_mm: 25.4, // 1 inch
            page_margin_bottom_mm: 25.4,
            page_margin_left_mm: 25.4,
            page_margin_right_mm: 25.4,
        },
        components: ComponentTokens {
            table: TableComponentTokens {
                border_width: 0.75,
                stripe_mode: TableStripeMode::None,
                header_emphasis: true,
            },
            list: ListComponentTokens {
                indentation_step: 18.0,
                marker_gap: 6.0,
            },
            heading: HeadingComponentTokens {
                spacing_above_h1: 18.0,
                spacing_below_h1: 10.0,
                spacing_above_h2: 14.0,
                spacing_below_h2: 8.0,
                spacing_above_h3: 12.0,
                spacing_below_h3: 6.0,
            },
        },
    }
}

/// Creates the Report style profile.
///
/// Report is a professional style designed for long-form documents.
/// It features a clear heading hierarchy with larger sizes, generous
/// spacing for readability, and table striping enabled.
///
/// This is the DEFAULT profile for rtfkit.
pub fn report() -> StyleProfile {
    StyleProfile {
        name: StyleProfileName::Report,
        colors: ColorTokens {
            text_primary: ColorHex::new(String::from("#1A1A1A")).unwrap(),
            text_muted: ColorHex::new(String::from("#5A5A5A")).unwrap(),
            surface_page: ColorHex::new(String::from("#FFFFFF")).unwrap(),
            surface_table_header: ColorHex::new(String::from("#E6E9ED")).unwrap(),
            surface_table_stripe: ColorHex::new(String::from("#F4F6F8")).unwrap(),
            border_default: ColorHex::new(String::from("#D1D5DB")).unwrap(),
            link_default: ColorHex::new(String::from("#2563EB")).unwrap(),
            link_hover: ColorHex::new(String::from("#1D4ED8")).unwrap(),
        },
        typography: TypographyTokens {
            font_body: String::from("Libertinus Serif"),
            font_heading: String::from("Libertinus Serif"),
            font_mono: String::from("DejaVu Sans Mono"),
            size_body: 11.0,
            size_small: 9.5,
            size_h1: 26.0,
            size_h2: 20.0,
            size_h3: 15.0,
            line_height_body: 1.6,
            line_height_heading: 1.3,
            weight_regular: 400,
            weight_semibold: 600,
            weight_bold: 700,
        },
        spacing: SpacingTokens {
            space_xs: 3.0,
            space_sm: 6.0,
            space_md: 10.0,
            space_lg: 18.0,
            space_xl: 28.0,
            paragraph_gap: 12.0,
            list_item_gap: 6.0,
            table_cell_padding_x: 8.0,
            table_cell_padding_y: 5.0,
        },
        layout: LayoutTokens {
            content_max_width_mm: 165.0,
            page_margin_top_mm: 25.0,
            page_margin_bottom_mm: 25.0,
            page_margin_left_mm: 22.0,
            page_margin_right_mm: 22.0,
        },
        components: ComponentTokens {
            table: TableComponentTokens {
                border_width: 0.5,
                stripe_mode: TableStripeMode::AlternateRows,
                header_emphasis: true,
            },
            list: ListComponentTokens {
                indentation_step: 20.0,
                marker_gap: 8.0,
            },
            heading: HeadingComponentTokens {
                spacing_above_h1: 28.0,
                spacing_below_h1: 14.0,
                spacing_above_h2: 22.0,
                spacing_below_h2: 12.0,
                spacing_above_h3: 16.0,
                spacing_below_h3: 10.0,
            },
        },
    }
}

/// Creates the Compact style profile.
///
/// Compact is a dense style designed for enterprise output where
/// space efficiency is important. It uses smaller font sizes,
/// tighter spacing, and minimal table styling.
pub fn compact() -> StyleProfile {
    StyleProfile {
        name: StyleProfileName::Compact,
        colors: ColorTokens {
            text_primary: ColorHex::new(String::from("#000000")).unwrap(),
            text_muted: ColorHex::new(String::from("#555555")).unwrap(),
            surface_page: ColorHex::new(String::from("#FFFFFF")).unwrap(),
            surface_table_header: ColorHex::new(String::from("#F0F0F0")).unwrap(),
            surface_table_stripe: ColorHex::new(String::from("#FFFFFF")).unwrap(),
            border_default: ColorHex::new(String::from("#CCCCCC")).unwrap(),
            link_default: ColorHex::new(String::from("#0066CC")).unwrap(),
            link_hover: ColorHex::new(String::from("#004499")).unwrap(),
        },
        typography: TypographyTokens {
            font_body: String::from("Libertinus Serif"),
            font_heading: String::from("Libertinus Serif"),
            font_mono: String::from("DejaVu Sans Mono"),
            size_body: 9.0,
            size_small: 8.0,
            size_h1: 14.0,
            size_h2: 12.0,
            size_h3: 10.0,
            line_height_body: 1.3,
            line_height_heading: 1.2,
            weight_regular: 400,
            weight_semibold: 600,
            weight_bold: 700,
        },
        spacing: SpacingTokens {
            space_xs: 1.0,
            space_sm: 2.0,
            space_md: 4.0,
            space_lg: 8.0,
            space_xl: 12.0,
            paragraph_gap: 4.0,
            list_item_gap: 2.0,
            table_cell_padding_x: 3.0,
            table_cell_padding_y: 2.0,
        },
        layout: LayoutTokens {
            content_max_width_mm: 180.0,
            page_margin_top_mm: 15.0,
            page_margin_bottom_mm: 15.0,
            page_margin_left_mm: 15.0,
            page_margin_right_mm: 15.0,
        },
        components: ComponentTokens {
            table: TableComponentTokens {
                border_width: 0.5,
                stripe_mode: TableStripeMode::None,
                header_emphasis: false,
            },
            list: ListComponentTokens {
                indentation_step: 12.0,
                marker_gap: 4.0,
            },
            heading: HeadingComponentTokens {
                spacing_above_h1: 10.0,
                spacing_below_h1: 6.0,
                spacing_above_h2: 8.0,
                spacing_below_h2: 4.0,
                spacing_above_h3: 6.0,
                spacing_below_h3: 3.0,
            },
        },
    }
}

/// Returns a profile by name.
///
/// # Arguments
///
/// * `name` - The profile name to look up
///
/// # Returns
///
/// Returns the matching built-in profile, or `None` if the name doesn't
/// match any built-in profile.
pub fn get_builtin(name: &str) -> Option<StyleProfile> {
    match name.to_lowercase().as_str() {
        "classic" => Some(classic()),
        "report" => Some(report()),
        "compact" => Some(compact()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_profile;

    #[test]
    fn classic_profile_is_valid() {
        let profile = classic();
        assert!(validate_profile(&profile).is_ok());
    }

    #[test]
    fn report_profile_is_valid() {
        let profile = report();
        assert!(validate_profile(&profile).is_ok());
    }

    #[test]
    fn compact_profile_is_valid() {
        let profile = compact();
        assert!(validate_profile(&profile).is_ok());
    }

    #[test]
    fn default_profile_is_report() {
        let default = default_profile();
        let report_profile = report();
        assert_eq!(default.name, report_profile.name);
    }

    #[test]
    fn get_builtin_classic() {
        let profile = get_builtin("classic").unwrap();
        assert_eq!(profile.name, StyleProfileName::Classic);
    }

    #[test]
    fn get_builtin_report() {
        let profile = get_builtin("report").unwrap();
        assert_eq!(profile.name, StyleProfileName::Report);
    }

    #[test]
    fn get_builtin_compact() {
        let profile = get_builtin("compact").unwrap();
        assert_eq!(profile.name, StyleProfileName::Compact);
    }

    #[test]
    fn get_builtin_case_insensitive() {
        assert!(get_builtin("CLASSIC").is_some());
        assert!(get_builtin("Report").is_some());
        assert!(get_builtin("COMPACT").is_some());
    }

    #[test]
    fn get_builtin_unknown_returns_none() {
        assert!(get_builtin("unknown").is_none());
        assert!(get_builtin("custom").is_none());
    }

    #[test]
    fn report_has_table_striping() {
        let profile = report();
        assert_eq!(
            profile.components.table.stripe_mode,
            TableStripeMode::AlternateRows
        );
    }

    #[test]
    fn classic_no_table_striping() {
        let profile = classic();
        assert_eq!(profile.components.table.stripe_mode, TableStripeMode::None);
    }

    #[test]
    fn compact_no_table_striping() {
        let profile = compact();
        assert_eq!(profile.components.table.stripe_mode, TableStripeMode::None);
    }

    #[test]
    fn compact_has_smaller_fonts() {
        let compact_profile = compact();
        let report_profile = report();

        assert!(compact_profile.typography.size_body < report_profile.typography.size_body);
        assert!(compact_profile.typography.size_h1 < report_profile.typography.size_h1);
    }

    #[test]
    fn compact_has_tighter_spacing() {
        let compact_profile = compact();
        let report_profile = report();

        assert!(compact_profile.spacing.paragraph_gap < report_profile.spacing.paragraph_gap);
        assert!(
            compact_profile.layout.page_margin_top_mm < report_profile.layout.page_margin_top_mm
        );
    }

    #[test]
    fn resolve_profile_matches_builtins() {
        assert_eq!(
            resolve_profile(&StyleProfileName::Classic).name,
            StyleProfileName::Classic
        );
        assert_eq!(
            resolve_profile(&StyleProfileName::Report).name,
            StyleProfileName::Report
        );
        assert_eq!(
            resolve_profile(&StyleProfileName::Compact).name,
            StyleProfileName::Compact
        );
    }

    #[test]
    fn resolve_profile_custom_falls_back_to_report() {
        assert_eq!(
            resolve_profile(&StyleProfileName::Custom("custom-theme".to_string())).name,
            StyleProfileName::Report
        );
    }
}
