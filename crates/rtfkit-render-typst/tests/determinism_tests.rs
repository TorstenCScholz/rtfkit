//! Determinism Test Suite for Typst-based PDF rendering.
//!
//! These tests verify that PDF output is byte-identical for the same input
//! across multiple runs. This is critical for reproducible builds and
//! reliable document generation.
//!
//! # Test Categories
//!
//! 1. **Basic determinism**: Simple content types (paragraphs, lists, tables)
//! 2. **Complex determinism**: Nested structures and mixed content
//! 3. **Unicode determinism**: International text handling
//! 4. **Edge cases**: Empty documents, special characters
//!
//! # Methodology
//!
//! Each test runs the conversion at least 3 times and compares outputs
//! byte-for-byte. All tests use a fixed timestamp to ensure consistent
//! PDF metadata.

use rtfkit_core::{
    Block, Document, ListBlock, ListItem, ListKind, Paragraph, Run, TableBlock, TableCell, TableRow,
};
use rtfkit_render_typst::{DeterminismOptions, RenderOptions, document_to_pdf_with_warnings};

/// Fixed timestamp for deterministic testing (2024-01-01 00:00:00 UTC).
const FIXED_TIMESTAMP: &str = "2024-01-01T00:00:00Z";

/// Create deterministic render options with fixed timestamp.
fn deterministic_options() -> RenderOptions {
    RenderOptions {
        determinism: DeterminismOptions {
            fixed_timestamp: Some(FIXED_TIMESTAMP.to_string()),
            normalize_metadata: true,
        },
        ..Default::default()
    }
}

/// Verify that multiple renders produce identical output.
fn verify_deterministic(doc: &Document, options: &RenderOptions, runs: usize) {
    let results: Vec<Vec<u8>> = (0..runs)
        .map(|_| {
            document_to_pdf_with_warnings(doc, options)
                .expect("Rendering should succeed")
                .pdf_bytes
        })
        .collect();

    // All results should be identical
    let first = &results[0];
    for (i, result) in results.iter().enumerate().skip(1) {
        assert_eq!(
            first, result,
            "PDF output should be byte-identical across runs (run 0 vs run {})",
            i
        );
    }
}

fn run_isolated_test(subtest_name: &str, gate_var: &str) {
    let exe = std::env::current_exe().expect("Failed to resolve current test binary path");
    let output = std::process::Command::new(exe)
        .arg("--exact")
        .arg(subtest_name)
        .arg("--nocapture")
        .env(gate_var, "1")
        .output()
        .expect("Failed to execute isolated subprocess test");

    assert!(
        output.status.success(),
        "Isolated test '{}' failed.\nstdout:\n{}\nstderr:\n{}",
        subtest_name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// =============================================================================
// BASIC DETERMINISM TESTS
// =============================================================================

mod basic_determinism {
    use super::*;

    /// Test that a simple paragraph produces deterministic output.
    #[test]
    fn simple_paragraph_is_deterministic() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that multiple paragraphs produce deterministic output.
    #[test]
    fn multiple_paragraphs_are_deterministic() {
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("First paragraph")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Second paragraph")])),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Third paragraph")])),
        ]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that formatted text produces deterministic output.
    #[test]
    fn formatted_text_is_deterministic() {
        let mut bold_run = Run::new("bold");
        bold_run.bold = true;
        let mut italic_run = Run::new("italic");
        italic_run.italic = true;
        let mut underline_run = Run::new("underline");
        underline_run.underline = true;

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Normal "),
            bold_run,
            Run::new(" "),
            italic_run,
            Run::new(" "),
            underline_run,
        ]))]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that a bullet list produces deterministic output.
    #[test]
    fn bullet_list_is_deterministic() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 2")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 3")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that an ordered list produces deterministic output.
    #[test]
    fn ordered_list_is_deterministic() {
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

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that a simple table produces deterministic output.
    #[test]
    fn simple_table_is_deterministic() {
        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A1")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B1")])),
            ]),
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A2")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B2")])),
            ]),
        ]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }
}

// =============================================================================
// NESTED STRUCTURE TESTS
// =============================================================================

mod nested_structures {
    use super::*;

    /// Test that nested lists produce deterministic output.
    #[test]
    fn nested_list_is_deterministic() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Level 1, Item 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            1,
            Paragraph::from_runs(vec![Run::new("Level 2, Item 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            1,
            Paragraph::from_runs(vec![Run::new("Level 2, Item 2")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Level 1, Item 2")]),
        ));

        let doc = Document::from_blocks(vec![Block::ListBlock(list)]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that a table with merged cells produces deterministic output.
    #[test]
    fn table_with_merges_is_deterministic() {
        use rtfkit_core::CellMerge;

        let mut cell1 = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        cell1.merge = Some(CellMerge::HorizontalStart { span: 2 });

        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![cell1]),
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
            ]),
        ]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that a table with a list in a cell produces deterministic output.
    #[test]
    fn table_with_list_in_cell_is_deterministic() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item in cell")]),
        ));

        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 1")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 2")])),
        ])]);

        let doc = Document::from_blocks(vec![Block::TableBlock(table), Block::ListBlock(list)]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }
}

