#![allow(clippy::collapsible_if)]

use crate::support::cli::{run_cli_convert, run_cli_to_docx};
use crate::support::docx_xml::{
    contains_text, count_elements, count_num_pr, extract_abstract_num_ids, extract_document_xml,
    extract_num_ids, extract_numbering_xml, extract_xml_from_docx, has_abstract_num,
    has_ilvl_with_value, has_num, has_num_pr,
};
use tempfile::TempDir;

#[test]
fn test_list_bullet_simple_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("list_bullet_simple.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify list paragraphs exist (w:numPr indicates list membership)
    assert!(
        has_num_pr(&xml),
        "Should have w:numPr elements for list paragraphs"
    );

    // Should have 3 list items
    let num_pr_count = count_num_pr(&xml);
    assert!(
        num_pr_count >= 3,
        "Should have at least 3 numPr elements for 3 list items, found {}",
        num_pr_count
    );

    // Verify text content
    assert!(
        contains_text(&xml, "First item"),
        "Should contain 'First item'"
    );
    assert!(
        contains_text(&xml, "Second item"),
        "Should contain 'Second item'"
    );
    assert!(
        contains_text(&xml, "Third item"),
        "Should contain 'Third item'"
    );

    // Verify numbering.xml exists and has proper structure
    if let Some(numbering_xml) = extract_numbering_xml(&docx_path) {
        assert!(
            has_abstract_num(&numbering_xml),
            "numbering.xml should have abstractNum definition"
        );
        assert!(
            has_num(&numbering_xml),
            "numbering.xml should have num instance"
        );
    }
}

#[test]
fn test_list_decimal_simple_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("list_decimal_simple.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify list paragraphs exist
    assert!(
        has_num_pr(&xml),
        "Should have w:numPr elements for list paragraphs"
    );

    // Should have 3 list items
    let num_pr_count = count_num_pr(&xml);
    assert!(
        num_pr_count >= 3,
        "Should have at least 3 numPr elements for 3 list items, found {}",
        num_pr_count
    );

    // Verify text content
    assert!(
        contains_text(&xml, "First item"),
        "Should contain 'First item'"
    );
    assert!(
        contains_text(&xml, "Second item"),
        "Should contain 'Second item'"
    );
    assert!(
        contains_text(&xml, "Third item"),
        "Should contain 'Third item'"
    );

    // Verify numbering.xml exists and has proper structure
    if let Some(numbering_xml) = extract_numbering_xml(&docx_path) {
        assert!(
            has_abstract_num(&numbering_xml),
            "numbering.xml should have abstractNum definition"
        );
        assert!(
            has_num(&numbering_xml),
            "numbering.xml should have num instance"
        );
    }
}

#[test]
fn test_list_nested_two_levels_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("list_nested_two_levels.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify list paragraphs exist
    assert!(
        has_num_pr(&xml),
        "Should have w:numPr elements for list paragraphs"
    );

    // Should have 6 list items total
    let num_pr_count = count_num_pr(&xml);
    assert!(
        num_pr_count >= 6,
        "Should have at least 6 numPr elements for 6 list items, found {}",
        num_pr_count
    );

    // Verify both level 0 and level 1 exist
    assert!(
        has_ilvl_with_value(&xml, 0),
        "Should have level 0 (top level) items"
    );
    assert!(
        has_ilvl_with_value(&xml, 1),
        "Should have level 1 (nested) items"
    );

    // Verify text content
    assert!(
        contains_text(&xml, "Top level item one"),
        "Should contain 'Top level item one'"
    );
    assert!(
        contains_text(&xml, "Nested item one"),
        "Should contain 'Nested item one'"
    );
    assert!(
        contains_text(&xml, "Top level item two"),
        "Should contain 'Top level item two'"
    );

    // Verify numbering.xml exists
    if let Some(numbering_xml) = extract_numbering_xml(&docx_path) {
        assert!(
            has_abstract_num(&numbering_xml),
            "numbering.xml should have abstractNum definition"
        );
    }
}

