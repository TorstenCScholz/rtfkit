//! Image block HTML emission.
//!
//! This module provides functions for converting `ImageBlock` elements
//! to HTML `<figure>` elements with embedded data URIs.

use crate::serialize::HtmlBuffer;
use rtfkit_core::{ImageBlock, ImageFormat};

/// Converts an image block to HTML.
///
/// Emits a `<figure>` element containing an `<img>` with a data URI.
/// The image data is base64-encoded and embedded directly in the HTML.
///
/// # HTML Output Format
///
/// ```html
/// <figure class="rtf-image">
///   <img src="data:image/png;base64,<base64_data>" alt="" [width="..." height="..."]>
/// </figure>
/// ```
///
/// # Dimension Conversion
///
/// Dimensions in twips are converted to pixels using the formula:
/// `pixels = twips / 15` (based on 96 DPI, 1440 twips per inch).
pub fn image_to_html(image: &ImageBlock, buf: &mut HtmlBuffer) {
    // Convert image data to base64
    let base64_data =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image.data);

    // Determine MIME type based on format
    let mime_type = match image.format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
    };

    // Build data URI
    let data_uri = format!("data:{};base64,{}", mime_type, base64_data);

    // Build attributes with stable ordering for determinism
    // Order: src, alt, width, height (when present)
    let mut attrs: Vec<(&str, String)> = vec![("src", data_uri), ("alt", String::new())];

    // Add dimensions if available (convert twips to pixels)
    if let Some(width_twips) = image.width_twips {
        let width_px = twips_to_pixels(width_twips);
        attrs.push(("width", width_px.to_string()));
    }

    if let Some(height_twips) = image.height_twips {
        let height_px = twips_to_pixels(height_twips);
        attrs.push(("height", height_px.to_string()));
    }

    // Emit figure wrapper
    buf.push_open_tag("figure", &[("class", "rtf-image")]);

    // Emit img tag with attributes
    // Convert String attrs to &str refs for the API
    let attrs_refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    buf.push_self_closing_tag("img", &attrs_refs);

    // Close figure
    buf.push_close_tag("figure");
}

/// Convert twips to pixels.
///
/// Conversion formula based on:
/// - 1 twip = 1/20 point
/// - 72 points per inch
/// - 96 pixels per inch (default screen DPI)
///
/// Therefore: 1440 twips per inch, 96 pixels per inch
/// pixels = twips * 96 / 1440 = twips / 15
fn twips_to_pixels(twips: i32) -> i32 {
    twips / 15
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twips_to_pixels() {
        // 1440 twips = 1 inch = 96 pixels
        assert_eq!(twips_to_pixels(1440), 96);

        // 720 twips = 0.5 inch = 48 pixels
        assert_eq!(twips_to_pixels(720), 48);

        // 15 twips = 1 pixel
        assert_eq!(twips_to_pixels(15), 1);

        // 0 twips = 0 pixels
        assert_eq!(twips_to_pixels(0), 0);
    }

    #[test]
    fn test_image_to_html_png_without_dimensions() {
        let image = ImageBlock::new(ImageFormat::Png, vec![0x89, 0x50, 0x4E, 0x47]);
        let mut buf = HtmlBuffer::new();

        image_to_html(&image, &mut buf);

        let html = buf.into_string();

        // Check structure
        assert!(html.starts_with(r#"<figure class="rtf-image">"#));
        assert!(html.ends_with("</figure>"));

        // Check img tag
        assert!(html.contains("<img"));
        assert!(html.contains("src=\"data:image/png;base64,"));
        assert!(html.contains("alt=\"\""));
        assert!(html.contains(" />"));

        // Should not have width/height
        assert!(!html.contains("width="));
        assert!(!html.contains("height="));
    }

    #[test]
    fn test_image_to_html_jpeg_with_dimensions() {
        let image = ImageBlock::with_dimensions(
            ImageFormat::Jpeg,
            vec![0xFF, 0xD8, 0xFF, 0xE0],
            1440, // 1 inch = 96 pixels
            720,  // 0.5 inch = 48 pixels
        );
        let mut buf = HtmlBuffer::new();

        image_to_html(&image, &mut buf);

        let html = buf.into_string();

        // Check MIME type
        assert!(html.contains("data:image/jpeg;base64,"));

        // Check dimensions
        assert!(html.contains(r#"width="96""#));
        assert!(html.contains(r#"height="48""#));
    }

    #[test]
    fn test_image_to_html_attribute_ordering() {
        let image = ImageBlock::with_dimensions(ImageFormat::Png, vec![0x00], 150, 300);
        let mut buf = HtmlBuffer::new();

        image_to_html(&image, &mut buf);

        let html = buf.into_string();

        // Verify attribute ordering: src, alt, width, height
        let src_pos = html.find("src=").expect("src attribute should exist");
        let alt_pos = html.find("alt=").expect("alt attribute should exist");
        let width_pos = html.find("width=").expect("width attribute should exist");
        let height_pos = html.find("height=").expect("height attribute should exist");

        assert!(src_pos < alt_pos, "src should come before alt");
        assert!(alt_pos < width_pos, "alt should come before width");
        assert!(width_pos < height_pos, "width should come before height");
    }

    #[test]
    fn test_image_to_html_deterministic() {
        let image = ImageBlock::with_dimensions(
            ImageFormat::Png,
            vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            720,
            720,
        );

        // Generate HTML multiple times
        let html1 = {
            let mut buf = HtmlBuffer::new();
            image_to_html(&image, &mut buf);
            buf.into_string()
        };

        let html2 = {
            let mut buf = HtmlBuffer::new();
            image_to_html(&image, &mut buf);
            buf.into_string()
        };

        // Should be identical
        assert_eq!(html1, html2, "HTML output should be deterministic");
    }

    #[test]
    fn test_image_to_html_base64_encoding() {
        // Test with known data
        let image = ImageBlock::new(ImageFormat::Png, b"test".to_vec());
        let mut buf = HtmlBuffer::new();

        image_to_html(&image, &mut buf);

        let html = buf.into_string();

        // "test" in base64 is "dGVzdA=="
        assert!(
            html.contains("dGVzdA=="),
            "Base64 encoding should be correct"
        );
    }

    #[test]
    fn test_image_to_html_only_width() {
        let mut image = ImageBlock::new(ImageFormat::Png, vec![0x00]);
        image.width_twips = Some(1440);
        // height_twips remains None

        let mut buf = HtmlBuffer::new();
        image_to_html(&image, &mut buf);

        let html = buf.into_string();

        assert!(html.contains(r#"width="96""#));
        assert!(!html.contains("height="));
    }

    #[test]
    fn test_image_to_html_only_height() {
        let mut image = ImageBlock::new(ImageFormat::Png, vec![0x00]);
        image.height_twips = Some(720);
        // width_twips remains None

        let mut buf = HtmlBuffer::new();
        image_to_html(&image, &mut buf);

        let html = buf.into_string();

        assert!(!html.contains("width="));
        assert!(html.contains(r#"height="48""#));
    }
}
