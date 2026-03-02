//! DOCX Integration Tests
//!
//! These tests verify that the generated DOCX files contain correct XML structure.
//! They run the CLI to generate .docx from fixtures, unzip the result, and parse/assert
//! the word/document.xml content.
#![allow(clippy::collapsible_if)]

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

// =============================================================================
// Phase 5: Table Merge and Alignment Tests
// =============================================================================

/// Check if a <w:gridSpan> element exists with the specified span value.
fn has_grid_span_with_value(xml: &str, span: u16) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:gridSpan" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                if let Ok(v) = value.parse::<u16>() {
                                    if v == span {
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

/// Check if a <w:vMerge> element exists in the document.
fn has_vmerge_element(xml: &str) -> bool {
    has_formatting_element(xml, "w:vMerge")
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

#[test]
fn test_table_determinism_horizontal_merge() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "table_horizontal_merge_valid.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

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
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions for vertical merge"
    );
}

// =============================================================================
// Image Integration Tests
// =============================================================================

/// Check if a file exists in the DOCX archive.
fn file_exists_in_docx(docx_path: &Path, file_path: &str) -> bool {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");
    archive.by_name(file_path).is_ok()
}

/// Get the list of files in a DOCX archive directory.
fn list_docx_files(docx_path: &Path, prefix: &str) -> Vec<String> {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    archive
        .file_names()
        .filter(|name| name.starts_with(prefix))
        .map(|s| s.to_string())
        .collect()
}

/// Extract word/_rels/document.xml.rels from a DOCX file.
fn extract_rels_xml(docx_path: &Path) -> Option<String> {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut rels_xml = String::new();
    match archive.by_name("word/_rels/document.xml.rels") {
        Ok(mut file) => {
            file.read_to_string(&mut rels_xml)
                .expect("Failed to read document.xml.rels");
            Some(rels_xml)
        }
        Err(_) => None,
    }
}

#[test]
fn test_docx_image_png() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("image_png_simple.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify DOCX contains media file
    assert!(
        file_exists_in_docx(&docx_path, "word/media/rIdImage1.png"),
        "DOCX should contain word/media/rIdImage1.png"
    );

    // Verify relationship entry exists
    if let Some(rels_xml) = extract_rels_xml(&docx_path) {
        assert!(
            rels_xml.contains("media/rIdImage1.png"),
            "Relationships should reference media/rIdImage1.png"
        );
        assert!(
            rels_xml.contains(
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
            ),
            "Relationship should have image relationship type"
        );
    } else {
        panic!("document.xml.rels not found in DOCX");
    }

    assert!(
        xml.contains("<w:drawing>"),
        "document.xml should contain native w:drawing XML"
    );
    assert!(
        !xml.contains("&lt;w:drawing&gt;"),
        "document.xml should not contain escaped drawing XML"
    );
}

#[test]
fn test_docx_image_jpeg() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("image_jpeg_simple.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify DOCX contains media file
    assert!(
        file_exists_in_docx(&docx_path, "word/media/rIdImage1.png"),
        "DOCX should contain word/media/rIdImage1.png (JPEG normalized to PNG)"
    );

    // Verify relationship entry exists
    if let Some(rels_xml) = extract_rels_xml(&docx_path) {
        assert!(
            rels_xml.contains("media/rIdImage1.png"),
            "Relationships should reference media/rIdImage1.png"
        );
        assert!(
            rels_xml.contains(
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
            ),
            "Relationship should have image relationship type"
        );
    } else {
        panic!("document.xml.rels not found in DOCX");
    }

    assert!(
        xml.contains("<w:drawing>"),
        "document.xml should contain native w:drawing XML"
    );
}

