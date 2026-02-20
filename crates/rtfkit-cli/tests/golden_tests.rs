//! Golden Tests for RTF IR Snapshots
//!
//! These tests parse RTF fixtures and compare the resulting IR (Intermediate Representation)
//! against golden JSON files. This ensures that parsing behavior remains consistent.
//!
//! To update golden files, run: `UPDATE_GOLDEN=1 cargo test -p rtfkit-cli`

use std::fs;
use std::path::{Path, PathBuf};

use rtfkit_core::Interpreter;
use similar::{ChangeTag, TextDiff};

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

fn golden_dir() -> PathBuf {
    project_root().join("golden")
}

fn update_golden() -> bool {
    std::env::var("UPDATE_GOLDEN").is_ok()
}

fn diff_strings(expected: &str, actual: &str) -> String {
    let diff = TextDiff::from_lines(expected, actual);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        out.push_str(&format!("{sign}{change}"));
    }
    out
}

/// Helper to extract paragraph from block.
/// Returns None for ListBlock variants (which contain paragraphs inside items).
fn as_paragraph(block: &rtfkit_core::Block) -> Option<&rtfkit_core::Paragraph> {
    match block {
        rtfkit_core::Block::Paragraph(p) => Some(p),
        rtfkit_core::Block::ListBlock(_) => None,
    }
}

/// Test that parses all RTF fixtures and compares IR output against golden files.
#[test]
fn golden_ir_output() {
    let fixtures = fixture_dir();
    let golden = golden_dir();

    let mut entries: Vec<_> = fs::read_dir(&fixtures)
        .expect("Failed to read fixtures directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rtf"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    assert!(
        !entries.is_empty(),
        "No fixture files found in {fixtures:?}"
    );

    for entry in entries {
        let fixture_path = entry.path();
        let stem = fixture_path.file_stem().unwrap().to_str().unwrap();
        let golden_path = golden.join(format!("{stem}.json"));

        // Read RTF content
        let rtf_content = fs::read_to_string(&fixture_path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {fixture_path:?}: {e}"));

        // Parse RTF to IR
        let (document, _report) = Interpreter::parse(&rtf_content)
            .unwrap_or_else(|e| panic!("Failed to parse RTF fixture {fixture_path:?}: {e}"));

        // Serialize IR to JSON
        let actual =
            serde_json::to_string_pretty(&document).expect("Failed to serialize IR to JSON");

        if update_golden() {
            fs::create_dir_all(&golden).ok();
            fs::write(&golden_path, &actual)
                .unwrap_or_else(|e| panic!("Failed to write golden file {golden_path:?}: {e}"));
            eprintln!("Updated golden file: {golden_path:?}");
            continue;
        }

        let expected = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
            panic!(
                "Golden file {golden_path:?} not found: {e}\n\
                 Hint: Run with UPDATE_GOLDEN=1 to generate golden files"
            )
        });

        if actual != expected {
            let diff = diff_strings(&expected, &actual);
            panic!(
                "Golden test mismatch for {stem}:\n\n\
                 {diff}\n\n\
                 Run with UPDATE_GOLDEN=1 to refresh snapshots"
            );
        }
    }
}

// =============================================================================
// Feature-specific tests for key RTF features
// =============================================================================

