//! Image block Typst mapping.
//!
//! This module provides functions for converting `ImageBlock` elements
//! to Typst markup with virtual asset paths.

use image::ImageFormat as RasterFormat;
use rtfkit_core::{ImageBlock, ImageFormat};

use super::{MappingWarning, TypstAssetAllocator};

/// Result of mapping an image block to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated while mapping this image.
    pub warnings: Vec<MappingWarning>,
}

/// Map an image block to Typst markup.
///
/// Emits a Typst `image` call referencing a deterministic in-memory asset path.
///
/// # Typst Output Format
///
/// ```typst
/// #image("assets/image-000001.png", width: 1.0in, height: 0.5in)
/// ```
///
/// For images without dimensions:
/// ```typst
/// #image("assets/image-000001.png")
/// ```
///
/// # Dimension Conversion
///
/// Dimensions in twips are converted to inches using the formula:
/// `inches = twips / 1440` (1440 twips per inch).
///
/// # Determinism
///
/// The output is deterministic: same input always produces same output.
/// Attribute ordering is fixed: width before height when both are present.
pub fn map_image_block(image: &ImageBlock) -> ImageOutput {
    let mut assets = TypstAssetAllocator::new();
    map_image_block_with_assets(image, &mut assets)
}

pub(crate) fn map_image_block_with_assets(
    image: &ImageBlock,
    assets: &mut TypstAssetAllocator,
) -> ImageOutput {
    // Validate bytes for declared format to keep renderer errors deterministic.
    let (extension, warning_kind, raster_format) = match image.format {
        ImageFormat::Png => (
            "png",
            MappingWarning::MalformedPngImagePayload,
            RasterFormat::Png,
        ),
        ImageFormat::Jpeg => (
            "jpg",
            MappingWarning::MalformedJpegImagePayload,
            RasterFormat::Jpeg,
        ),
    };

    if image::load_from_memory_with_format(&image.data, raster_format).is_err() {
        return ImageOutput {
            typst_source: String::new(),
            warnings: vec![warning_kind],
        };
    }

    let image_path = assets.allocate_image_path_and_store(extension, &image.data);

    // Build optional dimension parameters
    let mut params = Vec::new();

    if let Some(width_twips) = image.width_twips {
        let width = twips_to_typst_length(width_twips);
        params.push(format!("width: {}", width));
    }

    if let Some(height_twips) = image.height_twips {
        let height = twips_to_typst_length(height_twips);
        params.push(format!("height: {}", height));
    }

    // Build the image function call
    let typst_source = if params.is_empty() {
        format!("#image(\"{}\")", image_path)
    } else {
        format!("#image(\"{}\", {})", image_path, params.join(", "))
    };

    ImageOutput {
        typst_source,
        warnings: Vec::new(),
    }
}

