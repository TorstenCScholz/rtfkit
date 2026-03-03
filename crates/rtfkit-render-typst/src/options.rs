//! Configuration options for Typst-based PDF rendering.
//!
//! This module provides [`RenderOptions`], [`PageSize`], and [`Margins`]
//! for controlling PDF output behavior.

use rtfkit_style_tokens::StyleProfileName;

/// Page size options for PDF output.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PageSize {
    /// A4 page size (210mm × 297mm).
    #[default]
    A4,
    /// US Letter page size (8.5" × 11").
    Letter,
    /// US Legal page size (8.5" × 14").
    Legal,
    /// Custom page size with width and height in millimeters.
    Custom {
        /// Width in millimeters.
        width_mm: f32,
        /// Height in millimeters.
        height_mm: f32,
    },
}

/// Policy for page numbering behavior in Typst output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageNumberingMode {
    /// Enable page numbering only when the document semantics require it
    /// (e.g., page fields or a generated table of contents).
    Auto,
    /// Always enable page numbering, even if the document does not use page-related fields.
    Always,
    /// Never enable page numbering. Documents that require numbering will be rejected
    /// by the renderer as an invalid option.
    Never,
}

impl Default for PageNumberingMode {
    fn default() -> Self {
        PageNumberingMode::Auto
    }
}

impl PageSize {
    /// Returns the page dimensions in millimeters.
    pub fn dimensions_mm(&self) -> (f32, f32) {
        match self {
            PageSize::A4 => (210.0, 297.0),
            PageSize::Letter => (215.9, 279.4),
            PageSize::Legal => (215.9, 355.6),
            PageSize::Custom {
                width_mm,
                height_mm,
            } => (*width_mm, *height_mm),
        }
    }
}

/// Page margins in millimeters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    /// Top margin in millimeters.
    pub top: f32,
    /// Bottom margin in millimeters.
    pub bottom: f32,
    /// Left margin in millimeters.
    pub left: f32,
    /// Right margin in millimeters.
    pub right: f32,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 25.4,
            bottom: 25.4,
            left: 25.4,
            right: 25.4,
        }
    }
}

/// Determinism controls for reproducible output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeterminismOptions {
    /// Use a fixed timestamp for PDF metadata.
    ///
    /// When set, this timestamp will be used instead of the current time
    /// for PDF creation/modification dates.
    pub fixed_timestamp: Option<String>,

    /// Normalize metadata for deterministic output.
    ///
    /// When true, timestamps and other non-deterministic metadata
    /// are normalized to produce reproducible PDF output.
    pub normalize_metadata: bool,
}

/// Configuration options for PDF rendering.
///
/// Controls how the renderer generates PDF output from a `Document`.
///
/// # Example
///
/// ```rust
/// use rtfkit_render_typst::{
///     DeterminismOptions, Margins, PageNumberingMode, PageSize, RenderOptions,
/// };
/// use rtfkit_style_tokens::StyleProfileName;
///
/// // Default options: A4 page size, 1-inch margins, Report style profile
/// let options = RenderOptions::default();
///
/// // Custom options for deterministic output
/// let custom = RenderOptions {
///     page_size: PageSize::Letter,
///     margins: Margins {
///         top: 20.0,
///         bottom: 20.0,
///         left: 25.0,
///         right: 25.0,
///     },
///     determinism: DeterminismOptions {
///         fixed_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
///         normalize_metadata: true,
///     },
///     page_numbering: PageNumberingMode::Auto,
///     style_profile: StyleProfileName::Report,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderOptions {
    /// Page size for the output PDF.
    pub page_size: PageSize,

    /// Page margins in millimeters.
    ///
    /// When set to [`Margins::default()`], profile layout margins are used.
    /// Any non-default value is treated as an explicit override.
    pub margins: Margins,

    /// Determinism controls for reproducible output.
    pub determinism: DeterminismOptions,

    /// Page-numbering policy for Typst page setup (default: Auto).
    ///
    /// This controls whether the `numbering` parameter is enabled in the Typst
    /// `#set page(...)` directive. When left at the default [`PageNumberingMode::Auto`],
    /// numbering is enabled only when the document semantics require it.
    pub page_numbering: PageNumberingMode,

    /// Style profile for Typst preamble generation (default: Report).
    ///
    /// The style profile controls typography, colors, spacing, and component
    /// styles in the generated Typst output.
    pub style_profile: StyleProfileName,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options() {
        let options = RenderOptions::default();
        assert_eq!(options.page_size, PageSize::A4);
        assert!(options.determinism.fixed_timestamp.is_none());
        assert!(!options.determinism.normalize_metadata);
        assert_eq!(options.page_numbering, PageNumberingMode::Auto);
        assert_eq!(options.style_profile, StyleProfileName::Report);
    }

    #[test]
    fn default_margins() {
        let margins = Margins::default();
        assert!((margins.top - 25.4).abs() < f32::EPSILON);
        assert!((margins.bottom - 25.4).abs() < f32::EPSILON);
        assert!((margins.left - 25.4).abs() < f32::EPSILON);
        assert!((margins.right - 25.4).abs() < f32::EPSILON);
    }

    #[test]
    fn page_size_default() {
        assert_eq!(PageSize::default(), PageSize::A4);
    }

    #[test]
    fn page_size_dimensions() {
        let (w, h) = PageSize::A4.dimensions_mm();
        assert!((w - 210.0).abs() < f32::EPSILON);
        assert!((h - 297.0).abs() < f32::EPSILON);

        let (w, h) = PageSize::Letter.dimensions_mm();
        assert!((w - 215.9).abs() < f32::EPSILON);
        assert!((h - 279.4).abs() < f32::EPSILON);

        let custom = PageSize::Custom {
            width_mm: 100.0,
            height_mm: 200.0,
        };
        let (w, h) = custom.dimensions_mm();
        assert!((w - 100.0).abs() < f32::EPSILON);
        assert!((h - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn custom_margins() {
        let margins = Margins {
            top: 10.0,
            bottom: 15.0,
            left: 20.0,
            right: 25.0,
        };
        assert!((margins.top - 10.0).abs() < f32::EPSILON);
        assert!((margins.bottom - 15.0).abs() < f32::EPSILON);
        assert!((margins.left - 20.0).abs() < f32::EPSILON);
        assert!((margins.right - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn determinism_options() {
        let det = DeterminismOptions {
            fixed_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            normalize_metadata: true,
        };
        assert_eq!(
            det.fixed_timestamp,
            Some("2024-01-01T00:00:00Z".to_string())
        );
        assert!(det.normalize_metadata);
    }
}
