use std::fs;

use tempfile::tempdir;

/// Test that paragraph shading fixture passes strict mode without warnings.
#[test]
fn paragraph_shading_basic_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/paragraph_shading_basic.rtf");

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

/// Test that table cell shading fixture passes strict mode without warnings.
#[test]
fn table_cell_shading_basic_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_cell_shading_basic.rtf");

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

/// Test that shading precedence fixture passes strict mode.
#[test]
fn table_row_cell_shading_precedence_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_row_cell_shading_precedence.rtf");

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

/// Test that \pard/\plain reset fixture passes strict mode.
#[test]
fn paragraph_shading_plain_pard_reset_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/paragraph_shading_plain_pard_reset.rtf");

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

/// Test that shading pattern fixture passes strict mode.
#[test]
fn shading_pattern_basic_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/shading_pattern_basic.rtf");

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

/// Test that percent shading steps fixture passes strict mode.
#[test]
fn shading_percent_steps_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/shading_percent_steps.rtf");

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

/// Test that theme color shading fixture passes strict mode.
#[test]
fn shading_theme_color_reference_fixture_passes_strict_mode() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/shading_theme_color_reference.rtf");

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

/// Test that unresolved shading color index degrades gracefully (no DroppedContent).
#[test]
fn unresolved_shading_color_index_degrades_gracefully() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("unresolved_shading.rtf");
    // \cbpat999 references a non-existent color - should degrade gracefully
    fs::write(
            &input,
            r#"{\rtf1\ansi{\colortbl;\red0\green0\blue0;}\cbpat999 Paragraph with unresolved shading color\par}"#,
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
    // Should succeed - unresolved shading color is not a semantic loss
    cmd.assert().success();
}

/// Test that paragraph shading output contains expected RGB values in IR.
#[test]
fn paragraph_shading_output_contains_rgb_values() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/paragraph_shading_basic.rtf");

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

    // Read IR JSON and verify shading is present
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify shading field is present with fill_color
    assert!(
        output_str.contains("shading"),
        "Output should contain shading field"
    );
    assert!(
        output_str.contains("fill_color"),
        "Output should contain fill_color field in shading"
    );
}

/// Test that table cell shading output contains expected RGB values in IR.
#[test]
fn table_cell_shading_output_contains_rgb_values() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_cell_shading_basic.rtf");

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

    // Read IR JSON and verify cell shading is present
    let output_str = fs::read_to_string(&ir_output).unwrap();

    // Verify shading field is present in cells
    assert!(
        output_str.contains("shading"),
        "Output should contain shading field in cells"
    );
}

/// Test that \pard resets paragraph shading but \plain does not.
#[test]
fn pard_resets_shading_plain_does_not() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/paragraph_shading_plain_pard_reset.rtf");

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

    let blocks = parsed.get("blocks").unwrap().as_array().unwrap();

    // First paragraph should have shading
    let para1 = &blocks[0];
    assert!(
        para1.get("shading").is_some(),
        "First paragraph should have shading"
    );

    // Second paragraph (after \pard) should NOT have shading
    let para2 = &blocks[1];
    assert!(
        para2.get("shading").is_none(),
        "Second paragraph (after \\pard) should not have shading"
    );

    // Third paragraph should have shading again
    let para3 = &blocks[2];
    assert!(
        para3.get("shading").is_some(),
        "Third paragraph should have shading"
    );

    // Fourth paragraph (after \plain) should STILL have shading
    let para4 = &blocks[3];
    assert!(
        para4.get("shading").is_some(),
        "Fourth paragraph (after \\plain) should still have shading"
    );
}

