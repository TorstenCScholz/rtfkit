//! Table, row, cell, and border conversion from IR to docx-rs types.

use crate::DocxError;
use crate::context::ConvertCtx;
use crate::image::convert_image_block;
use crate::paragraph::{convert_paragraph, convert_paragraph_with_numbering};
use crate::shading::convert_shading;
use docx_rs::{
    BorderType, HeightRule as DocxHeightRule, IndentLevel, NumberingId, Shading, ShdType, Table,
    TableAlignmentType, TableBorder, TableBorderPosition, TableBorders, TableCell, TableCellBorder,
    TableCellBorderPosition, TableCellBorders, TableCellMargins, TableRow, VAlignType, VMergeType,
    WidthType,
};
use rtfkit_core::{
    Block, Border as IrBorder, BorderSet as IrBorderSet, BorderStyle as IrBorderStyle, CellMerge,
    CellVerticalAlign, RowAlignment, RowHeightRule, TableBlock, TableCell as IrTableCell,
    TableRow as IrTableRow, WidthUnit, resolve_effective_cell_borders,
};
use rtfkit_style_tokens::{StyleProfile, TableStripeMode};

/// Converts an IR TableBlock to a docx-rs Table.
///
/// Maps the IR table structure to DOCX elements:
/// - `TableBlock` → `w:tbl`
/// - `TableRow` → `w:tr`
/// - `TableCell` → `w:tc`
///
/// Cell widths are mapped from twips to DXA (1:1 ratio since both are 1/20th point).
pub(crate) fn convert_table(
    table: &TableBlock,
    ctx: &mut ConvertCtx<'_>,
) -> Result<Table, DocxError> {
    let rows: Vec<TableRow> = table
        .rows
        .iter()
        .enumerate()
        .map(|(row_idx, row)| convert_table_row(table, row_idx, row, ctx))
        .collect::<Result<Vec<_>, _>>()?;

    let mut docx_table = Table::new(rows);

    // Apply table-level layout from first row's RowProps (RTF stores these per-row)
    let first_row_props = table.rows.first().and_then(|r| r.row_props.as_ref());

    // Alignment
    if let Some(rp) = first_row_props {
        match rp.alignment {
            Some(RowAlignment::Center) => {
                docx_table = docx_table.align(TableAlignmentType::Center);
            }
            Some(RowAlignment::Right) => {
                docx_table = docx_table.align(TableAlignmentType::Right);
            }
            _ => {}
        }
        // Left indent
        if let Some(indent) = rp.left_indent {
            docx_table = docx_table.indent(indent);
        }
        // Row-default cell padding as table-level cell margins
        if let Some(ref pad) = rp.default_padding {
            let mut margins = TableCellMargins::new();
            if let Some(t) = pad.top {
                margins = margins.margin_top(t.max(0) as usize, WidthType::Dxa);
            }
            if let Some(r) = pad.right {
                // docx-rs TableCellMargins only has margin_top and margin_left;
                // right and bottom exposed via direct cell property below.
                let _ = r;
            }
            if let Some(l) = pad.left {
                margins = margins.margin_left(l.max(0) as usize, WidthType::Dxa);
            }
            if pad.top.is_some() || pad.left.is_some() {
                docx_table = docx_table.margins(margins);
            }
        }
    }

    // Preferred table width from TableProps
    if let Some(ref pw) = table.table_props.as_ref().and_then(|tp| tp.preferred_width) {
        match pw {
            WidthUnit::Auto => {
                docx_table = docx_table.width(0, WidthType::Auto);
            }
            WidthUnit::Twips(t) => {
                if *t >= 0 {
                    docx_table = docx_table.width(*t as usize, WidthType::Dxa);
                }
            }
            WidthUnit::Percent(bp) => {
                docx_table = docx_table.width(*bp as usize, WidthType::Pct);
            }
        }
    }

    // Apply profile table border defaults at table level.
    if let Some(p) = ctx.profile {
        docx_table = docx_table.set_borders(profile_table_borders(p));
    }

    // Apply profile cell margins when no RTF row-default padding was specified.
    if let Some(p) = ctx.profile
        && first_row_props
            .and_then(|rp| rp.default_padding.as_ref())
            .is_none()
    {
        let pad_x = (p.spacing.table_cell_padding_x * 20.0).round() as usize;
        let pad_y = (p.spacing.table_cell_padding_y * 20.0).round() as usize;
        let margins = TableCellMargins::new()
            .margin_top(pad_y, WidthType::Dxa)
            .margin_right(pad_x, WidthType::Dxa)
            .margin_bottom(pad_y, WidthType::Dxa)
            .margin_left(pad_x, WidthType::Dxa);
        docx_table = docx_table.margins(margins);
    }

    Ok(docx_table)
}

