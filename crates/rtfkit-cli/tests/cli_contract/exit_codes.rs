use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

// -------------------------------------------------------------------------
// Exit Code 0: Success Tests
// -------------------------------------------------------------------------

#[test]
fn simple_text_conversion_returns_success() {
    // Well-formed RTF with simple text should succeed with exit code 0
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn complex_document_conversion_returns_success() {
    // Complex RTF with mixed content should succeed with exit code 0
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/mixed_complex.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn table_conversion_returns_success() {
    // Well-formed table should succeed with exit code 0
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_simple_2x2.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn nested_table_conversion_returns_success() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_nested_2level_basic.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn list_conversion_returns_success() {
    // Well-formed list should succeed with exit code 0
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/list_bullet_simple.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn nested_list_conversion_returns_success() {
    // Nested lists should succeed with exit code 0
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/list_nested_two_levels.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn unicode_text_conversion_returns_success() {
    // Unicode content should succeed with exit code 0
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_unicode.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn empty_document_conversion_returns_success() {
    // Empty RTF should succeed with exit code 0
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_empty.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().code(0);
}

#[test]
fn docx_output_returns_success() {
    // DOCX output should succeed with exit code 0
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.rtf");
    let output = dir.path().join("out.docx");
    fs::write(&input, r#"{\rtf1\ansi Hello\par}"#).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success().code(0);

    // Verify the output file was created
    assert!(output.exists(), "Output DOCX file should be created");

    // Verify it's a valid ZIP/DOCX file (DOCX files start with PK signature)
    let bytes = fs::read(&output).unwrap();
    assert!(bytes.len() > 4, "DOCX file should have content");
    // PK zip signature: 0x50 0x4B
    assert_eq!(bytes[0], 0x50, "DOCX should be a ZIP file (PK signature)");
    assert_eq!(bytes[1], 0x4B, "DOCX should be a ZIP file (PK signature)");
}

#[test]
fn html_output_returns_success() {
    // HTML output should succeed with exit code 0
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.rtf");
    let output = dir.path().join("out.html");
    fs::write(&input, r#"{\rtf1\ansi Hello\par}"#).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success().code(0);

    assert!(output.exists(), "Output HTML file should be created");
    let html = fs::read_to_string(&output).unwrap();
    assert!(html.contains("<!doctype html>"));
    assert!(html.contains(r#"<p class="rtf-p">Hello</p>"#));
}

// -------------------------------------------------------------------------
// Exit Code 2: Parse/Validation Failure Tests
// -------------------------------------------------------------------------

#[test]
fn invalid_rtf_returns_parse_exit_code() {
    // Non-RTF content should fail with exit code 2
    let dir = tempdir().unwrap();
    let input = dir.path().join("invalid.rtf");
    fs::write(&input, "not rtf").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("Parse error"));
}

#[test]
fn table_cell_limit_violation_returns_parse_exit_code() {
    // Exceeding max_cells_per_row should fail with exit code 2
    let dir = tempdir().unwrap();
    let input = dir.path().join("too_wide.rtf");

    // Default max_cells_per_row is 1000; this row has 1001 cells.
    let mut rtf = String::from("{\\rtf1\\ansi\\n\\trowd");
    for i in 1..=1001 {
        rtf.push_str(&format!("\\cellx{}", i * 1000));
    }
    rtf.push_str("\\n\\intbl ");
    for i in 1..=1001 {
        rtf.push_str(&format!("C{}\\cell ", i));
    }
    rtf.push_str("\\row\\n}");
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("Parse error"));
}

#[test]
fn table_row_limit_violation_returns_parse_exit_code() {
    // Exceeding max_rows_per_table should fail with exit code 2
    let dir = tempdir().unwrap();
    let input = dir.path().join("too_many_rows.rtf");

    // Default max_rows_per_table is 10000; generate one more.
    let mut rtf = String::from("{\\rtf1\\ansi\\n");
    for i in 1..=10001 {
        rtf.push_str(&format!("\\trowd\\cellx1000\\intbl R{}\\cell\\row\\n", i));
    }
    rtf.push('}');
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("Parse error"));
}

#[test]
fn limits_cells_exceed_fixture_succeeds() {
    // Note: limits_cells_exceed fixture is a placeholder that doesn't actually exceed limits
    // It should succeed (the actual limit tests use programmatic RTF generation)
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_cells_exceed.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Placeholder fixture succeeds - actual limit tests are programmatic
    cmd.assert().success();
}

#[test]
fn limits_depth_exceed_fixture_succeeds() {
    // Note: limits_depth_exceed fixture is a placeholder that doesn't actually exceed limits
    // It should succeed (the actual limit tests use programmatic RTF generation)
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_depth_exceed.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Placeholder fixture succeeds - actual limit tests are programmatic
    cmd.assert().success();
}

#[test]
fn limits_rows_exceed_fixture_succeeds() {
    // Note: limits_rows_exceed fixture is a placeholder that doesn't actually exceed limits
    // It should succeed (the actual limit tests use programmatic RTF generation)
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_rows_exceed.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Placeholder fixture succeeds - actual limit tests are programmatic
    cmd.assert().success();
}

#[test]
fn limits_merge_exceed_fixture_succeeds() {
    // Note: limits_merge_exceed fixture is a placeholder that doesn't actually exceed limits
    // It should succeed (the actual limit tests use programmatic RTF generation)
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_merge_exceed.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Placeholder fixture succeeds - actual limit tests are programmatic
    cmd.assert().success();
}

#[test]
fn malformed_unclosed_groups_succeeds() {
    // Note: malformed_unclosed_groups fixture is actually valid RTF with nested groups
    // It tests parser's handling of deep nesting and group boundaries
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_unclosed_groups.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // The fixture is actually valid RTF
    cmd.assert().success();
}

// -------------------------------------------------------------------------
// Exit Code 3: Writer/IO Failure Tests
// -------------------------------------------------------------------------

#[test]
fn output_flag_refuses_overwrite_without_force() {
    // Attempting to overwrite existing file without --force should fail with exit code 3
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.rtf");
    let output = dir.path().join("out.docx");
    fs::write(&input, r#"{\rtf1\ansi Hello\par}"#).unwrap();

    // First conversion should succeed
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success();

    // Second conversion should fail without --force
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
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
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);
    cmd.assert().success().stderr(contains("DOCX written to"));
}

#[test]
fn html_output_refuses_overwrite_without_force() {
    // Attempting to overwrite existing HTML file without --force should fail with exit code 3
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.rtf");
    let output = dir.path().join("out.html");
    fs::write(&input, r#"{\rtf1\ansi Hello\par}"#).unwrap();

    // First conversion should succeed.
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success();

    // Second conversion should fail without --force.
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert()
        .failure()
        .code(3)
        .stderr(contains("Output file already exists"));

    // With --force, it should succeed.
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--to",
        "html",
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);
    cmd.assert().success().stderr(contains("HTML written to"));
}

#[test]
fn output_flag_validates_directory() {
    // Writing to non-existent directory should fail with exit code 3
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.rtf");
    let output = std::path::Path::new("/nonexistent/dir/out.docx");
    fs::write(&input, r#"{\rtf1\ansi Hello\par}"#).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert()
        .failure()
        .code(3)
        .stderr(contains("Output directory does not exist"));
}

#[test]
fn output_check_does_not_delete_existing_probe_filename() {
    // Verify that the probe file check doesn't delete existing files
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.rtf");
    let output = dir.path().join("out.docx");
    let sentinel = dir.path().join(".rtfkit_write_test");
    fs::write(&input, r#"{\rtf1\ansi Hello\par}"#).unwrap();
    fs::write(&sentinel, "keep me").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);
    cmd.assert().success();

    assert!(sentinel.exists(), "Pre-existing file should not be deleted");
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), "keep me");
}

// -------------------------------------------------------------------------
// Exit Code 4: Strict-Mode Violation Tests
// -------------------------------------------------------------------------

#[test]
fn strict_mode_fails_on_dropped_destination_content() {
    // DroppedContent from unknown destination should cause exit code 4 in strict mode
    let dir = tempdir().unwrap();
    let input = dir.path().join("destination.rtf");
    fs::write(&input, r#"{\rtf1\ansi {\*\foo hidden} shown}"#).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_fails_on_unresolved_list_override() {
    // Unresolved list override should cause exit code 4 in strict mode
    let dir = tempdir().unwrap();
    let input = dir.path().join("unresolved_list.rtf");
    // \ls999 references a non-existent list override
    fs::write(&input, r#"{\rtf1\ansi \ls999 Item text}"#).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_fails_on_unresolved_ls_fixture() {
    // Using malformed_list_unresolved_ls fixture - should fail with exit code 4
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_list_unresolved_ls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_fails_on_missing_cell_terminator() {
    // Missing cell terminator should cause exit code 4 in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_missing_cell_terminator.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_fails_on_missing_row_terminator() {
    // Missing row terminator should cause exit code 4 in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_missing_row_terminator.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_fails_on_orphan_controls() {
    // Orphan table controls should cause exit code 4 in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_orphan_controls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_fails_on_orphan_merge_continuation() {
    // Orphan merge continuation should cause exit code 4 in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_orphan_merge_continuation.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_succeeds_on_supported_hyperlink_fixture() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/hyperlink_simple.rtf");

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

#[test]
fn strict_mode_passes_on_unsupported_field_with_preserved_result() {
    // An unsupported field (PAGE) whose fldrslt text is preserved emits UnsupportedField
    // (non-strict warning), so strict mode should succeed.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/hyperlink_unsupported_field.rtf");

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

#[test]
fn strict_mode_fails_on_unsupported_hyperlink_scheme() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("hyperlink_unsupported_scheme.rtf");
    fs::write(
        &input,
        r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "ftp://example.com/file"}{\fldrslt FTP}}}"#,
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
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn strict_mode_succeeds_on_well_formed_content() {
    // Well-formed content should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_simple_2x2.rtf");

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

#[test]
fn strict_mode_succeeds_on_supported_nested_table_fixture() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_nested_2level_basic.rtf");

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

#[test]
fn strict_mode_fails_on_malformed_nested_table_fixture() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_nested_missing_nestrow.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}
