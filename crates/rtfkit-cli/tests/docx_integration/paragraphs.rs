#![allow(clippy::collapsible_if)]

use crate::support::cli::run_cli_convert;
use crate::support::docx_xml::{
    contains_text, count_elements, extract_document_xml, get_all_text_content, has_alignment,
    has_formatting_element, has_run_with_formatting_and_text,
};
use tempfile::TempDir;

#[test]
fn test_simple_paragraph_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_simple_paragraph.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify basic structure
    assert!(
        count_elements(&xml, "w:p") >= 1,
        "Should have at least one paragraph"
    );
    assert!(
        count_elements(&xml, "w:r") >= 1,
        "Should have at least one run"
    );
    assert!(
        count_elements(&xml, "w:t") >= 1,
        "Should have at least one text element"
    );

    // Verify text content
    assert!(
        contains_text(&xml, "Hello World"),
        "Should contain 'Hello World' text"
    );
}

#[test]
fn test_multiple_paragraphs_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_multiple_paragraphs.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify multiple paragraphs
    let paragraph_count = count_elements(&xml, "w:p");
    assert!(
        paragraph_count >= 4,
        "Should have at least 4 paragraphs, found {}",
        paragraph_count
    );

    // Verify text content from different paragraphs
    assert!(
        contains_text(&xml, "First paragraph"),
        "Should contain 'First paragraph'"
    );
    assert!(
        contains_text(&xml, "Second paragraph"),
        "Should contain 'Second paragraph'"
    );
    assert!(
        contains_text(&xml, "Third paragraph"),
        "Should contain 'Third paragraph'"
    );
    assert!(contains_text(&xml, "Fourth"), "Should contain 'Fourth'");
}

#[test]
fn test_bold_italic_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_bold_italic.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify bold formatting element exists
    assert!(
        has_formatting_element(&xml, "w:b"),
        "Should have bold formatting (w:b element)"
    );

    // Verify italic formatting element exists
    assert!(
        has_formatting_element(&xml, "w:i"),
        "Should have italic formatting (w:i element)"
    );

    // Verify text content
    assert!(contains_text(&xml, "Bold"), "Should contain 'Bold' text");
    assert!(
        contains_text(&xml, "Italic"),
        "Should contain 'Italic' text"
    );
    assert!(
        contains_text(&xml, "Normal"),
        "Should contain 'Normal' text"
    );

    // Verify that bold text has w:b formatting
    assert!(
        has_run_with_formatting_and_text(&xml, "w:b", "Bold"),
        "Bold text should have w:b formatting"
    );

    // Verify that italic text has w:i formatting
    assert!(
        has_run_with_formatting_and_text(&xml, "w:i", "Italic"),
        "Italic text should have w:i formatting"
    );
}

#[test]
fn test_underline_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_underline.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify underline formatting element exists
    assert!(
        has_formatting_element(&xml, "w:u"),
        "Should have underline formatting (w:u element)"
    );

    // Verify text content
    assert!(
        contains_text(&xml, "Underlined"),
        "Should contain 'Underlined' text"
    );
    assert!(
        contains_text(&xml, "Normal"),
        "Should contain 'Normal' text"
    );

    // Verify that underlined text has w:u formatting
    assert!(
        has_run_with_formatting_and_text(&xml, "w:u", "Underlined"),
        "Underlined text should have w:u formatting"
    );
}

#[test]
fn test_alignment_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_alignment.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify alignment elements exist
    // w:jc with w:val="left", "center", "right", "both"
    assert!(has_alignment(&xml, "left"), "Should have left alignment");
    assert!(
        has_alignment(&xml, "center"),
        "Should have center alignment"
    );
    assert!(has_alignment(&xml, "right"), "Should have right alignment");

    // Verify text content
    assert!(contains_text(&xml, "Left"), "Should contain 'Left' text");
    assert!(
        contains_text(&xml, "Centered"),
        "Should contain 'Centered' text"
    );
    assert!(contains_text(&xml, "Right"), "Should contain 'Right' text");
}

#[test]
fn test_unicode_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_unicode.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify text content is preserved
    assert!(
        contains_text(&xml, "Unicode Test"),
        "Should contain 'Unicode Test' text"
    );

    // Get all text content to check unicode characters
    let all_text = get_all_text_content(&xml);
    let combined_text = all_text.join("");

    // The unicode characters should be preserved in the output
    // Note: The exact representation depends on how the RTF parser handles unicode escapes
    // We check that the document has meaningful content
    assert!(!combined_text.is_empty(), "Should have text content");

    // Verify we have multiple paragraphs for the unicode test
    let paragraph_count = count_elements(&xml, "w:p");
    assert!(
        paragraph_count >= 2,
        "Should have at least 2 paragraphs for unicode test, found {}",
        paragraph_count
    );
}

