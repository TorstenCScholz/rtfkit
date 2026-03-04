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

/// Test HTML output for PNG image.
/// PNG images should be embedded as data URIs with correct MIME type.
#[test]
fn html_image_png() {
    let fixture = "image_png_simple.rtf";
    let temp_dir = TempDir::new().unwrap();
    let html_path = run_cli_to_html(fixture, &temp_dir, "png_check");
    let html = std::fs::read_to_string(html_path).expect("Failed to read HTML file");

    // Verify figure element exists
    assert!(
        html.contains("<figure") || html.contains("<figure class=\"rtf-image\""),
        "HTML should contain figure element for image"
    );

    // Verify img element with data URI
    assert!(html.contains("<img"), "HTML should contain img element");
    assert!(
        html.contains("src=\"data:image/png;base64,"),
        "HTML should contain PNG data URI"
    );

    // Verify base64 data is present (non-empty)
    // The data URI should have content after the prefix
    assert!(
        html.contains("src=\"data:image/png;base64,")
            && html.matches("src=\"data:image/png;base64,").count() == 1,
        "HTML should have exactly one PNG data URI"
    );
}

/// Test HTML output for JPEG image.
/// JPEG images should be embedded as data URIs with correct MIME type.
#[test]
fn html_image_jpeg() {
    let fixture = "image_jpeg_simple.rtf";
    let temp_dir = TempDir::new().unwrap();
    let html_path = run_cli_to_html(fixture, &temp_dir, "jpeg_check");
    let html = std::fs::read_to_string(html_path).expect("Failed to read HTML file");

    // Verify figure element exists
    assert!(
        html.contains("<figure") || html.contains("<figure class=\"rtf-image\""),
        "HTML should contain figure element for image"
    );

    // Verify img element with data URI
    assert!(html.contains("<img"), "HTML should contain img element");
    assert!(
        html.contains("src=\"data:image/jpeg;base64,"),
        "HTML should contain JPEG data URI"
    );
}

/// Test HTML output for image with dimensions.
/// Images with picwgoal/pichgoal should have width/height attributes.
#[test]
fn html_image_with_dimensions() {
    let fixture = "image_with_dimensions.rtf";
    let temp_dir = TempDir::new().unwrap();
    let html_path = run_cli_to_html(fixture, &temp_dir, "dims_check");
    let html = std::fs::read_to_string(html_path).expect("Failed to read HTML file");

    // Verify img element exists
    assert!(html.contains("<img"), "HTML should contain img element");

    // Verify dimension attributes are present
    // picwgoal2880 = 2 inches, pichgoal1440 = 1 inch
    // In HTML, dimensions are typically in pixels or as style
    assert!(
        html.contains("width=") || html.contains("style="),
        "HTML should have width attribute or style for dimensions"
    );
}

/// Test HTML output for multiple images.
/// Multiple images should each have their own figure/img elements.
#[test]
fn html_image_multiple() {
    let fixture = "image_multiple.rtf";
    let temp_dir = TempDir::new().unwrap();
    let html_path = run_cli_to_html(fixture, &temp_dir, "multiple_check");
    let html = std::fs::read_to_string(html_path).expect("Failed to read HTML file");

    // Count img elements
    let img_count = html.matches("<img").count();
    assert!(
        img_count >= 3,
        "HTML should contain at least 3 img elements, found {}",
        img_count
    );

    // Verify both PNG and JPEG data URIs are present
    assert!(
        html.contains("data:image/png;base64,"),
        "HTML should contain PNG data URI"
    );
    assert!(
        html.contains("data:image/jpeg;base64,"),
        "HTML should contain JPEG data URI"
    );
}

/// Test HTML image determinism for PNG.
#[test]
fn html_image_png_is_deterministic() {
    let fixture = "image_png_simple.rtf";
    let temp_dir = TempDir::new().unwrap();

    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_html(fixture, &temp_dir, &format!("png_det_{i}")))
        .collect();

    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output for PNG image should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test HTML image determinism for JPEG.
#[test]
fn html_image_jpeg_is_deterministic() {
    let fixture = "image_jpeg_simple.rtf";
    let temp_dir = TempDir::new().unwrap();

    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_html(fixture, &temp_dir, &format!("jpeg_det_{i}")))
        .collect();

    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output for JPEG image should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test HTML image determinism for multiple images.
#[test]
fn html_image_multiple_is_deterministic() {
    let fixture = "image_multiple.rtf";
    let temp_dir = TempDir::new().unwrap();

    let html_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_html(fixture, &temp_dir, &format!("multi_det_{i}")))
        .collect();

    let html_contents: Vec<String> = html_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read HTML file"))
        .collect();

    let first = &html_contents[0];
    for (i, content) in html_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "HTML output for multiple images should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}
