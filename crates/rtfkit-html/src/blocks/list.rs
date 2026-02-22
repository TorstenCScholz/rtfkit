//! List block HTML emission.
//!
//! This module handles converting IR lists to HTML.

use crate::serialize::HtmlBuffer;
use rtfkit_core::{Block, ListBlock, ListItem, ListKind};

/// Converts a list block to HTML.
///
/// Emits `<ul>` for bullet lists, `<ol>` for ordered lists, and
/// `<ul class="rtf-list-mixed">` for mixed list kinds.
pub fn list_to_html(list: &ListBlock, buf: &mut HtmlBuffer) {
    let mut dropped_reasons = Vec::new();
    list_to_html_with_warnings(list, buf, &mut dropped_reasons);
}

/// Converts a list block to HTML while recording semantic degradations.
pub fn list_to_html_with_warnings(
    list: &ListBlock,
    buf: &mut HtmlBuffer,
    dropped_reasons: &mut Vec<String>,
) {
    let (tag, class) = list_tag_and_class(list.kind);

    push_list_open(tag, class, buf);

    if list.items.is_empty() {
        buf.push_close_tag(tag);
        return;
    }

    // IR list items are flattened with a level value; reconstruct nested HTML lists.
    let root_level = list.items[0].level;
    let mut current_level = root_level;
    let mut li_open = false;

    for item in &list.items {
        let mut target_level = item.level;

        // Level jumps >1 are malformed; clamp to +1 and preserve with deterministic degradation.
        if target_level > current_level.saturating_add(1) {
            push_dropped_reason_once(dropped_reasons, "list_nesting_semantics");
            target_level = current_level.saturating_add(1);
        }

        if target_level > current_level {
            if !li_open {
                // No valid parent list item to attach a nested list; synthesize one.
                push_dropped_reason_once(dropped_reasons, "list_nesting_semantics");
                buf.push_open_tag("li", &[]);
            }
            push_list_open(tag, class, buf);
            current_level += 1;
        } else if target_level < current_level {
            if li_open {
                buf.push_close_tag("li");
            }
            while current_level > target_level {
                buf.push_close_tag(tag);
                current_level -= 1;
                // Close parent item that owned the nested list we just closed.
                buf.push_close_tag("li");
            }
        } else if li_open {
            buf.push_close_tag("li");
        }

        buf.push_open_tag("li", &[]);
        li_open = true;
        emit_item_blocks(item, buf, dropped_reasons);
    }

    if li_open {
        buf.push_close_tag("li");
    }

    while current_level > root_level {
        buf.push_close_tag(tag);
        current_level -= 1;
        buf.push_close_tag("li");
    }

    buf.push_close_tag(tag);
}

fn list_tag_and_class(kind: ListKind) -> (&'static str, Option<&'static str>) {
    match kind {
        ListKind::Bullet => ("ul", Some("rtf-list")),
        ListKind::OrderedDecimal => ("ol", Some("rtf-list")),
        ListKind::Mixed => ("ul", Some("rtf-list rtf-list-mixed")),
    }
}

fn push_list_open(tag: &str, class: Option<&str>, buf: &mut HtmlBuffer) {
    let attrs: Vec<(&str, &str)> = class.map(|c| vec![("class", c)]).unwrap_or_default();
    buf.push_open_tag(tag, &attrs);
}

fn emit_item_blocks(item: &ListItem, buf: &mut HtmlBuffer, dropped_reasons: &mut Vec<String>) {
    for block in &item.blocks {
        match block {
            Block::Paragraph(para) => {
                crate::blocks::paragraph::paragraph_to_html(para, buf);
            }
            Block::ListBlock(nested) => {
                list_to_html_with_warnings(nested, buf, dropped_reasons);
            }
            Block::TableBlock(table) => {
                crate::blocks::table::table_to_html_with_warnings(table, buf, dropped_reasons);
            }
        }
    }
}

