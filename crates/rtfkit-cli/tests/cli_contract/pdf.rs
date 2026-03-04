use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

/// Test that PDF output succeeds on a simple fixture.
#[test]
fn pdf_output_succeeds_on_simple_fixture() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("test.pdf");

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

    // Verify the output file was created
    assert!(output.exists(), "PDF file should be created");

    // Verify it's a valid PDF file (PDF files start with %PDF)
    let bytes = fs::read(&output).unwrap();
    assert!(bytes.len() > 4, "PDF file should have content");
    assert!(
        bytes.starts_with(b"%PDF"),
        "Output should be a valid PDF file (starts with %PDF)"
    );
}

/// Test that PDF output succeeds on a table fixture.
#[test]
fn pdf_output_succeeds_on_table_fixture() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_simple_2x2.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("table.pdf");

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
    assert!(output.exists(), "PDF file should be created");
}

/// Test that PDF output succeeds on a list fixture.
#[test]
fn pdf_output_succeeds_on_list_fixture() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/list_bullet_simple.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("list.pdf");

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
    assert!(output.exists(), "PDF file should be created");
}

/// Test that PDF output succeeds on mixed content.
#[test]
fn pdf_output_succeeds_on_mixed_content() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/mixed_complex.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("mixed.pdf");

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
    assert!(output.exists(), "PDF file should be created");
}

/// Test that PDF-specific flags are rejected for non-PDF targets.
#[test]
fn pdf_specific_flags_rejected_for_non_pdf_target() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.html");

    // --pdf-page-size should be rejected for HTML output
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "html",
        "--pdf-page-size",
        "a4",
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("only valid with --to pdf"));
}

/// Test that --fixed-timestamp is rejected for non-PDF output.
#[test]
fn fixed_timestamp_rejected_for_non_pdf_target() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.html");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "html",
        "--fixed-timestamp",
        "2024-01-01T00:00:00Z",
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("only valid with --to pdf"));
}

/// Test that explicit --to conflicting with output extension returns parse error.
#[test]
fn explicit_target_conflict_with_output_extension_fails() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("output.pdf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        output.to_str().unwrap(),
    ]);

    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("target format mismatch"));
}

/// Test that PDF output refuses to overwrite without --force.
#[test]
fn pdf_output_refuses_overwrite_without_force() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("out.pdf");

    // First conversion should succeed
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

    // Second conversion should fail without --force
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "pdf",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert()
        .failure()
        .code(3)
        .stderr(contains("Output file already exists"));

    // With --force, it should succeed
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "pdf",
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);
    cmd.assert().success();
}

/// Test that PDF output with --pdf-page-size succeeds.
#[test]
fn pdf_output_with_page_size_succeeds() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("test.pdf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "pdf",
        "-o",
        output.to_str().unwrap(),
        "--pdf-page-size",
        "letter",
    ]);

    cmd.assert().success();
    assert!(output.exists(), "PDF file should be created");
}

/// Test that PDF output with --fixed-timestamp succeeds.
#[test]
fn pdf_output_with_fixed_timestamp_succeeds() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("test.pdf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "pdf",
        "-o",
        output.to_str().unwrap(),
        "--fixed-timestamp",
        "2024-01-01T00:00:00Z",
    ]);

    cmd.assert().success();
    assert!(output.exists(), "PDF file should be created");
}

/// Test that strict mode does not fail for non-semantic PDF partial support.
#[test]
fn pdf_strict_mode_ignores_non_semantic_partial_support() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("font_size_only.rtf");
    let output = dir.path().join("out.pdf");

    // Font size is currently degraded by PDF mapper but should not trip strict mode.
    fs::write(&input, r#"{\rtf1\ansi\fs24 Hello\par}"#).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--to",
        "pdf",
        "--strict",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success();
}

/// Test that invalid --fixed-timestamp is rejected.
#[test]
fn invalid_fixed_timestamp_is_rejected() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("test.pdf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--to",
        "pdf",
        "-o",
        output.to_str().unwrap(),
        "--fixed-timestamp",
        "not-a-timestamp",
    ]);

    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("Invalid timestamp"));
}

/// Test that empty document produces valid PDF.
#[test]
fn pdf_output_empty_document_succeeds() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_empty.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("empty.pdf");

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
    assert!(
        output.exists(),
        "PDF file should be created for empty document"
    );
}

/// Test that PDF output with unicode content succeeds.
#[test]
fn pdf_output_unicode_succeeds() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_unicode.rtf");

    let dir = tempdir().unwrap();
    let output = dir.path().join("unicode.pdf");

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
    assert!(output.exists(), "PDF file should be created");
}