/// Converts an IR TableRow to a docx-rs TableRow.
///
/// Handles row properties and horizontal merge normalization.
///
/// Valid horizontal continuation cells are skipped because they are
/// represented by the start cell's gridSpan. Orphan continuation cells are
/// preserved as standalone cells to avoid silent text loss.
fn convert_table_row(
    table: &TableBlock,
    row_idx: usize,
    row: &IrTableRow,
    ctx: &mut ConvertCtx<'_>,
) -> Result<TableRow, DocxError> {
    let mut cells = Vec::with_capacity(row.cells.len());
    let mut expected_continuations = 0usize;

    for (col_idx, cell) in row.cells.iter().enumerate() {
        let effective_borders = resolve_effective_cell_borders(table, row_idx, col_idx);
        match cell.merge {
            Some(CellMerge::HorizontalStart { span }) if span > 1 => {
                cells.push(convert_table_cell(cell, row_idx, effective_borders, ctx)?);
                expected_continuations = span.saturating_sub(1) as usize;
            }
            Some(CellMerge::HorizontalStart { .. }) => {
                // Defensive: span=0/1 is not a real merge, emit as standalone.
                expected_continuations = 0;
                let mut standalone = cell.clone();
                standalone.merge = None;
                cells.push(convert_table_cell(
                    &standalone,
                    row_idx,
                    effective_borders,
                    ctx,
                )?);
            }
            Some(CellMerge::HorizontalContinue) if expected_continuations > 0 => {
                expected_continuations -= 1;
            }
            Some(CellMerge::HorizontalContinue) => {
                // Orphan continuation: preserve content rather than silently dropping it.
                let mut standalone = cell.clone();
                standalone.merge = None;
                cells.push(convert_table_cell(
                    &standalone,
                    row_idx,
                    effective_borders,
                    ctx,
                )?);
            }
            _ => {
                expected_continuations = 0;
                cells.push(convert_table_cell(cell, row_idx, effective_borders, ctx)?);
            }
        }
    }

    let mut docx_row = TableRow::new(cells);

    // Apply row height from RowProps
    if let Some((h, rule)) = row
        .row_props
        .as_ref()
        .and_then(|rp| rp.height_twips.zip(rp.height_rule))
    {
        docx_row = docx_row.row_height(h as f32);
        match rule {
            RowHeightRule::AtLeast => {
                docx_row = docx_row.height_rule(DocxHeightRule::AtLeast);
            }
            RowHeightRule::Exact => {
                docx_row = docx_row.height_rule(DocxHeightRule::Exact);
            }
        }
    }

    Ok(docx_row)
}

/// Maps an IR `BorderStyle` to a docx-rs `BorderType`.
pub(crate) fn border_style_to_docx(style: IrBorderStyle) -> BorderType {
    match style {
        IrBorderStyle::Single => BorderType::Single,
        IrBorderStyle::Double => BorderType::Double,
        IrBorderStyle::Dotted => BorderType::Dotted,
        IrBorderStyle::Dashed => BorderType::Dashed,
        IrBorderStyle::None => BorderType::Nil,
    }
}

