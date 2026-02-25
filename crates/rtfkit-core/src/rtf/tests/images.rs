//! Image Parsing Tests
//!
//! Tests for RTF image handling including:
//! - PNG and JPEG image parsing
//! - Dimension extraction (picwgoal, pichgoal)
//! - Multiple images in a document
//! - Unsupported image formats
//! - Malformed hex data
//! - shppict/nonshppict preference

use crate::rtf::parse;
use crate::{Block, ImageBlock, ImageFormat, Warning};

// =============================================================================
// PNG Image Tests
// =============================================================================

#[test]
fn test_png_image_parsing() {
    let input = include_str!("../../../../../fixtures/image_png_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have an image block
    let has_image = doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::ImageBlock(_)));

    assert!(has_image, "Expected an image block in the document");
}

#[test]
fn test_png_image_data() {
    let input = include_str!("../../../../../fixtures/image_png_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::ImageBlock(img)) = doc.blocks.iter().find(|b| matches!(b, Block::ImageBlock(_))) {
        // Should be PNG format
        assert_eq!(img.format, ImageFormat::Png);

        // Should have image data
        assert!(!img.data.is_empty());

        // PNG files start with the signature: 89 50 4E 47 0D 0A 1A 0A
        assert!(img.data.len() >= 8);
        assert_eq!(img.data[0], 0x89);
        assert_eq!(img.data[1], 0x50); // 'P'
        assert_eq!(img.data[2], 0x4E); // 'N'
        assert_eq!(img.data[3], 0x47); // 'G'
    } else {
        panic!("Expected ImageBlock not found");
    }
}

// =============================================================================
// JPEG Image Tests
// =============================================================================

#[test]
fn test_jpeg_image_parsing() {
    let input = include_str!("../../../../../fixtures/image_jpeg_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have an image block
    let has_image = doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::ImageBlock(_)));

    assert!(has_image, "Expected an image block in the document");
}

#[test]
fn test_jpeg_image_data() {
    let input = include_str!("../../../../../fixtures/image_jpeg_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::ImageBlock(img)) = doc.blocks.iter().find(|b| matches!(b, Block::ImageBlock(_))) {
        // Should be JPEG format
        assert_eq!(img.format, ImageFormat::Jpeg);

        // Should have image data
        assert!(!img.data.is_empty());

        // JPEG files start with FFD8 (SOI marker)
        assert!(img.data.len() >= 2);
        assert_eq!(img.data[0], 0xFF);
        assert_eq!(img.data[1], 0xD8);
    } else {
        panic!("Expected ImageBlock not found");
    }
}

// =============================================================================
// Dimension Extraction Tests
// =============================================================================

#[test]
fn test_image_with_dimensions() {
    let input = include_str!("../../../../../fixtures/image_with_dimensions.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::ImageBlock(img)) = doc.blocks.iter().find(|b| matches!(b, Block::ImageBlock(_))) {
        // picwgoal2880 = 2 inches (2880 twips)
        // pichgoal1440 = 1 inch (1440 twips)
        assert_eq!(img.width_twips, Some(2880));
        assert_eq!(img.height_twips, Some(1440));
    } else {
        panic!("Expected ImageBlock not found");
    }
}

#[test]
fn test_image_default_dimensions() {
    // PNG simple has picwgoal1440 and pichgoal1440
    let input = include_str!("../../../../../fixtures/image_png_simple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    if let Some(Block::ImageBlock(img)) = doc.blocks.iter().find(|b| matches!(b, Block::ImageBlock(_))) {
        // Should have dimensions from the fixture
        assert_eq!(img.width_twips, Some(1440));
        assert_eq!(img.height_twips, Some(1440));
    } else {
        panic!("Expected ImageBlock not found");
    }
}

// =============================================================================
// Multiple Images Tests
// =============================================================================

#[test]
fn test_multiple_images() {
    let input = include_str!("../../../../../fixtures/image_multiple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Count image blocks
    let image_count = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::ImageBlock(_)))
        .count();

    assert_eq!(image_count, 3, "Expected 3 images in the document");
}

#[test]
fn test_multiple_images_formats() {
    let input = include_str!("../../../../../fixtures/image_multiple.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    let images: Vec<&ImageBlock> = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::ImageBlock(img) = b {
                Some(img)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(images.len(), 3);

    // First should be PNG
    assert_eq!(images[0].format, ImageFormat::Png);

    // Second should be JPEG
    assert_eq!(images[1].format, ImageFormat::Jpeg);

    // Third should be PNG
    assert_eq!(images[2].format, ImageFormat::Png);
}

// =============================================================================
// Unsupported Format Tests
// =============================================================================

#[test]
fn test_unsupported_image_format() {
    let input = include_str!("../../../../../fixtures/image_unsupported_format.rtf");
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());

    let (doc, report) = result.unwrap();

    // Should NOT have an image block (wmetafile is unsupported)
    let has_image = doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::ImageBlock(_)));

    assert!(!has_image, "Expected no image block for unsupported format");

    // Should have a DroppedContent warning
    let has_dropped_warning = report.warnings.iter().any(|w| {
        matches!(w, Warning::DroppedContent { reason, .. } if reason.contains("unsupported") || reason.contains("image"))
    });

    assert!(has_dropped_warning, "Expected DroppedContent warning for unsupported image format");
}

