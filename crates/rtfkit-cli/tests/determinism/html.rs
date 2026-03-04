use crate::support::fs::fixture_dir;
use std::path::PathBuf;
use tempfile::TempDir;

/// Run the CLI to convert an RTF file to HTML.
/// Returns the path to the generated HTML file.
fn run_cli_to_html(fixture_name: &str, temp_dir: &TempDir, suffix: &str) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join(format!("output_{suffix}.html"));

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

    let output_result = cmd.output().expect("Failed to run CLI");

    if !output_result.status.success() {
        panic!(
            "CLI failed for fixture '{}':\nstdout: {}\nstderr: {}",
            fixture_name,
            String::from_utf8_lossy(&output_result.stdout),
            String::from_utf8_lossy(&output_result.stderr)
        );
    }

    assert!(output.exists(), "Output HTML file should be created");
    output
}

/// Test HTML determinism for simple text with default CSS.
#[test]
fn html_simple_text_is_deterministic() {
    let fixture = "text_simple_paragraph.rtf";
    let temp_dir = TempDir::new().unwrap();

    // Generate HTML files 3 times
    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_html(fixture, &temp_dir, &i.to_string()))
        .collect();

    // Read all HTML files
    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    // Verify all are identical
    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output for simple text should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test HTML determinism for mixed content with default CSS.
#[test]
fn html_mixed_complex_is_deterministic() {
    let fixture = "mixed_complex.rtf";
    let temp_dir = TempDir::new().unwrap();

    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_html(fixture, &temp_dir, &i.to_string()))
        .collect();

    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output for mixed complex content should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test HTML determinism for tables with CSS.
#[test]
fn html_table_is_deterministic() {
    let fixture = "table_simple_2x2.rtf";
    let temp_dir = TempDir::new().unwrap();

    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_html(fixture, &temp_dir, &i.to_string()))
        .collect();

    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output for table should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test HTML determinism for lists with CSS.
#[test]
fn html_list_is_deterministic() {
    let fixture = "list_nested_two_levels.rtf";
    let temp_dir = TempDir::new().unwrap();

    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_html(fixture, &temp_dir, &i.to_string()))
        .collect();

    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output for list should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test HTML determinism with --html-css none.
#[test]
fn html_css_none_is_deterministic() {
    let fixture = "text_simple_paragraph.rtf";
    let temp_dir = TempDir::new().unwrap();

    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| {
            let input = fixture_dir().join(fixture);
            let output = temp_dir.path().join(format!("output_none_{i}.html"));

            let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
            cmd.args([
                "convert",
                input.to_str().unwrap(),
                "--to",
                "html",
                "--html-css",
                "none",
                "-o",
                output.to_str().unwrap(),
                "--force",
            ]);

            let output_result = cmd.output().expect("Failed to run CLI");
            assert!(output_result.status.success(), "CLI should succeed");
            output
        })
        .collect();

    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output with --html-css none should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test that HTML output contains expected CSS tokens.
#[test]
fn html_contains_css_tokens() {
    let fixture = "text_simple_paragraph.rtf";
    let temp_dir = TempDir::new().unwrap();
    let html_path = run_cli_to_html(fixture, &temp_dir, "tokens_check");
    let html = std::fs::read_to_string(html_path).expect("Failed to read HTML file");

    // Verify CSS custom properties are present
    assert!(
        html.contains("--rtfkit-font-body"),
        "HTML should contain --rtfkit-font-body token"
    );
    assert!(
        html.contains("--rtfkit-color-text-primary"),
        "HTML should contain --rtfkit-color-text-primary token"
    );
    assert!(
        html.contains("--rtfkit-space-md"),
        "HTML should contain --rtfkit-space-md token"
    );

    // Verify CSS classes are present
    assert!(
        html.contains(".rtf-doc"),
        "HTML should contain .rtf-doc class"
    );
    assert!(
        html.contains(".rtf-content"),
        "HTML should contain .rtf-content class"
    );
    assert!(html.contains(".rtf-p"), "HTML should contain .rtf-p class");
}
