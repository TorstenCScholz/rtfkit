//! Validation logic for style profiles.
//!
//! This module provides validation functions to ensure style profiles
//! contain valid token values.

use crate::profile::{ColorHex, StyleProfile};

/// Validation error for style profiles.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StyleValidationError {
    /// Font size must be positive
    #[error("font size must be positive: {name} = {value}")]
    FontSizeNotPositive { name: String, value: String },

    /// Line height must be >= 1.0
    #[error("line height must be >= 1.0: {name} = {value}")]
    LineHeightTooSmall { name: String, value: String },

    /// Font weight must be in range 100-900
    #[error("font weight must be in range 100-900: {name} = {value}")]
    FontWeightOutOfRange { name: String, value: u16 },

    /// Spacing value must be non-negative
    #[error("spacing value must be non-negative: {name} = {value}")]
    SpacingNegative { name: String, value: String },

    /// Page margin must be non-negative
    #[error("page margin must be non-negative: {name} = {value}")]
    PageMarginNegative { name: String, value: String },

    /// Color hex value is invalid
    #[error("invalid color hex value: {name} = {value}")]
    InvalidColorHex { name: String, value: String },

    /// Table border width must be non-negative
    #[error("table border width must be non-negative: {value}")]
    TableBorderWidthNegative { value: String },

    /// List indentation step must be non-negative
    #[error("list indentation step must be non-negative: {value}")]
    ListIndentationNegative { value: String },

    /// List marker gap must be non-negative
    #[error("list marker gap must be non-negative: {value}")]
    ListMarkerGapNegative { value: String },

    /// Heading spacing must be non-negative
    #[error("heading spacing must be non-negative: {name} = {value}")]
    HeadingSpacingNegative { name: String, value: String },
}

/// Validates a style profile.
///
/// Returns `Ok(())` if all token values are valid, otherwise returns
/// the first validation error encountered.
///
/// # Validation Rules
///
/// - Font sizes must be positive (> 0)
/// - Line heights must be >= 1.0
/// - Font weights must be in range 100-900
/// - Spacing values must be non-negative (>= 0)
/// - Page margins must be non-negative (>= 0)
/// - Color hex values must be valid #RRGGBB format
/// - Component-specific values must be non-negative
pub fn validate_profile(profile: &StyleProfile) -> Result<(), StyleValidationError> {
    // Validate typography tokens
    validate_typography(&profile.typography)?;

    // Validate spacing tokens
    validate_spacing(&profile.spacing)?;

    // Validate layout tokens
    validate_layout(&profile.layout)?;

    // Validate color tokens
    validate_colors(&profile.colors)?;

    // Validate component tokens
    validate_components(&profile.components)?;

    Ok(())
}

fn validate_typography(
    typography: &crate::profile::TypographyTokens,
) -> Result<(), StyleValidationError> {
    // Font sizes must be positive
    if typography.size_body <= 0.0 {
        return Err(StyleValidationError::FontSizeNotPositive {
            name: String::from("size_body"),
            value: typography.size_body.to_string(),
        });
    }
    if typography.size_small <= 0.0 {
        return Err(StyleValidationError::FontSizeNotPositive {
            name: String::from("size_small"),
            value: typography.size_small.to_string(),
        });
    }
    if typography.size_h1 <= 0.0 {
        return Err(StyleValidationError::FontSizeNotPositive {
            name: String::from("size_h1"),
            value: typography.size_h1.to_string(),
        });
    }
    if typography.size_h2 <= 0.0 {
        return Err(StyleValidationError::FontSizeNotPositive {
            name: String::from("size_h2"),
            value: typography.size_h2.to_string(),
        });
    }
    if typography.size_h3 <= 0.0 {
        return Err(StyleValidationError::FontSizeNotPositive {
            name: String::from("size_h3"),
            value: typography.size_h3.to_string(),
        });
    }

    // Line heights must be >= 1.0
    if typography.line_height_body < 1.0 {
        return Err(StyleValidationError::LineHeightTooSmall {
            name: String::from("line_height_body"),
            value: typography.line_height_body.to_string(),
        });
    }
    if typography.line_height_heading < 1.0 {
        return Err(StyleValidationError::LineHeightTooSmall {
            name: String::from("line_height_heading"),
            value: typography.line_height_heading.to_string(),
        });
    }

    // Font weights must be in range 100-900
    if !(100..=900).contains(&typography.weight_regular) {
        return Err(StyleValidationError::FontWeightOutOfRange {
            name: String::from("weight_regular"),
            value: typography.weight_regular,
        });
    }
    if !(100..=900).contains(&typography.weight_semibold) {
        return Err(StyleValidationError::FontWeightOutOfRange {
            name: String::from("weight_semibold"),
            value: typography.weight_semibold,
        });
    }
    if !(100..=900).contains(&typography.weight_bold) {
        return Err(StyleValidationError::FontWeightOutOfRange {
            name: String::from("weight_bold"),
            value: typography.weight_bold,
        });
    }

    Ok(())
}