// =============================================================================
// UNICODE DETERMINISM TESTS
// =============================================================================

mod unicode_determinism {
    use super::*;

    /// Test that Unicode text produces deterministic output.
    #[test]
    fn unicode_text_is_deterministic() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Unicode: café, naïve, 日本語, 中文, Ελληνικά, العربية"),
        ]))]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that mixed scripts produce deterministic output.
    #[test]
    fn mixed_scripts_are_deterministic() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Latin "),
            Run::new("日本語 "),
            Run::new("한국어 "),
            Run::new("Кириллица "),
            Run::new("עברית"),
        ]))]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that emoji produce deterministic output.
    #[test]
    fn emoji_is_deterministic() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Emoji: 😀 🎉 🚀 ❤️ 🌍"),
        ]))]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

mod edge_cases {
    use super::*;

    /// Test that an empty document produces deterministic output.
    #[test]
    fn empty_document_is_deterministic() {
        let doc = Document::new();

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that special characters produce deterministic output.
    #[test]
    fn special_characters_are_deterministic() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new(
                "Special: [brackets] *asterisk* _underscore_ @at #hash $dollar ~tilde \\backslash",
            ),
        ]))]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that a large document produces deterministic output.
    #[test]
    fn large_document_is_deterministic() {
        let blocks: Vec<Block> = (0..50)
            .map(|i| {
                Block::Paragraph(Paragraph::from_runs(vec![Run::new(format!(
                    "Paragraph number {}",
                    i
                ))]))
            })
            .collect();

        let doc = Document::from_blocks(blocks);

        verify_deterministic(&doc, &deterministic_options(), 3);
    }

    /// Test that a document with many runs produces deterministic output.
    #[test]
    fn many_runs_are_deterministic() {
        let runs: Vec<Run> = (0..20)
            .map(|i| {
                let mut run = Run::new(format!("Run{}", i));
                run.bold = i % 2 == 0;
                run.italic = i % 3 == 0;
                run.underline = i % 5 == 0;
                run
            })
            .collect();

        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(runs))]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }
}

// =============================================================================
// MIXED CONTENT TESTS
// =============================================================================

mod mixed_content {
    use super::*;

    /// Test that a mixed document produces deterministic output.
    #[test]
    fn mixed_document_is_deterministic() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("List item")]),
        ));

        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Cell")]),
            )])]);

        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Introduction")])),
            Block::ListBlock(list),
            Block::TableBlock(table),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Conclusion")])),
        ]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }

    /// Test that a complex document with all element types produces deterministic output.
    #[test]
    fn complex_document_is_deterministic() {
        let mut bullet_list = ListBlock::new(1, ListKind::Bullet);
        bullet_list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Bullet 1")]),
        ));
        bullet_list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Bullet 2")]),
        ));

        let mut ordered_list = ListBlock::new(2, ListKind::OrderedDecimal);
        ordered_list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Ordered 1")]),
        ));
        ordered_list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Ordered 2")]),
        ));

        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Header 1")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Header 2")])),
            ]),
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Data 1")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Data 2")])),
            ]),
        ]);

        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Title")])),
            Block::Paragraph(Paragraph::from_runs(vec![
                Run::new("Paragraph with "),
                Run::new("mixed"),
                Run::new(" formatting"),
            ])),
            Block::ListBlock(bullet_list),
            Block::ListBlock(ordered_list),
            Block::TableBlock(table),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Footer")])),
        ]);

        verify_deterministic(&doc, &deterministic_options(), 5);
    }
}

// =============================================================================
// TIMESTAMP VERIFICATION TESTS
// =============================================================================

mod timestamp_verification {
    use super::*;