/// Build a single `TableCellBorder` from an IR `Border`.
pub(crate) fn convert_border(border: &IrBorder, pos: TableCellBorderPosition) -> TableCellBorder {
    let bt = border_style_to_docx(border.style);
    // docx-rs size unit is 1/8 pt; IR width is half-points, so multiply by 4.
    let size = border
        .width_half_pts
        .map(|hp| hp.saturating_mul(4))
        .unwrap_or(8) as usize;
    let color = border
        .color
        .as_ref()
        .map(|c| format!("{:02X}{:02X}{:02X}", c.r, c.g, c.b))
        .unwrap_or_else(|| "000000".to_string());
    TableCellBorder::new(pos)
        .border_type(bt)
        .size(size)
        .color(color)
}

/// Build a `TableCellBorders` from an IR `BorderSet`.
pub(crate) fn convert_border_set(borders: &IrBorderSet) -> TableCellBorders {
    // Use `with_empty()` so only explicitly-set sides are emitted.
    let mut docx_borders = TableCellBorders::with_empty();
    if let Some(ref b) = borders.top {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Top));
    }
    if let Some(ref b) = borders.left {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Left));
    }
    if let Some(ref b) = borders.bottom {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Bottom));
    }
    if let Some(ref b) = borders.right {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::Right));
    }
    if let Some(ref b) = borders.inside_h {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::InsideH));
    }
    if let Some(ref b) = borders.inside_v {
        docx_borders = docx_borders.set(convert_border(b, TableCellBorderPosition::InsideV));
    }
    docx_borders
}

fn profile_table_borders(profile: &StyleProfile) -> TableBorders {
    let size = (profile.components.table.border_width * 8.0)
        .round()
        .max(1.0) as usize;
    let color = profile.colors.border_default.without_hash().to_string();
    TableBorders::with_empty()
        .set(
            TableBorder::new(TableBorderPosition::Top)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Left)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Bottom)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Right)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::InsideH)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
        .set(
            TableBorder::new(TableBorderPosition::InsideV)
                .border_type(BorderType::Single)
                .size(size)
                .color(&color),
        )
}

