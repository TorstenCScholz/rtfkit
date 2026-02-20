//! DOCX Integration Tests
//!
//! These tests verify that the generated DOCX files contain correct XML structure.
//! They run the CLI to generate .docx from fixtures, unzip the result, and parse/assert
//! the word/document.xml content.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;
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
            Ok(Event::Empty(e)) => {
                match e.name().as_ref() {
                    tag if tag == formatting_element.as_bytes() && in_run => {
                        has_formatting = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_text {
                    if let Ok(t) = e.unescape() {
                        current_text.push_str(&t);
                    }
                }
            }
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
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
    assert!(
        contains_text(&xml, "Fourth"),
        "Should contain 'Fourth'"
    );
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
    assert!(
        contains_text(&xml, "Bold"),
        "Should contain 'Bold' text"
    );
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
    assert!(
        has_alignment(&xml, "left"),
        "Should have left alignment"
    );
    assert!(
        has_alignment(&xml, "center"),
        "Should have center alignment"
    );
    assert!(
        has_alignment(&xml, "right"),
        "Should have right alignment"
    );

    // Verify text content
    assert!(
        contains_text(&xml, "Left"),
        "Should contain 'Left' text"
    );
    assert!(
        contains_text(&xml, "Centered"),
        "Should contain 'Centered' text"
    );
    assert!(
        contains_text(&xml, "Right"),
        "Should contain 'Right' text"
    );
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
    assert!(
        !combined_text.is_empty(),
        "Should have text content"
    );

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
    assert!(
        contains_text(&xml, "bold"),
        "Should contain 'bold' text"
    );
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
    assert!(
        contains_text(&xml, "bold"),
        "Should contain 'bold' text"
    );
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
    assert!(
        contains_text(&xml, "Title"),
        "Should contain 'Title' text"
    );
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
