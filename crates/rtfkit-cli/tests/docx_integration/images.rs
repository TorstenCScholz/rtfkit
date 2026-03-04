#![allow(clippy::collapsible_if)]

use crate::support::cli::{run_cli_convert, run_cli_convert_determinism};
use crate::support::docx_xml::{
    extract_document_xml, extract_rels_xml, extract_xml_from_docx, file_exists_in_docx,
    list_docx_files,
};
use tempfile::TempDir;

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
