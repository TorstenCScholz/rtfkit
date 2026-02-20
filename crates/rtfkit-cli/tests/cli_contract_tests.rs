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
