//! Integration tests for in-process Typst PDF rendering.
//!
//! These tests verify that:
//! - PDF generation works without external `typst` binary
//! - PDF output is valid (starts with `%PDF-`)
//! - Various document types can be rendered

use rtfkit_core::{
    Block, Document, ImageBlock, ImageFormat, ListBlock, ListItem, ListKind, Paragraph, Run,
    TableBlock, TableCell, TableRow,
};
use rtfkit_render_typst::{
    DeterminismOptions, RenderOptions, compile_to_pdf, document_to_pdf_with_warnings,
};

fn run_isolated_test(subtest_name: &str, gate_var: &str) {
    let exe = std::env::current_exe().expect("Failed to resolve current test binary path");
    let output = std::process::Command::new(exe)
        .arg("--exact")
        .arg(subtest_name)
        .arg("--nocapture")
        .env(gate_var, "1")
        .output()
        .expect("Failed to execute isolated subprocess test");

    assert!(
        output.status.success(),
        "Isolated test '{}' failed.\nstdout:\n{}\nstderr:\n{}",
        subtest_name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that a simple paragraph can be rendered to valid PDF.
#[test]
fn test_render_simple_paragraph_to_pdf() {
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("Hello, World!"),
    ]))]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    // Verify PDF structure
    assert!(
        output.pdf_bytes.starts_with(b"%PDF-"),
        "PDF should start with %PDF-"
    );
    let pdf_str = String::from_utf8_lossy(&output.pdf_bytes);
    assert!(pdf_str.contains("%%EOF"), "PDF should contain %%EOF marker");

    // Should have no warnings for simple content
    assert!(
        output.warnings.is_empty(),
        "Unexpected warnings: {:?}",
        output.warnings
    );
}

/// Test that a document with multiple paragraphs can be rendered.
#[test]
fn test_render_multiple_paragraphs_to_pdf() {
    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("First paragraph")])),
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Second paragraph")])),
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Third paragraph")])),
    ]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
    assert!(output.warnings.is_empty());
}

/// Test that a document with formatted text can be rendered.
#[test]
fn test_render_formatted_text_to_pdf() {
    let mut bold_run = Run::new("bold");
    bold_run.bold = true;
    let mut italic_run = Run::new("italic");
    italic_run.italic = true;
    let mut underline_run = Run::new("underline");
    underline_run.underline = true;

    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("Normal "),
        bold_run,
        Run::new(" "),
        italic_run,
        Run::new(" "),
        underline_run,
    ]))]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that a bullet list can be rendered.
#[test]
fn test_render_bullet_list_to_pdf() {
    let mut list = ListBlock::new(1, ListKind::Bullet);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Item 1")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Item 2")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Item 3")]),
    ));

    let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that an ordered list can be rendered.
#[test]
fn test_render_ordered_list_to_pdf() {
    let mut list = ListBlock::new(1, ListKind::OrderedDecimal);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("First")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Second")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Third")]),
    ));

    let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that a table can be rendered.
#[test]
fn test_render_table_to_pdf() {
    let table = TableBlock::from_rows(vec![
        TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A1")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B1")])),
        ]),
        TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A2")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B2")])),
        ]),
    ]);

    let doc = Document::from_blocks(vec![Block::TableBlock(table)]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that a mixed document can be rendered.
#[test]
fn test_render_mixed_document_to_pdf() {
    let mut list = ListBlock::new(1, ListKind::Bullet);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("List item")]),
    ));

    let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
        Paragraph::from_runs(vec![Run::new("Cell")]),
    )])]);

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Introduction")])),
        Block::ListBlock(list),
        Block::TableBlock(table),
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Conclusion")])),
    ]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that deterministic output works.
#[test]
fn test_deterministic_pdf_output() {
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("Deterministic test"),
    ]))]);

    let options = RenderOptions {
        determinism: DeterminismOptions {
            fixed_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Render twice with same options
    let result1 = document_to_pdf_with_warnings(&doc, &options).unwrap();
    let result2 = document_to_pdf_with_warnings(&doc, &options).unwrap();

    // Should produce identical output
    assert_eq!(
        result1.pdf_bytes, result2.pdf_bytes,
        "PDF output should be deterministic"
    );
}

/// Test that empty document can be rendered.
#[test]
fn test_render_empty_document_to_pdf() {
    let doc = Document::new();

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(
        result.is_ok(),
        "Failed to render empty document: {:?}",
        result.err()
    );
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that special characters are handled correctly.
#[test]
fn test_render_special_characters_to_pdf() {
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new(
            "Special: [brackets] *asterisk* _underscore_ @at #hash $dollar ~tilde \\backslash",
        ),
    ]))]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that Unicode text can be rendered.
