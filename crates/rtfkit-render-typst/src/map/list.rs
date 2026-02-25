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

use super::{map_block, MappingWarning, TypstAssetAllocator};

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
    let mut assets = TypstAssetAllocator::new();
    map_list_with_assets(list, &mut assets)
}

pub(crate) fn map_list_with_assets(
    list: &IrListBlock,
    assets: &mut TypstAssetAllocator,
) -> ListOutput {
    let mut warnings = Vec::new();

    // Track mixed kind fallback
    if matches!(list.kind, IrListKind::Mixed) {
        warnings.push(MappingWarning::ListMixedKindFallbackToBullet);
    }

    let typst_source = map_list_items(&list.items, list.kind, assets, &mut warnings);

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
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut previous_level: u8 = 0;
    let marker = get_list_marker(kind);
    let mut lines: Vec<String> = Vec::new();

    for (idx, item) in items.iter().enumerate() {
        if idx > 0 && item.level > previous_level.saturating_add(1) {
            warnings.push(MappingWarning::ListLevelSkip {
                from: previous_level,
                to: item.level,
            });
        }

        let item_content = map_list_item(item, assets, warnings);
        if item_content.is_empty() {
            previous_level = item.level;
            continue;
        }

        let indent = "  ".repeat(item.level as usize);
        let mut content_lines = item_content.lines();
        if let Some(first_line) = content_lines.next() {
            lines.push(format!("{indent}{marker} {first_line}"));
            let continuation_indent = format!("{indent}  ");
            for continuation in content_lines {
                lines.push(format!("{continuation_indent}{continuation}"));
            }
        }
        previous_level = item.level;
    }

    lines.join("\n")
}

/// Map a single list item's content to Typst source.
fn map_list_item(
    item: &IrListItem,
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    let mut content_parts = Vec::new();

    for block in &item.blocks {
        let block_output = map_block(block, assets);
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

        assert_eq!(output.typst_source, "- Top level\n  - Nested item");
        assert!(!output.typst_source.contains('{'));
        assert!(!output.typst_source.contains('}'));
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
        assert!(output
            .warnings
            .iter()
            .any(|w| matches!(w, crate::map::MappingWarning::ListMixedKindFallbackToBullet)));
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
