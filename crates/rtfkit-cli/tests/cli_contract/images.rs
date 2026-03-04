use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

fn project_root() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

// -------------------------------------------------------------------------
// CLI Image Contract Tests: Success Cases
// -------------------------------------------------------------------------

/// Test that PNG image converts successfully to DOCX.
#[test]
fn cli_image_png_to_docx() {
    let fixture = project_root().join("fixtures/image_png_simple.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.docx");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert().success().code(0);

    // Verify the output file was created and is a valid ZIP/DOCX
    assert!(output.exists(), "DOCX file should be created");
    let bytes = fs::read(&output).unwrap();
    assert_eq!(bytes[0], 0x50, "DOCX should be a ZIP file (PK signature)");
    assert_eq!(bytes[1], 0x4B, "DOCX should be a ZIP file (PK signature)");
}

/// Test that PNG image converts successfully to HTML with data URI.
#[test]
fn cli_image_png_to_html() {
    let fixture = project_root().join("fixtures/image_png_simple.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.html");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert().success().code(0);

    // Verify the output file was created
    assert!(output.exists(), "HTML file should be created");
    let html = fs::read_to_string(&output).unwrap();

    // Verify HTML structure
    assert!(html.contains("<!doctype html>"), "HTML should have doctype");

    // Verify image is embedded as data URI with PNG MIME type
    assert!(
        html.contains("data:image/png;base64,"),
        "HTML should contain PNG data URI"
    );
    assert!(html.contains("<img"), "HTML should contain img element");
}

/// Test that JPEG image converts successfully to DOCX.
#[test]
fn cli_image_jpeg_to_docx() {
    let fixture = project_root().join("fixtures/image_jpeg_simple.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.docx");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert().success().code(0);

    // Verify the output file was created and is a valid ZIP/DOCX
    assert!(output.exists(), "DOCX file should be created");
    let bytes = fs::read(&output).unwrap();
    assert_eq!(bytes[0], 0x50, "DOCX should be a ZIP file (PK signature)");
    assert_eq!(bytes[1], 0x4B, "DOCX should be a ZIP file (PK signature)");
}

/// Test that JPEG image converts successfully to HTML with data URI.
#[test]
fn cli_image_jpeg_to_html() {
    let fixture = project_root().join("fixtures/image_jpeg_simple.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.html");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert().success().code(0);

    // Verify the output file was created
    assert!(output.exists(), "HTML file should be created");
    let html = fs::read_to_string(&output).unwrap();

    // Verify HTML structure
    assert!(html.contains("<!doctype html>"), "HTML should have doctype");

    // Verify image is embedded as data URI with JPEG MIME type
    assert!(
        html.contains("data:image/jpeg;base64,"),
        "HTML should contain JPEG data URI"
    );
    assert!(html.contains("<img"), "HTML should contain img element");
}

/// Test that document with multiple images converts successfully to DOCX.
#[test]
fn cli_image_multiple_to_docx() {
    let fixture = project_root().join("fixtures/image_multiple.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.docx");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert().success().code(0);

    // Verify the output file was created and is a valid ZIP/DOCX
    assert!(output.exists(), "DOCX file should be created");
    let bytes = fs::read(&output).unwrap();
    assert_eq!(bytes[0], 0x50, "DOCX should be a ZIP file (PK signature)");
    assert_eq!(bytes[1], 0x4B, "DOCX should be a ZIP file (PK signature)");
}