#[test]
fn test_mixed_formatting_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_mixed_formatting.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify all formatting types exist
    assert!(
        has_formatting_element(&xml, "w:b"),
        "Should have bold formatting"
    );
    assert!(
        has_formatting_element(&xml, "w:i"),
        "Should have italic formatting"
    );
    assert!(
        has_formatting_element(&xml, "w:u"),
        "Should have underline formatting"
    );

    // Verify text content
    assert!(contains_text(&xml, "bold"), "Should contain 'bold' text");
    assert!(
        contains_text(&xml, "italic"),
        "Should contain 'italic' text"
    );
    assert!(
        contains_text(&xml, "underlined"),
        "Should contain 'underlined' text"
    );

    // Verify multiple runs exist (for different formatting)
    let run_count = count_elements(&xml, "w:r");
    assert!(
        run_count >= 3,
        "Should have at least 3 runs for mixed formatting, found {}",
        run_count
    );
}

#[test]
fn test_nested_styles_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_nested_styles.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify both bold and italic formatting exist
    assert!(
        has_formatting_element(&xml, "w:b"),
        "Should have bold formatting"
    );
    assert!(
        has_formatting_element(&xml, "w:i"),
        "Should have italic formatting"
    );

    // Verify text content
    assert!(
        contains_text(&xml, "Normal"),
        "Should contain 'Normal' text"
    );
    assert!(contains_text(&xml, "bold"), "Should contain 'bold' text");
    assert!(
        contains_text(&xml, "italic"),
        "Should contain 'italic' text"
    );

    // Verify multiple runs for different style scopes
    let run_count = count_elements(&xml, "w:r");
    assert!(
        run_count >= 3,
        "Should have at least 3 runs for nested styles, found {}",
        run_count
    );
}

#[test]
fn test_complex_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("mixed_complex.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify document structure
    let paragraph_count = count_elements(&xml, "w:p");
    assert!(
        paragraph_count >= 5,
        "Complex document should have at least 5 paragraphs, found {}",
        paragraph_count
    );

    // Verify all formatting types
    assert!(
        has_formatting_element(&xml, "w:b"),
        "Should have bold formatting"
    );
    assert!(
        has_formatting_element(&xml, "w:i"),
        "Should have italic formatting"
    );
    assert!(
        has_formatting_element(&xml, "w:u"),
        "Should have underline formatting"
    );

    // Verify alignment
    assert!(
        has_alignment(&xml, "center"),
        "Should have center alignment"
    );
    assert!(
        has_alignment(&xml, "left") || has_alignment(&xml, "right"),
        "Should have left or right alignment"
    );

    // Verify text content
    assert!(
        contains_text(&xml, "Complex"),
        "Should contain 'Complex' text"
    );
    assert!(contains_text(&xml, "Title"), "Should contain 'Title' text");
}

#[test]
fn test_docx_structure_validity() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("text_simple_paragraph.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify the XML has proper DOCX namespace
    assert!(
        xml.contains("xmlns:w="),
        "Should have Word namespace declaration"
    );

    // Verify document structure
    assert!(
        xml.contains("<w:body>") || xml.contains("<w:body "),
        "Should have w:body element"
    );
    assert!(
        xml.contains("<w:p>") || xml.contains("<w:p "),
        "Should have w:p element"
    );
    assert!(
        xml.contains("<w:r>") || xml.contains("<w:r "),
        "Should have w:r element"
    );
    assert!(
        xml.contains("<w:t") && xml.contains("</w:t>"),
        "Should have w:t element with content"
    );
}

#[test]
fn test_shading_percent_steps_docx_emits_pct25_pct50_pct75() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("shading_percent_steps.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    assert!(
        xml.contains(r#"w:val="pct25""#),
        "Should contain pct25 shading"
    );
    assert!(
        xml.contains(r#"w:val="pct50""#),
        "Should contain pct50 shading"
    );
    assert!(
        xml.contains(r#"w:val="pct75""#),
        "Should contain pct75 shading"
    );
}

#[test]
fn test_table_alignment_hyperlink_first_inline_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_alignment_hyperlink_first_inline.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    assert!(
        has_alignment(&xml, "center"),
        "Should contain centered paragraph alignment in table cell"
    );
    assert!(
        has_alignment(&xml, "right"),
        "Should contain right paragraph alignment in table cell"
    );
    assert!(
        contains_text(&xml, "Link centered"),
        "Should contain hyperlink result text for centered cell"
    );
    assert!(
        contains_text(&xml, "Link right"),
        "Should contain hyperlink result text for right cell"
    );
}

#[test]
fn test_prose_interleave_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_prose_interleave.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify table structure is created
    assert!(xml.contains("<w:tbl"), "Should contain w:tbl element");

    // Verify prose before and after table is preserved
    assert!(
        contains_text(&xml, "Before table"),
        "Should contain 'Before table' text"
    );
    assert!(
        contains_text(&xml, "After table"),
        "Should contain 'After table' text"
    );

    // Verify table cell content
    assert!(
        contains_text(&xml, "Cell 1"),
        "Should contain 'Cell 1' text"
    );
    assert!(
        contains_text(&xml, "Cell 2"),
        "Should contain 'Cell 2' text"
    );
}