#[test]
fn test_list_mixed_kinds_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("list_mixed_kinds.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify list paragraphs exist
    assert!(
        has_num_pr(&xml),
        "Should have w:numPr elements for list paragraphs"
    );

    // Should have 6 list items total (3 bullet + 3 numbered)
    let num_pr_count = count_num_pr(&xml);
    assert!(
        num_pr_count >= 6,
        "Should have at least 6 numPr elements for 6 list items, found {}",
        num_pr_count
    );

    // Verify text content for both lists
    assert!(
        contains_text(&xml, "Bullet List:"),
        "Should contain 'Bullet List:'"
    );
    assert!(
        contains_text(&xml, "Bullet item one"),
        "Should contain 'Bullet item one'"
    );
    assert!(
        contains_text(&xml, "Ordered List:"),
        "Should contain 'Ordered List:'"
    );
    assert!(
        contains_text(&xml, "Numbered item one"),
        "Should contain 'Numbered item one'"
    );

    // Verify numbering.xml exists with multiple definitions
    if let Some(numbering_xml) = extract_numbering_xml(&docx_path) {
        // Should have abstractNum definitions for both list types
        let abstract_num_count = count_elements(&numbering_xml, "w:abstractNum");
        assert!(
            abstract_num_count >= 1,
            "numbering.xml should have at least 1 abstractNum definition, found {}",
            abstract_num_count
        );
    }
}

#[test]
fn test_list_malformed_fallback_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("malformed_list_fallback.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Malformed list references should still produce content
    // The document should have paragraphs with text content
    assert!(
        contains_text(&xml, "invalid list reference"),
        "Should contain text about invalid list reference"
    );
    assert!(
        contains_text(&xml, "Valid list item"),
        "Should contain 'Valid list item'"
    );
    assert!(
        contains_text(&xml, "Another invalid reference"),
        "Should contain 'Another invalid reference'"
    );

    // Should have paragraphs (either as list items or regular paragraphs)
    let paragraph_count = count_elements(&xml, "w:p");
    assert!(
        paragraph_count >= 3,
        "Should have at least 3 paragraphs, found {}",
        paragraph_count
    );
}

#[test]
fn test_list_determinism_nested_two_levels() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "list_nested_two_levels.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_to_docx(fixture, &temp_dir, "1");
    let output2 = run_cli_to_docx(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions"
    );

    // Extract and compare numbering.xml
    let num1 = extract_xml_from_docx(&output1, "word/numbering.xml");
    let num2 = extract_xml_from_docx(&output2, "word/numbering.xml");
    assert_eq!(
        num1, num2,
        "numbering.xml should be byte-identical across conversions"
    );

    // Verify numId values are stable
    let num_ids1 = extract_num_ids(&doc1);
    let num_ids2 = extract_num_ids(&doc2);
    assert_eq!(
        num_ids1, num_ids2,
        "numId values should be stable across conversions"
    );

    // Verify abstractNumId values are stable
    let abstract_ids1 = extract_abstract_num_ids(&num1);
    let abstract_ids2 = extract_abstract_num_ids(&num2);
    assert_eq!(
        abstract_ids1, abstract_ids2,
        "abstractNumId values should be stable across conversions"
    );
}

#[test]
fn test_list_determinism_mixed_kinds() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "list_mixed_kinds.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_to_docx(fixture, &temp_dir, "1");
    let output2 = run_cli_to_docx(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions"
    );

    // Extract and compare numbering.xml
    let num1 = extract_xml_from_docx(&output1, "word/numbering.xml");
    let num2 = extract_xml_from_docx(&output2, "word/numbering.xml");
    assert_eq!(
        num1, num2,
        "numbering.xml should be byte-identical across conversions"
    );

    // Verify numId values are stable
    let num_ids1 = extract_num_ids(&doc1);
    let num_ids2 = extract_num_ids(&doc2);
    assert_eq!(
        num_ids1, num_ids2,
        "numId values should be stable across conversions"
    );

    // Verify abstractNumId values are stable
    let abstract_ids1 = extract_abstract_num_ids(&num1);
    let abstract_ids2 = extract_abstract_num_ids(&num2);
    assert_eq!(
        abstract_ids1, abstract_ids2,
        "abstractNumId values should be stable across conversions"
    );
}

#[test]
fn test_list_determinism_bullet_simple() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "list_bullet_simple.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_to_docx(fixture, &temp_dir, "1");
    let output2 = run_cli_to_docx(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions"
    );

    // Extract and compare numbering.xml
    let num1 = extract_xml_from_docx(&output1, "word/numbering.xml");
    let num2 = extract_xml_from_docx(&output2, "word/numbering.xml");
    assert_eq!(
        num1, num2,
        "numbering.xml should be byte-identical across conversions"
    );
}

#[test]
fn test_list_determinism_decimal_simple() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "list_decimal_simple.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_to_docx(fixture, &temp_dir, "1");
    let output2 = run_cli_to_docx(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions"
    );

    // Extract and compare numbering.xml
    let num1 = extract_xml_from_docx(&output1, "word/numbering.xml");
    let num2 = extract_xml_from_docx(&output2, "word/numbering.xml");
    assert_eq!(
        num1, num2,
        "numbering.xml should be byte-identical across conversions"
    );
}
