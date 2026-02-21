use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn invalid_rtf_returns_parse_exit_code() {
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
fn strict_mode_fails_on_dropped_destination_content() {
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
fn strict_mode_fails_even_if_warning_cap_is_hit_first() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("warning_cap_then_drop.rtf");

    let mut rtf = String::from(r"{\rtf1\ansi ");
    for i in 0..1100 {
        rtf.push_str(&format!("\\unknown{i} "));
    }
    rtf.push_str(r"{\*\pict hidden} visible}");
    fs::write(&input, rtf).unwrap();

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
fn output_flag_creates_docx_file() {
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
    cmd.assert().success().stderr(contains("DOCX written to"));

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
fn output_check_does_not_delete_existing_probe_filename() {
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

#[test]
fn output_flag_refuses_overwrite_without_force() {
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
fn output_flag_validates_directory() {
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
fn strict_mode_fails_on_unresolved_list_override() {
    // Test that unresolved list override triggers strict mode failure
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
    // Test using the dedicated fixture file
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/list_unresolved_ls_strict_fail.rtf");

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
fn non_strict_mode_succeeds_with_warnings_on_unresolved_ls() {
    // Test that non-strict mode succeeds but emits warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/list_unresolved_ls_strict_fail.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Warnings are in JSON output (stdout), not stderr
    cmd.assert().success().stdout(contains("dropped_content"));
}

#[test]
fn strict_mode_fails_on_dropped_content_from_list() {
    // Test that DroppedContent warning causes strict-mode failure
    let dir = tempdir().unwrap();
    let input = dir.path().join("dropped_list_content.rtf");
    // Create RTF with unresolved list reference that causes content to be dropped
    fs::write(&input, r#"{\rtf1\ansi \ls999 Item with dropped content}"#).unwrap();

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
fn warning_cap_preserves_strict_mode_signal() {
    // Test that strict-mode signal is preserved even when warnings are capped
    let dir = tempdir().unwrap();
    let input = dir.path().join("warning_cap_strict.rtf");

    // Create RTF with many unknown keywords (to hit warning cap) AND dropped content
    let mut rtf = String::from(r"{\rtf1\ansi ");
    for i in 0..1100 {
        rtf.push_str(&format!("\\unknown{i} "));
    }
    // Add unresolved list reference that causes DroppedContent
    rtf.push_str(r"\ls999 dropped list item}");
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    // Should fail with exit code 4 (strict violation), not 0 (success)
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn dropped_content_always_causes_strict_failure() {
    // Ensure DroppedContent warnings always cause strict-mode failure regardless of count
    let dir = tempdir().unwrap();
    let input = dir.path().join("multiple_dropped.rtf");

    // Create RTF with multiple dropped content scenarios
    let rtf = r#"{\rtf1\ansi {\*\unknown1 hidden1} {\*\unknown2 hidden2} \ls999 list item}"#;
    fs::write(&input, rtf).unwrap();

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

// =============================================================================
// Table Strict Mode Contract Tests (PHASE4 section 7.4)
// =============================================================================

#[test]
fn test_table_simple_2x2_strict_succeeds() {
    // Well-formed table should succeed in strict mode (no semantic loss)
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
    cmd.assert().success();
}

#[test]
fn test_table_missing_cell_terminator_strict_fails() {
    // Missing cell terminator should fail in strict mode due to UnclosedTableCell warning
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_missing_cell_terminator.rtf");

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
fn test_table_missing_row_terminator_strict_fails() {
    // Missing row terminator should fail in strict mode due to UnclosedTableRow warning
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_missing_row_terminator.rtf");

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
fn test_table_orphan_controls_strict_fails() {
    // Orphan table controls should fail in strict mode due to MalformedTableStructure warning
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_orphan_controls.rtf");

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
fn test_table_merge_controls_non_strict_succeeds() {
    // Degraded merge controls should succeed in non-strict mode with warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_merge_controls_degraded.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Should succeed (exit code 0) with warnings in output
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn test_table_missing_cell_terminator_non_strict_succeeds() {
    // Missing cell terminator should succeed in non-strict mode with warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_missing_cell_terminator.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Should succeed (exit code 0) with warnings in output
    cmd.assert().success().stdout(contains("warnings"));
}

// =============================================================================
// Merge Conflict Strict Mode Contract Tests (PHASE5)
// =============================================================================

#[test]
fn test_merge_conflict_causes_strict_mode_failure() {
    // Test that merge conflicts (orphan continuation) cause strict mode failure
    // Use the orphan_merge_continuation fixture which has proper RTF syntax
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_orphan_merge_continuation.rtf");

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
fn test_table_geometry_conflict_causes_strict_mode_failure() {
    // Test that table geometry conflicts (span exceeding bounds) cause strict mode failure
    let dir = tempdir().unwrap();
    let input = dir.path().join("geometry_conflict.rtf");

    // Create RTF with merge span exceeding available cells
    // This should trigger TableGeometryConflict + DroppedContent
    // Note: clmrg must come before cellx in the row definition
    let rtf = r#"{\rtf1\ansi
\trowd\clmgf\cellx2880\clmrg\cellx5760\clmrg\cellx8640\clmrg\cellx11520
\intbl Cell 1\cell \cell \cell\row
}"#;
    fs::write(&input, rtf).unwrap();

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
fn test_merge_conflict_non_strict_succeeds_with_warnings() {
    // Test that merge conflicts succeed in non-strict mode with warnings
    // Use the orphan_merge_continuation fixture which has proper RTF syntax
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_orphan_merge_continuation.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Should succeed with warnings in output
    cmd.assert()
        .success()
        .stdout(contains("merge_conflict"))
        .stdout(contains("dropped_content"));
}

// =============================================================================
// Determinism Contract Tests (PHASE5)
// =============================================================================

#[test]
fn test_deterministic_output_for_merge_tables() {
    // Run conversion twice on a table with merge, verify identical output
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_horizontal_merge_valid.rtf");

    // First conversion - capture stdout for JSON format
    let mut cmd1 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd1.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    let output1 = cmd1.assert().success().get_output().stdout.clone();

    // Second conversion
    let mut cmd2 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd2.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    let output2 = cmd2.assert().success().get_output().stdout.clone();

    // Compare outputs
    assert_eq!(
        output1, output2,
        "JSON output should be deterministic for merge tables"
    );
}

#[test]
fn test_deterministic_output_for_vertical_merge_tables() {
    // Run conversion twice on a table with vertical merge, verify identical output
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_vertical_merge_valid.rtf");

    // First conversion - capture stdout for JSON format
    let mut cmd1 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd1.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    let output1 = cmd1.assert().success().get_output().stdout.clone();

    // Second conversion
    let mut cmd2 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd2.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    let output2 = cmd2.assert().success().get_output().stdout.clone();

    // Compare outputs
    assert_eq!(
        output1, output2,
        "JSON output should be deterministic for vertical merge tables"
    );
}

// =============================================================================
// Large Table Contract Tests (PHASE5)
// =============================================================================

#[test]
fn test_large_table_within_limits() {
    // Test that a large table (20 rows, 5 cells each) processes successfully
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_large_stress.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success();
}

#[test]
fn test_large_table_produces_correct_structure() {
    // Test that a large table produces the correct number of rows and cells
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_large_stress.rtf");

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

    // Parse IR JSON and verify structure
    let json = fs::read_to_string(&ir_output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Check that we have a table block
    let blocks = parsed.get("blocks").unwrap().as_array().unwrap();
    assert_eq!(blocks.len(), 1, "Should have exactly one block");

    let table = blocks[0].get("rows").unwrap().as_array().unwrap();
    assert_eq!(table.len(), 20, "Should have 20 rows");

    // Check first and last row have 5 cells each
    let first_row = table[0].get("cells").unwrap().as_array().unwrap();
    assert_eq!(first_row.len(), 5, "First row should have 5 cells");

    let last_row = table[19].get("cells").unwrap().as_array().unwrap();
    assert_eq!(last_row.len(), 5, "Last row should have 5 cells");
}

// =============================================================================
// New Fixture Contract Tests (PHASE5)
// =============================================================================

#[test]
fn test_horizontal_merge_valid_strict_succeeds() {
    // Valid horizontal merge should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_horizontal_merge_valid.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn test_vertical_merge_valid_strict_succeeds() {
    // Valid vertical merge should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_vertical_merge_valid.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn test_mixed_merge_strict_succeeds() {
    // Mixed merge should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_mixed_merge.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn test_orphan_merge_continuation_strict_fails() {
    // Orphan merge continuation should fail in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_orphan_merge_continuation.rtf");

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
fn test_conflicting_merge_strict_succeeds() {
    // Conflicting merge chains should succeed (they are resolved deterministically)
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_conflicting_merge.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn test_non_monotonic_cellx_non_strict_succeeds() {
    // Non-monotonic cellx should succeed in non-strict mode with warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_non_monotonic_cellx.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn test_prose_interleave_strict_succeeds() {
    // Prose/table interleave should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_prose_interleave.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}
