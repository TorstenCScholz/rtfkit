//! Image block Typst mapping.
//!
//! This module provides functions for converting `ImageBlock` elements
//! to Typst markup with embedded data URIs.

use rtfkit_core::{ImageBlock, ImageFormat};

/// Result of mapping an image block to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageOutput {
    /// The generated Typst source code.
    pub typst_source: String,
}

/// Map an image block to Typst markup.
///
/// Emits a Typst `image` function with a data URI containing the base64-encoded image.
///
/// # Typst Output Format
///
/// ```typst
/// #image("data:image/png;base64,<base64_data>", width: 1.0in, height: 0.5in)
/// ```
///
/// For images without dimensions:
/// ```typst
/// #image("data:image/png;base64,<base64_data>")
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
    // Convert image data to base64
    let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image.data);

    // Determine MIME type based on format
    let mime_type = match image.format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
    };

    // Build data URI
    let data_uri = format!("data:{};base64,{}", mime_type, base64_data);

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
        format!("#image(\"{}\")", data_uri)
    } else {
        format!("#image(\"{}\", {})", data_uri, params.join(", "))
    };

    ImageOutput { typst_source }
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
        let image = ImageBlock::new(ImageFormat::Png, vec![0x89, 0x50, 0x4E, 0x47]);
        let output = map_image_block(&image);

        assert!(output.typst_source.starts_with("#image(\"data:image/png;base64,"));
        assert!(output.typst_source.ends_with(")"));
        assert!(!output.typst_source.contains("width:"));
        assert!(!output.typst_source.contains("height:"));
    }

    #[test]
    fn test_map_image_block_jpeg_with_dimensions() {
        let image = ImageBlock::with_dimensions(
            ImageFormat::Jpeg,
            vec![0xFF, 0xD8, 0xFF, 0xE0],
            1440, // 1 inch
            720,  // 0.5 inch
        );
        let output = map_image_block(&image);

        assert!(output.typst_source.contains("data:image/jpeg;base64,"));
        assert!(output.typst_source.contains("width: 1.00in"));
        assert!(output.typst_source.contains("height: 0.50in"));
    }

    #[test]
    fn test_map_image_block_attribute_ordering() {
        let image = ImageBlock::with_dimensions(
            ImageFormat::Png,
            vec![0x00],
            1440,
            720,
        );
        let output = map_image_block(&image);

        // Verify width comes before height
        let width_pos = output.typst_source.find("width:").expect("width should exist");
        let height_pos = output.typst_source.find("height:").expect("height should exist");
        assert!(width_pos < height_pos, "width should come before height");
    }

    #[test]
    fn test_map_image_block_deterministic() {
        let image = ImageBlock::with_dimensions(
            ImageFormat::Png,
            vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            720,
            720,
        );

        // Generate Typst source multiple times
        let output1 = map_image_block(&image);
        let output2 = map_image_block(&image);
        let output3 = map_image_block(&image);

        // Should be identical
        assert_eq!(output1.typst_source, output2.typst_source);
        assert_eq!(output2.typst_source, output3.typst_source);
    }

    #[test]
    fn test_map_image_block_base64_encoding() {
        // Test with known data
        let image = ImageBlock::new(ImageFormat::Png, b"test".to_vec());
        let output = map_image_block(&image);

        // "test" in base64 is "dGVzdA=="
        assert!(output.typst_source.contains("dGVzdA=="), "Base64 encoding should be correct");
    }

    #[test]
    fn test_map_image_block_only_width() {
        let mut image = ImageBlock::new(ImageFormat::Png, vec![0x00]);
        image.width_twips = Some(1440);
        // height_twips remains None

        let output = map_image_block(&image);

        assert!(output.typst_source.contains("width: 1.00in"));
        assert!(!output.typst_source.contains("height:"));
    }

    #[test]
    fn test_map_image_block_only_height() {
        let mut image = ImageBlock::new(ImageFormat::Png, vec![0x00]);
        image.height_twips = Some(720);
        // width_twips remains None

        let output = map_image_block(&image);

        assert!(!output.typst_source.contains("width:"));
        assert!(output.typst_source.contains("height: 0.50in"));
    }

    #[test]
    fn test_map_image_block_format_png() {
        let image = ImageBlock::new(ImageFormat::Png, vec![0x00]);
        let output = map_image_block(&image);

        assert!(output.typst_source.contains("image/png"));
    }

    #[test]
    fn test_map_image_block_format_jpeg() {
        let image = ImageBlock::new(ImageFormat::Jpeg, vec![0x00]);
        let output = map_image_block(&image);

        assert!(output.typst_source.contains("image/jpeg"));
    }
}