/// Converts an IR TableCell to a docx-rs TableCell.
///
/// Handles cell content (paragraphs and lists), width mapping, merge semantics,
/// vertical alignment, shading, and borders.
///
/// Width is stored in twips in the IR and mapped to DXA for DOCX (1:1 ratio).
fn convert_table_cell(
    cell: &IrTableCell,
    row_idx: usize,
    effective_borders: Option<IrBorderSet>,
    ctx: &mut ConvertCtx<'_>,
) -> Result<TableCell, DocxError> {
    let mut docx_cell = TableCell::new();

    // Apply preferred cell width if present, otherwise fall back to cellx-derived width
    if let Some(ref pw) = cell.preferred_width {
        match pw {
            WidthUnit::Auto => {
                docx_cell = docx_cell.width(0, WidthType::Auto);
            }
            WidthUnit::Twips(t) => {
                if *t >= 0 {
                    docx_cell = docx_cell.width(*t as usize, WidthType::Dxa);
                }
            }
            WidthUnit::Percent(bp) => {
                docx_cell = docx_cell.width(*bp as usize, WidthType::Pct);
            }
        }
    } else if let Some(width_twips) = cell.width_twips
        && width_twips >= 0
    {
        docx_cell = docx_cell.width(width_twips as usize, WidthType::Dxa);
    }

    // Apply cell-level padding (overrides table default margins)
    if let Some(ref pad) = cell.padding {
        if let Some(t) = pad.top {
            docx_cell.property = docx_cell
                .property
                .margin_top(t.max(0) as usize, WidthType::Dxa);
        }
        if let Some(r) = pad.right {
            docx_cell.property = docx_cell
                .property
                .margin_right(r.max(0) as usize, WidthType::Dxa);
        }
        if let Some(b) = pad.bottom {
            docx_cell.property = docx_cell
                .property
                .margin_bottom(b.max(0) as usize, WidthType::Dxa);
        }
        if let Some(l) = pad.left {
            docx_cell.property = docx_cell
                .property
                .margin_left(l.max(0) as usize, WidthType::Dxa);
        }
    }

    // Handle merge semantics
    if let Some(merge) = &cell.merge {
        match merge {
            CellMerge::HorizontalStart { span } => {
                docx_cell = docx_cell.grid_span(*span as usize);
            }
            CellMerge::HorizontalContinue => {
                // Filtered out in convert_table_row
            }
            CellMerge::VerticalStart => {
                docx_cell = docx_cell.vertical_merge(VMergeType::Restart);
            }
            CellMerge::VerticalContinue => {
                docx_cell = docx_cell.vertical_merge(VMergeType::Continue);
            }
            CellMerge::None => {}
        }
    }

    // Handle vertical alignment
    if let Some(v_align) = cell.v_align {
        match v_align {
            CellVerticalAlign::Top => {
                docx_cell = docx_cell.vertical_align(VAlignType::Top);
            }
            CellVerticalAlign::Center => {
                docx_cell = docx_cell.vertical_align(VAlignType::Center);
            }
            CellVerticalAlign::Bottom => {
                docx_cell = docx_cell.vertical_align(VAlignType::Bottom);
            }
        }
    }

    // Apply cell shading: IR-source shading takes priority; fall back to profile row striping.
    if let Some(ref shading) = cell.shading
        && let Some(docx_shading) = convert_shading(shading)
    {
        docx_cell = docx_cell.shading(docx_shading);
    } else if let Some(p) = ctx.profile
        && p.components.table.stripe_mode == TableStripeMode::AlternateRows
        && row_idx % 2 == 1
    {
        let fill = p.colors.surface_table_stripe.without_hash().to_string();
        docx_cell = docx_cell.shading(
            Shading::new()
                .shd_type(ShdType::Clear)
                .color("auto")
                .fill(fill),
        );
    }

    // Apply cell borders if present
    if let Some(ref borders) = effective_borders {
        docx_cell = docx_cell.set_borders(convert_border_set(borders));
    }

    // Convert cell content
    for block in &cell.blocks {
        match block {
            Block::Paragraph(para) => {
                docx_cell = docx_cell.add_paragraph(convert_paragraph(para, ctx));
            }
            Block::ListBlock(list) => {
                if let Some(num_id) = ctx.numbering.num_id_for(list.list_id) {
                    for item in &list.items {
                        for item_block in &item.blocks {
                            match item_block {
                                Block::Paragraph(para) => {
                                    let paragraph = convert_paragraph_with_numbering(
                                        para, num_id, item.level, ctx,
                                    );
                                    docx_cell = docx_cell.add_paragraph(paragraph);
                                }
                                Block::ImageBlock(image) => {
                                    let paragraph = convert_image_block(image, ctx.images)?
                                        .numbering(
                                            NumberingId::new(num_id as usize),
                                            IndentLevel::new(item.level as usize),
                                        );
                                    docx_cell = docx_cell.add_paragraph(paragraph);
                                }
                                Block::TableBlock(nested_table) => {
                                    docx_cell =
                                        docx_cell.add_table(convert_table(nested_table, ctx)?);
                                }
                                Block::ListBlock(_) => {}
                            }
                        }
                    }
                } else {
                    // Fallback for malformed IR without registered numbering.
                    for item in &list.items {
                        for item_block in &item.blocks {
                            match item_block {
                                Block::Paragraph(para) => {
                                    docx_cell =
                                        docx_cell.add_paragraph(convert_paragraph(para, ctx));
                                }
                                Block::ImageBlock(image) => {
                                    docx_cell = docx_cell
                                        .add_paragraph(convert_image_block(image, ctx.images)?);
                                }
                                Block::TableBlock(nested_table) => {
                                    docx_cell =
                                        docx_cell.add_table(convert_table(nested_table, ctx)?);
                                }
                                Block::ListBlock(_) => {}
                            }
                        }
                    }
                }
            }
            Block::TableBlock(nested_table) => {
                docx_cell = docx_cell.add_table(convert_table(nested_table, ctx)?);
            }
            Block::ImageBlock(image) => {
                docx_cell = docx_cell.add_paragraph(convert_image_block(image, ctx.images)?);
            }
        }
    }

    Ok(docx_cell)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocxWriterOptions, write_docx_to_bytes};
    use rtfkit_core::{
        Block, Border, BorderSet, CellMerge, CellVerticalAlign, Color, Document, Paragraph,
        RowProps, Run, ShadingPattern, TableBlock, TableCell, TableRow,
    };

    fn zip_entry_string(bytes: &[u8], entry_name: &str) -> String {
        use std::io::{Cursor, Read};
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        let mut entry = archive
            .by_name(entry_name)
            .unwrap_or_else(|_| panic!("missing ZIP entry: {entry_name}"));
        let mut xml = String::new();
        entry.read_to_string(&mut xml).unwrap();
        xml
    }

    // =========================================================================
    // Border pure-function tests
    // =========================================================================

    #[test]
    fn border_style_single_maps_to_single() {
        assert!(matches!(
            border_style_to_docx(IrBorderStyle::Single),
            BorderType::Single
        ));
    }

    #[test]
    fn border_style_none_maps_to_nil() {
        assert!(matches!(
            border_style_to_docx(IrBorderStyle::None),
            BorderType::Nil
        ));
    }

    #[test]
    fn border_style_double_maps_to_double() {
        assert!(matches!(
            border_style_to_docx(IrBorderStyle::Double),
            BorderType::Double
        ));
    }

    #[test]
    fn convert_border_color_is_uppercase_hex() {
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: Some(4),
            color: Some(Color::new(255, 128, 0)),
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.color, "FF8000");
    }

    #[test]
    fn convert_border_no_color_defaults_to_black() {
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: Some(4),
            color: None,
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.color, "000000");
    }

    #[test]
    fn convert_border_size_maps_half_points_to_eighth_points() {
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: Some(4),
            color: None,
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.size, 16); // 2pt = 16 eighth-points
    }

    #[test]
    fn convert_border_default_size_is_one_point() {
        let border = IrBorder {
            style: IrBorderStyle::Single,
            width_half_pts: None,
            color: None,
        };
        let docx_border = convert_border(&border, TableCellBorderPosition::Top);
        assert_eq!(docx_border.size, 8); // 1pt default
    }

    #[test]
    fn convert_border_set_top_left_only() {
        let borders = IrBorderSet {
            top: Some(IrBorder {
                style: IrBorderStyle::Single,
                width_half_pts: Some(4),
                color: None,
            }),
            left: Some(IrBorder {
                style: IrBorderStyle::Dashed,
                width_half_pts: Some(8),
                color: None,
            }),
            ..Default::default()
        };
        let docx_borders = convert_border_set(&borders);
        let _ = docx_borders;
    }

    // =========================================================================
    // Table structure tests
    // =========================================================================

    #[test]
    fn test_simple_table() {
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 1")])),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Cell 2")])),
        ])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
        use std::io::Cursor;
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn test_table_multiple_rows() {
        let table = TableBlock::from_rows(vec![
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R1C1")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R1C2")])),
            ]),
            TableRow::from_cells(vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R2C1")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("R2C2")])),
            ]),
        ]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_with_width() {
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph_with_width(
                Paragraph::from_runs(vec![Run::new("Cell with width")]),
                2880,
            ),
        ])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_with_formatted_content() {
        let mut bold_run = Run::new("bold");
        bold_run.bold = true;
        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![bold_run]),
            )])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_mixed_with_paragraphs() {
        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Table cell")]),
            )])]);
        let doc = Document::from_blocks(vec![
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("Before table")])),
            Block::TableBlock(table),
            Block::Paragraph(Paragraph::from_runs(vec![Run::new("After table")])),
        ]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_empty_table() {
        let table = TableBlock::new();
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_empty_cell() {
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::new(),
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Content")])),
        ])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_nested_table() {
        let inner_table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![TableCell::from_paragraph(
                Paragraph::from_runs(vec![Run::new("Inner cell")]),
            )])]);
        let outer_table = TableBlock::from_rows(vec![TableRow::from_cells(vec![
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Outer cell")])),
            TableCell::from_blocks(vec![Block::TableBlock(inner_table)], None),
        ])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(outer_table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Merge Tests
    // =========================================================================

    #[test]
    fn test_table_cell_horizontal_merge_start() {
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 2 });
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_horizontal_merge_continue_filtered() {
        let mut start_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Start")]));
        start_cell.merge = Some(CellMerge::HorizontalStart { span: 2 });
        let mut continue_cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Continue")]));
        continue_cell.merge = Some(CellMerge::HorizontalContinue);
        let table =
            TableBlock::from_rows(vec![TableRow::from_cells(vec![start_cell, continue_cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
        use std::io::Cursor;
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        assert!(archive.by_name("word/document.xml").is_ok());
    }

    #[test]
    fn test_orphan_horizontal_continue_preserves_text() {
        use std::io::Read;
        let start = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Alpha")]));
        let mut orphan = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bravo")]));
        orphan.merge = Some(CellMerge::HorizontalContinue);
        let end = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Charlie")]));
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![start, orphan, end])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        use std::io::Cursor;
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("Should be valid ZIP");
        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .unwrap();
        assert!(document_xml.contains("Alpha"));
        assert!(document_xml.contains("Bravo"));
        assert!(document_xml.contains("Charlie"));
    }

    #[test]
    fn test_table_cell_vertical_merge_start() {
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top")]));
        cell.merge = Some(CellMerge::VerticalStart);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_merge_continue() {
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Below")]));
        cell.merge = Some(CellMerge::VerticalContinue);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_align_top() {
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Top aligned")]));
        cell.v_align = Some(CellVerticalAlign::Top);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_align_center() {
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Center aligned")]));
        cell.v_align = Some(CellVerticalAlign::Center);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_vertical_align_bottom() {
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Bottom aligned")]));
        cell.v_align = Some(CellVerticalAlign::Bottom);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_cell_merge_none() {
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal")]));
        cell.merge = Some(CellMerge::None);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_table_combined_merge_and_alignment() {
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Combined")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 3 });
        cell.v_align = Some(CellVerticalAlign::Center);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        assert!(!bytes.is_empty());
    }

    // =========================================================================
    // Table Cell Shading Tests
    // =========================================================================

    #[test]
    fn test_table_cell_with_shading() {
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Shaded cell")]));
        cell.shading = Some(rtfkit_core::Shading::solid(Color::new(255, 0, 0)));
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:tcPr>"));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="FF0000""#));
    }

    #[test]
    fn test_table_cell_without_shading_no_shd() {
        let cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Normal cell")]));
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(!document_xml.contains("<w:shd"));
    }

    #[test]
    fn test_table_cell_shading_with_merge() {
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Merged and shaded")]));
        cell.merge = Some(CellMerge::HorizontalStart { span: 2 });
        cell.shading = Some(rtfkit_core::Shading::solid(Color::new(0, 0, 255)));
        let mut cont_cell = TableCell::new();
        cont_cell.merge = Some(CellMerge::HorizontalContinue);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell, cont_cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:gridSpan w:val="2""#));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="0000FF""#));
    }

    #[test]
    fn test_table_cell_shading_with_vertical_align() {
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Aligned and shaded")]));
        cell.v_align = Some(CellVerticalAlign::Center);
        cell.shading = Some(rtfkit_core::Shading::solid(Color::new(128, 128, 128)));
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains(r#"<w:vAlign w:val="center""#));
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:fill="808080""#));
    }

    #[test]
    fn test_table_cell_shading_with_percent_pattern() {
        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 255));
        shading.pattern_color = Some(Color::new(0, 0, 0));
        shading.pattern = Some(ShadingPattern::Percent25);
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("25% pattern")]));
        cell.shading = Some(shading);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="pct25""#));
        assert!(document_xml.contains(r#"w:fill="FFFFFF""#));
        assert!(document_xml.contains(r#"w:color="000000""#));
    }

    #[test]
    fn test_table_cell_shading_with_horz_stripe_pattern() {
        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(Color::new(200, 200, 200));
        shading.pattern_color = Some(Color::new(100, 100, 100));
        shading.pattern = Some(ShadingPattern::HorzStripe);
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Horizontal stripes")]));
        cell.shading = Some(shading);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="horzStripe""#));
        assert!(document_xml.contains(r#"w:fill="C8C8C8""#));
        assert!(document_xml.contains(r#"w:color="646464""#));
    }

    #[test]
    fn test_table_cell_shading_with_diag_cross_pattern() {
        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(Color::new(255, 255, 0));
        shading.pattern_color = Some(Color::new(255, 0, 0));
        shading.pattern = Some(ShadingPattern::DiagCross);
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Diagonal cross")]));
        cell.shading = Some(shading);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="diagCross""#));
        assert!(document_xml.contains(r#"w:fill="FFFF00""#));
        assert!(document_xml.contains(r#"w:color="FF0000""#));
    }

    #[test]
    fn test_table_cell_shading_solid_without_pattern_color() {
        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(Color::new(0, 128, 0));
        shading.pattern = Some(ShadingPattern::Solid);
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Solid green")]));
        cell.shading = Some(shading);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="solid""#));
        assert!(document_xml.contains(r#"w:fill="008000""#));
        assert!(document_xml.contains(r#"w:color="auto""#));
    }

    #[test]
    fn test_table_cell_shading_clear_pattern() {
        let mut shading = rtfkit_core::Shading::new();
        shading.fill_color = Some(Color::new(200, 200, 255));
        shading.pattern = Some(ShadingPattern::Clear);
        let mut cell =
            TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("Clear pattern")]));
        cell.shading = Some(shading);
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = Document::from_blocks(vec![Block::TableBlock(table)]);
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:shd"));
        assert!(document_xml.contains(r#"w:val="clear""#));
        assert!(document_xml.contains(r#"w:fill="C8C8FF""#));
    }

    // =========================================================================
    // Border integration tests
    // =========================================================================

    #[test]
    fn cell_with_borders_builds_docx_without_error() {
        let mut cell = TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("bordered")]));
        cell.borders = Some(BorderSet {
            top: Some(Border {
                style: IrBorderStyle::Single,
                width_half_pts: Some(4),
                color: None,
            }),
            right: Some(Border {
                style: IrBorderStyle::Double,
                width_half_pts: Some(8),
                color: None,
            }),
            ..Default::default()
        });
        let table = TableBlock::from_rows(vec![TableRow::from_cells(vec![cell])]);
        let doc = rtfkit_core::Document {
            blocks: vec![Block::TableBlock(table)],
            structure: None,
            page_management: None,
        };
        let result = write_docx_to_bytes(&doc, &DocxWriterOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn row_border_fallback_emits_tc_borders() {
        let row = TableRow {
            cells: vec![
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("A")])),
                TableCell::from_paragraph(Paragraph::from_runs(vec![Run::new("B")])),
            ],
            row_props: Some(RowProps {
                borders: Some(BorderSet {
                    top: Some(Border {
                        style: IrBorderStyle::Single,
                        width_half_pts: Some(4),
                        color: None,
                    }),
                    bottom: Some(Border {
                        style: IrBorderStyle::Single,
                        width_half_pts: Some(4),
                        color: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        let table = TableBlock::from_rows(vec![row]);
        let doc = rtfkit_core::Document {
            blocks: vec![Block::TableBlock(table)],
            structure: None,
            page_management: None,
        };
        let bytes = write_docx_to_bytes(&doc, &DocxWriterOptions::default()).unwrap();
        let document_xml = zip_entry_string(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:tcBorders"));
        assert!(document_xml.contains("w:top"));
        assert!(document_xml.contains("w:bottom"));
        assert!(document_xml.contains("w:sz=\"16\""));
    }
}
