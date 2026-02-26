//! Shading color resolution helpers for output backends.
//!
//! This module centralizes backend-facing shading behavior so HTML and Typst
//! can share deterministic approximation logic for percentage patterns.

use crate::{Color, Shading, ShadingPattern};

/// Rendering policy used when resolving a visual fill color from shading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingRenderPolicy {
    /// Always use the raw fill color (legacy behavior).
    Exact,
    /// Approximate percentage patterns by blending fill and pattern colors.
    ApproximatePercentPatterns,
}

/// Resolve the final fill color to render for a shading object.
///
/// Returns `None` when no fill color is available.
pub fn resolve_shading_fill_color(
    shading: Option<&Shading>,
    policy: ShadingRenderPolicy,
) -> Option<Color> {
    let shading = shading?;
    let fill = shading.fill_color.clone()?;

    if !matches!(policy, ShadingRenderPolicy::ApproximatePercentPatterns) {
        return Some(fill);
    }

    let pattern = shading.pattern.unwrap_or(ShadingPattern::Solid);
    let Some(density) = percent_pattern_density(pattern) else {
        return Some(fill);
    };

    // DOCX uses "auto" for missing pattern color; approximate that as black.
    let pattern_color = shading.pattern_color.clone().unwrap_or(Color::new(0, 0, 0));

    Some(blend_colors(&fill, &pattern_color, density))
}

/// Return percentage density for percent patterns.
///
/// E.g. `Percent25 -> Some(25)`, non-percent patterns -> `None`.
pub fn percent_pattern_density(pattern: ShadingPattern) -> Option<u8> {
    match pattern {
        ShadingPattern::Percent5 => Some(5),
        ShadingPattern::Percent10 => Some(10),
        ShadingPattern::Percent20 => Some(20),
        ShadingPattern::Percent25 => Some(25),
        ShadingPattern::Percent30 => Some(30),
        ShadingPattern::Percent40 => Some(40),
        ShadingPattern::Percent50 => Some(50),
        ShadingPattern::Percent60 => Some(60),
        ShadingPattern::Percent70 => Some(70),
        ShadingPattern::Percent75 => Some(75),
        ShadingPattern::Percent80 => Some(80),
        ShadingPattern::Percent90 => Some(90),
        _ => None,
    }
}

/// Blend background and foreground using foreground percentage.
fn blend_colors(background: &Color, foreground: &Color, foreground_percent: u8) -> Color {
    let p = foreground_percent as u32;
    let bg_weight = 100u32.saturating_sub(p);

    // Integer math with +50 for round-to-nearest.
    let blend =
        |bg: u8, fg: u8| -> u8 { (((bg as u32 * bg_weight) + (fg as u32 * p) + 50) / 100) as u8 };

    Color::new(
        blend(background.r, foreground.r),
        blend(background.g, foreground.g),
        blend(background.b, foreground.b),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_pattern_density() {
        assert_eq!(percent_pattern_density(ShadingPattern::Percent25), Some(25));
        assert_eq!(percent_pattern_density(ShadingPattern::Percent50), Some(50));
        assert_eq!(percent_pattern_density(ShadingPattern::DiagCross), None);
    }

    #[test]
    fn test_resolve_shading_fill_color_exact_uses_fill() {
        let shading = Shading::with_pattern(
            Color::new(255, 255, 255),
            Color::new(0, 0, 0),
            ShadingPattern::Percent25,
        );
        let color = resolve_shading_fill_color(Some(&shading), ShadingRenderPolicy::Exact);
        assert_eq!(color, Some(Color::new(255, 255, 255)));
    }

    #[test]
    fn test_resolve_shading_fill_color_approximates_percent_pattern() {
        let shading = Shading::with_pattern(
            Color::new(255, 255, 255),
            Color::new(0, 0, 0),
            ShadingPattern::Percent25,
        );
        let color = resolve_shading_fill_color(
            Some(&shading),
            ShadingRenderPolicy::ApproximatePercentPatterns,
        );
        assert_eq!(color, Some(Color::new(191, 191, 191)));
    }

    #[test]
    fn test_resolve_shading_fill_color_uses_auto_pattern_fallback() {
        let mut shading = Shading::new();
        shading.fill_color = Some(Color::new(200, 200, 200));
        shading.pattern = Some(ShadingPattern::Percent50);

        let color = resolve_shading_fill_color(
            Some(&shading),
            ShadingRenderPolicy::ApproximatePercentPatterns,
        );
        assert_eq!(color, Some(Color::new(100, 100, 100)));
    }

    #[test]
    fn test_resolve_shading_fill_color_non_percent_pattern_stays_fill() {
        let shading = Shading::with_pattern(
            Color::new(255, 255, 0),
            Color::new(255, 0, 0),
            ShadingPattern::DiagCross,
        );
        let color = resolve_shading_fill_color(
            Some(&shading),
            ShadingRenderPolicy::ApproximatePercentPatterns,
        );
        assert_eq!(color, Some(Color::new(255, 255, 0)));
    }
}