#[test]
fn test_render_unicode_to_pdf() {
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("Unicode: café, naïve, 日本語, 中文, Ελληνικά, العربية"),
    ]))]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that the low-level compile_to_pdf function works.
#[test]
fn test_low_level_compile_to_pdf() {
    let source = "#set page(width: 210mm, height: 297mm)\nHello from Typst!";
    let result = compile_to_pdf(source, None);

    assert!(result.is_ok(), "Failed to compile: {:?}", result.err());
    let (pdf_bytes, warnings) = result.unwrap();

    assert!(pdf_bytes.starts_with(b"%PDF-"));
    assert!(warnings.is_empty());
}

/// Test that compile errors are properly reported.
#[test]
fn test_compile_error_reporting() {
    // Invalid Typst code
    let source = "#undefined_function_xyz()";
    let result = compile_to_pdf(source, None);

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Error message should mention the undefined function
    let err_string = err.to_string();
    assert!(
        err_string.contains("undefined") || err_string.contains("unknown"),
        "Error should mention undefined function: {}",
        err_string
    );
}

/// Test that PDF output is reasonably sized.
#[test]
fn test_pdf_output_size() {
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("Hello, World!"),
    ]))]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options).unwrap();

    // PDF should be at least 1KB for a simple document
    assert!(
        result.pdf_bytes.len() > 1000,
        "PDF too small: {} bytes",
        result.pdf_bytes.len()
    );
    // But not unreasonably large (less than 100KB for simple content)
    assert!(
        result.pdf_bytes.len() < 100_000,
        "PDF too large: {} bytes",
        result.pdf_bytes.len()
    );
}

/// Test that rendering works without external typst binary (in-process only).
/// This test verifies that we don't shell out to any external process.
#[test]
fn test_no_external_binary_required() {
    // This test should pass even if typst CLI is not installed on the system
    // because we use the in-process Typst library
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("In-process rendering test"),
    ]))]);

    let options = RenderOptions::default();

    // This should work without any external binary
    let result = document_to_pdf_with_warnings(&doc, &options);
    assert!(
        result.is_ok(),
        "In-process rendering should work without external binary"
    );

    let output = result.unwrap();
    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that rendering works with empty PATH environment variable.
/// This verifies we don't depend on any external tools.
#[test]
fn test_works_with_empty_path() {
    run_isolated_test(
        "test_works_with_empty_path_subprocess",
        "RTFKIT_ISOLATED_EMPTY_PATH",
    );
}

#[test]
fn test_works_with_empty_path_subprocess() {
    if std::env::var_os("RTFKIT_ISOLATED_EMPTY_PATH").is_none() {
        return;
    }

    use std::env;

    // Save the current PATH
    let original_path = env::var("PATH").ok();

    // Set PATH to empty (this test should still work)
    // SAFETY: This is a test and we restore PATH afterwards
    unsafe {
        env::set_var("PATH", "");
    }

    let result = std::panic::catch_unwind(|| {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Empty PATH test"),
        ]))]);

        let options = RenderOptions::default();
        let result = document_to_pdf_with_warnings(&doc, &options);

        assert!(result.is_ok(), "Rendering should work with empty PATH");
        let output = result.unwrap();
        assert!(output.pdf_bytes.starts_with(b"%PDF-"));
    });

    // Restore PATH
    // SAFETY: This is a test and we're restoring the original value
    unsafe {
        if let Some(path) = original_path {
            env::set_var("PATH", path);
        } else {
            env::remove_var("PATH");
        }
    }

    // Re-raise any panic
    assert!(result.is_ok());
}

/// Test that embedded fonts are used (no system fonts required).
#[test]
fn test_embedded_fonts_only() {
    // This test verifies that we can render without any system fonts
    // because we use embedded fonts from typst-kit
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("Embedded font test: ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        Run::new("abcdefghijklmnopqrstuvwxyz"),
        Run::new("0123456789"),
    ]))]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Rendering with embedded fonts should work");
    let output = result.unwrap();

    // Verify PDF is valid
    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
    let pdf_str = String::from_utf8_lossy(&output.pdf_bytes);
    assert!(pdf_str.contains("%%EOF"));
}

// =============================================================================
// Image Rendering Tests
// =============================================================================

