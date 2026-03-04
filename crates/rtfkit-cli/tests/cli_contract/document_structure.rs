use std::fs;
use std::io::Read;

use tempfile::tempdir;

fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn emit_ir(fixture: &std::path::Path) -> serde_json::Value {
    let dir = tempdir().unwrap();
    let ir_output = dir.path().join("ir.json");
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        ir_output.to_str().unwrap(),
    ]);
    cmd.assert().success();
    let json = fs::read_to_string(&ir_output).unwrap();
    serde_json::from_str(&json).expect("IR should be valid JSON")
}

fn convert_to_html(fixture: &std::path::Path) -> String {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("output.html");
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
    ]);
    cmd.assert().success();
    fs::read_to_string(&output_path).unwrap()
}

fn convert_to_docx_bytes(fixture: &std::path::Path) -> Vec<u8> {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("output.docx");
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
    ]);
    cmd.assert().success();
    fs::read(&output_path).unwrap()
}

fn zip_entry_string(bytes: &[u8], entry_name: &str) -> String {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
    let mut entry = archive
        .by_name(entry_name)
        .unwrap_or_else(|_| panic!("missing ZIP entry: {entry_name}"));
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .expect("Failed to read ZIP entry as UTF-8");
    xml
}

/// HTML output for a header fixture must contain a `<header>` landmark element.
#[test]
fn html_header_fixture_contains_header_element() {
    let fixture = project_root().join("fixtures/structure_header_default.rtf");
    let html = convert_to_html(&fixture);
    assert!(
        html.contains("<header class=\"rtf-header-footer rtf-header-default\">"),
        "HTML should contain <header> landmark for default header channel"
    );
}

/// HTML output for a footnote fixture must contain note-ref link and note section.
#[test]
fn html_footnote_fixture_contains_note_ref_and_section() {
    let fixture = project_root().join("fixtures/structure_footnote_simple.rtf");
    let html = convert_to_html(&fixture);
    assert!(
        html.contains("class=\"rtf-note-ref"),
        "HTML should contain note-ref anchor"
    );
    assert!(
        html.contains("id=\"note-1\""),
        "HTML should contain note body with id 'note-1'"
    );
    assert!(
        html.contains("<section class=\"rtf-notes\">"),
        "HTML should contain notes section"
    );
}

/// IR JSON for a header fixture must contain `structure.headers.default`.
#[test]
fn json_header_fixture_has_structure_field() {
    let fixture = project_root().join("fixtures/structure_header_default.rtf");
    let parsed = emit_ir(&fixture);
    let structure = parsed
        .get("structure")
        .expect("IR must have structure field for header fixture");
    let headers = structure
        .get("headers")
        .expect("structure.headers must be present");
    let default = headers
        .get("default")
        .and_then(|d| d.as_array())
        .expect("headers.default must be an array");
    assert!(
        !default.is_empty(),
        "headers.default must contain at least one block"
    );
}

/// IR JSON for a footnote fixture must have NoteRef inline in body and notes in structure.
#[test]
fn json_footnote_fixture_has_note_ref_and_structure_notes() {
    let fixture = project_root().join("fixtures/structure_footnote_simple.rtf");
    let parsed = emit_ir(&fixture);

    // Verify NoteRef inline exists in body paragraph
    let blocks = parsed
        .get("blocks")
        .and_then(|b| b.as_array())
        .expect("IR must have blocks array");
    let has_note_ref = blocks.iter().any(|b| {
        b.get("inlines")
            .and_then(|i| i.as_array())
            .map(|inlines| {
                inlines
                    .iter()
                    .any(|inline| inline.get("type").and_then(|t| t.as_str()) == Some("note_ref"))
            })
            .unwrap_or(false)
    });
    assert!(has_note_ref, "Body paragraph must contain a NoteRef inline");

    // Verify structure.notes is populated
    let notes = parsed
        .get("structure")
        .and_then(|s| s.get("notes"))
        .and_then(|n| n.as_array())
        .expect("structure.notes must be present");
    assert!(
        !notes.is_empty(),
        "structure.notes must contain at least one note"
    );
}

/// DOCX output for a header fixture must be a valid ZIP/DOCX file.
#[test]
fn docx_header_fixture_produces_valid_output() {
    let fixture = project_root().join("fixtures/structure_header_default.rtf");
    let bytes = convert_to_docx_bytes(&fixture);
    assert!(
        bytes.len() > 1000,
        "DOCX output should be a non-trivial file"
    );
    assert_eq!(bytes[0], 0x50, "DOCX should be a ZIP file (PK signature)");
    assert_eq!(bytes[1], 0x4B, "DOCX should be a ZIP file (PK signature)");
}

/// DOCX output for a footnote fixture must include a footnote reference and body.
#[test]
fn docx_footnote_fixture_contains_reference_and_body() {
    let fixture = project_root().join("fixtures/structure_footnote_simple.rtf");
    let bytes = convert_to_docx_bytes(&fixture);
    let document_xml = zip_entry_string(&bytes, "word/document.xml");
    let footnotes_xml = zip_entry_string(&bytes, "word/footnotes.xml");
    assert!(
        document_xml.contains("<w:footnoteReference"),
        "document.xml should contain a footnote reference element"
    );
    assert!(
        footnotes_xml.contains("Footnote content"),
        "footnotes.xml should contain the note body text"
    );
}
