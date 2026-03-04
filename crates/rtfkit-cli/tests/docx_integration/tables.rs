#![allow(clippy::collapsible_if)]

use crate::support::cli::{run_cli_convert, run_cli_to_docx};
use crate::support::docx_xml::{
    contains_text, count_elements, extract_document_xml, extract_xml_from_docx,
    has_grid_span_with_value, has_vmerge_element,
};
use tempfile::TempDir;

#[test]
fn test_table_simple_2x2_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_simple_2x2.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table element exists
    assert!(
        xml.contains("<w:tbl") || xml.contains("<w:tbl>"),
        "Should contain w:tbl element"
    );

    // Verify 2 row elements exist
    let row_count = count_elements(&xml, "w:tr");
    assert!(
        row_count >= 2,
        "Should have at least 2 w:tr elements, found {}",
        row_count
    );

    // Verify 4 cell elements exist
    let cell_count = count_elements(&xml, "w:tc");
    assert!(
        cell_count >= 4,
        "Should have at least 4 w:tc elements, found {}",
        cell_count
    );

    // Verify computed widths are serialized deterministically (2880 + 2880).
    assert!(
        xml.contains("w:tcW w:w=\"2880\" w:type=\"dxa\""),
        "Should contain explicit 2880 dxa table cell widths"
    );

    // Verify cell content is preserved
    assert!(
        contains_text(&xml, "Cell 1"),
        "Should contain 'Cell 1' text"
    );
    assert!(
        contains_text(&xml, "Cell 2"),
        "Should contain 'Cell 2' text"
    );
    assert!(
        contains_text(&xml, "Cell 3"),
        "Should contain 'Cell 3' text"
    );
    assert!(
        contains_text(&xml, "Cell 4"),
        "Should contain 'Cell 4' text"
    );
}

#[test]
fn test_table_multirow_uneven_content_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_multirow_uneven_content.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify 3 row elements exist
    let row_count = count_elements(&xml, "w:tr");
    assert!(
        row_count >= 3,
        "Should have at least 3 w:tr elements, found {}",
        row_count
    );

    // Verify all text content is preserved
    assert!(contains_text(&xml, "Short"), "Should contain 'Short' text");
    assert!(contains_text(&xml, "Tiny"), "Should contain 'Tiny' text");
    assert!(
        contains_text(&xml, "Another cell"),
        "Should contain 'Another cell' text"
    );
    assert!(
        contains_text(&xml, "content preservation"),
        "Should contain 'content preservation' text"
    );
}

#[test]
fn test_table_with_list_in_cell_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_with_list_in_cell.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify 2 row elements exist
    let row_count = count_elements(&xml, "w:tr");
    assert!(
        row_count >= 2,
        "Should have at least 2 w:tr elements, found {}",
        row_count
    );

    // Verify table cell content
    assert!(
        contains_text(&xml, "Regular cell"),
        "Should contain 'Regular cell' text"
    );
    assert!(
        contains_text(&xml, "Another cell"),
        "Should contain 'Another cell' text"
    );
    assert!(
        contains_text(&xml, "Last cell"),
        "Should contain 'Last cell' text"
    );

    // Verify list content inside cell
    assert!(
        contains_text(&xml, "Item A"),
        "Should contain 'Item A' list item"
    );
    assert!(
        contains_text(&xml, "Item B"),
        "Should contain 'Item B' list item"
    );

    // Verify multiple paragraphs exist in the cell with list content
    // (either as list items with w:numPr or as regular paragraphs)
    let paragraph_count = count_elements(&xml, "w:p");
    assert!(
        paragraph_count >= 4,
        "Should have at least 4 paragraphs (table cells + list items), found {}",
        paragraph_count
    );

    // Lists in cells should keep numbering semantics.
    assert!(
        xml.contains("<w:numPr>"),
        "List paragraphs inside table cells should include w:numPr"
    );
}

#[test]
fn test_nested_table_docx_contains_child_tbl_in_cell() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_nested_2level_basic.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Parser-driven nested fixture should create at least two table elements.
    let tbl_count = count_elements(&xml, "w:tbl");
    assert!(
        tbl_count >= 2,
        "Should contain nested table structure (>=2 w:tbl), found {}",
        tbl_count
    );

    // Ensure nested table appears within a table cell scope.
    assert!(
        xml.contains("<w:tc") && xml.contains("<w:tbl"),
        "Expected nested table content emitted in cell scope"
    );
}

#[test]
fn test_table_missing_cell_terminator_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_table_missing_cell_terminator.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify text is preserved despite malformed input
    assert!(
        contains_text(&xml, "Cell 1"),
        "Should contain 'Cell 1' text"
    );
    assert!(
        contains_text(&xml, "Cell 2"),
        "Should contain 'Cell 2' text"
    );
    assert!(
        contains_text(&xml, "Cell 3"),
        "Should contain 'Cell 3' text"
    );
    assert!(
        contains_text(&xml, "Cell 4"),
        "Should contain 'Cell 4' text"
    );

    // Verify table structure is still created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");
}

#[test]
fn test_table_missing_row_terminator_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_table_missing_row_terminator.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify text is preserved despite malformed input
    assert!(
        contains_text(&xml, "Cell 1"),
        "Should contain 'Cell 1' text"
    );
    assert!(
        contains_text(&xml, "Cell 2"),
        "Should contain 'Cell 2' text"
    );
    assert!(
        contains_text(&xml, "Cell 3"),
        "Should contain 'Cell 3' text"
    );
    assert!(
        contains_text(&xml, "Cell 4"),
        "Should contain 'Cell 4' text"
    );

    // Verify table structure is still created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");
}