fn push_dropped_reason_once(dropped_reasons: &mut Vec<String>, reason: &str) {
    if !dropped_reasons.iter().any(|existing| existing == reason) {
        dropped_reasons.push(reason.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{ListItem, Paragraph, Run};

    #[test]
    fn empty_bullet_list() {
        let list = ListBlock::new(1, ListKind::Bullet);
        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);
        assert_eq!(buf.as_str(), r#"<ul class="rtf-list"></ul>"#);
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn bullet_list_with_item() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item 1")]),
        ));

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);
        // Paragraphs inside list items are wrapped in <p> tags with rtf-p class
        assert_eq!(
            buf.as_str(),
            r#"<ul class="rtf-list"><li><p class="rtf-p">Item 1</p></li></ul>"#
        );
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn ordered_list() {
        let list = ListBlock::new(1, ListKind::OrderedDecimal);
        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);
        assert_eq!(buf.as_str(), r#"<ol class="rtf-list"></ol>"#);
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn ordered_list_with_items() {
        let mut list = ListBlock::new(1, ListKind::OrderedDecimal);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("First")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Second")]),
        ));

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);
        assert_eq!(
            buf.as_str(),
            r#"<ol class="rtf-list"><li><p class="rtf-p">First</p></li><li><p class="rtf-p">Second</p></li></ol>"#
        );
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn mixed_list_has_class() {
        let mut list = ListBlock::new(1, ListKind::Mixed);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Item")]),
        ));

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);
        // Mixed lists use <ul> with both rtf-list and rtf-list-mixed classes
        assert_eq!(
            buf.as_str(),
            r#"<ul class="rtf-list rtf-list-mixed"><li><p class="rtf-p">Item</p></li></ul>"#
        );
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn nested_list() {
        let mut outer = ListBlock::new(1, ListKind::Bullet);
        outer.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Outer item")]),
        ));

        // Create nested list
        let mut nested = ListBlock::new(2, ListKind::OrderedDecimal);
        nested.add_item(ListItem::from_paragraph(
            1,
            Paragraph::from_runs(vec![Run::new("Nested item")]),
        ));

        // Add nested list as second item's content
        let mut item_with_nested = ListItem::new(0);
        item_with_nested
            .blocks
            .push(Block::Paragraph(Paragraph::from_runs(vec![Run::new(
                "Item with nested:",
            )])));
        item_with_nested.blocks.push(Block::ListBlock(nested));
        outer.add_item(item_with_nested);

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&outer, &mut buf, &mut dropped_reasons);
        assert_eq!(
            buf.as_str(),
            r#"<ul class="rtf-list"><li><p class="rtf-p">Outer item</p></li><li><p class="rtf-p">Item with nested:</p><ol class="rtf-list"><li><p class="rtf-p">Nested item</p></li></ol></li></ul>"#
        );
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn list_item_with_multiple_blocks() {
        let mut list = ListBlock::new(1, ListKind::Bullet);

        let mut item = ListItem::new(0);
        item.blocks
            .push(Block::Paragraph(Paragraph::from_runs(vec![Run::new(
                "First paragraph",
            )])));
        item.blocks
            .push(Block::Paragraph(Paragraph::from_runs(vec![Run::new(
                "Second paragraph",
            )])));
        list.add_item(item);

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);
        assert_eq!(
            buf.as_str(),
            r#"<ul class="rtf-list"><li><p class="rtf-p">First paragraph</p><p class="rtf-p">Second paragraph</p></li></ul>"#
        );
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn deeply_nested_list() {
        // Create a three-level nested list
        let mut level3 = ListBlock::new(3, ListKind::Bullet);
        level3.add_item(ListItem::from_paragraph(
            2,
            Paragraph::from_runs(vec![Run::new("Level 3")]),
        ));

        let mut level2 = ListBlock::new(2, ListKind::OrderedDecimal);
        level2.add_item(ListItem::from_paragraph(
            1,
            Paragraph::from_runs(vec![Run::new("Level 2")]),
        ));
        let mut level2_item_with_nested = ListItem::new(1);
        level2_item_with_nested
            .blocks
            .push(Block::ListBlock(level3));
        level2.add_item(level2_item_with_nested);

        let mut level1 = ListBlock::new(1, ListKind::Bullet);
        level1.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Level 1")]),
        ));
        let mut level1_item_with_nested = ListItem::new(0);
        level1_item_with_nested
            .blocks
            .push(Block::ListBlock(level2));
        level1.add_item(level1_item_with_nested);

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&level1, &mut buf, &mut dropped_reasons);
        assert_eq!(
            buf.as_str(),
            r#"<ul class="rtf-list"><li><p class="rtf-p">Level 1</p></li><li><ol class="rtf-list"><li><p class="rtf-p">Level 2</p></li><li><ul class="rtf-list"><li><p class="rtf-p">Level 3</p></li></ul></li></ol></li></ul>"#
        );
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn level_based_nesting_from_flat_ir_items() {
        let mut list = ListBlock::new(1, ListKind::OrderedDecimal);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Top 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            1,
            Paragraph::from_runs(vec![Run::new("Nested 1")]),
        ));
        list.add_item(ListItem::from_paragraph(
            1,
            Paragraph::from_runs(vec![Run::new("Nested 2")]),
        ));
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Top 2")]),
        ));

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);

        assert_eq!(
            buf.as_str(),
            r#"<ol class="rtf-list"><li><p class="rtf-p">Top 1</p><ol class="rtf-list"><li><p class="rtf-p">Nested 1</p></li><li><p class="rtf-p">Nested 2</p></li></ol></li><li><p class="rtf-p">Top 2</p></li></ol>"#
        );
        assert!(dropped_reasons.is_empty());
    }

    #[test]
    fn malformed_level_jump_records_dropped_reason() {
        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::from_paragraph(
            0,
            Paragraph::from_runs(vec![Run::new("Top")]),
        ));
        list.add_item(ListItem::from_paragraph(
            3,
            Paragraph::from_runs(vec![Run::new("Too deep")]),
        ));

        let mut buf = HtmlBuffer::new();
        let mut dropped_reasons = Vec::new();
        list_to_html_with_warnings(&list, &mut buf, &mut dropped_reasons);

        assert!(buf.as_str().contains("Too deep"));
        assert_eq!(dropped_reasons, vec!["list_nesting_semantics".to_string()]);
    }
}
