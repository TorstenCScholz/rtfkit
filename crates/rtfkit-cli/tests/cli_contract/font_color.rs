use std::fs;

use tempfile::tempdir;

/// Test that font family fixture passes strict mode without warnings.
#[test]
fn font_family_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_font_family.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

/// Test that font size fixture passes strict mode without warnings.
#[test]
fn font_size_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_font_size.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

/// Test that color fixture passes strict mode without warnings.
#[test]
fn color_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_color.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

/// Test that combined font/color fixture passes strict mode without warnings.
#[test]
fn font_color_combined_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_font_color_combined.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

/// Test that \plain reset fixture passes strict mode without warnings.
#[test]
fn plain_reset_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_plain_reset.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

/// Test that \deff default font fixture passes strict mode without warnings.
#[test]
fn default_font_deff_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_default_font_deff.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

/// Test that unresolved font index degrades gracefully (no DroppedContent).
#[test]
fn unresolved_font_index_degrades_gracefully() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("unresolved_font.rtf");
    // \f999 references a non-existent font - should degrade gracefully
    fs::write(
        &input,
        r#"{\rtf1\ansi{\fonttbl{\f0\fnil Arial;}}\f999 Text with unresolved font}"#,
    )
    .unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    // Should succeed - unresolved font is not a semantic loss
    cmd.assert().success();
}

/// Test that unresolved color index degrades gracefully (no DroppedContent).
#[test]
fn unresolved_color_index_degrades_gracefully() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("unresolved_color.rtf");
    // \cf999 references a non-existent color - should degrade gracefully
    fs::write(
        &input,
        r#"{\rtf1\ansi{\colortbl;\red0\green0\blue0;}\cf999 Text with unresolved color}"#,
    )
    .unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    // Should succeed - unresolved color is not a semantic loss
    cmd.assert().success();
}

/// Test that font family output contains expected font names in IR.
#[test]
fn font_family_output_contains_font_names() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_font_family.rtf");

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

    // Read IR JSON and verify font names are present
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify font names are present in output
    assert!(
        output_str.contains("Arial"),
        "Output should contain 'Arial' font"
    );
    assert!(
        output_str.contains("Helvetica"),
        "Output should contain 'Helvetica' font"
    );
    assert!(
        output_str.contains("Times New Roman"),
        "Output should contain 'Times New Roman' font"
    );
}

/// Test that color output contains expected RGB values in IR.
#[test]
fn color_output_contains_rgb_values() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_color.rtf");

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

    // Read IR JSON and verify RGB values are present
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify RGB values are present in output
    assert!(
        output_str.contains("\"r\": 255"),
        "Output should contain red color component"
    );
    assert!(
        output_str.contains("\"g\": 128"),
        "Output should contain green color component"
    );
    assert!(
        output_str.contains("\"b\": 255"),
        "Output should contain blue color component"
    );
}

/// Test that font size output contains expected sizes in IR.
#[test]
fn font_size_output_contains_sizes() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_font_size.rtf");

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

    // Read IR JSON and verify font sizes are present
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify font sizes are present in output (sizes are in half-points)
    // fs12 = 6pt, fs24 = 12pt, fs48 = 24pt, fs72 = 36pt
    assert!(
        output_str.contains("\"font_size\": 6"),
        "Output should contain 6pt font size"
    );
    assert!(
        output_str.contains("\"font_size\": 12"),
        "Output should contain 12pt font size"
    );
    assert!(
        output_str.contains("\"font_size\": 24"),
        "Output should contain 24pt font size"
    );
    assert!(
        output_str.contains("\"font_size\": 36"),
        "Output should contain 36pt font size"
    );
}

/// Test that \plain resets character formatting but preserves paragraph alignment.
#[test]
fn plain_reset_preserves_paragraph_alignment() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("plain_alignment.rtf");
    // Create RTF with center alignment, then apply \plain - alignment should persist
    fs::write(
        &input,
        r#"{\rtf1\ansi\qc\b\i Bold italic centered.\plain Still centered but not bold or italic.}"#,
    )
    .unwrap();

    let ir_output = dir.path().join("ir.json");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--emit-ir",
        ir_output.to_str().unwrap(),
    ]);

    cmd.assert().success();

    // Read IR JSON and verify alignment is still centered after \plain
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify alignment is still centered after \plain
    assert!(
        output_str.contains("\"alignment\": \"center\""),
        "Alignment should remain centered after \\plain"
    );
}

/// Test that font/color fixtures produce valid DOCX output.
#[test]
fn font_color_fixtures_produce_valid_docx() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_font_color_combined.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.docx");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success();

    // Verify the output file was created and is a valid ZIP/DOCX
    assert!(output.exists(), "DOCX file should be created");
    let bytes = fs::read(&output).unwrap();
    assert_eq!(bytes[0], 0x50, "DOCX should be a ZIP file (PK signature)");
    assert_eq!(bytes[1], 0x4B, "DOCX should be a ZIP file (PK signature)");
}

/// Test that font/color fixtures produce valid HTML output.
#[test]
fn font_color_fixtures_produce_valid_html() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_font_color_combined.rtf");

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
    cmd.assert().success();

    let html = fs::read_to_string(&output).unwrap();
    assert!(html.contains("<!doctype html>"));
    // Verify font-family and color styles are present
    assert!(
        html.contains("font-family"),
        "HTML should contain font-family styles"
    );
    assert!(html.contains("color:"), "HTML should contain color styles");
}