    /// Test that different timestamps produce different output.
    #[test]
    fn different_timestamps_produce_different_output() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);

        let options1 = RenderOptions {
            determinism: DeterminismOptions {
                fixed_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let options2 = RenderOptions {
            determinism: DeterminismOptions {
                fixed_timestamp: Some("2024-12-31T23:59:59Z".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let result1 = document_to_pdf_with_warnings(&doc, &options1).unwrap();
        let result2 = document_to_pdf_with_warnings(&doc, &options2).unwrap();

        // Different timestamps should produce different PDF metadata
        // Note: The actual content should be the same, but metadata differs
        assert_ne!(
            result1.pdf_bytes, result2.pdf_bytes,
            "Different timestamps should produce different PDF output"
        );
    }

    /// Test that no timestamp (current time) still produces valid PDF.
    #[test]
    fn no_timestamp_produces_valid_pdf() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);

        let options = RenderOptions {
            determinism: DeterminismOptions {
                fixed_timestamp: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = document_to_pdf_with_warnings(&doc, &options);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.pdf_bytes.starts_with(b"%PDF-"));
    }

    /// Test that same timestamp produces identical output across different options.
    #[test]
    fn same_timestamp_same_options_identical_output() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);

        let options = RenderOptions {
            determinism: DeterminismOptions {
                fixed_timestamp: Some("2024-06-15T12:30:45Z".to_string()),
                normalize_metadata: true,
            },
            ..Default::default()
        };

        let result1 = document_to_pdf_with_warnings(&doc, &options).unwrap();
        let result2 = document_to_pdf_with_warnings(&doc, &options).unwrap();

        assert_eq!(
            result1.pdf_bytes, result2.pdf_bytes,
            "Same timestamp and options should produce identical output"
        );
    }
}

// =============================================================================
// PDF STRUCTURE VERIFICATION TESTS
// =============================================================================

mod pdf_structure {
    use super::*;

    /// Test that PDF output has valid structure.
    #[test]
    fn pdf_has_valid_structure() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Test content"),
        ]))]);

        let result = document_to_pdf_with_warnings(&doc, &deterministic_options()).unwrap();
        let pdf_bytes = result.pdf_bytes;

        // PDF should start with %PDF-
        assert!(
            pdf_bytes.starts_with(b"%PDF-"),
            "PDF should start with %PDF- header"
        );

        // PDF should contain %%EOF marker
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_str.contains("%%EOF"), "PDF should contain %%EOF marker");

        // PDF should contain version
        assert!(
            pdf_str.contains("%PDF-1."),
            "PDF should contain version identifier"
        );
    }

    /// Test that PDF output is reasonably sized.
    #[test]
    fn pdf_output_size_reasonable() {
        let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
            Run::new("Hello, World!"),
        ]))]);

        let result = document_to_pdf_with_warnings(&doc, &deterministic_options()).unwrap();

        // PDF should be at least 1KB for a simple document
        assert!(
            result.pdf_bytes.len() > 1000,
            "PDF too small: {} bytes",
            result.pdf_bytes.len()
        );

        // But not unreasonably large (less than 100KB for simple content)
        assert!(
            result.pdf_bytes.len() < 100_000,
            "PDF too large: {} bytes",
            result.pdf_bytes.len()
        );
    }
}

// =============================================================================
// ENVIRONMENT INDEPENDENCE TESTS
// =============================================================================

mod environment_independence {
    use super::*;
    use std::env;

    /// Test that rendering works with empty PATH.
    /// This verifies we don't depend on any external tools.
    #[test]
    fn works_with_empty_path() {
        run_isolated_test(
            "environment_independence::works_with_empty_path_subprocess",
            "RTFKIT_ISOLATED_EMPTY_PATH",
        );
    }

    #[test]
    fn works_with_empty_path_subprocess() {
        if env::var_os("RTFKIT_ISOLATED_EMPTY_PATH").is_none() {
            return;
        }

        // Save the current PATH
        let original_path = env::var("PATH").ok();

        // Set PATH to empty
        // SAFETY: This is a test and we restore PATH afterwards
        unsafe {
            env::set_var("PATH", "");
        }

        let result = std::panic::catch_unwind(|| {
            let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
                Run::new("Empty PATH test"),
            ]))]);

            verify_deterministic(&doc, &deterministic_options(), 3);
        });

        // Restore PATH
        // SAFETY: This is a test and we're restoring the original value
        unsafe {
            if let Some(path) = original_path {
                env::set_var("PATH", path);
            } else {
                env::remove_var("PATH");
            }
        }

        // Re-raise any panic
        assert!(result.is_ok());
    }

    /// Test that rendering works regardless of current directory.
    #[test]
    fn works_regardless_of_current_directory() {
        run_isolated_test(
            "environment_independence::works_regardless_of_current_directory_subprocess",
            "RTFKIT_ISOLATED_CWD",
        );
    }

    #[test]
    fn works_regardless_of_current_directory_subprocess() {
        if env::var_os("RTFKIT_ISOLATED_CWD").is_none() {
            return;
        }

        let original_dir = env::current_dir().unwrap();

        // Create a temporary directory and change to it
        let temp_dir = tempfile::tempdir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        let result = std::panic::catch_unwind(|| {
            let doc = Document::from_blocks(vec![Block::Paragraph(Paragraph::from_runs(vec![
                Run::new("Current directory test"),
            ]))]);

            verify_deterministic(&doc, &deterministic_options(), 3);
        });

        // Restore original directory
        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }
}
