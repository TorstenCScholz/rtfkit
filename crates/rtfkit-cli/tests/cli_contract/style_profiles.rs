use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

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
fn docx_classic_style_profile_accepted() {
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
        "classic",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success();
    assert!(output.exists(), "DOCX file should be created");
}

#[test]
fn docx_report_style_profile_accepted() {
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
    cmd.assert().success();
    assert!(output.exists(), "DOCX file should be created");
}

#[test]
fn docx_compact_style_profile_accepted() {
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
        "compact",
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success();
    assert!(output.exists(), "DOCX file should be created");
}

#[test]
fn docx_no_profile_succeeds() {
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
        "-o",
        output.to_str().unwrap(),
    ]);
    cmd.assert().success();
    assert!(output.exists(), "DOCX file should be created");
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
