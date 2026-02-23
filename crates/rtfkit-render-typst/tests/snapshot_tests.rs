//! Snapshot tests for Typst source generation.
//!
//! These tests verify that the generated Typst source is deterministic
//! and matches expected output patterns.

use rtfkit_core::{
    Alignment, Block, CellMerge, Document, ListBlock, ListItem, ListKind, Paragraph, Run,
    TableBlock, TableCell, TableRow,
};
use rtfkit_render_typst::{RenderOptions, map_document};

/// Helper to load expected snapshot from file.
fn load_snapshot(name: &str) -> String {
    std::fs::read_to_string(format!("tests/snapshots/{}.typ", name))
        .unwrap_or_else(|_| panic!("Failed to load snapshot: {}", name))
}

/// Helper to save actual output as snapshot (for initial creation).
#[allow(dead_code)]
fn save_snapshot(name: &str, content: &str) {
    std::fs::write(format!("tests/snapshots/{}.typ", name), content)
        .unwrap_or_else(|_| panic!("Failed to save snapshot: {}", name));
}

fn assert_snapshot(name: &str, actual: &str) {
    if std::env::var("UPDATE_SNAPSHOTS").ok().as_deref() == Some("1") {
        save_snapshot(name, actual);
        return;
    }
    let expected = load_snapshot(name);
    assert_eq!(actual, expected);
}

#[test]
fn test_snapshot_simple_paragraph() {
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("Hello, World!"),
    ]))]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("simple_paragraph", &output.typst_source);
}

#[test]
fn test_snapshot_formatted_paragraph() {
    let mut bold_run = Run::new("bold");
    bold_run.bold = true;

    let mut italic_run = Run::new("italic");
    italic_run.italic = true;

    let mut underline_run = Run::new("underline");
    underline_run.underline = true;

    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new("This is "),
        bold_run,
        Run::new(", "),
        italic_run,
        Run::new(", and "),
        underline_run,
        Run::new("."),
    ]))]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("formatted_paragraph", &output.typst_source);
}

#[test]
fn test_snapshot_aligned_paragraphs() {
    let mut center_para = Paragraph::from_runs(vec![Run::new("Centered text")]);
    center_para.alignment = Alignment::Center;

    let mut right_para = Paragraph::from_runs(vec![Run::new("Right-aligned text")]);
    right_para.alignment = Alignment::Right;

    let mut justify_para = Paragraph::from_runs(vec![Run::new("Justified text")]);
    justify_para.alignment = Alignment::Justify;

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Left-aligned text")])),
        Block::Paragraph(center_para),
        Block::Paragraph(right_para),
        Block::Paragraph(justify_para),
    ]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("aligned_paragraphs", &output.typst_source);
}

#[test]
fn test_snapshot_bullet_list() {
    let mut list = ListBlock::new(1, ListKind::Bullet);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("First item")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Second item")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Third item")]),
    ));

    let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("bullet_list", &output.typst_source);
}

#[test]
fn test_snapshot_ordered_list() {
    let mut list = ListBlock::new(1, ListKind::OrderedDecimal);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("First")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Second")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Third")]),
    ));

    let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("ordered_list", &output.typst_source);
}

#[test]
fn test_snapshot_nested_list() {
    let mut list = ListBlock::new(1, ListKind::Bullet);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Level 0")]),
    ));
    list.add_item(ListItem::from_paragraph(
        1,
        Paragraph::from_runs(vec![Run::new("Level 1")]),
    ));
    list.add_item(ListItem::from_paragraph(
        2,
        Paragraph::from_runs(vec![Run::new("Level 2")]),
    ));
    list.add_item(ListItem::from_paragraph(
        1,
        Paragraph::from_runs(vec![Run::new("Back to Level 1")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Back to Level 0")]),
    ));

    let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("nested_list", &output.typst_source);
}

#[test]
fn test_snapshot_simple_table() {
    let table = TableBlock::from_rows(vec![
        TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
        ]),
        TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("C")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("D")])),
        ]),
    ]);

    let doc = Document::from_blocks(vec![Block::TableBlock(table)]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("simple_table", &output.typst_source);
}

#[test]
fn test_snapshot_table_with_merges() {
    let mut h_start =
        TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged 2 cols")]));
    h_start.merge = Some(CellMerge::HorizontalStart { span: 2 });

    let h_cont = TableCell::new();

    let mut v_start =
        TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged 2 rows")]));
    v_start.merge = Some(CellMerge::VerticalStart);

    let v_cont = TableCell::new();

    let table = TableBlock::from_rows(vec![
        TableRow::from_cells(vec![h_start, h_cont, v_start]),
        TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
            v_cont,
        ]),
    ]);

    let doc = Document::from_blocks(vec![Block::TableBlock(table)]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("table_with_merges", &output.typst_source);
}

#[test]
fn test_snapshot_mixed_document() {
    let mut list = ListBlock::new(1, ListKind::Bullet);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("List item 1")]),
    ));
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("List item 2")]),
    ));

    let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
        TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 1")])),
        TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 2")])),
    ])]);

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new(
            "Introduction paragraph",
        )])),
        Block::ListBlock(list),
        Block::TableBlock(table),
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Conclusion paragraph")])),
    ]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("mixed_document", &output.typst_source);
}

#[test]
fn test_snapshot_special_characters() {
    let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
        Run::new(
            "Special: [brackets] *asterisk* _underscore_ @at #hash $dollar ~tilde \\backslash",
        ),
    ]))]);

    let options = RenderOptions::default();
    let output = map_document(&doc, &options);

    assert_snapshot("special_characters", &output.typst_source);
}

#[test]
fn test_snapshot_determinism_verification() {
    // Create a complex document
    let mut list = ListBlock::new(1, ListKind::Bullet);
    list.add_item(ListItem::from_paragraph(
        0,
        Paragraph::from_runs(vec![Run::new("Item")]),
    ));

    let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
        Paragraph::from_runs(vec![Run::new("Cell")]),
    )])]);

    let doc = Document::from_blocks(vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Para")])),
        Block::ListBlock(list),
        Block::TableBlock(table),
    ]);

    let options = RenderOptions::default();

    // Generate multiple times and verify they're identical
    let output1 = map_document(&doc, &options);
    let output2 = map_document(&doc, &options);
    let output3 = map_document(&doc, &options);

    assert_eq!(output1.typst_source, output2.typst_source);
    assert_eq!(output2.typst_source, output3.typst_source);

    // Also verify against snapshot
    assert_snapshot("determinism_verification", &output1.typst_source);
}
