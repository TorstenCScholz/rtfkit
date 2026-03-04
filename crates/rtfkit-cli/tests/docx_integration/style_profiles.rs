#![allow(clippy::collapsible_if)]

use crate::support::cli::{run_cli_convert, run_cli_convert_with_profile};
use crate::support::docx_xml::extract_xml_from_docx;
use tempfile::TempDir;

#[test]
fn test_style_profile_classic_doc_defaults_in_styles_xml() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("text_simple_paragraph.rtf", &temp_dir, "classic", "1");
    let styles_xml = extract_xml_from_docx(&docx, "word/styles.xml");

    // Classic profile: size_body = 12pt → 24 half-points; font = "Libertinus Serif"
    assert!(
        styles_xml.contains(r#"w:sz w:val="24""#),
        "Classic profile should set font size 24 half-points (12pt) in styles.xml: {styles_xml}"
    );
    assert!(
        styles_xml.contains(r#"w:rFonts w:ascii="Libertinus Serif" w:hAnsi="Libertinus Serif""#),
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
        styles_xml.contains(r#"w:sz w:val="18""#),
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
        styles_xml.contains(r#"w:sz w:val="22""#),
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
        numbering_xml.contains(r#"w:ind w:left="360" w:right="0" w:hanging="120""#),
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
        document_xml.contains(r#"w:shd w:val="clear" w:color="auto" w:fill="F4F6F8""#),
        "Report profile should apply stripe color F4F6F8 to alternate rows: {document_xml}"
    );
}

#[test]
fn test_style_profile_report_table_border_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("table_simple_2x2.rtf", &temp_dir, "report", "1");
    let document_xml = extract_xml_from_docx(&docx, "word/document.xml");

    // Report profile: table.border_width = 0.5pt -> 4 eighth-points, border color D1D5DB
    assert!(
        document_xml.contains(r#"w:top w:val="single" w:sz="4" w:space="0" w:color="D1D5DB""#),
        "Report profile should set table border size/color defaults in document.xml: {document_xml}"
    );
}

#[test]
fn test_style_profile_report_list_item_gap() {
    let temp_dir = TempDir::new().unwrap();
    let docx = run_cli_convert_with_profile("list_bullet_simple.rtf", &temp_dir, "report", "1");
    let document_xml = extract_xml_from_docx(&docx, "word/document.xml");

    // Report profile: list_item_gap = 6pt -> 120 twips
    assert!(
        document_xml.contains(r#"w:spacing w:after="120""#),
        "Report profile should set list item paragraph spacing in document.xml: {document_xml}"
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
