use crate::support::cli::run_cli_to_docx;
use crate::support::docx_xml::extract_xml_from_docx;
use std::path::PathBuf;
use tempfile::TempDir;

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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/numbering.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/numbering.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
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
        .map(|path| extract_xml_from_docx(path, "word/document.xml"))
        .collect();

    let first = &doc_xmls[0];
    for (i, xml) in doc_xmls.iter().enumerate().skip(1) {
        assert_eq!(
            first, xml,
            "document.xml for stress table should be byte-identical across runs (run 0 vs run {i})"
        );
    }
}