/// Test that image converts to IR JSON with correct ImageBlock structure.
#[test]
fn cli_image_to_ir() {
    let fixture = project_root().join("fixtures/image_png_simple.rtf");

    let dir = tempdir().unwrap();
    let ir_output = dir.path().join("ir.json");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        ir_output.to_str().unwrap(),
    ]);

    cmd.assert().success().code(0);

    // Verify IR file was created
    assert!(ir_output.exists(), "IR JSON file should be created");

    // Parse and verify IR structure
    let json = fs::read_to_string(&ir_output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("IR should be valid JSON");

    // Check for blocks array
    let blocks = parsed
        .get("blocks")
        .expect("IR should have blocks field")
        .as_array()
        .expect("blocks should be an array");

    // Find an image block (type is "imageblock")
    let image_block = blocks
        .iter()
        .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("imageblock"))
        .expect("IR should contain an imageblock block");

    // Verify image block structure
    assert!(
        image_block.get("format").is_some(),
        "Image block should have format field"
    );
    assert!(
        image_block.get("data").is_some(),
        "Image block should have data field"
    );

    // Verify format is PNG
    let format = image_block
        .get("format")
        .and_then(|f| f.as_str())
        .expect("format should be a string");
    assert_eq!(format.to_lowercase(), "png", "Image format should be PNG");

    // Verify data is present (data is a byte array in JSON)
    let data = image_block
        .get("data")
        .and_then(|d| d.as_array())
        .expect("data should be an array");
    assert!(!data.is_empty(), "Image data should not be empty");

    // Verify the data starts with PNG signature (137, 80, 78, 71, 13, 10, 26, 10)
    let first_bytes: Vec<i64> = data.iter().take(8).filter_map(|v| v.as_i64()).collect();
    assert_eq!(
        first_bytes,
        vec![137, 80, 78, 71, 13, 10, 26, 10],
        "Image data should start with PNG signature"
    );
}

