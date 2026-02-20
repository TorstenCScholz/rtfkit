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

/// Helper to extract paragraph from block (currently the only block type).
fn as_paragraph(block: &rtfkit_core::Block) -> &rtfkit_core::Paragraph {
    match block {
        rtfkit_core::Block::Paragraph(p) => p,
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

    let para = as_paragraph(&doc.blocks[0]);
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
    let has_bold = doc
        .blocks
        .iter()
        .any(|b| as_paragraph(b).runs.iter().any(|r| r.bold));

    let has_italic = doc
        .blocks
        .iter()
        .any(|b| as_paragraph(b).runs.iter().any(|r| r.italic));

    assert!(has_bold, "Should have bold text");
    assert!(has_italic, "Should have italic text");
}

#[test]
fn test_underline_formatting() {
    let input = include_str!("../../../fixtures/underline.rtf");
    let (doc, _report) = Interpreter::parse(input).unwrap();

    let has_underline = doc
        .blocks
        .iter()
        .any(|b| as_paragraph(b).runs.iter().any(|r| r.underline));

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
        .map(|b| as_paragraph(b).alignment)
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
                let p = as_paragraph(b);
                p.runs.is_empty() || p.runs.iter().all(|r| r.text.trim().is_empty())
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
        let p = as_paragraph(block);
        let bold_runs = p.runs.iter().filter(|r| r.bold).count();
        let italic_runs = p.runs.iter().filter(|r| r.italic).count();
        if bold_runs > 0 && italic_runs > 0 {
            style_combinations += 1;
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
    let total_runs: usize = doc.blocks.iter().map(|b| as_paragraph(b).runs.len()).sum();

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
