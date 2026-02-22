//! List mapping from IR to Typst source.
//!
//! This module provides functions to convert rtfkit-core `ListBlock` types
//! to Typst markup source code.
//!
//! ## Typst List Mapping
//!
//! | IR | Typst |
//! |---|---|
//! | `ListKind::Bullet` | `- item` |
//! | `ListKind::OrderedDecimal` | `+ item` |
//! | `ListKind::Mixed` | `- item` (fallback to bullet) |
//! | `ListItem.level` | Indentation (nested enum blocks) |
//!
//! ## Level Preservation
//!
//! The IR `ListItem.level` is preserved through Typst's nested enumeration syntax.
//! Each level increase creates a nested list block.
//!
//! ## Deterministic Level Transitions
//!
//! For malformed level transitions (e.g., jumping from level 0 to level 3),
//! the mapper deterministically creates intermediate empty levels to maintain
//! valid Typst structure.

use rtfkit_core::{ListBlock as IrListBlock, ListItem as IrListItem, ListKind as IrListKind};

use super::{MappingWarning, map_block};

/// Result of mapping a list to Typst source.
#[derive(Debug, Clone, PartialEq)]
pub struct ListOutput {
    /// The generated Typst source code.
    pub typst_source: String,
    /// Warnings generated during mapping.
    pub warnings: Vec<MappingWarning>,
}

/// Map a rtfkit-core ListBlock to Typst source code.
///
/// # Arguments
///
/// * `list` - The source list block from rtfkit-core.
///
/// # Returns
///
/// A `ListOutput` containing the Typst source and any warnings.
///
/// # Determinism
///
/// This function is deterministic: the same input always produces the same output.
pub fn map_list(list: &IrListBlock) -> ListOutput {
    let mut warnings = Vec::new();

    // Track mixed kind fallback
    if matches!(list.kind, IrListKind::Mixed) {
        warnings.push(MappingWarning::ListMixedKindFallbackToBullet);
    }

    let typst_source = map_list_items(&list.items, list.kind, &mut warnings);

    ListOutput {
        typst_source,
        warnings,
    }
}

/// Map list items to Typst source.
///
/// This function handles level transitions and creates proper nesting.
fn map_list_items(
    items: &[IrListItem],
    kind: IrListKind,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut current_level: u8 = 0;
    let mut level_stack: Vec<String> = Vec::new();

    // Start with empty string for level 0
    level_stack.push(String::new());

    for item in items {
        let target_level = item.level;

        // Handle level transitions
        if target_level > current_level {
            // Going deeper - create nested lists
            for level in current_level..target_level {
                // Check for level skip (malformed)
                if level + 1 < target_level && level == current_level {
                    warnings.push(MappingWarning::ListLevelSkip {
                        from: current_level,
                        to: target_level,
                    });
                }

                // Push a new nesting level
                level_stack.push(String::new());
            }
            current_level = target_level;
        } else if target_level < current_level {
            // Going back up - close nested lists
            while current_level > target_level && level_stack.len() > 1 {
                let nested_content = level_stack.pop().unwrap_or_default();
                let parent = level_stack.last_mut().unwrap();

                // Add the nested enum block to parent
                if !nested_content.is_empty() {
                    let marker = get_list_marker(kind);
                    // Each nested level needs proper indentation
                    let indented = indent_nested_list(&nested_content, current_level);
                    parent.push_str(&format!("{} {{\n{}}}\n", marker, indented));
                }
                current_level -= 1;
            }
            current_level = target_level;
        }

        // Map the item content
        let item_content = map_list_item(item, warnings);

        // Add to current level
        let current_level_content = level_stack.last_mut().unwrap();
        if !item_content.is_empty() {
            let marker = get_list_marker(kind);
            current_level_content.push_str(&format!("{} {}\n", marker, item_content));
        }
    }

    // Close any remaining nested levels
    while level_stack.len() > 1 {
        let nested_content = level_stack.pop().unwrap_or_default();
        let parent = level_stack.last_mut().unwrap();

        if !nested_content.is_empty() {
            let marker = get_list_marker(kind);
            let indented = indent_nested_list(&nested_content, current_level);
            parent.push_str(&format!("{} {{\n{}}}\n", marker, indented));
        }
        current_level = current_level.saturating_sub(1);
    }

    // Get the final result from level 0
    let mut result = level_stack.pop().unwrap_or_default();

    // Trim trailing newline
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Map a single list item's content to Typst source.
fn map_list_item(item: &IrListItem, warnings: &mut Vec<MappingWarning>) -> String {
    let mut content_parts = Vec::new();

    for block in &item.blocks {
        let block_output = map_block(block);
        if !block_output.typst_source.is_empty() {
            content_parts.push(block_output.typst_source);
            warnings.extend(block_output.warnings);
        }
    }

    // Join multiple blocks with newlines
    // For single paragraph, just return the text
    // For multiple blocks, we need special handling
    if content_parts.is_empty() {
        String::new()
    } else if content_parts.len() == 1 {
        content_parts.remove(0)
    } else {
        // Multiple blocks - join with line breaks
        content_parts.join(" \\\n")
    }
}

/// Get the Typst list marker for the given list kind.
fn get_list_marker(kind: IrListKind) -> &'static str {
    match kind {
        IrListKind::Bullet => "-",
        IrListKind::OrderedDecimal => "+",
        IrListKind::Mixed => "-", // Fallback to bullet
    }
}

