//! Tests for document structure parsing: headers, footers, footnotes, endnotes.

use crate::{Block, Inline, NoteKind, parse};

// =============================================================================
// Helper
// =============================================================================

fn parse_ok(rtf: &str) -> crate::Document {
    let (doc, _report) = parse(rtf).expect("parse failed");
    doc
}

fn body_text(doc: &crate::Document) -> String {
    let mut out = String::new();
    for block in &doc.blocks {
        if let Block::Paragraph(para) = block {
            for inline in &para.inlines {
                if let Inline::Run(run) = inline {
                    out.push_str(&run.text);
                }
            }
        }
    }
    out
}

// =============================================================================
// No structure — document.structure is None
// =============================================================================

#[test]
fn no_structure_when_absent() {
    let doc = parse_ok(r#"{\rtf1\ansi Hello}"#);
    assert!(
        doc.structure.is_none(),
        "structure should be None when no headers/footers/notes present"
    );
}

// =============================================================================
// Headers
// =============================================================================

#[test]
fn header_default_captured() {
    let doc = parse_ok(r#"{\rtf1\ansi{\header Header text}Body}"#);
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.headers.default.len(), 1, "default header should have one block");
    if let Block::Paragraph(para) = &s.headers.default[0] {
        let text: String = para.inlines.iter().filter_map(|i| {
            if let Inline::Run(r) = i { Some(r.text.as_str()) } else { None }
        }).collect();
        assert_eq!(text, "Header text");
    } else {
        panic!("expected Paragraph block in header");
    }
    // Body unaffected
    assert_eq!(body_text(&doc), "Body");
}

#[test]
fn header_even_captured_via_headerl() {
    let doc = parse_ok(r#"{\rtf1\ansi{\headerl Even header}Body}"#);
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.headers.even.len(), 1);
    assert!(s.headers.default.is_empty());
    assert!(s.headers.first.is_empty());
}

#[test]
fn header_first_captured_via_headerf() {
    let doc = parse_ok(r#"{\rtf1\ansi{\headerf First header}Body}"#);
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.headers.first.len(), 1);
    assert!(s.headers.default.is_empty());
    assert!(s.headers.even.is_empty());
}

#[test]
fn footer_default_captured() {
    let doc = parse_ok(r#"{\rtf1\ansi{\footer Footer text}Body}"#);
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.footers.default.len(), 1);
    if let Block::Paragraph(para) = &s.footers.default[0] {
        let text: String = para.inlines.iter().filter_map(|i| {
            if let Inline::Run(r) = i { Some(r.text.as_str()) } else { None }
        }).collect();
        assert_eq!(text, "Footer text");
    } else {
        panic!("expected Paragraph in footer");
    }
}

#[test]
fn header_and_footer_together() {
    let doc = parse_ok(r#"{\rtf1\ansi{\header H}{\footer F}Body}"#);
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.headers.default.len(), 1);
    assert_eq!(s.footers.default.len(), 1);
    assert_eq!(body_text(&doc), "Body");
}

// =============================================================================
// Footnotes
// =============================================================================

#[test]
fn footnote_inserts_note_ref_inline() {
    let doc = parse_ok(r#"{\rtf1\ansi Before{\footnote Note body} after.}"#);

    // Body should contain NoteRef inline
    let mut found_note_ref = false;
    for block in &doc.blocks {
        if let Block::Paragraph(para) = block {
            for inline in &para.inlines {
                if let Inline::NoteRef(nr) = inline {
                    assert_eq!(nr.id, 1);
                    assert_eq!(nr.kind, NoteKind::Footnote);
                    found_note_ref = true;
                }
            }
        }
    }
    assert!(found_note_ref, "NoteRef should be in body paragraph");

    // Note body captured in structure
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.notes.len(), 1);
    assert_eq!(s.notes[0].id, 1);
    assert_eq!(s.notes[0].kind, NoteKind::Footnote);
    assert!(!s.notes[0].blocks.is_empty(), "note must have content blocks");
    if let Block::Paragraph(p) = &s.notes[0].blocks[0] {
        let text: String = p.inlines.iter().filter_map(|i| {
            if let Inline::Run(r) = i { Some(r.text.as_str()) } else { None }
        }).collect();
        assert_eq!(text, "Note body");
    }
}

#[test]
fn footnote_multiple_sequential_ids() {
    let doc = parse_ok(r#"{\rtf1\ansi A{\footnote First note}B{\footnote Second note}.}"#);

    // Collect NoteRef IDs from body
    let mut note_ref_ids: Vec<u32> = Vec::new();
    for block in &doc.blocks {
        if let Block::Paragraph(para) = block {
            for inline in &para.inlines {
                if let Inline::NoteRef(nr) = inline {
                    note_ref_ids.push(nr.id);
                }
            }
        }
    }
    assert_eq!(note_ref_ids, vec![1, 2], "footnote IDs should be 1 and 2");

    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.notes.len(), 2);
    assert_eq!(s.notes[0].id, 1);
    assert_eq!(s.notes[1].id, 2);
}

// =============================================================================
// Endnotes
// =============================================================================

#[test]
fn endnote_kind_is_endnote() {
    let doc = parse_ok(r#"{\rtf1\ansi Text{\endnote Endnote body}.}"#);

    let mut found = false;
    for block in &doc.blocks {
        if let Block::Paragraph(para) = block {
            for inline in &para.inlines {
                if let Inline::NoteRef(nr) = inline {
                    assert_eq!(nr.kind, NoteKind::Endnote);
                    found = true;
                }
            }
        }
    }
    assert!(found, "NoteRef with kind=endnote must appear in body");

    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.notes[0].kind, NoteKind::Endnote);
}

// =============================================================================
// Mixed footnotes and endnotes
// =============================================================================

#[test]
fn mixed_footnote_and_endnote() {
    let doc = parse_ok(r#"{\rtf1\ansi A{\footnote Fn}B{\endnote En}C.}"#);
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.notes.len(), 2);
    assert_eq!(s.notes[0].kind, NoteKind::Footnote);
    assert_eq!(s.notes[1].kind, NoteKind::Endnote);
}

// =============================================================================
// Combined: header + footnote
// =============================================================================

#[test]
fn header_and_footnote_combined() {
    let doc = parse_ok(r#"{\rtf1\ansi{\header Page header}Body{\footnote Note}.}"#);
    let s = doc.structure.as_ref().expect("structure must be Some");
    assert_eq!(s.headers.default.len(), 1, "header captured");
    assert_eq!(s.notes.len(), 1, "footnote captured");
    // Body paragraph has NoteRef
    let mut has_note_ref = false;
    for block in &doc.blocks {
        if let Block::Paragraph(p) = block {
            for i in &p.inlines {
                if matches!(i, Inline::NoteRef(_)) {
                    has_note_ref = true;
                }
            }
        }
    }
    assert!(has_note_ref);
}
