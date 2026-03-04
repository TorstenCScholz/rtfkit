use crate::support::cli::run_cli_to_ir;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test IR JSON determinism for simple text paragraph.
/// Simple text is the baseline case - any non-determinism here would be a bug.
#[test]
fn ir_simple_text_is_deterministic() {
    let fixture = "text_simple_paragraph.rtf";

    // Run 3 times and collect IR outputs
    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    // Read all IR files
    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    // Verify all are identical
    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for simple text should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for nested lists.
/// Nested structures must maintain consistent ordering.
#[test]
fn ir_nested_lists_is_deterministic() {
    let fixture = "list_nested_two_levels.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for nested lists should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for horizontal merge tables.
/// Table merge resolution must be deterministic.
#[test]
fn ir_horizontal_merge_table_is_deterministic() {
    let fixture = "table_horizontal_merge_valid.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for horizontal merge table should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for vertical merge tables.
/// Vertical merge resolution must be deterministic.
#[test]
fn ir_vertical_merge_table_is_deterministic() {
    let fixture = "table_vertical_merge_valid.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for vertical merge table should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for degraded malformed input.
/// Degraded content handling must be deterministic.
#[test]
fn ir_degraded_content_is_deterministic() {
    let fixture = "malformed_table_merge_controls_degraded.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for degraded content should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for mixed complex content.
/// Complex documents with multiple element types must be deterministic.
#[test]
fn ir_mixed_complex_is_deterministic() {
    let fixture = "mixed_complex.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for mixed complex content should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for mixed list kinds.
/// Multiple list types in one document must be handled deterministically.
#[test]
fn ir_mixed_list_kinds_is_deterministic() {
    let fixture = "list_mixed_kinds.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for mixed list kinds should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for table with list in cell.
/// Nested content in table cells must be deterministic.
#[test]
fn ir_table_with_list_in_cell_is_deterministic() {
    let fixture = "table_with_list_in_cell.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &i.to_string()))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for table with list in cell should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for PNG image.
/// Image data must be deterministic.
#[test]
fn ir_png_image_is_deterministic() {
    let fixture = "image_png_simple.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &format!("png_{i}")))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for PNG image should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for JPEG image.
#[test]
fn ir_jpeg_image_is_deterministic() {
    let fixture = "image_jpeg_simple.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &format!("jpeg_{i}")))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for JPEG image should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}

/// Test IR JSON determinism for multiple images.
#[test]
fn ir_multiple_images_is_deterministic() {
    let fixture = "image_multiple.rtf";

    let temp_dir = TempDir::new().unwrap();
    let ir_paths: Vec<PathBuf> = (0..3)
        .map(|i| run_cli_to_ir(fixture, &temp_dir, &format!("multi_{i}")))
        .collect();

    let ir_contents: Vec<String> = ir_paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("Failed to read IR file"))
        .collect();

    let first = &ir_contents[0];
    for (i, content) in ir_contents.iter().enumerate().skip(1) {
        assert_eq!(
            first, content,
            "IR JSON for multiple images should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}
