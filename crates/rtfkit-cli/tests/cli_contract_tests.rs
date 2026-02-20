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
fn output_flag_is_rejected_until_writer_exists() {
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
        "--format",
        "text",
    ]);
    cmd.assert()
        .failure()
        .code(3)
        .stderr(contains("--output is not supported yet"));
}