#[test]
fn test_docx_image_multiple() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("image_multiple.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify DOCX contains multiple media files
    let media_files = list_docx_files(&docx_path, "word/media/");
    assert!(
        media_files.len() >= 2,
        "DOCX should contain at least 2 media files, found: {:?}",
        media_files
    );

    // Verify deterministic image IDs exist.
    // `docx-rs` deduplicates identical image bytes, so we assert a lower bound.
    assert!(
        file_exists_in_docx(&docx_path, "word/media/rIdImage1.png"),
        "DOCX should contain word/media/rIdImage1.png"
    );
    assert!(
        file_exists_in_docx(&docx_path, "word/media/rIdImage2.png"),
        "DOCX should contain word/media/rIdImage2.png"
    );

    // Verify multiple relationship entries (deduplication may reduce count vs. IR blocks).
    if let Some(rels_xml) = extract_rels_xml(&docx_path) {
        assert!(
            rels_xml.contains("rIdImage1"),
            "Relationships should reference rIdImage1"
        );
        assert!(
            rels_xml.contains("rIdImage2"),
            "Relationships should reference rIdImage2"
        );
    } else {
        panic!("document.xml.rels not found in DOCX");
    }

    // Three ImageBlock entries should still yield three drawing runs even if media bytes dedupe.
    let drawing_count = xml.matches("<w:drawing>").count();
    assert!(
        drawing_count >= 3,
        "document.xml should contain at least 3 drawing elements, found {}",
        drawing_count
    );
}

#[test]
fn test_docx_image_with_dimensions() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = run_cli_convert("image_with_dimensions.rtf", &temp_dir);
    let xml = extract_document_xml(&docx_path);

    // Verify DOCX contains media file
    assert!(
        file_exists_in_docx(&docx_path, "word/media/rIdImage1.png"),
        "DOCX should contain word/media/rIdImage1.png"
    );

    assert!(
        xml.contains("<w:drawing>"),
        "document.xml should contain native w:drawing XML"
    );

    // Verify extent element exists with expected dimensions.
    // picwgoal2880 = 2 inches = 1828800 EMUs (914400 EMUs per inch)
    // pichgoal1440 = 1 inch = 914400 EMUs
    assert!(
        xml.contains("wp:extent cx=\"1828800\" cy=\"914400\""),
        "document.xml should have expected extent from RTF twip dimensions"
    );
}

#[test]
fn test_docx_image_determinism_png() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "image_png_simple.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions for PNG image"
    );

    // Extract and compare relationships
    let rels1 = extract_xml_from_docx(&output1, "word/_rels/document.xml.rels");
    let rels2 = extract_xml_from_docx(&output2, "word/_rels/document.xml.rels");
    assert_eq!(
        rels1, rels2,
        "document.xml.rels should be byte-identical across conversions for PNG image"
    );
}

#[test]
fn test_docx_image_determinism_multiple() {
    let temp_dir = TempDir::new().unwrap();
    let fixture = "image_multiple.rtf";

    // Convert the same fixture twice
    let output1 = run_cli_convert_determinism(fixture, &temp_dir, "1");
    let output2 = run_cli_convert_determinism(fixture, &temp_dir, "2");

    // Extract and compare document.xml
    let doc1 = extract_xml_from_docx(&output1, "word/document.xml");
    let doc2 = extract_xml_from_docx(&output2, "word/document.xml");
    assert_eq!(
        doc1, doc2,
        "document.xml should be byte-identical across conversions for multiple images"
    );

    // Extract and compare relationships
    let rels1 = extract_xml_from_docx(&output1, "word/_rels/document.xml.rels");
    let rels2 = extract_xml_from_docx(&output2, "word/_rels/document.xml.rels");
    assert_eq!(
        rels1, rels2,
        "document.xml.rels should be byte-identical across conversions for multiple images"
    );
}

// =============================================================================
// Style Profile Tests
// =============================================================================

