//! Document structure mapping to Typst page setup.
//!
//! Headers and footers are mapped to `#set page(header: [...], footer: [...])`.
//! Only the default channel is used; first/even page channels emit a
//! `PartialSupport` warning because Typst has no direct equivalent.
//!
//! Notes (footnotes, endnotes) are rendered inline via `map_note_ref` in
//! `paragraph.rs`; no separate section is emitted in Typst output.

use rtfkit_core::{Block as IrBlock, DocumentStructure, HeaderFooterSet};

use super::{MappingWarning, TypstAssetAllocator, map_block};

/// Map a `DocumentStructure` to a Typst `#set page(...)` header/footer directive.
///
/// Returns an empty string if neither headers nor footers have content.
/// Emits `PartialSupport` warnings for first/even page channels.
pub(crate) fn map_structure_page_setup(
    structure: &DocumentStructure,
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    let header_typst = map_header_footer_set_default(&structure.headers, assets, warnings);
    let footer_typst = map_header_footer_set_default(&structure.footers, assets, warnings);

    // Warn about unsupported first/even channels
    if !structure.headers.first.is_empty() || !structure.headers.even.is_empty() {
        warnings.push(MappingWarning::PartialSupport {
            feature: "header_first_even".into(),
            reason: "Typst does not support distinct first/even page headers; only default header emitted".into(),
        });
    }
    if !structure.footers.first.is_empty() || !structure.footers.even.is_empty() {
        warnings.push(MappingWarning::PartialSupport {
            feature: "footer_first_even".into(),
            reason: "Typst does not support distinct first/even page footers; only default footer emitted".into(),
        });
    }

    if header_typst.is_empty() && footer_typst.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    if !header_typst.is_empty() {
        parts.push(format!("  header: [{}]", header_typst));
    }
    if !footer_typst.is_empty() {
        parts.push(format!("  footer: [{}]", footer_typst));
    }

    format!("#set page(\n{},\n)", parts.join(",\n"))
}

/// Map the default channel of a `HeaderFooterSet` to inline Typst content.
///
/// Returns empty string if the default channel is empty.
fn map_header_footer_set_default(
    set: &HeaderFooterSet,
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    map_blocks_inline(&set.default, assets, warnings)
}

/// Map a slice of blocks to inline Typst content (space-joined block sources).
fn map_blocks_inline(
    blocks: &[IrBlock],
    assets: &mut TypstAssetAllocator,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        let output = map_block(block, assets);
        warnings.extend(output.warnings);
        if !output.typst_source.is_empty() {
            parts.push(output.typst_source);
        }
    }
    parts.join(" ")
}
