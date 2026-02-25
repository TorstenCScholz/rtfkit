//! Shading helpers for paragraph/cell/row/table shading resolution.

use super::super::state::RuntimeState;
use crate::{Shading, ShadingPattern};

/// Map RTF shading percentage (0-10000) to ShadingPattern.
///
/// RTF `\\shadingN` and `\\clshdngN` use percentage values where:
/// - 0 = Clear (transparent)
/// - 10000 = Solid (100%)
/// - Other values map to Percent patterns
pub fn shading_percentage_to_pattern(percentage: i32) -> Option<ShadingPattern> {
    // Clamp to valid range
    let clamped = percentage.clamp(0, 10000);

    match clamped {
        0 => Some(ShadingPattern::Clear),
        10000 => Some(ShadingPattern::Solid),
        // Map percentage to closest Percent pattern
        // RTF uses 0-10000, we map to discrete percentages
        p if p <= 75 => Some(ShadingPattern::Percent5),
        p if p <= 150 => Some(ShadingPattern::Percent10),
        p if p <= 250 => Some(ShadingPattern::Percent20),
        p if p <= 375 => Some(ShadingPattern::Percent25),
        p if p <= 450 => Some(ShadingPattern::Percent30),
        p if p <= 550 => Some(ShadingPattern::Percent40),
        p if p <= 650 => Some(ShadingPattern::Percent50),
        p if p <= 750 => Some(ShadingPattern::Percent60),
        p if p <= 825 => Some(ShadingPattern::Percent70),
        p if p <= 875 => Some(ShadingPattern::Percent75),
        p if p <= 950 => Some(ShadingPattern::Percent80),
        p if p < 10000 => Some(ShadingPattern::Percent90),
        _ => Some(ShadingPattern::Solid),
    }
}

/// Build a Shading object from fill color index, pattern color index, and shading percentage.
///
/// This combines the three shading-related RTF controls into a single Shading struct:
/// - `cbpat`/`clcbpat`: fill/background color index
/// - `cfpat`/`clcfpat`: pattern/foreground color index
/// - `shading`/`clshdng`: shading percentage (0-10000)
pub fn build_shading(
    state: &RuntimeState,
    fill_color_idx: Option<i32>,
    pattern_color_idx: Option<i32>,
    shading_percentage: Option<i32>,
) -> Option<Shading> {
    // Resolve fill color (required for any shading)
    let fill_color = fill_color_idx.and_then(|idx| state.resolve_color_from_index(idx));

    // If no fill color, no shading
    fill_color.map(|fill| {
        // Resolve pattern color (optional)
        let pattern_color = pattern_color_idx.and_then(|idx| state.resolve_color_from_index(idx));

        // Map shading percentage to pattern
        let pattern = shading_percentage.and_then(shading_percentage_to_pattern);

        // Determine the final pattern:
        // - If we have an explicit shading percentage, use the mapped pattern
        // - If we have a pattern color but no shading percentage, default to Solid
        // - If we have neither, leave pattern as None (flat fill, no pattern overlay)
        let final_pattern = match (pattern, pattern_color.is_some()) {
            (Some(p), _) => Some(p),
            (None, true) => Some(ShadingPattern::Solid),
            (None, false) => None,
        };

        Shading {
            fill_color: Some(fill),
            pattern_color,
            pattern: final_pattern,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shading_percentage_to_pattern() {
        assert_eq!(
            shading_percentage_to_pattern(0),
            Some(ShadingPattern::Clear)
        );
        assert_eq!(
            shading_percentage_to_pattern(10000),
            Some(ShadingPattern::Solid)
        );
        // The thresholds are designed for specific boundary values
        assert_eq!(
            shading_percentage_to_pattern(75),
            Some(ShadingPattern::Percent5)
        );
        assert_eq!(
            shading_percentage_to_pattern(650),
            Some(ShadingPattern::Percent50)
        );
        assert_eq!(
            shading_percentage_to_pattern(5000),
            Some(ShadingPattern::Percent90)
        );
        assert_eq!(
            shading_percentage_to_pattern(-100),
            Some(ShadingPattern::Clear)
        );
        assert_eq!(
            shading_percentage_to_pattern(20000),
            Some(ShadingPattern::Solid)
        );
    }
}