fn validate_spacing(spacing: &crate::profile::SpacingTokens) -> Result<(), StyleValidationError> {
    // All spacing values must be non-negative
    if spacing.space_xs < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("space_xs"),
            value: spacing.space_xs.to_string(),
        });
    }
    if spacing.space_sm < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("space_sm"),
            value: spacing.space_sm.to_string(),
        });
    }
    if spacing.space_md < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("space_md"),
            value: spacing.space_md.to_string(),
        });
    }
    if spacing.space_lg < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("space_lg"),
            value: spacing.space_lg.to_string(),
        });
    }
    if spacing.space_xl < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("space_xl"),
            value: spacing.space_xl.to_string(),
        });
    }
    if spacing.paragraph_gap < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("paragraph_gap"),
            value: spacing.paragraph_gap.to_string(),
        });
    }
    if spacing.list_item_gap < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("list_item_gap"),
            value: spacing.list_item_gap.to_string(),
        });
    }
    if spacing.table_cell_padding_x < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("table_cell_padding_x"),
            value: spacing.table_cell_padding_x.to_string(),
        });
    }
    if spacing.table_cell_padding_y < 0.0 {
        return Err(StyleValidationError::SpacingNegative {
            name: String::from("table_cell_padding_y"),
            value: spacing.table_cell_padding_y.to_string(),
        });
    }

    Ok(())
}

fn validate_layout(layout: &crate::profile::LayoutTokens) -> Result<(), StyleValidationError> {
    // All page margins must be non-negative
    if layout.content_max_width_mm < 0.0 {
        return Err(StyleValidationError::PageMarginNegative {
            name: String::from("content_max_width_mm"),
            value: layout.content_max_width_mm.to_string(),
        });
    }
    if layout.page_margin_top_mm < 0.0 {
        return Err(StyleValidationError::PageMarginNegative {
            name: String::from("page_margin_top_mm"),
            value: layout.page_margin_top_mm.to_string(),
        });
    }
    if layout.page_margin_bottom_mm < 0.0 {
        return Err(StyleValidationError::PageMarginNegative {
            name: String::from("page_margin_bottom_mm"),
            value: layout.page_margin_bottom_mm.to_string(),
        });
    }
    if layout.page_margin_left_mm < 0.0 {
        return Err(StyleValidationError::PageMarginNegative {
            name: String::from("page_margin_left_mm"),
            value: layout.page_margin_left_mm.to_string(),
        });
    }
    if layout.page_margin_right_mm < 0.0 {
        return Err(StyleValidationError::PageMarginNegative {
            name: String::from("page_margin_right_mm"),
            value: layout.page_margin_right_mm.to_string(),
        });
    }

    Ok(())
}

fn validate_colors(colors: &crate::profile::ColorTokens) -> Result<(), StyleValidationError> {
    // Validate all color hex values by attempting to parse them
    validate_color_hex("text_primary", &colors.text_primary)?;
    validate_color_hex("text_muted", &colors.text_muted)?;
    validate_color_hex("surface_page", &colors.surface_page)?;
    validate_color_hex("surface_table_header", &colors.surface_table_header)?;
    validate_color_hex("surface_table_stripe", &colors.surface_table_stripe)?;
    validate_color_hex("border_default", &colors.border_default)?;
    validate_color_hex("link_default", &colors.link_default)?;
    validate_color_hex("link_hover", &colors.link_hover)?;

    Ok(())
}

fn validate_color_hex(name: &str, color: &ColorHex) -> Result<(), StyleValidationError> {
    // The ColorHex type already validates on construction, but we verify
    // the format is still valid (in case of unsafe construction)
    let value = color.as_str();
    if value.len() != 7 || !value.starts_with('#') {
        return Err(StyleValidationError::InvalidColorHex {
            name: String::from(name),
            value: String::from(value),
        });
    }
    for c in value[1..].chars() {
        if !c.is_ascii_hexdigit() {
            return Err(StyleValidationError::InvalidColorHex {
                name: String::from(name),
                value: String::from(value),
            });
        }
    }
    Ok(())
}