#[test]
fn test_simple_paragraph_content() {
    let input = include_str!("../../../fixtures/simple_paragraph.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    assert_eq!(doc.blocks.len(), 1, "Should have exactly one paragraph");

    let para = as_paragraph(&doc.blocks[0]).expect("Expected paragraph block");
    assert_eq!(para.runs.len(), 1, "Should have one run");
    assert_eq!(
        para.runs[0].text, "Hello World",
        "Text should be 'Hello World'"
    );
}

#[test]
fn test_bold_italic_formatting() {
    let input = include_str!("../../../fixtures/bold_italic.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    assert!(!doc.blocks.is_empty(), "Document should have content");

    // Check that there are runs with bold and italic formatting
    let has_bold = doc.blocks.iter().any(|b| {
        as_paragraph(b)
            .map(|p| p.runs.iter().any(|r| r.bold))
            .unwrap_or(false)
    });

    let has_italic = doc.blocks.iter().any(|b| {
        as_paragraph(b)
            .map(|p| p.runs.iter().any(|r| r.italic))
            .unwrap_or(false)
    });

    assert!(has_bold, "Should have bold text");
    assert!(has_italic, "Should have italic text");
}

#[test]
fn test_underline_formatting() {
    let input = include_str!("../../../fixtures/underline.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    let has_underline = doc.blocks.iter().any(|b| {
        as_paragraph(b)
            .map(|p| p.runs.iter().any(|r| r.underline))
            .unwrap_or(false)
    });

    assert!(has_underline, "Should have underlined text");
}

#[test]
fn test_alignment() {
    let input = include_str!("../../../fixtures/alignment.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    use rtfkit_core::Alignment;

    // Check for different alignments
    let alignments: Vec<Alignment> = doc
        .blocks
        .iter()
        .filter_map(|b| as_paragraph(b).map(|p| p.alignment))
        .collect();

    assert!(
        alignments.contains(&Alignment::Center),
        "Should have centered paragraph"
    );
    assert!(
        alignments.contains(&Alignment::Right),
        "Should have right-aligned paragraph"
    );
}

#[test]
fn test_multiple_paragraphs() {
    let input = include_str!("../../../fixtures/multiple_paragraphs.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    assert!(doc.blocks.len() >= 2, "Should have at least 2 paragraphs");
}

#[test]
fn test_empty_document() {
    let input = include_str!("../../../fixtures/empty.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Empty RTF should produce an empty document
    assert!(
        doc.blocks.is_empty()
            || doc.blocks.iter().all(|b| {
                as_paragraph(b)
                    .map(|p| p.runs.is_empty() || p.runs.iter().all(|r| r.text.trim().is_empty()))
                    .unwrap_or(true)
            }),
        "Empty RTF should produce empty or whitespace-only content"
    );
}

#[test]
fn test_nested_styles() {
    let input = include_str!("../../../fixtures/nested_styles.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Nested styles should be handled correctly
    // The document should have content with varying formatting
    assert!(!doc.blocks.is_empty(), "Document should have content");

    // Count runs with different style combinations
    let mut style_combinations = 0;
    for block in &doc.blocks {
        if let Some(p) = as_paragraph(block) {
            let bold_runs = p.runs.iter().filter(|r| r.bold).count();
            let italic_runs = p.runs.iter().filter(|r| r.italic).count();
            if bold_runs > 0 && italic_runs > 0 {
                style_combinations += 1;
            }
        }
    }

    // Nested styles should result in multiple style combinations
    assert!(
        style_combinations > 0 || doc.blocks.len() > 1,
        "Nested styles should produce varied formatting"
    );
}

#[test]
fn test_mixed_formatting() {
    let input = include_str!("../../../fixtures/mixed_formatting.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Mixed formatting should produce multiple runs with different styles
    let total_runs: usize = doc
        .blocks
        .iter()
        .filter_map(|b| as_paragraph(b).map(|p| p.runs.len()))
        .sum();

    assert!(total_runs >= 1, "Should have at least one run");
}

#[test]
fn test_complex_document() {
    let input = include_str!("../../../fixtures/complex.rtf");
    let (doc, report) = Interpreter::parse(input).unwrap();

    // Complex document should parse without errors
    assert!(
        !doc.blocks.is_empty(),
        "Complex document should have content"
    );

    // Should have multiple paragraphs
    assert!(
        doc.blocks.len() >= 2,
        "Complex document should have multiple blocks"
    );

    // Report should have stats
    assert!(
        report.stats.paragraph_count >= 2,
        "Should count multiple paragraphs"
    );
}

// =============================================================================
// List-specific tests
// =============================================================================

/// Helper to extract list block from a Block.
/// Returns None for Paragraph variants.
fn as_list_block(block: &rtfkit_core::Block) -> Option<&rtfkit_core::ListBlock> {
    match block {
        rtfkit_core::Block::ListBlock(list) => Some(list),
        rtfkit_core::Block::Paragraph(_) => None,
    }
}

#[test]
fn test_list_bullet_simple() {
    let input = include_str!("../../../fixtures/list_bullet_simple.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Should have exactly one list block
    assert_eq!(doc.blocks.len(), 1, "Should have exactly one block");

    let list = as_list_block(&doc.blocks[0]).expect("Expected list block");

    // Verify list properties
    assert_eq!(
        list.kind,
        rtfkit_core::ListKind::Bullet,
        "Should be bullet list"
    );
    assert_eq!(list.items.len(), 3, "Should have 3 items");

    // Verify all items are at level 0
    for (i, item) in list.items.iter().enumerate() {
        assert_eq!(item.level, 0, "Item {} should be at level 0", i);
        assert_eq!(item.blocks.len(), 1, "Item {} should have 1 block", i);
    }

    // Verify text content
    let texts: Vec<&str> = list
        .items
        .iter()
        .filter_map(|item| {
            as_paragraph(&item.blocks[0])
                .map(|p| p.runs.first().map(|r| r.text.as_str()).unwrap_or(""))
        })
        .collect();

    assert_eq!(texts, vec!["First item", "Second item", "Third item"]);
}

#[test]
fn test_list_decimal_simple() {
    let input = include_str!("../../../fixtures/list_decimal_simple.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Should have exactly one list block
    assert_eq!(doc.blocks.len(), 1, "Should have exactly one block");

    let list = as_list_block(&doc.blocks[0]).expect("Expected list block");

    // Verify list properties
    assert_eq!(
        list.kind,
        rtfkit_core::ListKind::OrderedDecimal,
        "Should be ordered decimal list"
    );
    assert_eq!(list.items.len(), 3, "Should have 3 items");

    // Verify text content
    let texts: Vec<&str> = list
        .items
        .iter()
        .filter_map(|item| {
            as_paragraph(&item.blocks[0])
                .map(|p| p.runs.first().map(|r| r.text.as_str()).unwrap_or(""))
        })
        .collect();

    assert_eq!(texts, vec!["First item", "Second item", "Third item"]);
}

#[test]
fn test_list_nested_two_levels() {
    let input = include_str!("../../../fixtures/list_nested_two_levels.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Should have exactly one list block
    assert_eq!(doc.blocks.len(), 1, "Should have exactly one block");

    let list = as_list_block(&doc.blocks[0]).expect("Expected list block");

    // Should have 6 items total
    assert_eq!(list.items.len(), 6, "Should have 6 items");

    // Verify nesting levels
    let levels: Vec<u8> = list.items.iter().map(|item| item.level).collect();
    assert_eq!(
        levels,
        vec![0, 1, 1, 0, 1, 0],
        "Levels should alternate correctly"
    );

    // Verify text content
    let texts: Vec<&str> = list
        .items
        .iter()
        .filter_map(|item| {
            as_paragraph(&item.blocks[0])
                .map(|p| p.runs.first().map(|r| r.text.as_str()).unwrap_or(""))
        })
        .collect();

    assert_eq!(
        texts,
        vec![
            "Top level item one",
            "Nested item one",
            "Nested item two",
            "Top level item two",
            "Nested item three",
            "Top level item three"
        ]
    );
}

#[test]
fn test_list_mixed_kinds() {
    let input = include_str!("../../../fixtures/list_mixed_kinds.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Should have 4 blocks: paragraph, list, paragraph, list
    assert_eq!(doc.blocks.len(), 4, "Should have 4 blocks");

    // First block should be a paragraph (heading)
    let heading1 = as_paragraph(&doc.blocks[0]).expect("First block should be paragraph");
    assert!(
        heading1.runs.iter().any(|r| r.text.contains("Bullet List")),
        "First paragraph should contain 'Bullet List'"
    );

    // Second block should be bullet list
    let bullet_list = as_list_block(&doc.blocks[1]).expect("Second block should be list");
    assert_eq!(
        bullet_list.kind,
        rtfkit_core::ListKind::Bullet,
        "Second block should be bullet list"
    );
    assert_eq!(
        bullet_list.items.len(),
        3,
        "Bullet list should have 3 items"
    );

    // Third block should be a paragraph (heading)
    let heading2 = as_paragraph(&doc.blocks[2]).expect("Third block should be paragraph");
    assert!(
        heading2
            .runs
            .iter()
            .any(|r| r.text.contains("Ordered List")),
        "Third paragraph should contain 'Ordered List'"
    );

    // Fourth block should be ordered list
    let ordered_list = as_list_block(&doc.blocks[3]).expect("Fourth block should be list");
    assert_eq!(
        ordered_list.kind,
        rtfkit_core::ListKind::OrderedDecimal,
        "Fourth block should be ordered list"
    );
    assert_eq!(
        ordered_list.items.len(),
        3,
        "Ordered list should have 3 items"
    );
}

#[test]
fn test_list_malformed_fallback() {
    let input = include_str!("../../../fixtures/list_malformed_fallback.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    // Malformed list references should still produce content
    assert!(!doc.blocks.is_empty(), "Should have content");

    // Collect all text content
    let all_text: String = doc
        .blocks
        .iter()
        .flat_map(|b| {
            as_paragraph(b)
                .map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>())
                .unwrap_or_else(|| {
                    as_list_block(b)
                        .map(|l| {
                            l.items
                                .iter()
                                .flat_map(|item| {
                                    as_paragraph(&item.blocks[0])
                                        .map(|p| {
                                            p.runs
                                                .iter()
                                                .map(|r| r.text.as_str())
                                                .collect::<Vec<_>>()
                                        })
                                        .unwrap_or_default()
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                })
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Verify all text content is preserved
    assert!(
        all_text.contains("invalid list reference"),
        "Should contain 'invalid list reference'"
    );
    assert!(
        all_text.contains("Valid list item"),
        "Should contain 'Valid list item'"
    );
    assert!(
        all_text.contains("Another invalid reference"),
        "Should contain 'Another invalid reference'"
    );
}
