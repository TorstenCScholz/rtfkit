use crate::support::fs::fixture_dir;
use std::path::PathBuf;
use tempfile::TempDir;

/// Run the CLI to convert an RTF file to PDF.
/// Returns the path to the generated PDF file.
fn run_cli_to_pdf(fixture_name: &str, temp_dir: &TempDir, suffix: &str) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join(format!("output_{suffix}.pdf"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--to",
        "pdf",
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

    assert!(output.exists(), "Output PDF file should be created");
    output
}

/// Test PDF determinism for simple text.
/// Simple text is the baseline case - any non-determinism here would be a bug.
#[test]
fn pdf_simple_text_is_deterministic() {
    let fixture = "text_simple_paragraph.rtf";
    let temp_dir = TempDir::new().unwrap();

    // Generate PDF files 3 times
    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &i.to_string()))
        .collect();

    // Read all PDF files
    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    // Verify all are identical
    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for simple text should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for mixed content.
#[test]
fn pdf_mixed_complex_is_deterministic() {
    let fixture = "mixed_complex.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &i.to_string()))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for mixed complex content should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for tables.
#[test]
fn pdf_table_is_deterministic() {
    let fixture = "table_simple_2x2.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &i.to_string()))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for table should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for lists.
#[test]
fn pdf_list_is_deterministic() {
    let fixture = "list_nested_two_levels.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &i.to_string()))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for list should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for horizontal merge table.
#[test]
fn pdf_horizontal_merge_table_is_deterministic() {
    let fixture = "table_horizontal_merge_valid.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &i.to_string()))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for horizontal merge table should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for vertical merge table.
#[test]
fn pdf_vertical_merge_table_is_deterministic() {
    let fixture = "table_vertical_merge_valid.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &i.to_string()))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for vertical merge table should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test that PDF output starts with valid PDF header.
#[test]
fn pdf_output_has_valid_header() {
    let fixture = "text_simple_paragraph.rtf";
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = run_cli_to_pdf(fixture, &temp_dir, "header_check");
    let pdf_bytes = std::fs::read(pdf_path).expect("Failed to read PDF file");

    // PDF files start with %PDF-
    assert!(
        pdf_bytes.starts_with(b"%PDF-"),
        "PDF output should start with %PDF- header"
    );

    // PDF files typically end with %%EOF
    assert!(
        pdf_bytes.ends_with(b"%%EOF") || pdf_bytes.windows(5).any(|w| w == b"%%EOF"),
        "PDF output should contain %%EOF marker"
    );
}
