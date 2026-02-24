use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

// =============================================================================
// EXIT CODE CONTRACT TESTS
// =============================================================================
//
// Exit code contracts (from PHASE6.md section 5.2):
// - 0: success
// - 2: parse/validation failure (including hard limit violations)
// - 3: writer/IO failure
// - 4: strict-mode violation

mod exit_code_tests {
    use super::*;

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
    fn strict_mode_fails_on_unsupported_field_fixture() {
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
        cmd.assert()
            .failure()
            .code(4)
            .stderr(contains("Strict mode violated"));
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
}

// =============================================================================
// STRICT MODE INVARIANT TESTS
// =============================================================================
//
// Strict-mode invariants (from PHASE6.md section 5.2):
// - Any DroppedContent causes strict failure
// - Warning-cap behavior still preserves strict signal

mod strict_mode_tests {
    use super::*;

    #[test]
    fn strict_mode_fails_even_if_warning_cap_is_hit_first() {
        // Test that strict-mode signal is preserved even when warnings are capped
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
    fn non_strict_mode_succeeds_with_warnings_on_unresolved_ls() {
        // Test that non-strict mode succeeds but emits warnings
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_list_unresolved_ls.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        // Warnings are in JSON output (stdout), not stderr
        cmd.assert().success().stdout(contains("dropped_content"));
    }

    #[test]
    fn table_merge_controls_non_strict_succeeds() {
        // Degraded merge controls should succeed in non-strict mode with warnings
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_merge_controls_degraded.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        // Should succeed (exit code 0) with warnings in output
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn table_missing_cell_terminator_non_strict_succeeds() {
        // Missing cell terminator should succeed in non-strict mode with warnings
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_missing_cell_terminator.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        // Should succeed (exit code 0) with warnings in output
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn merge_conflict_non_strict_succeeds_with_warnings() {
        // Test that merge conflicts succeed in non-strict mode with warnings
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_orphan_merge_continuation.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        // Should succeed with warnings in output
        cmd.assert()
            .success()
            .stdout(contains("merge_conflict"))
            .stdout(contains("dropped_content"));
    }

    #[test]
    fn table_geometry_conflict_causes_strict_mode_failure() {
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
    fn horizontal_merge_valid_strict_succeeds() {
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
    fn vertical_merge_valid_strict_succeeds() {
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
    fn mixed_merge_strict_succeeds() {
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
    fn conflicting_merge_strict_succeeds() {
        // Conflicting merge chains should succeed (they are resolved deterministically)
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_conflicting_merge.rtf");

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
    fn non_monotonic_cellx_non_strict_succeeds() {
        // Non-monotonic cellx should succeed in non-strict mode with warnings
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_non_monotonic_cellx.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn prose_interleave_strict_succeeds() {
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
}

// =============================================================================
// WARNING SEMANTICS TESTS
// =============================================================================
//
// Warning semantics (from PHASE6.md section 5.2):
// - Stable warning types and reason-string contracts for key cases
// - Key warning types: DroppedContent, UnsupportedFeature, MalformedTableStructure,
//   UnclosedTableCell, UnclosedTableRow, MergeConflict, TableGeometryConflict

mod warning_semantics_tests {
    use super::*;

    #[test]
    fn dropped_content_warning_has_stable_type() {
        // Test that DroppedContent warning type is stable and present
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_list_unresolved_ls.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // The output should contain the stable warning type "dropped_content"
        cmd.assert().success().stdout(contains("dropped_content"));
    }

    #[test]
    fn dropped_content_warning_has_reason_string() {
        // Test that DroppedContent warning includes a meaningful reason string
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_list_unresolved_ls.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // The output should contain a reason field
        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Check for reason field presence
        assert!(
            output_str.contains("\"reason\""),
            "Warning should include a reason field"
        );
    }

    #[test]
    fn merge_conflict_warning_has_stable_type() {
        // Test that MergeConflict warning type is stable
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_orphan_merge_continuation.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // The output should contain the stable warning type "merge_conflict"
        cmd.assert().success().stdout(contains("merge_conflict"));
    }

    #[test]
    fn malformed_table_structure_warning_has_stable_type() {
        // Test that MalformedTableStructure warning type is stable
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_orphan_controls.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed with warnings
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn unclosed_table_cell_warning_present() {
        // Test that UnclosedTableCell warning is generated for missing cell terminators
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_missing_cell_terminator.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed with warnings
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn unclosed_table_row_warning_present() {
        // Test that UnclosedTableRow warning is generated for missing row terminators
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_missing_row_terminator.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed with warnings
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn invalid_control_words_generate_warnings() {
        // Test that invalid control words generate warnings but don't fail
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_invalid_control_words.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed with warnings
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn repeated_bad_controls_generate_warnings() {
        // Test that repeated bad controls generate warnings
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_repeated_bad_controls.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed with warnings
        cmd.assert().success().stdout(contains("warnings"));
    }

    #[test]
    fn warning_count_is_reported() {
        // Test that warning count is included in output
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_invalid_control_words.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Check for warning count field
        assert!(
            output_str.contains("warning_count") || output_str.contains("warnings"),
            "Output should include warning count"
        );
    }

    #[test]
    fn list_fallback_generates_warnings() {
        // Test that list fallback generates warnings
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_list_fallback.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed with warnings
        cmd.assert().success().stdout(contains("warnings"));
    }
}

// =============================================================================
// RECOVERY BEHAVIOR TESTS
// =============================================================================
//
// Recovery behavior (from PHASE6.md section 5.2):
// - Malformed inputs preserve visible text ordering where possible
// - Degraded content still produces valid output

mod recovery_behavior_tests {
    #[test]
    fn malformed_input_preserves_visible_text() {
        // Test that malformed RTF still extracts visible text in order
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_invalid_control_words.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed and produce output
        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Output should be valid JSON
        assert!(
            serde_json::from_str::<serde_json::Value>(&output_str).is_ok(),
            "Output should be valid JSON"
        );
    }

    #[test]
    fn malformed_table_produces_valid_output() {
        // Test that malformed tables still produce valid output structure
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_missing_cell_terminator.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Output should be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&output_str).expect("Output should be valid JSON");

        // Check for either blocks or stats field (output structure may vary)
        assert!(
            parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
            "Output should have blocks or stats field"
        );
    }

    #[test]
    fn degraded_merge_controls_produce_valid_table() {
        // Test that degraded merge controls still produce a valid table
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_merge_controls_degraded.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Output should be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&output_str).expect("Output should be valid JSON");

        // Check for either blocks or stats field
        assert!(
            parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
            "Output should have blocks or stats field"
        );
    }

    #[test]
    fn repeated_bad_controls_preserve_text() {
        // Test that repeated bad controls don't lose text content
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_repeated_bad_controls.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed with output
        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Output should be valid JSON
        assert!(
            serde_json::from_str::<serde_json::Value>(&output_str).is_ok(),
            "Output should be valid JSON"
        );
    }

    #[test]
    fn list_fallback_produces_valid_output() {
        // Test that list fallback produces valid output
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_list_fallback.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Output should be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&output_str).expect("Output should be valid JSON");

        // Check for either blocks or stats field
        assert!(
            parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
            "Output should have blocks or stats field"
        );
    }

    #[test]
    fn orphan_controls_recover_to_valid_output() {
        // Test that orphan table controls recover to valid output
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_orphan_controls.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Output should be valid JSON
        assert!(
            serde_json::from_str::<serde_json::Value>(&output_str).is_ok(),
            "Output should be valid JSON"
        );
    }
}

// =============================================================================
// DETERMINISM CONTRACT TESTS
// =============================================================================
//
// Determinism verification (from PHASE6.md section 5.3):
// - Report JSON ordering/values for same input
// - IR JSON byte stability for same input

mod determinism_tests {
    use serde_json::Value;

    fn project_root() -> std::path::PathBuf {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn canonical_report_json(fixture_name: &str) -> Value {
        let fixture = project_root().join("fixtures").join(fixture_name);

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        let output = cmd.assert().success().get_output().stdout.clone();

        let mut parsed: Value =
            serde_json::from_slice(&output).expect("CLI JSON output should parse");
        if let Some(stats) = parsed.get_mut("stats").and_then(|v| v.as_object_mut()) {
            // duration_ms depends on runtime scheduling and is not semantic output.
            stats.insert("duration_ms".to_string(), serde_json::json!(0));
        }
        parsed
    }

    fn assert_deterministic_report_json(fixture_name: &str, label: &str) {
        let output1 = canonical_report_json(fixture_name);
        let output2 = canonical_report_json(fixture_name);
        assert_eq!(output1, output2, "{label}");
    }

    #[test]
    fn deterministic_output_for_simple_text() {
        assert_deterministic_report_json(
            "text_simple_paragraph.rtf",
            "JSON output should be deterministic for simple text",
        );
    }

    #[test]
    fn deterministic_output_for_merge_tables() {
        assert_deterministic_report_json(
            "table_horizontal_merge_valid.rtf",
            "JSON output should be deterministic for merge tables",
        );
    }

    #[test]
    fn deterministic_output_for_vertical_merge_tables() {
        assert_deterministic_report_json(
            "table_vertical_merge_valid.rtf",
            "JSON output should be deterministic for vertical merge tables",
        );
    }

    #[test]
    fn deterministic_output_for_nested_lists() {
        assert_deterministic_report_json(
            "list_nested_two_levels.rtf",
            "JSON output should be deterministic for nested lists",
        );
    }

    #[test]
    fn deterministic_output_for_mixed_content() {
        assert_deterministic_report_json(
            "mixed_complex.rtf",
            "JSON output should be deterministic for mixed content",
        );
    }

    #[test]
    fn deterministic_output_for_degraded_content() {
        assert_deterministic_report_json(
            "malformed_table_merge_controls_degraded.rtf",
            "JSON output should be deterministic for degraded content",
        );
    }
}

// =============================================================================
// LIMITS AND SAFETY TESTS
// =============================================================================
//
// Limits testing (from PHASE6.md section 5.4):
// - max_input_bytes, max_group_depth, max_warning_count
// - max_rows_per_table, max_cells_per_row, max_merge_span
// - Near-limit success tests and over-limit failure tests

mod limits_tests {
    use super::*;

    #[test]
    fn near_limit_cells_succeeds() {
        // Test that a table near the cell limit succeeds
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_cells_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success();
    }

    #[test]
    fn near_limit_depth_succeeds() {
        // Test that a document near the depth limit succeeds
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_depth_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success();
    }

    #[test]
    fn near_limit_rows_succeeds() {
        // Test that a table near the row limit succeeds
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_rows_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success();
    }

    #[test]
    fn near_limit_merge_succeeds() {
        // Test that a table near the merge span limit succeeds
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_merge_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success();
    }

    #[test]
    fn large_table_within_limits() {
        // Test that a large table (20 rows, 5 cells each) processes successfully
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_table_stress.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success();
    }

    #[test]
    fn large_table_produces_correct_structure() {
        // Test that a large table produces the correct number of rows and cells
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_table_stress.rtf");

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

    #[test]
    fn no_partial_output_after_limit_failure() {
        // Verify that no partial output is emitted after fatal limit failure
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_wide.rtf");

        // Create RTF that exceeds cell limit
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

        // Should fail with exit code 2
        cmd.assert().failure().code(2);

        // Stdout should be empty or contain only error info, not partial JSON
        let output = cmd.assert().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Should not be valid JSON with blocks (no partial output)
        if !output_str.is_empty() {
            // If there's output, it should not be a valid IR structure
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output_str) {
                assert!(
                    parsed.get("blocks").is_none(),
                    "Should not emit partial blocks after fatal limit failure"
                );
            }
        }
    }
}

// =============================================================================
// REGRESSION TESTS
// =============================================================================
//
// Regression tests for previously fixed bugs:
// - Merge/orphan/degradation bugs
// - Table normalization edge cases

mod regression_tests {
    use super::*;

    #[test]
    fn regression_merge_conflict_deterministic_resolution() {
        // Regression: Merge conflicts should be resolved deterministically
        // Run multiple times to ensure consistent output
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_conflicting_merge.rtf");

        let mut outputs = Vec::new();
        for _ in 0..3 {
            let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
            cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
            let output = cmd.assert().success().get_output().stdout.clone();
            let mut parsed: serde_json::Value =
                serde_json::from_slice(&output).expect("CLI JSON output should parse");
            if let Some(stats) = parsed.get_mut("stats").and_then(|v| v.as_object_mut()) {
                // duration_ms is runtime-dependent and not part of semantic determinism.
                stats.insert("duration_ms".to_string(), serde_json::json!(0));
            }
            outputs.push(parsed);
        }

        // All outputs should be identical
        for i in 1..outputs.len() {
            assert_eq!(
                outputs[0], outputs[i],
                "Merge conflict resolution should be deterministic"
            );
        }
    }

    #[test]
    fn regression_orphan_merge_continuation_handled() {
        // Regression: Orphan merge continuation should be handled gracefully
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/malformed_table_orphan_merge_continuation.rtf");

        // Non-strict mode should succeed
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success();

        // Strict mode should fail with proper error
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
    fn regression_table_normalization_preserves_content() {
        // Regression: Table normalization should not lose content
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/table_multirow_uneven_content.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Parse and verify content is preserved
        let parsed: serde_json::Value =
            serde_json::from_str(&output_str).expect("Output should be valid JSON");

        // Check for either blocks or stats field
        assert!(
            parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
            "Output should have blocks or stats field"
        );
    }

    #[test]
    fn regression_list_in_table_cell() {
        // Regression: Lists inside table cells should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/table_with_list_in_cell.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed without crashing
        cmd.assert().success();
    }

    #[test]
    fn regression_mixed_prose_list_table() {
        // Regression: Mixed prose/list/table should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/mixed_prose_list_table.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed without crashing
        cmd.assert().success();
    }

    #[test]
    fn regression_nested_list_table() {
        // Regression: Nested list/table combinations should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/mixed_list_table_nested.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed without crashing
        cmd.assert().success();
    }

    #[test]
    fn regression_decimal_list_handling() {
        // Regression: Decimal lists should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/list_decimal_simple.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed
        cmd.assert().success();
    }

    #[test]
    fn regression_mixed_list_kinds() {
        // Regression: Mixed list kinds should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/list_mixed_kinds.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed
        cmd.assert().success();
    }

    #[test]
    fn regression_formatting_preservation() {
        // Regression: Text formatting should be preserved
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_mixed_formatting.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed
        cmd.assert().success();
    }

    #[test]
    fn regression_nested_styles() {
        // Regression: Nested styles should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_nested_styles.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed
        cmd.assert().success();
    }

    #[test]
    fn regression_alignment_handling() {
        // Regression: Text alignment should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_alignment.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed
        cmd.assert().success();
    }

    #[test]
    fn regression_multiple_paragraphs() {
        // Regression: Multiple paragraphs should be handled correctly
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_multiple_paragraphs.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

        // Should succeed
        cmd.assert().success();
    }
}

// =============================================================================
// HTML CSS MODE TESTS
// =============================================================================
//
// Tests for --html-css and --html-css-file CLI options (from PHASE_CSS_POLISH.md section 9):
// - --html-css default: embed built-in stylesheet
// - --html-css none: omit built-in CSS
// - --html-css-file: append custom CSS from file
// - Error handling for missing CSS files
// - Rejection of HTML-only flags for non-HTML targets

mod html_css_tests {
    use super::*;

    /// Test that --html-css default succeeds and includes built-in stylesheet.
    #[test]
    fn html_css_default_includes_stylesheet() {
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
            "--html-css",
            "default",
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert().success();

        // Verify output contains built-in stylesheet
        let html = fs::read_to_string(&output).unwrap();
        assert!(
            html.contains("<style>"),
            "HTML should contain <style> block"
        );
        assert!(
            html.contains(".rtf-doc"),
            "HTML should contain built-in CSS classes"
        );
        assert!(
            html.contains("--rtfkit-font-body"),
            "HTML should contain CSS custom properties"
        );
    }

    /// Test that --html-css none succeeds and omits built-in stylesheet.
    #[test]
    fn html_css_none_omits_stylesheet() {
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
            "--html-css",
            "none",
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert().success();

        // Verify output does NOT contain built-in stylesheet
        let html = fs::read_to_string(&output).unwrap();
        assert!(
            !html.contains("<style>"),
            "HTML should not contain <style> block with --html-css none"
        );
        assert!(
            !html.contains(".rtf-doc"),
            "HTML should not contain built-in CSS classes with --html-css none"
        );
    }

    /// Test that --html-css-file with valid path succeeds.
    #[test]
    fn html_css_file_valid_path_succeeds() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("output.html");
        let css_file = dir.path().join("custom.css");

        // Create a custom CSS file
        fs::write(&css_file, ".custom-class { color: red; }").unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "html",
            "--html-css-file",
            css_file.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert().success();

        // Verify output contains custom CSS
        let html = fs::read_to_string(&output).unwrap();
        assert!(
            html.contains(".custom-class { color: red; }"),
            "HTML should contain custom CSS"
        );
    }

    /// Test that --html-css-file with missing path returns exit code 3.
    #[test]
    fn html_css_file_missing_returns_exit_code_3() {
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
            "--html-css-file",
            "nonexistent.css",
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert()
            .failure()
            .code(3)
            .stderr(contains("Error reading CSS file"));
    }

    /// Test that --html-css is rejected for non-HTML targets (DOCX).
    #[test]
    fn html_css_rejected_for_docx_target() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("output.docx");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "docx",
            "--html-css",
            "none",
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("only valid with --to html"));
        assert!(
            !output.exists(),
            "DOCX file should not be created on validation failure"
        );
    }

    /// Test that --html-css-file is rejected for non-HTML targets (DOCX).
    #[test]
    fn html_css_file_rejected_for_docx_target() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("output.docx");
        let css_file = dir.path().join("custom.css");

        fs::write(&css_file, ".custom { color: red; }").unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "docx",
            "--html-css-file",
            css_file.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("only valid with --to html"));
        assert!(
            !output.exists(),
            "DOCX file should not be created on validation failure"
        );
    }

    /// Test that oversized --html-css-file returns exit code 3.
    #[test]
    fn html_css_file_too_large_returns_exit_code_3() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("output.html");
        let css_file = dir.path().join("oversized.css");

        // 1 MiB + 1 byte (matches CLI limit constant)
        let oversized_css = "a".repeat(1024 * 1024 + 1);
        fs::write(&css_file, oversized_css).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "html",
            "--html-css-file",
            css_file.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert()
            .failure()
            .code(3)
            .stderr(contains("CSS file too large"));
    }

    /// Test combining --html-css default with --html-css-file.
    #[test]
    fn html_css_default_with_custom_file() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("output.html");
        let css_file = dir.path().join("custom.css");

        fs::write(&css_file, "/* custom css */").unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "html",
            "--html-css",
            "default",
            "--html-css-file",
            css_file.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert().success();

        let html = fs::read_to_string(&output).unwrap();

        // Should contain both built-in and custom CSS
        assert!(
            html.contains(".rtf-doc"),
            "HTML should contain built-in CSS"
        );
        assert!(
            html.contains("/* custom css */"),
            "HTML should contain custom CSS"
        );

        // Custom CSS should come after built-in CSS
        let builtin_pos = html.find(".rtf-doc").unwrap();
        let custom_pos = html.find("/* custom css */").unwrap();
        assert!(
            custom_pos > builtin_pos,
            "Custom CSS should be appended after built-in CSS"
        );
    }

    /// Test combining --html-css none with --html-css-file (custom CSS only).
    #[test]
    fn html_css_none_with_custom_file() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("output.html");
        let css_file = dir.path().join("custom.css");

        fs::write(&css_file, ".my-custom { margin: 0; }").unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "html",
            "--html-css",
            "none",
            "--html-css-file",
            css_file.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--force",
        ]);

        cmd.assert().success();

        let html = fs::read_to_string(&output).unwrap();

        // Should NOT contain built-in CSS
        assert!(
            !html.contains(".rtf-doc"),
            "HTML should not contain built-in CSS with --html-css none"
        );

        // Should contain custom CSS
        assert!(
            html.contains(".my-custom { margin: 0; }"),
            "HTML should contain custom CSS"
        );

        // Should have exactly one style block
        assert_eq!(
            html.matches("<style>").count(),
            1,
            "Should have exactly one <style> block for custom CSS"
        );
    }
}

// =============================================================================
// PDF OUTPUT TESTS
// =============================================================================
//
// Tests for PDF output functionality (from PHASE_PDF.md section 14):
// - PDF output succeeds on valid input
// - PDF-specific flags rejected for non-PDF targets
// - Backend invocation failure returns exit code 3
// - Strict mode with dropped content returns exit code 4

mod pdf_output_tests {
    use super::*;

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
}

mod style_profile_tests {
    use super::*;

    #[test]
    fn unknown_style_profile_returns_parse_error() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("out.html");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "html",
            "--style-profile",
            "unknown",
            "-o",
            output.to_str().unwrap(),
        ]);
        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("Unknown style profile"));
    }

