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

/// Test PDF determinism for PNG image.
#[test]
fn pdf_image_png_is_deterministic() {
    let fixture = "image_png_simple.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &format!("png_{i}")))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for PNG image should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for JPEG image.
#[test]
fn pdf_image_jpeg_is_deterministic() {
    let fixture = "image_jpeg_simple.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &format!("jpeg_{i}")))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for JPEG image should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for multiple images.
#[test]
fn pdf_image_multiple_is_deterministic() {
    let fixture = "image_multiple.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &format!("multi_{i}")))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for multiple images should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test PDF determinism for image with dimensions.
#[test]
fn pdf_image_with_dimensions_is_deterministic() {
    let fixture = "image_with_dimensions.rtf";
    let temp_dir = TempDir::new().unwrap();

    let pdf_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_pdf(fixture, &temp_dir, &format!("dims_{i}")))
        .collect();

    let pdf_contents: Vec<Vec<u8>> = pdf_paths
        .iter()
        .map(|path| std::fs::read(path).expect("Failed to read PDF file"))
        .collect();

    let first = &pdf_contents[0];
    for (i, content) in pdf_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "PDF output for image with dimensions should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}
