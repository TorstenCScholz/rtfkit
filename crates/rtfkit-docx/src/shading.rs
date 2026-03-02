//! Shading (background fill / pattern) conversion from IR to docx-rs types.

use docx_rs::{Shading, ShdType};
use rtfkit_core::{Shading as IrShading, ShadingPattern};

/// Converts IR Shading to docx-rs Shading.
///
/// Maps IR `ShadingPattern` to DOCX `w:val` attribute:
/// - Clear → "clear"
/// - Solid → "solid"
/// - HorzStripe → "horzStripe"
/// - VertStripe → "vertStripe"
/// - DiagStripe → "diagStripe"
/// - ReverseDiagStripe → "reverseDiagStripe"
/// - HorzCross → "horzCross"
/// - DiagCross → "diagCross"
/// - Percent5-90 → "pct5"-"pct90"
///
/// Emits full `w:shd` attributes:
/// - `w:val` = pattern type
/// - `w:fill` = fill_color (background)
/// - `w:color` = pattern_color (foreground)
pub(crate) fn convert_shading(shading: &IrShading) -> Option<Shading> {
    // Only emit shading if we have a fill color
    let fill_color = shading.fill_color.as_ref()?;
    let fill_hex = format!(
        "{:02X}{:02X}{:02X}",
        fill_color.r, fill_color.g, fill_color.b
    );

    // Get pattern type, defaulting to Solid if fill_color is present
    let pattern = shading.pattern.unwrap_or(ShadingPattern::Solid);
    let shd_type = pattern_to_shd_type(pattern);

    // Build shading with pattern and fill color
    let mut docx_shading = Shading::new().shd_type(shd_type).fill(fill_hex);

    // Add pattern color if present (foreground for patterns)
    if let Some(ref pattern_color) = shading.pattern_color {
        let pattern_hex = format!(
            "{:02X}{:02X}{:02X}",
            pattern_color.r, pattern_color.g, pattern_color.b
        );
        docx_shading = docx_shading.color(pattern_hex);
    } else {
        // Use "auto" for color when no pattern color specified
        docx_shading = docx_shading.color("auto");
    }

    Some(docx_shading)
}

/// Maps IR ShadingPattern to docx-rs ShdType.
pub(crate) fn pattern_to_shd_type(pattern: ShadingPattern) -> ShdType {
    match pattern {
        ShadingPattern::Clear => ShdType::Clear,
        ShadingPattern::Solid => ShdType::Solid,
        ShadingPattern::HorzStripe => ShdType::HorzStripe,
        ShadingPattern::VertStripe => ShdType::VertStripe,
        ShadingPattern::DiagStripe => ShdType::DiagStripe,
        ShadingPattern::ReverseDiagStripe => ShdType::ReverseDiagStripe,
        ShadingPattern::HorzCross => ShdType::HorzCross,
        ShadingPattern::DiagCross => ShdType::DiagCross,
        ShadingPattern::Percent5 => ShdType::Pct5,
        ShadingPattern::Percent10 => ShdType::Pct10,
        ShadingPattern::Percent20 => ShdType::Pct20,
        ShadingPattern::Percent25 => ShdType::Pct25,
        ShadingPattern::Percent30 => ShdType::Pct30,
        ShadingPattern::Percent40 => ShdType::Pct40,
        ShadingPattern::Percent50 => ShdType::Pct50,
        ShadingPattern::Percent60 => ShdType::Pct60,
        ShadingPattern::Percent70 => ShdType::Pct70,
        ShadingPattern::Percent75 => ShdType::Pct75,
        ShadingPattern::Percent80 => ShdType::Pct80,
        ShadingPattern::Percent90 => ShdType::Pct90,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_to_shd_type_all_patterns() {
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Clear),
            ShdType::Clear
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Solid),
            ShdType::Solid
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::HorzStripe),
            ShdType::HorzStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::VertStripe),
            ShdType::VertStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::DiagStripe),
            ShdType::DiagStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::ReverseDiagStripe),
            ShdType::ReverseDiagStripe
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::HorzCross),
            ShdType::HorzCross
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::DiagCross),
            ShdType::DiagCross
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent5),
            ShdType::Pct5
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent10),
            ShdType::Pct10
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent20),
            ShdType::Pct20
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent25),
            ShdType::Pct25
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent30),
            ShdType::Pct30
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent40),
            ShdType::Pct40
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent50),
            ShdType::Pct50
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent60),
            ShdType::Pct60
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent70),
            ShdType::Pct70
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent75),
            ShdType::Pct75
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent80),
            ShdType::Pct80
        ));
        assert!(matches!(
            pattern_to_shd_type(ShadingPattern::Percent90),
            ShdType::Pct90
        ));
    }
}
