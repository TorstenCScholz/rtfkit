//! Unit conversion utilities for DOCX/OOXML dimensions.

/// 96 DPI pixel to EMU conversion used by OOXML drawing sizes.
/// 1 pixel at 96 DPI = 914400 / 96 = 9525 EMUs.
pub(crate) const PX_TO_EMU: u32 = 9525;

/// Default image size in EMUs when no dimension is available (1 inch).
pub(crate) const DEFAULT_IMAGE_EMU: u32 = 914_400;

/// Converts twips to English Metric Units (EMUs).
///
/// EMUs are used in DrawingML for image dimensions.
/// 1 twip = 635 EMUs (1 twip = 1/20 point, 1 point = 914400 EMUs / 72)
/// Therefore: 1 twip = 914400 / 72 / 20 = 635 EMUs
pub(crate) fn twips_to_emu(twips: i32) -> u32 {
    (twips.max(1) as i64 * 635).clamp(1, u32::MAX as i64) as u32
}

/// Converts a pixel count (at 96 DPI) to EMUs.
pub(crate) fn px_to_emu(px: u32) -> u32 {
    (px.max(1) as u64 * PX_TO_EMU as u64).min(u32::MAX as u64) as u32
}

/// Scales an EMU value by a numerator/denominator ratio.
///
/// Used to compute proportional image dimensions (e.g., if width is specified but
/// height must be derived from the intrinsic aspect ratio).
pub(crate) fn scale_emu(base_emu: u32, numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return base_emu.max(1);
    }
    ((base_emu as u64 * numerator.max(1) as u64) / denominator as u64).clamp(1, u32::MAX as u64)
        as u32
}