/// Test that cell shading takes precedence over row/table shading.
#[test]
fn cell_shading_precedence_over_row_table() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_row_cell_shading_precedence.rtf");

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

    // Read IR JSON and verify precedence
    let output_str = fs::read_to_string(&ir_output).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");

    let blocks = parsed.get("blocks").unwrap().as_array().unwrap();
    assert_eq!(blocks.len(), 1, "Fixture should produce one table block");
    let table = &blocks[0];
    let rows = table.get("rows").unwrap().as_array().unwrap();
    assert_eq!(rows.len(), 3, "Fixture should contain three rows");

    // Row 1: explicit cell shading for first two cells, row/table fallback for third.
    let row1 = &rows[0];
    let cells1 = row1.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells1.len(), 3);

    let fill1 = cells1[0]
        .get("shading")
        .and_then(|s| s.get("fill_color"))
        .unwrap();
    assert_eq!(fill1.get("r").unwrap().as_u64().unwrap(), 255);
    assert_eq!(fill1.get("g").unwrap().as_u64().unwrap(), 0);
    assert_eq!(fill1.get("b").unwrap().as_u64().unwrap(), 0);

    let fill2 = cells1[1]
        .get("shading")
        .and_then(|s| s.get("fill_color"))
        .unwrap();
    assert_eq!(fill2.get("r").unwrap().as_u64().unwrap(), 0);
    assert_eq!(fill2.get("g").unwrap().as_u64().unwrap(), 128);
    assert_eq!(fill2.get("b").unwrap().as_u64().unwrap(), 0);

    let fill3 = cells1[2]
        .get("shading")
        .and_then(|s| s.get("fill_color"))
        .unwrap();
    assert_eq!(fill3.get("r").unwrap().as_u64().unwrap(), 255);
    assert_eq!(fill3.get("g").unwrap().as_u64().unwrap(), 255);
    assert_eq!(fill3.get("b").unwrap().as_u64().unwrap(), 0);

    // Row 2: table fallback only (yellow).
    let row2 = &rows[1];
    let cells2 = row2.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells2.len(), 3);
    for cell in cells2 {
        let fill = cell
            .get("shading")
            .and_then(|s| s.get("fill_color"))
            .unwrap();
        assert_eq!(fill.get("r").unwrap().as_u64().unwrap(), 255);
        assert_eq!(fill.get("g").unwrap().as_u64().unwrap(), 255);
        assert_eq!(fill.get("b").unwrap().as_u64().unwrap(), 0);
    }

    // Row 3: explicit row shading (green) overrides table fallback.
    let row3 = &rows[2];
    let cells3 = row3.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells3.len(), 3);
    for cell in cells3 {
        let fill = cell
            .get("shading")
            .and_then(|s| s.get("fill_color"))
            .unwrap();
        assert_eq!(fill.get("r").unwrap().as_u64().unwrap(), 0);
        assert_eq!(fill.get("g").unwrap().as_u64().unwrap(), 128);
        assert_eq!(fill.get("b").unwrap().as_u64().unwrap(), 0);
    }
}

/// Test that shading fixtures produce valid DOCX output.
#[test]
fn shading_fixtures_produce_valid_docx() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/paragraph_shading_basic.rtf");

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

/// Test that shading fixtures produce valid HTML output with background-color.
#[test]
fn shading_fixtures_produce_valid_html() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/paragraph_shading_basic.rtf");

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

/// Test that shading fixtures produce valid PDF output.
#[test]
fn shading_fixtures_produce_valid_pdf() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/paragraph_shading_basic.rtf");

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

/// Test that percent shading patterns stay distinct in IR and HTML outputs.
#[test]
fn shading_percent_steps_distinct_in_ir_and_html() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/shading_percent_steps.rtf");

    let dir = tempdir().unwrap();
    let ir_output = dir.path().join("steps.ir.json");
    let html_output = dir.path().join("steps.html");

    let mut ir_cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    ir_cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        ir_output.to_str().unwrap(),
    ]);
    ir_cmd.assert().success();

    let mut html_cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    html_cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        html_output.to_str().unwrap(),
        "--force",
    ]);
    html_cmd.assert().success();

    let ir = fs::read_to_string(ir_output).unwrap();
    assert!(ir.contains("\"pattern\": \"percent25\""));
    assert!(ir.contains("\"pattern\": \"percent50\""));
    assert!(ir.contains("\"pattern\": \"percent75\""));

    let html = fs::read_to_string(html_output).unwrap();
    assert!(html.contains("background-color: #bfbfbf"));
    assert!(html.contains("background-color: #808080"));
    assert!(html.contains("background-color: #404040"));
}