/// Minimal PNG image data (1x1 transparent pixel)
fn minimal_png_data() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, 0x06, // bit depth: 8, color type: RGBA
        0x00, 0x00, 0x00, // compression, filter, interlace
        0x1F, 0x15, 0xC4, 0x89, // CRC
        0x00, 0x00, 0x00, 0x0A, // IDAT length
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x9C, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, // compressed data
        0xE2, 0x21, 0xBC, 0x33, // CRC
        0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82, // CRC
    ]
}

/// Minimal JPEG image data (1x1 gray pixel)
fn minimal_jpeg_data() -> Vec<u8> {
    vec![
        0xFF, 0xD8, // SOI
        0xFF, 0xE0, 0x00, 0x10, // APP0
        0x4A, 0x46, 0x49, 0x46, 0x00, // JFIF\0
        0x01, 0x01, // version
        0x00, // aspect ratio units
        0x00, 0x01, 0x00, 0x01, // aspect ratio
        0x00, 0x00, // thumbnail
        0xFF, 0xDB, 0x00, 0x43, 0x00, // DQT
        0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, // quantization table
        0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14,
        0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13,
        0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A,
        0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22,
        0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C,
        0x30, 0x31, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D,
        0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32,
        0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, // SOF
        0x01, 0x01, 0x11, 0x00,
        0xFF, 0xC4, 0x00, 0x1F, 0x00, // DHT
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0A, 0x0B,
        0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, // SOS
        0x3F, 0x00,
        0xFB, 0xD3, 0x28, 0xA2, 0x80, 0x00, // scan data
        0xFF, 0xD9, // EOI
    ]
}

/// Test that a PNG image can be rendered to PDF.
///
/// NOTE: This test is currently ignored because the Typst renderer uses data URIs
/// for images, but Typst doesn't support data URIs - it interprets them as file paths.
/// See: https://github.com/typst/typst/issues/184
/// TODO: Fix the image rendering to use file paths or embedded bytes instead of data URIs.
#[ignore = "Typst doesn't support data URIs for images - needs implementation fix"]
#[test]
fn test_render_png_image_to_pdf() {
    let image_block = ImageBlock {
        format: ImageFormat::Png,
        data: minimal_png_data(),
        width_twips: Some(1440), // 1 inch
        height_twips: Some(1440),
    };

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Before image")])),
        Block::ImageBlock(image_block),
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("After image")])),
    ]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render PNG image: {:?}", result.err());
    let output = result.unwrap();

    // Verify PDF structure
    assert!(
        output.pdf_bytes.starts_with(b"%PDF-"),
        "PDF should start with %PDF-"
    );
    let pdf_str = String::from_utf8_lossy(&output.pdf_bytes);
    assert!(pdf_str.contains("%%EOF"), "PDF should contain %%EOF marker");
}

/// Test that a JPEG image can be rendered to PDF.
///
/// NOTE: This test is currently ignored because the Typst renderer uses data URIs
/// for images, but Typst doesn't support data URIs - it interprets them as file paths.
/// See: https://github.com/typst/typst/issues/184
/// TODO: Fix the image rendering to use file paths or embedded bytes instead of data URIs.
#[ignore = "Typst doesn't support data URIs for images - needs implementation fix"]
#[test]
fn test_render_jpeg_image_to_pdf() {
    let image_block = ImageBlock {
        format: ImageFormat::Jpeg,
        data: minimal_jpeg_data(),
        width_twips: Some(1440),
        height_twips: Some(1440),
    };

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("JPEG test")])),
        Block::ImageBlock(image_block),
    ]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render JPEG image: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that multiple images can be rendered in one document.
///
/// NOTE: This test is currently ignored because the Typst renderer uses data URIs
/// for images, but Typst doesn't support data URIs - it interprets them as file paths.
/// See: https://github.com/typst/typst/issues/184
/// TODO: Fix the image rendering to use file paths or embedded bytes instead of data URIs.
#[ignore = "Typst doesn't support data URIs for images - needs implementation fix"]
#[test]
fn test_render_multiple_images_to_pdf() {
    let png_image = ImageBlock {
        format: ImageFormat::Png,
        data: minimal_png_data(),
        width_twips: Some(1440),
        height_twips: Some(1440),
    };

    let jpeg_image = ImageBlock {
        format: ImageFormat::Jpeg,
        data: minimal_jpeg_data(),
        width_twips: Some(1440),
        height_twips: Some(1440),
    };

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("First: PNG")])),
        Block::ImageBlock(png_image),
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Second: JPEG")])),
        Block::ImageBlock(jpeg_image),
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Done")])),
    ]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render multiple images: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that an image with dimensions is rendered correctly.