fn validate_components(
    components: &crate::profile::ComponentTokens,
) -> Result<(), StyleValidationError> {
    // Validate table tokens
    if components.table.border_width < 0.0 {
        return Err(StyleValidationError::TableBorderWidthNegative {
            value: components.table.border_width.to_string(),
        });
    }

    // Validate list tokens
    if components.list.indentation_step < 0.0 {
        return Err(StyleValidationError::ListIndentationNegative {
            value: components.list.indentation_step.to_string(),
        });
    }
    if components.list.marker_gap < 0.0 {
        return Err(StyleValidationError::ListMarkerGapNegative {
            value: components.list.marker_gap.to_string(),
        });
    }

    // Validate heading tokens
    if components.heading.spacing_above_h1 < 0.0 {
        return Err(StyleValidationError::HeadingSpacingNegative {
            name: String::from("spacing_above_h1"),
            value: components.heading.spacing_above_h1.to_string(),
        });
    }
    if components.heading.spacing_below_h1 < 0.0 {
        return Err(StyleValidationError::HeadingSpacingNegative {
            name: String::from("spacing_below_h1"),
            value: components.heading.spacing_below_h1.to_string(),
        });
    }
    if components.heading.spacing_above_h2 < 0.0 {
        return Err(StyleValidationError::HeadingSpacingNegative {
            name: String::from("spacing_above_h2"),
            value: components.heading.spacing_above_h2.to_string(),
        });
    }
    if components.heading.spacing_below_h2 < 0.0 {
        return Err(StyleValidationError::HeadingSpacingNegative {
            name: String::from("spacing_below_h2"),
            value: components.heading.spacing_below_h2.to_string(),
        });
    }
    if components.heading.spacing_above_h3 < 0.0 {
        return Err(StyleValidationError::HeadingSpacingNegative {
            name: String::from("spacing_above_h3"),
            value: components.heading.spacing_above_h3.to_string(),
        });
    }
    if components.heading.spacing_below_h3 < 0.0 {
        return Err(StyleValidationError::HeadingSpacingNegative {
            name: String::from("spacing_below_h3"),
            value: components.heading.spacing_below_h3.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::*;

    #[test]
    fn valid_default_profile() {
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        assert!(validate_profile(&profile).is_ok());
    }

    #[test]
    fn invalid_font_size_zero() {
        let typography = TypographyTokens {
            size_body: 0.0,
            ..TypographyTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography,
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::FontSizeNotPositive { name, .. }) if name == "size_body"
        ));
    }

    #[test]
    fn invalid_font_size_negative() {
        let typography = TypographyTokens {
            size_h1: -5.0,
            ..TypographyTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography,
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::FontSizeNotPositive { name, .. }) if name == "size_h1"
        ));
    }

    #[test]
    fn invalid_line_height() {
        let typography = TypographyTokens {
            line_height_body: 0.8,
            ..TypographyTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography,
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::LineHeightTooSmall { name, .. }) if name == "line_height_body"
        ));
    }

    #[test]
    fn invalid_font_weight_low() {
        let typography = TypographyTokens {
            weight_regular: 50,
            ..TypographyTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography,
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::FontWeightOutOfRange { name, value, .. }) if name == "weight_regular" && value == 50
        ));
    }

    #[test]
    fn invalid_font_weight_high() {
        let typography = TypographyTokens {
            weight_bold: 1000,
            ..TypographyTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography,
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::FontWeightOutOfRange { name, value, .. }) if name == "weight_bold" && value == 1000
        ));
    }

    #[test]
    fn invalid_spacing_negative() {
        let spacing = SpacingTokens {
            space_md: -1.0,
            ..SpacingTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            spacing,
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::SpacingNegative { name, .. }) if name == "space_md"
        ));
    }

    #[test]
    fn invalid_page_margin_negative() {
        let layout = LayoutTokens {
            page_margin_left_mm: -10.0,
            ..LayoutTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            spacing: SpacingTokens::default(),
            layout,
            components: ComponentTokens::default(),
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::PageMarginNegative { name, .. }) if name == "page_margin_left_mm"
        ));
    }

    #[test]
    fn invalid_table_border_width() {
        let mut components = ComponentTokens::default();
        components.table.border_width = -0.5;
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components,
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::TableBorderWidthNegative { .. })
        ));
    }

    #[test]
    fn invalid_list_indentation() {
        let mut components = ComponentTokens::default();
        components.list.indentation_step = -5.0;
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components,
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::ListIndentationNegative { .. })
        ));
    }

    #[test]
    fn invalid_heading_spacing() {
        let mut components = ComponentTokens::default();
        components.heading.spacing_above_h2 = -2.0;
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components,
        };
        let result = validate_profile(&profile);
        assert!(matches!(
            result,
            Err(StyleValidationError::HeadingSpacingNegative { name, .. }) if name == "spacing_above_h2"
        ));
    }

    #[test]
    fn line_height_exactly_one_is_valid() {
        let typography = TypographyTokens {
            line_height_body: 1.0,
            ..TypographyTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography,
            spacing: SpacingTokens::default(),
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        assert!(validate_profile(&profile).is_ok());
    }

    #[test]
    fn spacing_zero_is_valid() {
        let spacing = SpacingTokens {
            space_xs: 0.0,
            ..SpacingTokens::default()
        };
        let profile = StyleProfile {
            name: StyleProfileName::default(),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            spacing,
            layout: LayoutTokens::default(),
            components: ComponentTokens::default(),
        };
        assert!(validate_profile(&profile).is_ok());
    }
}
