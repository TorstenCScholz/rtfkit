use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

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