/// Test that image converts successfully to PDF.
#[test]
fn cli_image_to_pdf() {
    let fixture = project_root().join("fixtures/image_png_simple.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.pdf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "pdf",
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert().success().code(0);

    assert!(output.exists(), "PDF file should be created");
    let bytes = fs::read(&output).unwrap();
    assert!(
        bytes.starts_with(b"%PDF"),
        "Output should be a valid PDF file"
    );
}

// -------------------------------------------------------------------------
// Strict Mode Tests for Images
// -------------------------------------------------------------------------

/// Test that valid PNG image passes strict mode.
#[test]
fn strict_mode_valid_png_image_succeeds() {
    let fixture = project_root().join("fixtures/image_png_simple.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);

    // Should succeed - valid PNG image should pass strict mode
    cmd.assert().success().code(0);
}

/// Test that valid JPEG image passes strict mode.
#[test]
fn strict_mode_valid_jpeg_image_succeeds() {
    let fixture = project_root().join("fixtures/image_jpeg_simple.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);

    // Should succeed - valid JPEG image should pass strict mode
    cmd.assert().success().code(0);
}

/// Test that unsupported image format fails strict mode with exit code 4.
#[test]
fn strict_mode_unsupported_image_format_fails() {
    let fixture = project_root().join("fixtures/image_unsupported_format.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);

    // Should fail with exit code 4 (strict mode violation due to dropped content)
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

/// Test that malformed hex in image fails strict mode with exit code 4.
#[test]
fn strict_mode_malformed_image_hex_fails() {
    let fixture = project_root().join("fixtures/image_malformed_hex.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);

    // Should fail with exit code 4 (strict mode violation due to dropped content)
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

/// Test that malformed image payload fails strict mode for PDF output.
#[test]
fn strict_mode_malformed_image_hex_pdf_fails() {
    let fixture = project_root().join("fixtures/image_malformed_hex.rtf");
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.pdf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--to",
        "pdf",
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

/// Test that multiple valid images pass strict mode.
#[test]
fn strict_mode_multiple_images_succeeds() {
    let fixture = project_root().join("fixtures/image_multiple.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);

    // Should succeed - all images are valid PNG/JPEG
    cmd.assert().success().code(0);
}

// -------------------------------------------------------------------------
// Warning Verification Tests
// -------------------------------------------------------------------------

/// Test that unsupported image format generates warning but succeeds.
#[test]
fn image_unsupported_format_warning() {
    let fixture = project_root().join("fixtures/image_unsupported_format.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with exit code 0 (success with warnings)
    cmd.assert().success().code(0);

    // Verify warning is present in output
    let output = cmd.assert().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Should contain dropped_content warning
    assert!(
        output_str.contains("dropped_content") || output_str.contains("warnings"),
        "Output should contain dropped_content warning or warnings field"
    );
}

/// Test that malformed hex in image generates warning but succeeds.
#[test]
fn image_malformed_hex_warning() {
    let fixture = project_root().join("fixtures/image_malformed_hex.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with exit code 0 (success with warnings)
    cmd.assert().success().code(0);

    // Verify warning is present in output
    let output = cmd.assert().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Should contain dropped_content warning
    assert!(
        output_str.contains("dropped_content") || output_str.contains("warnings"),
        "Output should contain dropped_content warning or warnings field"
    );
}

/// Test that unsupported format warning has stable reason string.
#[test]
fn image_unsupported_format_warning_has_stable_reason() {
    let fixture = project_root().join("fixtures/image_unsupported_format.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Check for stable reason string (from PLAN_IMAGES.md section 6.2)
    assert!(
        output_str.contains("unsupported image format")
            || output_str.contains("unsupported_image_format")
            || output_str.contains("unsupported format"),
        "Warning should mention unsupported image format"
    );
}

/// Test that malformed hex warning has stable reason string.
#[test]
fn image_malformed_hex_warning_has_stable_reason() {
    let fixture = project_root().join("fixtures/image_malformed_hex.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Check for stable reason string (from PLAN_IMAGES.md section 6.2)
    assert!(
        output_str.contains("malformed")
            || output_str.contains("hex")
            || output_str.contains("decode"),
        "Warning should mention malformed hex or decode issue"
    );
}

// -------------------------------------------------------------------------
// Determinism Tests for Images
// -------------------------------------------------------------------------

/// Test that image output is deterministic for PNG.
#[test]
fn image_png_output_is_deterministic() {
    let fixture = project_root().join("fixtures/image_png_simple.rtf");

    let dir1 = tempdir().unwrap();
    let output1 = dir1.path().join("output1.json");

    let dir2 = tempdir().unwrap();
    let output2 = dir2.path().join("output2.json");

    // First conversion
    let mut cmd1 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd1.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        output1.to_str().unwrap(),
    ]);
    cmd1.assert().success();

    // Second conversion
    let mut cmd2 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd2.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        output2.to_str().unwrap(),
    ]);
    cmd2.assert().success();

    // Compare outputs
    let json1 = fs::read_to_string(&output1).unwrap();
    let json2 = fs::read_to_string(&output2).unwrap();

    // Parse and normalize (remove stats.duration_ms)
    let mut parsed1: serde_json::Value = serde_json::from_str(&json1).expect("JSON should parse");
    let mut parsed2: serde_json::Value = serde_json::from_str(&json2).expect("JSON should parse");

    if let Some(stats) = parsed1.get_mut("stats").and_then(|v| v.as_object_mut()) {
        stats.insert("duration_ms".to_string(), serde_json::json!(0));
    }
    if let Some(stats) = parsed2.get_mut("stats").and_then(|v| v.as_object_mut()) {
        stats.insert("duration_ms".to_string(), serde_json::json!(0));
    }

    assert_eq!(parsed1, parsed2, "Image output should be deterministic");
}

/// Test that image output is deterministic for JPEG.
#[test]
fn image_jpeg_output_is_deterministic() {
    let fixture = project_root().join("fixtures/image_jpeg_simple.rtf");

    let dir1 = tempdir().unwrap();
    let output1 = dir1.path().join("output1.json");

    let dir2 = tempdir().unwrap();
    let output2 = dir2.path().join("output2.json");

    // First conversion
    let mut cmd1 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd1.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        output1.to_str().unwrap(),
    ]);
    cmd1.assert().success();

    // Second conversion
    let mut cmd2 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd2.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        output2.to_str().unwrap(),
    ]);
    cmd2.assert().success();

    // Compare outputs
    let json1 = fs::read_to_string(&output1).unwrap();
    let json2 = fs::read_to_string(&output2).unwrap();

    // Parse and normalize
    let mut parsed1: serde_json::Value = serde_json::from_str(&json1).expect("JSON should parse");
    let mut parsed2: serde_json::Value = serde_json::from_str(&json2).expect("JSON should parse");

    if let Some(stats) = parsed1.get_mut("stats").and_then(|v| v.as_object_mut()) {
        stats.insert("duration_ms".to_string(), serde_json::json!(0));
    }
    if let Some(stats) = parsed2.get_mut("stats").and_then(|v| v.as_object_mut()) {
        stats.insert("duration_ms".to_string(), serde_json::json!(0));
    }

    assert_eq!(
        parsed1, parsed2,
        "JPEG image output should be deterministic"
    );
}

/// Test that multiple images output is deterministic.
#[test]
fn image_multiple_output_is_deterministic() {
    let fixture = project_root().join("fixtures/image_multiple.rtf");

    let dir1 = tempdir().unwrap();
    let output1 = dir1.path().join("output1.json");

    let dir2 = tempdir().unwrap();
    let output2 = dir2.path().join("output2.json");

    // First conversion
    let mut cmd1 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd1.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        output1.to_str().unwrap(),
    ]);
    cmd1.assert().success();

    // Second conversion
    let mut cmd2 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd2.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        output2.to_str().unwrap(),
    ]);
    cmd2.assert().success();

    // Compare outputs
    let json1 = fs::read_to_string(&output1).unwrap();
    let json2 = fs::read_to_string(&output2).unwrap();

    // Parse and normalize
    let mut parsed1: serde_json::Value = serde_json::from_str(&json1).expect("JSON should parse");
    let mut parsed2: serde_json::Value = serde_json::from_str(&json2).expect("JSON should parse");

    if let Some(stats) = parsed1.get_mut("stats").and_then(|v| v.as_object_mut()) {
        stats.insert("duration_ms".to_string(), serde_json::json!(0));
    }
    if let Some(stats) = parsed2.get_mut("stats").and_then(|v| v.as_object_mut()) {
        stats.insert("duration_ms".to_string(), serde_json::json!(0));
    }

    assert_eq!(
        parsed1, parsed2,
        "Multiple images output should be deterministic"
    );
}

// -------------------------------------------------------------------------
// shppict/nonshppict Preference Tests
// -------------------------------------------------------------------------

/// Test that shppict is preferred over nonshppict for the same shape-picture context.
#[test]
fn shppict_preferred_over_nonshppict() {
    let fixture = project_root().join("fixtures/image_shppict_nonshppict.rtf");

    let dir = tempdir().unwrap();
    let ir_output = dir.path().join("ir.json");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        ir_output.to_str().unwrap(),
    ]);

    cmd.assert().success();

    // Parse IR and verify we have image blocks
    let json = fs::read_to_string(&ir_output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("IR should be valid JSON");

    let blocks = parsed
        .get("blocks")
        .and_then(|b| b.as_array())
        .expect("Should have blocks array");

    // Count image blocks
    let image_count = blocks
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("imageblock"))
        .count();

    // Preferred behavior: only shppict is emitted.
    assert_eq!(
        image_count, 1,
        "Should have one preferred image (shppict), not both branches"
    );

    // Verify emitted image is PNG (from shppict)
    let image = blocks
        .iter()
        .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("imageblock"))
        .expect("Should have at least one image");
    let format = image
        .get("format")
        .and_then(|f| f.as_str())
        .expect("Should have format");
    assert_eq!(
        format.to_lowercase(),
        "png",
        "Preferred shppict image should be PNG"
    );
}