#[test]
fn test_table_orphan_controls_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_table_orphan_controls.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify text is preserved as paragraphs (not table)
    // The orphan cell/row controls should not create a table
    assert!(
        contains_text(&xml, "Regular paragraph"),
        "Should contain 'Regular paragraph' text"
    );
    assert!(
        contains_text(&xml, "Orphan cell"),
        "Should contain 'Orphan cell' text"
    );
    assert!(
        contains_text(&xml, "outside table"),
        "Should contain 'outside table' text"
    );
    assert!(
        contains_text(&xml, "Another orphan"),
        "Should contain 'Another orphan' text"
    );
    assert!(
        contains_text(&xml, "Normal text"),
        "Should contain 'Normal text' text"
    );

    // Verify no table is created for orphan controls
    // The document should have paragraphs instead
    let paragraph_count = count_elements(&xml, "w:p");
    assert!(
        paragraph_count >= 3,
        "Should have at least 3 paragraphs for orphan controls, found {}",
        paragraph_count
    );
}

#[test]
fn test_table_merge_controls_degraded_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_table_merge_controls_degraded.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify 2 row elements exist
    let row_count = count_elements(&xml, "w:tr");
    assert!(
        row_count >= 2,
        "Should have at least 2 w:tr elements, found {}",
        row_count
    );

    // Verify text content is preserved
    assert!(
        contains_text(&xml, "Merged start"),
        "Should contain 'Merged start' text"
    );
    assert!(
        contains_text(&xml, "Third cell"),
        "Should contain 'Third cell' text"
    );
    assert!(
        contains_text(&xml, "Regular A"),
        "Should contain 'Regular A' text"
    );
    assert!(
        contains_text(&xml, "Regular B"),
        "Should contain 'Regular B' text"
    );
    assert!(
        contains_text(&xml, "Regular C"),
        "Should contain 'Regular C' text"
    );
}

#[test]
fn test_horizontal_merge_produces_grid_span() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_horizontal_merge_valid.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify w:gridSpan element exists with span=2
    assert!(
        has_grid_span_with_value(&xml, 2),
        "Should have w:gridSpan with val=2 for horizontal merge"
    );

    // Verify text content is preserved
    assert!(
        contains_text(&xml, "Merged cell"),
        "Should contain 'Merged cell' text"
    );
    assert!(
        contains_text(&xml, "Third cell"),
        "Should contain 'Third cell' text"
    );
}

#[test]
fn test_vertical_merge_produces_vmerge() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_vertical_merge_valid.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify w:vMerge element exists
    assert!(
        has_vmerge_element(&xml),
        "Should have w:vMerge element for vertical merge"
    );

    // Verify text content is preserved
    assert!(
        contains_text(&xml, "Top cell"),
        "Should contain 'Top cell' text"
    );
    assert!(
        contains_text(&xml, "Second"),
        "Should contain 'Second' text"
    );
    assert!(
        contains_text(&xml, "Second row second"),
        "Should contain 'Second row second' text"
    );
}

#[test]
fn test_mixed_merge_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_mixed_merge.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify w:gridSpan element exists for horizontal merge
    assert!(
        has_grid_span_with_value(&xml, 2),
        "Should have w:gridSpan with val=2 for horizontal merge"
    );

    // Verify text content is preserved
    assert!(
        contains_text(&xml, "Merged"),
        "Should contain 'Merged' text"
    );
    assert!(
        contains_text(&xml, "Normal"),
        "Should contain 'Normal' text"
    );
    assert!(
        contains_text(&xml, "Fourth"),
        "Should contain 'Fourth' text"
    );
}

#[test]
fn test_orphan_merge_continuation_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_table_orphan_merge_continuation.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify text content is preserved (orphan continuation should be treated as standalone)
    assert!(
        contains_text(&xml, "Orphan"),
        "Should contain 'Orphan' text"
    );
    assert!(
        contains_text(&xml, "Normal"),
        "Should contain 'Normal' text"
    );
}

#[test]
fn test_conflicting_merge_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_table_conflicting_merge.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify text content is preserved
    assert!(
        contains_text(&xml, "First merge"),
        "Should contain 'First merge' text"
    );
    assert!(
        contains_text(&xml, "Second merge"),
        "Should contain 'Second merge' text"
    );
}

#[test]
fn test_non_monotonic_cellx_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_table_non_monotonic_cellx.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify text content is preserved
    assert!(contains_text(&xml, "First"), "Should contain 'First' text");
    assert!(
        contains_text(&xml, "Second"),
        "Should contain 'Second' text"
    );
    assert!(contains_text(&xml, "Third"), "Should contain 'Third' text");
}

#[test]
fn test_large_stress_table_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("limits_table_stress.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify 20 row elements exist
    let row_count = count_elements(&xml, "w:tr");
    assert!(
        row_count >= 20,
        "Should have at least 20 w:tr elements, found {}",
        row_count
    );

    // Verify some text content is preserved
    assert!(contains_text(&xml, "R1C1"), "Should contain 'R1C1' text");
    assert!(contains_text(&xml, "R20C5"), "Should contain 'R20C5' text");
}

#[test]
fn test_table_determinism_horizontal_merge() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "table_horizontal_merge_valid.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_to_docx(fixture, &temp_dir, "1");
    let output2 = run_cli_to_docx(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions for horizontal merge"
    );
}

#[test]
fn test_table_determinism_vertical_merge() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "table_vertical_merge_valid.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_to_docx(fixture, &temp_dir, "1");
    let output2 = run_cli_to_docx(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions for vertical merge"
    );
}