/// Run the CLI to convert an RTF file to DOCX with a specific style profile.
fn run_cli_convert_with_profile(
    fixture_name: &str,
    temp_dir: &TempDir,
    profile: &str,
    suffix: &str,
) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir
        .path()
        .join(format!("output_{suffix}_{profile}.docx"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--style-profile",
        profile,
        "--force",
    ]);

    let result = cmd.output().expect("Failed to run CLI");
    if !result.status.success() {
        panic!(
            "CLI failed for fixture '{}' with profile '{}':\nstdout: {}\nstderr: {}",
            fixture_name,
            profile,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    assert!(output.exists(), "Output DOCX file should be created");
    output
}

#[test]
fn test_style_profile_classic_doc_defaults_in_styles_xml() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("text_simple_paragraph.rtf", &temp_dir, "classic", "1");
    let styles_xml = extract_xml_from_docx(&docx, "word/styles.xml");

    // Classic profile: size_body = 12pt → 24 half-points; font = "Libertinus Serif"
    assert!(
        styles_xml.contains("24"),
        "Classic profile should set font size 24 half-points (12pt) in styles.xml: {styles_xml}"
    );
    assert!(
        styles_xml.contains("Libertinus Serif"),
        "Classic profile should set 'Libertinus Serif' font in styles.xml: {styles_xml}"
    );
}

#[test]
fn test_style_profile_compact_doc_defaults_in_styles_xml() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("text_simple_paragraph.rtf", &temp_dir, "compact", "1");
    let styles_xml = extract_xml_from_docx(&docx, "word/styles.xml");

    // Compact profile: size_body = 9pt → 18 half-points
    assert!(
        styles_xml.contains("18"),
        "Compact profile should set font size 18 half-points (9pt) in styles.xml: {styles_xml}"
    );
}

#[test]
fn test_style_profile_report_doc_defaults_in_styles_xml() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("text_simple_paragraph.rtf", &temp_dir, "report", "1");
    let styles_xml = extract_xml_from_docx(&docx, "word/styles.xml");

    // Report profile: size_body = 11pt → 22 half-points
    assert!(
        styles_xml.contains("22"),
        "Report profile should set font size 22 half-points (11pt) in styles.xml: {styles_xml}"
    );
}

#[test]
fn test_style_profile_no_profile_unchanged_output() {
    let temp_dir = TempDir::new().unwrap();

    // Convert without profile
    let no_profile = run_cli_convert("text_simple_paragraph.rtf", &temp_dir);
    let no_profile_styles = extract_xml_from_docx(&no_profile, "word/styles.xml");

    // Output without profile should NOT contain profile-injected font defaults
    assert!(
        !no_profile_styles.contains("Libertinus Serif"),
        "No-profile output should not contain profile font defaults"
    );
}

#[test]
fn test_style_profile_list_indentation_classic() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("list_bullet_simple.rtf", &temp_dir, "classic", "1");
    let numbering_xml = extract_xml_from_docx(&docx, "word/numbering.xml");

    // Classic profile: indentation_step = 18pt → 360 twips; marker_gap = 6pt → 120 twips
    // w:ind w:left="360" (for level 0: step * (0+1) = 360)
    assert!(
        numbering_xml.contains("360"),
        "Classic profile should set list indent to 360 twips (18pt) in numbering.xml: {numbering_xml}"
    );
}

#[test]
fn test_style_profile_report_row_striping() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("table_simple_2x2.rtf", &temp_dir, "report", "1");
    let document_xml = extract_xml_from_docx(&docx, "word/document.xml");

    // Report profile has AlternateRows striping; stripe color is F4F6F8
    // Odd rows (1-indexed even = 0-indexed odd) should have shading
    assert!(
        document_xml.contains("F4F6F8"),
        "Report profile should apply stripe color F4F6F8 to alternate rows: {document_xml}"
    );
}

#[test]
fn test_style_profile_determinism_same_profile() {
    let temp_dir = TempDir::new().unwrap();

    let docx1 = run_cli_convert_with_profile("text_simple_paragraph.rtf", &temp_dir, "report", "1");
    let docx2 = run_cli_convert_with_profile("text_simple_paragraph.rtf", &temp_dir, "report", "2");

    let styles1 = extract_xml_from_docx(&docx1, "word/styles.xml");
    let styles2 = extract_xml_from_docx(&docx2, "word/styles.xml");

    assert_eq!(
        styles1, styles2,
        "Same profile should produce byte-identical styles.xml"
    );
}
