//! Determinism Test Suite (Phase 6, Workstream 5.3)
//!
//! These tests verify output stability for identical inputs across multiple runs.
//! Determinism is critical for reproducible builds and reliable conversion behavior.
//!
//! # Determinism Targets
//!
//! 1. Report JSON ordering/values for same input
//! 2. IR JSON byte stability for same input
//! 3. `word/document.xml` stability for same input (excluding zip metadata noise)
//!
//! # Test Methodology
//!
//! Each test runs the conversion at least 3 times and compares outputs byte-for-byte
//! (for IR/Report) or XML content (for DOCX, since zip metadata may vary).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use zip::ZipArchive;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get the project root directory (workspace root)
fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Get the fixtures directory path
fn fixture_dir() -> PathBuf {
    project_root().join("fixtures")
}

/// Run rtfkit multiple times with the given arguments and collect outputs.
/// Returns a vector of stdout strings (one per run).
fn run_rtfkit_multiple_times(args: &[&str], runs: usize) -> Vec<Vec<u8>> {
    let mut outputs = Vec::with_capacity(runs);

    for _ in 0..runs {
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(args);

        let output = cmd.output().expect("Failed to execute rtfkit");

        if !output.status.success() {
            panic!(
                "CLI failed with args {:?}:\nstdout: {}\nstderr: {}",
                args,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        outputs.push(output.stdout);
    }

    outputs
}

/// Verify that all outputs are byte-identical.
/// Returns true if all outputs match the first one.
fn verify_identical_outputs(outputs: &[Vec<u8>]) -> bool {
    if outputs.is_empty() {
        return true;
    }

    let first = &outputs[0];
    outputs.iter().all(|output| output == first)
}

/// Extract an XML file from a DOCX archive.
/// Returns the XML content as a string.
fn extract_docx_xml(docx_path: &Path, xml_name: &str) -> String {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut xml_content = String::new();
    archive
        .by_name(xml_name)
        .unwrap_or_else(|_| panic!("{xml_name} not found in DOCX"))
        .read_to_string(&mut xml_content)
        .expect("Failed to read XML content");

    xml_content
}

/// Run the CLI to convert an RTF file to DOCX.
/// Returns the path to the generated DOCX file.
fn run_cli_to_docx(fixture_name: &str, temp_dir: &TempDir, suffix: &str) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join(format!("output_{suffix}.docx"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
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

    assert!(output.exists(), "Output DOCX file should be created");
    output
}

/// Run the CLI to emit IR JSON to a file.
/// Returns the path to the generated IR JSON file.
fn run_cli_to_ir(fixture_name: &str, temp_dir: &TempDir, suffix: &str) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join(format!("ir_{suffix}.json"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--emit-ir",
        output.to_str().unwrap(),
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

    assert!(output.exists(), "IR JSON file should be created");
    output
}

// =============================================================================
// IR JSON DETERMINISM TESTS
// =============================================================================
//
// These tests verify that the IR (Intermediate Representation) JSON output is
// byte-identical across multiple runs for the same input. This is critical for:
// - Reproducible builds
// - Reliable debugging and analysis
// - Consistent downstream processing

mod ir_determinism {
    use super::*;

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
}

// =============================================================================
// REPORT JSON DETERMINISM TESTS
// =============================================================================
//
// These tests verify that the Report JSON output is stable across multiple runs.
// The report includes warnings, statistics, and other metadata that must be
// consistently ordered and valued for the same input.

mod report_determinism {
    use super::*;

    /// Test Report JSON determinism for simple text.
    /// Simple case - report should be consistent for simple input.
    #[test]
    fn report_simple_text_is_deterministic() {
        let fixture = "text_simple_paragraph.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for simple text should be byte-identical across runs"
        );
    }

    /// Test Report JSON determinism for malformed input with warnings.
    /// Warning ordering and content must be stable.
    #[test]
    fn report_malformed_input_warnings_are_stable() {
        let fixture = "malformed_table_merge_controls_degraded.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for malformed input should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for invalid control words.
    /// Multiple warnings must be consistently ordered.
    #[test]
    fn report_invalid_control_words_warnings_are_stable() {
        let fixture = "malformed_invalid_control_words.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for invalid control words should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for repeated bad controls.
    /// Large numbers of warnings must be consistently ordered.
    #[test]
    fn report_repeated_bad_controls_warnings_are_stable() {
        let fixture = "malformed_repeated_bad_controls.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for repeated bad controls should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for orphan controls.
    /// Table-related warnings must be consistently ordered.
    #[test]
    fn report_orphan_controls_warnings_are_stable() {
        let fixture = "malformed_table_orphan_controls.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for orphan controls should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for list fallback.
    /// List-related warnings must be consistently ordered.
    #[test]
    fn report_list_fallback_warnings_are_stable() {
        let fixture = "malformed_list_fallback.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for list fallback should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for missing cell terminator.
    /// Table structure warnings must be consistently ordered.
    #[test]
    fn report_missing_cell_terminator_warnings_are_stable() {
        let fixture = "malformed_table_missing_cell_terminator.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for missing cell terminator should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for missing row terminator.
    /// Table structure warnings must be consistently ordered.
    #[test]
    fn report_missing_row_terminator_warnings_are_stable() {
        let fixture = "malformed_table_missing_row_terminator.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for missing row terminator should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for conflicting merge.
    /// Merge conflict warnings must be consistently ordered.
    #[test]
    fn report_conflicting_merge_warnings_are_stable() {
        let fixture = "malformed_table_conflicting_merge.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for conflicting merge should have stable warning ordering"
        );
    }

    /// Test Report JSON determinism for mixed complex content.
    /// Complex documents must have stable report output.
    #[test]
    fn report_mixed_complex_is_deterministic() {
        let fixture = "mixed_complex.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 3);

        assert!(
            verify_identical_outputs(&outputs),
            "Report JSON for mixed complex content should be byte-identical across runs"
        );
    }
}

// =============================================================================
// DOCX XML DETERMINISM TESTS
// =============================================================================
//
// These tests verify that the DOCX XML content is stable across multiple runs.
// Note: We compare extracted XML content, not raw DOCX bytes, because ZIP archive
// metadata (timestamps, compression details) may vary between runs.
//
// Key files tested:
// - word/document.xml: Main document content
// - word/numbering.xml: List numbering definitions (for list fixtures)

mod docx_xml_determinism {
    use super::*;

    /// Test document.xml determinism for simple text.
    /// Baseline case - any non-determinism here indicates a bug.
    #[test]
    fn docx_document_xml_simple_text_is_deterministic() {
        let fixture = "text_simple_paragraph.rtf";
        let temp_dir = TempDir::new().unwrap();

        // Generate DOCX files 3 times
        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        // Extract document.xml from each
        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        // Verify all are identical
        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for simple text should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for nested lists.
    /// List numbering references must be stable.
    #[test]
    fn docx_document_xml_nested_lists_is_deterministic() {
        let fixture = "list_nested_two_levels.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for nested lists should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test numbering.xml determinism for nested lists.
    /// List numbering definitions must be stable.
    #[test]
    fn docx_numbering_xml_nested_lists_is_deterministic() {
        let fixture = "list_nested_two_levels.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let num_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/numbering.xml"))
            .collect();

        let first = &num_xmls[0];
        for (i, xml) in num_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "numbering.xml for nested lists should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for horizontal merge table.
    /// Grid span values must be stable.
    #[test]
    fn docx_document_xml_horizontal_merge_is_deterministic() {
        let fixture = "table_horizontal_merge_valid.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for horizontal merge table should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for vertical merge table.
    /// Vertical merge values must be stable.
    #[test]
    fn docx_document_xml_vertical_merge_is_deterministic() {
        let fixture = "table_vertical_merge_valid.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for vertical merge table should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for degraded malformed input.
    /// Degraded content handling must produce stable output.
    #[test]
    fn docx_document_xml_degraded_content_is_deterministic() {
        let fixture = "malformed_table_merge_controls_degraded.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for degraded content should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for mixed complex content.
    /// Complex documents must produce stable output.
    #[test]
    fn docx_document_xml_mixed_complex_is_deterministic() {
        let fixture = "mixed_complex.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for mixed complex content should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for mixed list kinds.
    /// Multiple list types must produce stable output.
    #[test]
    fn docx_document_xml_mixed_list_kinds_is_deterministic() {
        let fixture = "list_mixed_kinds.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for mixed list kinds should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test numbering.xml determinism for mixed list kinds.
    /// Multiple list definitions must be stable.
    #[test]
    fn docx_numbering_xml_mixed_list_kinds_is_deterministic() {
        let fixture = "list_mixed_kinds.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let num_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/numbering.xml"))
            .collect();

        let first = &num_xmls[0];
        for (i, xml) in num_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "numbering.xml for mixed list kinds should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for table with list in cell.
    /// Nested content in table cells must produce stable output.
    #[test]
    fn docx_document_xml_table_with_list_in_cell_is_deterministic() {
        let fixture = "table_with_list_in_cell.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for table with list in cell should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for bullet list.
    /// Simple list must produce stable output.
    #[test]
    fn docx_document_xml_bullet_list_is_deterministic() {
        let fixture = "list_bullet_simple.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for bullet list should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for decimal list.
    /// Numbered list must produce stable output.
    #[test]
    fn docx_document_xml_decimal_list_is_deterministic() {
        let fixture = "list_decimal_simple.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for decimal list should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for simple 2x2 table.
    /// Basic table structure must be stable.
    #[test]
    fn docx_document_xml_simple_table_is_deterministic() {
        let fixture = "table_simple_2x2.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for simple table should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for conflicting merge.
    /// Merge conflict resolution must be deterministic.
    #[test]
    fn docx_document_xml_conflicting_merge_is_deterministic() {
        let fixture = "malformed_table_conflicting_merge.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for conflicting merge should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }

    /// Test document.xml determinism for large stress table.
    /// Large tables must produce stable output.
    #[test]
    fn docx_document_xml_stress_table_is_deterministic() {
        let fixture = "limits_table_stress.rtf";
        let temp_dir = TempDir::new().unwrap();

        let docx_paths: Vec<PathBuf> = (0..3)
            .map(|i| run_cli_to_docx(fixture, &temp_dir, &i.to_string()))
            .collect();

        let doc_xmls: Vec<String> = docx_paths
            .iter()
            .map(|path| extract_docx_xml(path, "word/document.xml"))
            .collect();

        let first = &doc_xmls[0];
        for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
            assert_eq!(
                first, xml,
                "document.xml for stress table should be byte-identical across runs (run 0 vs run {i})"
            );
        }
    }
}

// =============================================================================
// KNOWN NON-CONTRACTUAL VARIABILITY DOCUMENTATION
// =============================================================================
//
// The following variability is expected and documented:
//
// 1. DOCX ZIP METADATA: Raw DOCX bytes may differ between runs due to:
//    - ZIP file modification timestamps
//    - ZIP compression algorithm implementation details
//    - File ordering within the ZIP archive
//
//    MITIGATION: We compare extracted XML content, not raw DOCX bytes.
//
// 2. NO TIMESTAMPS IN OUTPUT: The rtfkit output does NOT include timestamps
//    or other non-deterministic values in:
//    - IR JSON (no timestamps)
//    - Report JSON (no timestamps)
//    - DOCX XML content (no timestamps in generated XML)
//
// 3. STABLE ORDERING: All collections are serialized in deterministic order:
//    - Warnings are ordered by occurrence in the source document
//    - Document blocks are ordered by position
//    - Table cells are ordered left-to-right
//    - List items are ordered by position
//
// If non-determinism is detected, it should be reported as a bug.

#[cfg(test)]
mod variability_documentation {
    use super::*;

    // This module exists solely to document known variability.
    // No tests are needed here - the documentation above serves as the record.

    /// Verify that IR JSON does not contain timestamps.
    /// This test ensures we don't accidentally introduce non-determinism.
    #[test]
    fn ir_json_contains_no_timestamps() {
        let fixture = "text_simple_paragraph.rtf";
        let temp_dir = TempDir::new().unwrap();
        let ir_path = run_cli_to_ir(fixture, &temp_dir, "timestamp_check");

        let content = std::fs::read_to_string(ir_path).expect("Failed to read IR file");

        // Check for common timestamp field names
        assert!(
            !content.contains("\"timestamp\""),
            "IR JSON should not contain timestamp fields"
        );
        assert!(
            !content.contains("\"created_at\""),
            "IR JSON should not contain created_at fields"
        );
        assert!(
            !content.contains("\"modified_at\""),
            "IR JSON should not contain modified_at fields"
        );
        assert!(
            !content.contains("\"date\""),
            "IR JSON should not contain date fields"
        );
        assert!(
            !content.contains("\"time\""),
            "IR JSON should not contain time fields"
        );
    }

    /// Verify that Report JSON does not contain timestamps.
    #[test]
    fn report_json_contains_no_timestamps() {
        let fixture = "text_simple_paragraph.rtf";
        let fixture_path = fixture_dir().join(fixture);
        let args = [
            "convert",
            fixture_path.to_str().unwrap(),
            "--format",
            "json",
        ];

        let outputs = run_rtfkit_multiple_times(&args, 1);
        let content = String::from_utf8_lossy(&outputs[0]);

        // Check for common timestamp field names
        assert!(
            !content.contains("\"timestamp\""),
            "Report JSON should not contain timestamp fields"
        );
        assert!(
            !content.contains("\"created_at\""),
            "Report JSON should not contain created_at fields"
        );
        assert!(
            !content.contains("\"modified_at\""),
            "Report JSON should not contain modified_at fields"
        );
        // Note: "date" and "time" might appear in warning reasons, so we don't check those
    }
}

// =============================================================================
// HTML OUTPUT DETERMINISM TESTS
// =============================================================================
//
// These tests verify that HTML output is deterministic across multiple runs.
// This is critical for the CSS Polish phase to ensure consistent styling output.

mod html_determinism {
    use super::*;

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
}

// =============================================================================
// PDF OUTPUT DETERMINISM TESTS
// =============================================================================
//
// These tests verify that PDF output is deterministic across multiple runs.
// This is critical for reproducible builds and reliable document generation.

mod pdf_determinism {
    use super::*;

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
}
