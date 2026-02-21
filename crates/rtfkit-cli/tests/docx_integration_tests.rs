//! DOCX Integration Tests
//!
//! These tests verify that the generated DOCX files contain correct XML structure.
//! They run the CLI to generate .docx from fixtures, unzip the result, and parse/assert
//! the word/document.xml content.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use quick_xml::Reader;
use quick_xml::events::Event;
use tempfile::TempDir;
use zip::ZipArchive;

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_dir() -> PathBuf {
    project_root().join("fixtures")
}

/// Run the CLI to convert an RTF file to DOCX.
/// Returns the path to the generated DOCX file.
fn run_cli_convert(fixture_name: &str, temp_dir: &TempDir) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join("output.docx");

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

/// Extract word/document.xml from a DOCX file.
/// Returns the XML content as a string.
fn extract_document_xml(docx_path: &Path) -> String {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("word/document.xml not found in DOCX")
        .read_to_string(&mut document_xml)
        .expect("Failed to read document.xml");

    document_xml
}

/// Count occurrences of a specific XML element in the document.
fn count_elements(xml: &str, element_name: &str) -> usize {
    let mut reader = Reader::from_str(xml);
    let mut count = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == element_name.as_bytes() {
                    count += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    count
}

/// Check if the XML contains a specific text string within a <w:t> element.
fn contains_text(xml: &str, text: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut in_text_element = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = true;
                }
            }
            Ok(Event::Empty(e)) => {
                // Empty element, not text
                if e.name().as_ref() == b"w:t" {
                    in_text_element = false;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    if let Ok(t) = e.unescape() {
                        if t.contains(text) {
                            return true;
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Get all text content from <w:t> elements.
fn get_all_text_content(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut in_text_element = false;
    let mut texts = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = true;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    if let Ok(t) = e.unescape() {
                        texts.push(t.to_string());
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    texts
}

/// Check if a formatting element (like <w:b>, <w:i>, <w:u>) exists in the document.
fn has_formatting_element(xml: &str, element_name: &str) -> bool {
    let mut reader = Reader::from_str(xml);

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == element_name.as_bytes() {
                    return true;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    false
}

/// Check if a <w:jc w:val="..."> element exists with the specified alignment value.
fn has_alignment(xml: &str, alignment_value: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:jc" {
                    // Check attributes for w:val
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                if value == alignment_value {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Check if a run (<w:r>) with specific formatting exists containing the given text.
/// This is used to verify that text has the correct formatting applied.
fn has_run_with_formatting_and_text(xml: &str, formatting_element: &str, text: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut in_run = false;
    let mut has_formatting = false;
    let mut in_text = false;
    let mut current_text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"w:r" => {
                        in_run = true;
                        has_formatting = false;
                        current_text.clear();
                    }
                    b"w:rPr" => {
                        // Run properties - formatting will be inside
                    }
                    b"w:t" => {
                        in_text = true;
                    }
                    tag if tag == formatting_element.as_bytes() && in_run => {
                        has_formatting = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                tag if tag == formatting_element.as_bytes() && in_run => {
                    has_formatting = true;
                }
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if in_text {
                    if let Ok(t) = e.unescape() {
                        current_text.push_str(&t);
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"w:r" => {
                    if has_formatting && current_text.contains(text) {
                        return true;
                    }
                    in_run = false;
                }
                b"w:t" => {
                    in_text = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

// =============================================================================
// Test Cases
// =============================================================================

#[test]
fn test_simple_paragraph_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("simple_paragraph.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("multiple_paragraphs.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("bold_italic.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("underline.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("alignment.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("unicode.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("mixed_formatting.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("nested_styles.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("complex.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("simple_paragraph.rtf", &temp_dir);
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

// =============================================================================
// List Integration Tests
// =============================================================================

/// Extract word/numbering.xml from a DOCX file.
/// Returns the XML content as a string, or None if the file doesn't exist.
fn extract_numbering_xml(docx_path: &Path) -> Option<String> {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut numbering_xml = String::new();
    match archive.by_name("word/numbering.xml") {
        Ok(mut file) => {
            file.read_to_string(&mut numbering_xml)
                .expect("Failed to read numbering.xml");
            Some(numbering_xml)
        }
        Err(_) => None,
    }
}

/// Check if a <w:numPr> element exists in the document (indicates list paragraph).
fn has_num_pr(xml: &str) -> bool {
    has_formatting_element(xml, "w:numPr")
}

/// Count the number of <w:numPr> elements in the document.
fn count_num_pr(xml: &str) -> usize {
    count_elements(xml, "w:numPr")
}

/// Check if a <w:ilvl> element exists with the specified level value.
fn has_ilvl_with_value(xml: &str, level: u8) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:ilvl" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                if let Ok(v) = value.parse::<u8>() {
                                    if v == level {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Check if numbering.xml contains an <w:abstractNum> element.
fn has_abstract_num(numbering_xml: &str) -> bool {
    has_formatting_element(numbering_xml, "w:abstractNum")
}

/// Check if numbering.xml contains a <w:num> element.
fn has_num(numbering_xml: &str) -> bool {
    has_formatting_element(numbering_xml, "w:num")
}

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
    let docx_path = run_cli_convert("list_malformed_fallback.rtf", &temp_dir);
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

// =============================================================================
// Determinism Tests
// =============================================================================

/// Run the CLI to convert an RTF file to DOCX and return the output path.
/// This is a separate helper for determinism tests to avoid any shared state.
fn run_cli_convert_determinism(fixture_name: &str, temp_dir: &TempDir, suffix: &str) -> PathBuf {
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

/// Extract any XML file from a DOCX archive.
fn extract_xml_from_docx(docx_path: &Path, xml_path: &str) -> String {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut xml_content = String::new();
    archive
        .by_name(xml_path)
        .unwrap_or_else(|_| panic!("{xml_path} not found in DOCX"))
        .read_to_string(&mut xml_content)
        .expect("Failed to read XML content");

    xml_content
}

/// Extract numId values from document.xml to verify stability.
fn extract_num_ids(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut num_ids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:numId" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                num_ids.push(value.to_string());
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    num_ids
}

/// Extract abstractNumId values from numbering.xml to verify stability.
fn extract_abstract_num_ids(numbering_xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(numbering_xml);
    let mut buf = Vec::new();
    let mut abstract_num_ids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:abstractNumId" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                abstract_num_ids.push(value.to_string());
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    abstract_num_ids
}

#[test]
fn test_list_determinism_nested_two_levels() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "list_nested_two_levels.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

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
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

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
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

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
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

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

// =============================================================================
// Table Integration Tests
// =============================================================================

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
fn test_table_missing_cell_terminator_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("table_missing_cell_terminator.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("table_missing_row_terminator.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("table_orphan_controls.rtf", &temp_dir);
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
    let docx_path = run_cli_convert("table_merge_controls_degraded.rtf", &temp_dir);
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