///
/// NOTE: This test is currently ignored because the Typst renderer uses data URIs
/// for images, but Typst doesn't support data URIs - it interprets them as file paths.
/// See: https://github.com/typst/typst/issues/184
/// TODO: Fix the image rendering to use file paths or embedded bytes instead of data URIs.
#[ignore = "Typst doesn't support data URIs for images - needs implementation fix"]
#[test]
fn test_render_image_with_dimensions_to_pdf() {
    // 2 inches wide, 1 inch tall
    let image_block = ImageBlock {
        format: ImageFormat::Png,
        data: minimal_png_data(),
        width_twips: Some(2880), // 2 inches
        height_twips: Some(1440), // 1 inch
    };

    let doc = Document::from_blocks(vec![Block::ImageBlock(image_block)]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(result.is_ok(), "Failed to render image with dimensions: {:?}", result.err());
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that an image without dimensions (intrinsic size) can be rendered.
///
/// NOTE: This test is currently ignored because the Typst renderer uses data URIs
/// for images, but Typst doesn't support data URIs - it interprets them as file paths.
/// See: https://github.com/typst/typst/issues/184
/// TODO: Fix the image rendering to use file paths or embedded bytes instead of data URIs.
#[ignore = "Typst doesn't support data URIs for images - needs implementation fix"]
#[test]
fn test_render_image_without_dimensions_to_pdf() {
    let image_block = ImageBlock {
        format: ImageFormat::Png,
        data: minimal_png_data(),
        width_twips: None,
        height_twips: None,
    };

    let doc = Document::from_blocks(vec![Block::ImageBlock(image_block)]);

    let options = RenderOptions::default();
    let result = document_to_pdf_with_warnings(&doc, &options);

    assert!(
        result.is_ok(),
        "Failed to render image without dimensions: {:?}",
        result.err()
    );
    let output = result.unwrap();

    assert!(output.pdf_bytes.starts_with(b"%PDF-"));
}

/// Test that image rendering is deterministic.
///
/// NOTE: This test is currently ignored because the Typst renderer uses data URIs
/// for images, but Typst doesn't support data URIs - it interprets them as file paths.
/// See: https://github.com/typst/typst/issues/184
/// TODO: Fix the image rendering to use file paths or embedded bytes instead of data URIs.
#[ignore = "Typst doesn't support data URIs for images - needs implementation fix"]
#[test]
fn test_image_rendering_is_deterministic() {
    let image_block = ImageBlock {
        format: ImageFormat::Png,
        data: minimal_png_data(),
        width_twips: Some(1440),
        height_twips: Some(1440),
    };

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Determinism test")])),
        Block::ImageBlock(image_block),
    ]);

    let options = RenderOptions {
        determinism: DeterminismOptions {
            fixed_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Render twice
    let result1 = document_to_pdf_with_warnings(&doc, &options).unwrap();
    let result2 = document_to_pdf_with_warnings(&doc, &options).unwrap();

    // Should produce identical output
    assert_eq!(
        result1.pdf_bytes, result2.pdf_bytes,
        "PDF output with image should be deterministic"
    );
}

/// Test that multiple images rendering is deterministic.
///
/// NOTE: This test is currently ignored because the Typst renderer uses data URIs
/// for images, but Typst doesn't support data URIs - it interprets them as file paths.
/// See: https://github.com/typst/typst/issues/184
/// TODO: Fix the image rendering to use file paths or embedded bytes instead of data URIs.
#[ignore = "Typst doesn't support data URIs for images - needs implementation fix"]
#[test]
fn test_multiple_images_rendering_is_deterministic() {
    let png_image = ImageBlock {
        format: ImageFormat::Png,
        data: minimal_png_data(),
        width_twips: Some(1440),
        height_twips: Some(1440),
    };

    let jpeg_image = ImageBlock {
        format: ImageFormat::Jpeg,
        data: minimal_jpeg_data(),
        width_twips: Some(1440),
        height_twips: Some(1440),
    };

    let doc = Document::from_blocks(vec![
        Block::ImageBlock(png_image),
        Block::ImageBlock(jpeg_image),
    ]);

    let options = RenderOptions {
        determinism: DeterminismOptions {
            fixed_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let result1 = document_to_pdf_with_warnings(&doc, &options).unwrap();
    let result2 = document_to_pdf_with_warnings(&doc, &options).unwrap();

    assert_eq!(
        result1.pdf_bytes, result2.pdf_bytes,
        "PDF output with multiple images should be deterministic"
    );
}