/// Indent a nested list's content for proper Typst formatting.
fn indent_nested_list(content: &str, _level: u8) -> String {
    // Add one level of indentation to each line
    content
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {}", line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{Block as IrBlock, Paragraph, Run};

    fn make_bullet_list() -> IrListBlock {
        IrListBlock::new(1, IrListKind::Bullet)
    }

    fn make_ordered_list() -> IrListBlock {
        IrListBlock::new(1, IrListKind::OrderedDecimal)
    }

    fn make_item(level: u8, text: &str) -> IrListItem {
        IrListItem::from_paragraph(level, Paragraph::from_runs(vec![Run::new(text)]))
    }

    #[test]
    fn test_map_empty_list() {
        let list = make_bullet_list();
        let output = map_list(&list);

        assert!(output.typst_source.is_empty());
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_single_bullet_item() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "First item"));

        let output = map_list(&list);

        assert_eq!(output.typst_source, "- First item");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_single_ordered_item() {
        let mut list = make_ordered_list();
        list.add_item(make_item(0, "First item"));

        let output = map_list(&list);

        assert_eq!(output.typst_source, "+ First item");
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_map_multiple_items_same_level() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "First"));
        list.add_item(make_item(0, "Second"));
        list.add_item(make_item(0, "Third"));

        let output = map_list(&list);

        assert_eq!(output.typst_source, "- First\n- Second\n- Third");
    }

    #[test]
    fn test_map_nested_list_two_levels() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "Top level"));
        list.add_item(make_item(1, "Nested item"));

        let output = map_list(&list);

        // Should have nested structure
        assert!(output.typst_source.contains("- Top level"));
        assert!(output.typst_source.contains("{"));
        assert!(output.typst_source.contains("  - Nested item"));
    }

    #[test]
    fn test_map_nested_list_three_levels() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "Level 0"));
        list.add_item(make_item(1, "Level 1"));
        list.add_item(make_item(2, "Level 2"));

        let output = map_list(&list);

        // Verify three levels of nesting
        assert!(output.typst_source.contains("- Level 0"));
        assert!(output.typst_source.contains("  - Level 1"));
        assert!(output.typst_source.contains("    - Level 2"));
    }

    #[test]
    fn test_map_nested_list_return_to_parent() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "Item 1"));
        list.add_item(make_item(1, "Nested"));
        list.add_item(make_item(0, "Item 2"));

        let output = map_list(&list);

        // Should properly close nested list and return to level 0
        let lines: Vec<&str> = output.typst_source.lines().collect();
        assert!(lines.iter().any(|l| l.starts_with("- Item 1")));
        assert!(lines.iter().any(|l| l.starts_with("- Item 2")));
    }

    #[test]
    fn test_map_mixed_kind_fallback() {
        let mut list = IrListBlock::new(1, IrListKind::Mixed);
        list.add_item(make_item(0, "Item"));

        let output = map_list(&list);

        // Should use bullet marker
        assert!(output.typst_source.starts_with('-'));
        assert!(
            output
                .warnings
                .iter()
                .any(|w| matches!(w, crate::map::MappingWarning::ListMixedKindFallbackToBullet))
        );
    }

    #[test]
    fn test_map_level_skip_warning() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "Level 0"));
        list.add_item(make_item(3, "Level 3")); // Skip levels 1 and 2

        let output = map_list(&list);

        // Should warn about level skip
        assert!(output.warnings.iter().any(|w| matches!(
            w,
            crate::map::MappingWarning::ListLevelSkip { from: 0, to: 3 }
        )));
    }

    #[test]
    fn test_map_empty_item() {
        let mut list = make_bullet_list();
        list.add_item(IrListItem::new(0)); // Empty item

        let output = map_list(&list);

        // Empty items produce no output
        assert!(output.typst_source.is_empty() || !output.typst_source.contains('-'));
    }

    #[test]
    fn test_map_item_with_multiple_blocks() {
        let mut item = IrListItem::new(0);
        item.blocks
            .push(IrBlock::Paragraph(Paragraph::from_runs(vec![Run::new(
                "First",
            )])));
        item.blocks
            .push(IrBlock::Paragraph(Paragraph::from_runs(vec![Run::new(
                "Second",
            )])));

        let mut list = make_bullet_list();
        list.add_item(item);

        let output = map_list(&list);

        // Multiple blocks should be joined with line breaks
        assert!(output.typst_source.contains("First"));
        assert!(output.typst_source.contains("Second"));
    }

    #[test]
    fn test_map_ordered_list_multiple_items() {
        let mut list = make_ordered_list();
        list.add_item(make_item(0, "One"));
        list.add_item(make_item(0, "Two"));
        list.add_item(make_item(0, "Three"));

        let output = map_list(&list);

        assert_eq!(output.typst_source, "+ One\n+ Two\n+ Three");
    }

    #[test]
    fn test_map_nested_ordered_list() {
        let mut list = make_ordered_list();
        list.add_item(make_item(0, "Top"));
        list.add_item(make_item(1, "Nested"));

        let output = map_list(&list);

        assert!(output.typst_source.contains("+ Top"));
        assert!(output.typst_source.contains("  + Nested"));
    }

    #[test]
    fn test_determinism() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "First"));
        list.add_item(make_item(1, "Nested"));
        list.add_item(make_item(0, "Second"));

        // Run multiple times to verify determinism
        let output1 = map_list(&list);
        let output2 = map_list(&list);
        let output3 = map_list(&list);

        assert_eq!(output1.typst_source, output2.typst_source);
        assert_eq!(output2.typst_source, output3.typst_source);
        assert_eq!(output1.warnings, output2.warnings);
        assert_eq!(output2.warnings, output3.warnings);
    }

    #[test]
    fn test_deep_nesting_four_levels() {
        let mut list = make_bullet_list();
        list.add_item(make_item(0, "L0"));
        list.add_item(make_item(1, "L1"));
        list.add_item(make_item(2, "L2"));
        list.add_item(make_item(3, "L3"));

        let output = map_list(&list);

        // Verify all four levels are present
        assert!(output.typst_source.contains("- L0"));
        assert!(output.typst_source.contains("  - L1"));
        assert!(output.typst_source.contains("    - L2"));
        // L3 should be even more indented
        assert!(output.typst_source.contains("L3"));
    }
}
