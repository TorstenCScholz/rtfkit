//! Page-management normalization and metadata assembly.

use super::super::state::RuntimeState;
use crate::{
    Block, GeneratedBlock, GeneratedBlockKind, Inline, PageManagement, Paragraph,
    RunningContentPlan, TocOptions,
};

/// Build and attach normalized page-management metadata to the document.
pub fn finalize_page_management(state: &mut RuntimeState) {
    let mut generated_blocks = Vec::new();
    normalize_generated_block_markers(state, &mut generated_blocks);

    // Hybrid TOC strategy: explicit markers first, inferred fallback when headings
    // are present and no explicit TOC marker exists.
    if generated_blocks.is_empty() && has_inferred_headings(&state.document.blocks) {
        generated_blocks.push(GeneratedBlock {
            insertion_index: 0,
            kind: GeneratedBlockKind::TableOfContents {
                options: TocOptions::default(),
            },
            explicit: false,
        });
    }

    let running_content = build_running_content_plan(state);
    let has_page_fields = document_has_page_fields(&state.document.blocks)
        || state
            .document
            .structure
            .as_ref()
            .is_some_and(structure_has_page_fields);

    let has_page_management = has_page_fields
        || !generated_blocks.is_empty()
        || state.section_plans.len() > 1
        || running_content != RunningContentPlan::default();

    if has_page_management {
        state.document.page_management = Some(PageManagement {
            sections: state.section_plans.clone(),
            running_content,
            generated_blocks,
        });
    }
}

fn normalize_generated_block_markers(state: &mut RuntimeState, out: &mut Vec<GeneratedBlock>) {
    let mut normalized_blocks = Vec::new();
    for block in std::mem::take(&mut state.document.blocks) {
        match block {
            Block::Paragraph(mut para) => {
                let markers = take_generated_markers(&mut para);
                for kind in markers {
                    out.push(GeneratedBlock {
                        insertion_index: normalized_blocks.len(),
                        kind,
                        explicit: true,
                    });
                }
                if !para.inlines.is_empty() {
                    normalized_blocks.push(Block::Paragraph(para));
                }
            }
            other => normalized_blocks.push(other),
        }
    }
    state.document.blocks = normalized_blocks;
}

fn take_generated_markers(paragraph: &mut Paragraph) -> Vec<GeneratedBlockKind> {
    let mut markers = Vec::new();
    paragraph.inlines.retain(|inline| match inline {
        Inline::GeneratedBlockMarker(kind) => {
            markers.push(kind.clone());
            false
        }
        _ => true,
    });
    markers
}

fn build_running_content_plan(state: &RuntimeState) -> RunningContentPlan {
    if let Some(structure) = state.document.structure.as_ref() {
        return RunningContentPlan {
            header_default: !structure.headers.default.is_empty(),
            header_first: !structure.headers.first.is_empty(),
            header_even: !structure.headers.even.is_empty(),
            footer_default: !structure.footers.default.is_empty(),
            footer_first: !structure.footers.first.is_empty(),
            footer_even: !structure.footers.even.is_empty(),
        };
    }
    RunningContentPlan::default()
}

fn has_inferred_headings(blocks: &[Block]) -> bool {
    let count = blocks
        .iter()
        .filter_map(|b| {
            if let Block::Paragraph(p) = b {
                Some(p)
            } else {
                None
            }
        })
        .filter(|p| crate::paragraph_looks_like_heading(p))
        .count();
    count >= 3
}

fn document_has_page_fields(blocks: &[Block]) -> bool {
    blocks.iter().any(block_has_page_fields)
}

fn block_has_page_fields(block: &Block) -> bool {
    match block {
        Block::Paragraph(paragraph) => paragraph
            .inlines
            .iter()
            .any(|inline| matches!(inline, Inline::PageField(_))),
        Block::ListBlock(list) => list
            .items
            .iter()
            .any(|item| item.blocks.iter().any(block_has_page_fields)),
        Block::TableBlock(table) => table.rows.iter().any(|row| {
            row.cells
                .iter()
                .any(|cell| cell.blocks.iter().any(block_has_page_fields))
        }),
        Block::ImageBlock(_) => false,
    }
}

fn structure_has_page_fields(structure: &crate::DocumentStructure) -> bool {
    let mut all: Vec<&[Block]> = vec![
        &structure.headers.default,
        &structure.headers.first,
        &structure.headers.even,
        &structure.footers.default,
        &structure.footers.first,
        &structure.footers.even,
    ];
    for note in &structure.notes {
        all.push(&note.blocks);
    }
    all.into_iter()
        .any(|blocks| blocks.iter().any(block_has_page_fields))
}
