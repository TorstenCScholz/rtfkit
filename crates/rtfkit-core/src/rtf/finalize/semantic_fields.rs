//! Semantic field cross-reference resolution.
//!
//! This module provides a post-parse finalization pass that:
//!
//! 1. Collects all `BookmarkAnchor` names from the document.
//! 2. Resolves `REF` and `NOTEREF` fields against the collected bookmark set.
//! 3. Marks unresolved fields with `resolved: false`.
//! 4. Emits one `UnresolvedCrossReference` warning per unique unresolved target.

use std::collections::HashSet;

use crate::report::ReportBuilder;
use crate::{Block, BookmarkAnchor, Document, DocumentStructure, Inline, SemanticFieldRef};

/// Run the semantic field cross-reference resolver on a completed document.
///
/// This function must be called after the main parse-pipeline finalization
/// so all `BookmarkAnchor` inlines are in place.
pub fn resolve_semantic_cross_references(doc: &mut Document, report: &mut ReportBuilder) {
    let bookmarks = collect_bookmark_names(doc);
    let mut warned: HashSet<String> = HashSet::new();
    resolve_blocks(&mut doc.blocks, &bookmarks, &mut warned, report);
    if let Some(structure) = &mut doc.structure {
        resolve_structure(structure, &bookmarks, &mut warned, report);
    }
}

// =============================================================================
// Bookmark collection
// =============================================================================

fn collect_bookmark_names(doc: &Document) -> HashSet<String> {
    let mut names = HashSet::new();
    for block in &doc.blocks {
        collect_from_block(block, &mut names);
    }
    if let Some(structure) = &doc.structure {
        for block in structure
            .headers
            .default
            .iter()
            .chain(structure.headers.first.iter())
            .chain(structure.headers.even.iter())
            .chain(structure.footers.default.iter())
            .chain(structure.footers.first.iter())
            .chain(structure.footers.even.iter())
        {
            collect_from_block(block, &mut names);
        }
        for note in &structure.notes {
            for block in &note.blocks {
                collect_from_block(block, &mut names);
            }
        }
    }
    names
}

fn collect_from_block(block: &Block, names: &mut HashSet<String>) {
    match block {
        Block::Paragraph(p) => {
            for inline in &p.inlines {
                collect_from_inline(inline, names);
            }
        }
        Block::ListBlock(lb) => {
            for item in &lb.items {
                for block in &item.blocks {
                    collect_from_block(block, names);
                }
            }
        }
        Block::TableBlock(tb) => {
            for row in &tb.rows {
                for cell in &row.cells {
                    for block in &cell.blocks {
                        collect_from_block(block, names);
                    }
                }
            }
        }
        Block::ImageBlock(_) => {}
    }
}

fn collect_from_inline(inline: &Inline, names: &mut HashSet<String>) {
    if let Inline::BookmarkAnchor(BookmarkAnchor { name }) = inline {
        names.insert(name.clone());
    }
}

// =============================================================================
// Resolution
// =============================================================================

fn resolve_blocks(
    blocks: &mut [Block],
    bookmarks: &HashSet<String>,
    warned: &mut HashSet<String>,
    report: &mut ReportBuilder,
) {
    for block in blocks.iter_mut() {
        resolve_block(block, bookmarks, warned, report);
    }
}

fn resolve_block(
    block: &mut Block,
    bookmarks: &HashSet<String>,
    warned: &mut HashSet<String>,
    report: &mut ReportBuilder,
) {
    match block {
        Block::Paragraph(p) => {
            for inline in p.inlines.iter_mut() {
                resolve_inline(inline, bookmarks, warned, report);
            }
        }
        Block::ListBlock(lb) => {
            for item in lb.items.iter_mut() {
                for block in item.blocks.iter_mut() {
                    resolve_block(block, bookmarks, warned, report);
                }
            }
        }
        Block::TableBlock(tb) => {
            for row in tb.rows.iter_mut() {
                for cell in row.cells.iter_mut() {
                    for block in cell.blocks.iter_mut() {
                        resolve_block(block, bookmarks, warned, report);
                    }
                }
            }
        }
        Block::ImageBlock(_) => {}
    }
}

fn resolve_inline(
    inline: &mut Inline,
    bookmarks: &HashSet<String>,
    warned: &mut HashSet<String>,
    report: &mut ReportBuilder,
) {
    if let Inline::SemanticField(sf) = inline {
        match &sf.reference {
            SemanticFieldRef::Ref { target, .. } | SemanticFieldRef::NoteRef { target, .. } => {
                if !bookmarks.contains(target) {
                    sf.resolved = false;
                    if warned.insert(target.clone()) {
                        report.unresolved_cross_reference(target);
                    }
                }
            }
            _ => {}
        }
    }
}

fn resolve_structure(
    structure: &mut DocumentStructure,
    bookmarks: &HashSet<String>,
    warned: &mut HashSet<String>,
    report: &mut ReportBuilder,
) {
    let channel_blocks: Vec<&mut [Block]> = vec![
        &mut structure.headers.default,
        &mut structure.headers.first,
        &mut structure.headers.even,
        &mut structure.footers.default,
        &mut structure.footers.first,
        &mut structure.footers.even,
    ];
    for blocks in channel_blocks {
        resolve_blocks(blocks, bookmarks, warned, report);
    }
    for note in structure.notes.iter_mut() {
        resolve_blocks(&mut note.blocks, bookmarks, warned, report);
    }
}