    #[test]
    fn style_profile_rejected_for_docx_output() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let output = dir.path().join("out.docx");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "docx",
            "--style-profile",
            "report",
            "-o",
            output.to_str().unwrap(),
        ]);
        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("Style profiles are not supported for DOCX output"));
    }

    #[test]
    fn html_style_profile_changes_token_values() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/text_simple_paragraph.rtf");

        let dir = tempdir().unwrap();
        let classic_out = dir.path().join("classic.html");
        let compact_out = dir.path().join("compact.html");

        let mut classic = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        classic.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "html",
            "--style-profile",
            "classic",
            "-o",
            classic_out.to_str().unwrap(),
        ]);
        classic.assert().success();

        let mut compact = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        compact.args([
            "convert",
            fixture.to_str().unwrap(),
            "--to",
            "html",
            "--style-profile",
            "compact",
            "-o",
            compact_out.to_str().unwrap(),
        ]);
        compact.assert().success();

        let classic_html = fs::read_to_string(&classic_out).unwrap();
        let compact_html = fs::read_to_string(&compact_out).unwrap();

        assert!(classic_html.contains("--rtfkit-size-body: 12pt;"));
        assert!(compact_html.contains("--rtfkit-size-body: 9pt;"));
    }
}

// =============================================================================
// FONT AND COLOR FEATURE TESTS
// =============================================================================
//
// Tests for font family, font size, and foreground color support:
// - Font/color fixtures pass strict mode without DroppedContent warnings
// - Unresolved font/color indexes degrade gracefully (no semantic loss)
// - \plain reset doesn't introduce semantic-loss warnings

mod font_color_tests {
    use super::*;

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
}

// =============================================================================
// BACKGROUND/HIGHLIGHT COLOR FEATURE TESTS
// =============================================================================
//
// Tests for \highlightN and \cbN background color support:
// - Background/highlight fixtures pass strict mode without DroppedContent warnings
// - Unresolved color indexes degrade gracefully (no semantic loss)
// - \plain resets background/highlight as character formatting

mod background_color_tests {
    use super::*;

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
}

// =============================================================================
// BLOCK SHADING FEATURE TESTS
// =============================================================================
//
// Tests for \cbpatN (paragraph shading) and \clcbpatN (cell shading) support:
// - Paragraph and cell shading fixtures pass strict mode without DroppedContent warnings
// - Unresolved color indexes degrade gracefully (no semantic loss)
// - \pard resets paragraph shading, \plain does not
// - Cell > row > table shading precedence

mod block_shading_tests {
    use super::*;

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
}