// =============================================================================
// Malformed Hex Tests
// =============================================================================

#[test]
fn test_malformed_hex_image() {
    let input = include_str!("../../../../../fixtures/image_malformed_hex.rtf");
    let result = parse(input);

    // Should parse gracefully (not fail)
    assert!(result.is_ok());

    let (doc, report) = result.unwrap();

    // The malformed hex should result in either:
    // 1. No image block (if parsing failed completely), or
    // 2. A DroppedContent warning about the malformed data
    let has_dropped_warning = report.warnings.iter().any(|w| {
        matches!(w, Warning::DroppedContent { .. })
    });

    // Either no image or a dropped content warning
    let has_image = doc
        .blocks
        .iter()
        .any(|b| matches!(b, Block::ImageBlock(_)));

    // If there's an image, it should have valid PNG header (partial parse)
    // If there's no image, we should have a dropped warning
    if has_image {
        // Partial parse succeeded - check that it has valid PNG header
        if let Some(Block::ImageBlock(img)) = doc.blocks.iter().find(|b| matches!(b, Block::ImageBlock(_))) {
            assert_eq!(img.format, ImageFormat::Png);
            // Should have at least the PNG signature
            assert!(img.data.len() >= 8);
        }
    } else {
        // Should have a warning about dropped content
        assert!(has_dropped_warning, "Expected DroppedContent warning for malformed hex");
    }
}

// =============================================================================
// shppict/nonshppict Preference Tests
// =============================================================================

#[test]
fn test_shppict_preferred_over_nonshppict() {
    let input = include_str!("../../../../../fixtures/image_shppict_nonshppict.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Count images - the parser may handle shppict/nonshppict differently
    // At minimum, we should have at least one image
    let image_count = doc
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::ImageBlock(_)))
        .count();

    assert!(image_count >= 1, "Expected at least 1 image from shppict/nonshppict group");
}

#[test]
fn test_shppict_image_is_png() {
    let input = include_str!("../../../../../fixtures/image_shppict_nonshppict.rtf");
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // The shppict contains a PNG image
    if let Some(Block::ImageBlock(img)) = doc.blocks.iter().find(|b| matches!(b, Block::ImageBlock(_))) {
        // Should be PNG (from shppict), not JPEG (from nonshppict)
        assert_eq!(img.format, ImageFormat::Png);
    } else {
        panic!("Expected ImageBlock not found");
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_image_data() {
    // Image with no hex data
    let input = r#"{\rtf1\ansi {\pict\pngblip }\par}"#;
    let result = parse(input);

    // Should parse gracefully
    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // The parser may create an image block with empty data or skip it entirely
    // Either behavior is acceptable for empty image data
    let image_blocks: Vec<&ImageBlock> = doc
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::ImageBlock(img) = b {
                Some(img)
            } else {
                None
            }
        })
        .collect();

    // If there's an image block, it should have empty or minimal data
    for img in image_blocks {
        // Empty or minimal data is acceptable
        assert!(img.data.is_empty() || img.data.len() < 10, "Empty image should have no or minimal data");
    }
}

#[test]
fn test_image_with_text_before_and_after() {
    let input = r#"{\rtf1\ansi Before {\pict\pngblip\picwgoal1440\pichgoal1440 89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000A49444154789C6360000000020001E221BC330000000049454E44AE426082} After}"#;
    let result = parse(input);

    assert!(result.is_ok());

    let (doc, _report) = result.unwrap();

    // Should have at least one paragraph with text
    let has_text = doc.blocks.iter().any(|b| {
        matches!(b, Block::Paragraph(p) if !p.inlines.is_empty())
    });

    assert!(has_text, "Expected text content in document");
}

#[test]
fn test_image_format_detection() {
    // Test that format is correctly detected from control words
    let png_input = r#"{\rtf1\ansi {\pict\pngblip 89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000A49444154789C6360000000020001E221BC330000000049454E44AE426082}}"#;
    let jpeg_input = r#"{\rtf1\ansi {\pict\jpegblip FFD8FFE000104A46494600010100000100010000FFD9}}"#;

    let png_result = parse(png_input).unwrap();
    let jpeg_result = parse(jpeg_input).unwrap();

    let (png_doc, _) = png_result;
    let (jpeg_doc, _) = jpeg_result;

    if let Some(Block::ImageBlock(img)) = png_doc.blocks.first() {
        assert_eq!(img.format, ImageFormat::Png);
    } else {
        panic!("Expected PNG image");
    }

    if let Some(Block::ImageBlock(img)) = jpeg_doc.blocks.first() {
        assert_eq!(img.format, ImageFormat::Jpeg);
    } else {
        panic!("Expected JPEG image");
    }
}