use std::fs;

use tempfile::tempdir;

/// Test that text highlight fixture passes strict mode without warnings.
#[test]
fn text_highlight_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_highlight.rtf");

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

/// Test that text background (\cb) fixture passes strict mode without warnings.
#[test]
fn text_background_cb_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_background_cb.rtf");

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

/// Test that highlight/background precedence fixture passes strict mode.
#[test]
fn text_highlight_background_precedence_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_highlight_background_precedence.rtf");

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

/// Test that \plain reset fixture for background passes strict mode.
#[test]
fn text_background_plain_reset_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_background_plain_reset.rtf");

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

/// Test that unresolved highlight index degrades gracefully (no DroppedContent).
#[test]
fn unresolved_highlight_index_degrades_gracefully() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("unresolved_highlight.rtf");
    // \highlight999 references a non-existent color - should degrade gracefully
    fs::write(
            &input,
            r#"{\rtf1\ansi{\colortbl;\red0\green0\blue0;}\highlight999 Text with unresolved highlight}"#,
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
    // Should succeed - unresolved highlight is not a semantic loss
    cmd.assert().success();
}

/// Test that unresolved background index degrades gracefully (no DroppedContent).
#[test]
fn unresolved_background_index_degrades_gracefully() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("unresolved_background.rtf");
    // \cb999 references a non-existent color - should degrade gracefully
    fs::write(
        &input,
        r#"{\rtf1\ansi{\colortbl;\red0\green0\blue0;}\cb999 Text with unresolved background}"#,
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
    // Should succeed - unresolved background is not a semantic loss
    cmd.assert().success();
}

/// Test that highlight output contains expected RGB values in IR.
#[test]
fn highlight_output_contains_rgb_values() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_highlight.rtf");

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

    // Read IR JSON and verify background_color is present
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify background_color field is present with RGB values
    assert!(
        output_str.contains("background_color"),
        "Output should contain background_color field"
    );
    assert!(
        output_str.contains("\"r\": 255"),
        "Output should contain red color component"
    );
    assert!(
        output_str.contains("\"g\": 255"),
        "Output should contain green color component for yellow"
    );
}

/// Test that background (\cb) output contains expected RGB values in IR.
#[test]
fn background_cb_output_contains_rgb_values() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_background_cb.rtf");

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

    // Read IR JSON and verify background_color is present
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify background_color field is present
    assert!(
        output_str.contains("background_color"),
        "Output should contain background_color field"
    );
}

/// Test that highlight takes precedence over \cb in IR.
#[test]
fn highlight_takes_precedence_over_cb() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_highlight_background_precedence.rtf");

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

    // Read IR JSON
    let output_str = fs::read_to_string(&ir_output).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");

    // Check that we have inlines with background_color
    let blocks = parsed.get("blocks").unwrap().as_array().unwrap();
    let paragraph = &blocks[0];
    let inlines = paragraph.get("inlines").unwrap().as_array().unwrap();

    // Verify run-level background colors exactly, in fixture order:
    // 1) CB only => (200, 200, 255)
    // 2) highlight + cb => highlight wins => (255, 255, 0)
    // 3) cb changed while highlight still set => highlight still wins => (255, 255, 0)
    // 4) highlight reset to 0 => fallback to cb => (0, 255, 255)
    let run_backgrounds: Vec<Option<(u64, u64, u64)>> = inlines
        .iter()
        .filter(|inline| inline.get("type").and_then(|t| t.as_str()) == Some("run"))
        .map(|inline| {
            inline.get("background_color").map(|bg| {
                (
                    bg.get("r").unwrap().as_u64().unwrap(),
                    bg.get("g").unwrap().as_u64().unwrap(),
                    bg.get("b").unwrap().as_u64().unwrap(),
                )
            })
        })
        .collect();

    assert!(
        run_backgrounds
            == vec![
                Some((200, 200, 255)),
                Some((255, 255, 0)),
                Some((255, 255, 0)),
                Some((0, 255, 255))
            ],
        "Unexpected background-color precedence behavior: {run_backgrounds:?}"
    );
}

/// Test that \plain resets background/highlight but preserves paragraph alignment.
#[test]
fn plain_reset_clears_background_preserves_alignment() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("plain_background.rtf");
    // Create RTF with center alignment and background, then apply \plain
    fs::write(
            &input,
            r#"{\rtf1\ansi{\colortbl;\red255\green255\blue0;}\qc\highlight1 Centered with highlight.\plain Still centered, no highlight.}"#,
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

    // Read IR JSON and verify alignment persists and background is reset after \plain
    let output_str = fs::read_to_string(&ir_output).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");

    let blocks = parsed.get("blocks").unwrap().as_array().unwrap();
    let paragraph = &blocks[0];

    // Verify alignment is still centered after \plain.
    assert!(
        paragraph.get("alignment").and_then(|a| a.as_str()) == Some("center"),
        "Alignment should remain centered after \\plain"
    );

    let inlines = paragraph.get("inlines").unwrap().as_array().unwrap();
    let run_backgrounds: Vec<Option<(u64, u64, u64)>> = inlines
        .iter()
        .filter(|inline| inline.get("type").and_then(|t| t.as_str()) == Some("run"))
        .map(|inline| {
            inline.get("background_color").map(|bg| {
                (
                    bg.get("r").unwrap().as_u64().unwrap(),
                    bg.get("g").unwrap().as_u64().unwrap(),
                    bg.get("b").unwrap().as_u64().unwrap(),
                )
            })
        })
        .collect();

    assert!(
        run_backgrounds == vec![Some((255, 255, 0)), None],
        "Expected \\plain to clear background on the second run: {run_backgrounds:?}"
    );
}

/// Test that background/highlight fixtures produce valid DOCX output.
#[test]
fn background_color_fixtures_produce_valid_docx() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_highlight.rtf");

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

/// Test that background/highlight fixtures produce valid HTML output with background-color.
#[test]
fn background_color_fixtures_produce_valid_html() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_highlight.rtf");

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
    // Verify background-color style is present
    assert!(
        html.contains("background-color"),
        "HTML should contain background-color style"
    );
}

/// Test that background/highlight fixtures produce valid PDF output.
#[test]
fn background_color_fixtures_produce_valid_pdf() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_highlight.rtf");

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
    cmd.assert().success();

    // Verify the output file was created and is a valid PDF
    assert!(output.exists(), "PDF file should be created");
    let bytes = fs::read(&output).unwrap();
    assert!(
        bytes.starts_with(b"%PDF"),
        "Output should be a valid PDF file"
    );
}