/// Convert twips to Typst length string.
///
/// Typst supports various length units. We use inches for RTF compatibility.
///
/// Conversion: 1440 twips per inch
fn twips_to_typst_length(twips: i32) -> String {
    let inches = twips as f64 / 1440.0;
    format!("{:.2}in", inches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{codecs::jpeg::JpegEncoder, codecs::png::PngEncoder, ColorType, ImageEncoder};

    fn valid_png_data() -> Vec<u8> {
        let mut bytes = Vec::new();
        let rgba = [255_u8, 0, 0, 255];
        PngEncoder::new(&mut bytes)
            .write_image(&rgba, 1, 1, ColorType::Rgba8.into())
            .unwrap();
        bytes
    }

    fn valid_jpeg_data() -> Vec<u8> {
        let mut bytes = Vec::new();
        let rgb = [255_u8, 0, 0];
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 85);
        encoder.encode(&rgb, 1, 1, ColorType::Rgb8.into()).unwrap();
        bytes
    }

    #[test]
    fn test_twips_to_typst_length() {
        // 1440 twips = 1 inch
        assert_eq!(twips_to_typst_length(1440), "1.00in");

        // 720 twips = 0.5 inch
        assert_eq!(twips_to_typst_length(720), "0.50in");

        // 2880 twips = 2 inches
        assert_eq!(twips_to_typst_length(2880), "2.00in");

        // 0 twips = 0 inches
        assert_eq!(twips_to_typst_length(0), "0.00in");

        // 360 twips = 0.25 inch
        assert_eq!(twips_to_typst_length(360), "0.25in");
    }

    #[test]
    fn test_map_image_block_png_without_dimensions() {
        let image = ImageBlock::new(ImageFormat::Png, valid_png_data());
        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(output
            .typst_source
            .starts_with("#image(\"assets/image-000001.png\""));
        assert!(output.typst_source.ends_with(")"));
        assert!(!output.typst_source.contains("width:"));
        assert!(!output.typst_source.contains("height:"));
        assert!(output.warnings.is_empty());
        assert_eq!(assets.bundle.files.len(), 1);
    }

    #[test]
    fn test_map_image_block_jpeg_with_dimensions() {
        let image = ImageBlock::with_dimensions(
            ImageFormat::Jpeg,
            valid_jpeg_data(),
            1440, // 1 inch
            720,  // 0.5 inch
        );
        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(output.typst_source.contains("assets/image-000001.jpg"));
        assert!(output.typst_source.contains("width: 1.00in"));
        assert!(output.typst_source.contains("height: 0.50in"));
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_image_block_attribute_ordering() {
        let image = ImageBlock::with_dimensions(ImageFormat::Png, valid_png_data(), 1440, 720);
        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        // Verify width comes before height
        let width_pos = output
            .typst_source
            .find("width:")
            .expect("width should exist");
        let height_pos = output
            .typst_source
            .find("height:")
            .expect("height should exist");
        assert!(width_pos < height_pos, "width should come before height");
    }

    #[test]
    fn test_map_image_block_deterministic() {
        let image = ImageBlock::with_dimensions(ImageFormat::Png, valid_png_data(), 720, 720);

        // Generate Typst source multiple times
        let mut assets1 = TypstAssetAllocator::new();
        let mut assets2 = TypstAssetAllocator::new();
        let mut assets3 = TypstAssetAllocator::new();
        let output1 = map_image_block_with_assets(&image, &mut assets1);
        let output2 = map_image_block_with_assets(&image, &mut assets2);
        let output3 = map_image_block_with_assets(&image, &mut assets3);

        // Should be identical
        assert_eq!(output1.typst_source, output2.typst_source);
        assert_eq!(output2.typst_source, output3.typst_source);
        assert_eq!(assets1.bundle.files, assets2.bundle.files);
        assert_eq!(assets2.bundle.files, assets3.bundle.files);
    }

    #[test]
    fn test_map_image_block_emits_deterministic_asset_paths() {
        let image = ImageBlock::new(ImageFormat::Png, valid_png_data());
        let mut assets = TypstAssetAllocator::new();

        let output1 = map_image_block_with_assets(&image, &mut assets);
        let output2 = map_image_block_with_assets(&image, &mut assets);

        assert!(output1.typst_source.contains("assets/image-000001.png"));
        assert!(output2.typst_source.contains("assets/image-000002.png"));
        assert_eq!(assets.bundle.files.len(), 2);
    }

    #[test]
    fn test_map_image_block_only_width() {
        let mut image = ImageBlock::new(ImageFormat::Png, valid_png_data());
        image.width_twips = Some(1440);
        // height_twips remains None

        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(output.typst_source.contains("width: 1.00in"));
        assert!(!output.typst_source.contains("height:"));
    }

    #[test]
    fn test_map_image_block_only_height() {
        let mut image = ImageBlock::new(ImageFormat::Png, valid_png_data());
        image.height_twips = Some(720);
        // width_twips remains None

        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(!output.typst_source.contains("width:"));
        assert!(output.typst_source.contains("height: 0.50in"));
    }

    #[test]
    fn test_map_image_block_format_png() {
        let image = ImageBlock::new(ImageFormat::Png, valid_png_data());
        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(output.typst_source.contains(".png"));
    }

    #[test]
    fn test_map_image_block_format_jpeg() {
        let image = ImageBlock::new(ImageFormat::Jpeg, valid_jpeg_data());
        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(output.typst_source.contains(".jpg"));
    }

    #[test]
    fn test_map_image_block_invalid_png_payload_is_dropped() {
        let image = ImageBlock::new(ImageFormat::Png, vec![0x89, 0x50, 0x4E, 0x47]);
        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(output.typst_source.is_empty());
        assert_eq!(
            output.warnings,
            vec![MappingWarning::MalformedPngImagePayload]
        );
        assert!(assets.bundle.files.is_empty());
    }

    #[test]
    fn test_map_image_block_invalid_jpeg_payload_is_dropped() {
        let image = ImageBlock::new(ImageFormat::Jpeg, vec![0xFF, 0xD8, 0xFF, 0xE0]);
        let mut assets = TypstAssetAllocator::new();
        let output = map_image_block_with_assets(&image, &mut assets);

        assert!(output.typst_source.is_empty());
        assert_eq!(
            output.warnings,
            vec![MappingWarning::MalformedJpegImagePayload]
        );
        assert!(assets.bundle.files.is_empty());
    }
}
