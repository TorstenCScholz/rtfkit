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
