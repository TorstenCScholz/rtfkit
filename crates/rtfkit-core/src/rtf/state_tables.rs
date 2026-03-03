//! Table State Module
//!
//! This module contains table, merge, and shading state for handling
//! RTF table parsing.

use crate::{
    BorderStyle, BoxSpacingTwips, CellMerge, CellVerticalAlign, RowProps, TableBlock, TableCell,
    TableRow,
};

// =============================================================================
// Parser-only border state types (not part of IR)
// =============================================================================

/// Which border side is currently being defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderTarget {
    CellTop,
    CellLeft,
    CellBottom,
    CellRight,
    RowTop,
    RowLeft,
    RowBottom,
    RowRight,
    RowInsideH,
    RowInsideV,
}

/// Partially-resolved border properties (color is still a color-table index).
#[derive(Debug, Clone)]
pub struct PendingBorderAttrs {
    pub style: BorderStyle,
    /// Width in half-points (from `\brdrwN`).
    pub width_half_pts: Option<u32>,
    /// Color table index (from `\brdrcfN`); `None` → auto/black.
    pub color_idx: Option<i32>,
}

/// Cell-side borders captured at one `\cellx` position.
#[derive(Debug, Clone, Default)]
pub struct CellBorderCapture {
    pub top: Option<PendingBorderAttrs>,
    pub left: Option<PendingBorderAttrs>,
    pub bottom: Option<PendingBorderAttrs>,
    pub right: Option<PendingBorderAttrs>,
}

/// Row-level border capture (6 sides).
#[derive(Debug, Clone, Default)]
pub struct RowBorderCapture {
    pub top: Option<PendingBorderAttrs>,
    pub left: Option<PendingBorderAttrs>,
    pub bottom: Option<PendingBorderAttrs>,
    pub right: Option<PendingBorderAttrs>,
    pub inside_h: Option<PendingBorderAttrs>,
    pub inside_v: Option<PendingBorderAttrs>,
}

/// Table parsing state.
///
/// Tracks current table structure, cell boundaries, merge state, and shading.
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// Parent table contexts for nested tables (outermost first).
    pub context_stack: Vec<SavedTableContext>,

    // =============================================================================
    // Current Table Structure
    // =============================================================================
    /// Current table being built
    pub current_table: Option<TableBlock>,
    /// Current row being built
    pub current_row: Option<TableRow>,
    /// Current cell being built
    pub current_cell: Option<TableCell>,

    // =============================================================================
    // Cell Boundaries and Properties (per \cellx)
    // =============================================================================
    /// Cell boundaries encountered in current row (from \cellxN)
    pub pending_cellx: Vec<i32>,
    /// Merge state per cell boundary (from \clmgf, \clmrg, etc. before each \cellx)
    pub pending_cell_merges: Vec<Option<CellMerge>>,
    /// Vertical alignment per cell boundary (from \clvertalt, etc. before each \cellx)
    pub pending_cell_v_aligns: Vec<Option<CellVerticalAlign>>,
    /// Cell background color indexes per cell boundary (stored at each \cellx)
    pub pending_cell_cbpats: Vec<Option<i32>>,
    /// Cell pattern color indexes per cell boundary - for Slice B
    pub pending_cell_cfpats: Vec<Option<i32>>,
    /// Cell shading percentages per cell boundary - for Slice B
    pub pending_cell_shadings: Vec<Option<i32>>,

    // =============================================================================
    // Current Cell Properties (reset per cell)
    // =============================================================================
    /// Pending cell merge state (reset per cell)
    pub pending_cell_merge: Option<CellMerge>,
    /// Pending cell vertical alignment (reset per cell)
    pub pending_cell_v_align: Option<CellVerticalAlign>,
    /// Pending cell background color index (from \clcbpatN, reset per cell)
    pub pending_cell_cbpat: Option<i32>,
    /// Pending cell pattern color index (from \clcfpatN) - for Slice B
    pub pending_cell_cfpat: Option<i32>,
    /// Pending cell shading percentage (from \clshdngN) - for Slice B
    pub pending_cell_shading: Option<i32>,

    // =============================================================================
    // Row Properties (reset per row)
    // =============================================================================
    /// Pending row properties (reset per row)
    pub pending_row_props: RowProps,
    /// Pending row background color index (from \trcbpatN, reset per row)
    pub pending_row_cbpat: Option<i32>,
    /// Pending row pattern color index (from \trcfpatN) - for Slice B
    pub pending_row_cfpat: Option<i32>,
    /// Pending row shading percentage (from \trshdngN) - for Slice B
    pub pending_row_shading: Option<i32>,

    // =============================================================================
    // Table Properties (set from first row)
    // =============================================================================
    /// Pending table background color index (from \trcbpatN at table level)
    pub pending_table_cbpat: Option<i32>,
    /// Pending table pattern color index (from first-row \trcfpatN)
    pub pending_table_cfpat: Option<i32>,
    /// Pending table shading percentage (from first-row \trshdngN)
    pub pending_table_shading: Option<i32>,
    /// Pending table preferred width ftsWidth selector (from \trftsWidth, first row)
    pub pending_table_fts_width: Option<i32>,
    /// Pending table preferred width value (from \trwWidth, first row)
    pub pending_table_w_width: Option<i32>,

    // =============================================================================
    // Width pending (per-row for table, per-cell for cells)
    // =============================================================================
    /// ftsWidth selector for current row's table-level preferred width
    pub pending_row_fts_width: Option<i32>,
    /// wWidth value for current row's table-level preferred width
    pub pending_row_w_width: Option<i32>,
    /// Per-cell ftsWidth selectors (parallel to pending_cellx)
    pub pending_cell_fts_widths: Vec<Option<i32>>,
    /// Per-cell wWidth values (parallel to pending_cellx)
    pub pending_cell_w_widths: Vec<Option<i32>>,
    /// Current cell's ftsWidth selector (reset per cellx)
    pub pending_cell_fts_width: Option<i32>,
    /// Current cell's wWidth value (reset per cellx)
    pub pending_cell_w_width: Option<i32>,

    // =============================================================================
    // Row height (from \trrh)
    // =============================================================================
    /// Raw \trrh value (positive = AtLeast, negative = Exact, zero = unset)
    pub pending_row_height_raw: Option<i32>,

    // =============================================================================
    // Row padding (from \trpadd* / \trpaddf*)
    // =============================================================================
    /// Accumulated row-default padding
    pub pending_row_padding: BoxSpacingTwips,
    /// Per-side padding unit selectors (3 = twips/dxa; others warn and ignore)
    pub pending_row_padding_fmt: [Option<i32>; 4],

    // =============================================================================
    // Cell padding (from \clpad* / \clpadf*)
    // =============================================================================
    /// Current-cell accumulated padding (reset per cellx)
    pub pending_cell_padding: BoxSpacingTwips,
    /// Current-cell per-side padding unit (reset per cellx)
    pub pending_cell_padding_fmt: [Option<i32>; 4],
    /// Per-cellx captured paddings (parallel to pending_cellx)
    pub pending_cell_padding_captures: Vec<BoxSpacingTwips>,
    /// Per-cellx captured padding unit selectors (parallel to pending_cellx)
    pub pending_cell_padding_fmt_captures: Vec<[Option<i32>; 4]>,

    // =============================================================================
    // Border accumulation (cleared at \cellx and \row boundaries)
    // =============================================================================
    /// Which border side the next descriptor applies to.
    pub pending_border_target: Option<BorderTarget>,
    /// Border style from `\brdrs`, `\brdrdb`, etc.
    pub pending_border_style: Option<BorderStyle>,
    /// Border width in half-points from `\brdrwN`.
    pub pending_border_width_hp: Option<u32>,
    /// Border color index from `\brdrcfN`.
    pub pending_border_color_idx: Option<i32>,
    /// Cell border sides being accumulated before the next `\cellx`.
    pub current_cell_borders: CellBorderCapture,
    /// Per-cellx captured cell borders (parallel to `pending_cellx`).
    pub pending_cell_border_captures: Vec<CellBorderCapture>,
    /// Row-level border sides accumulated before `\row`.
    pub pending_row_borders: RowBorderCapture,

    // =============================================================================
    // Flags
    // =============================================================================
    /// Whether the current paragraph saw \intbl.
    ///
    /// This flag is scoped to the current paragraph and reset at paragraph boundaries.
    /// Table membership itself is derived from active row/cell state.
    pub seen_intbl_in_paragraph: bool,

    /// Pending `\itapN` depth hint for the next row definition.
    pub pending_itap: Option<i32>,
    /// Whether `\nesttableprops` was seen before the next row definition.
    pub seen_nesttableprops: bool,
}

/// Snapshot of one active table context used to support nested parsing.
#[derive(Debug, Clone)]
pub struct SavedTableContext {
    pub current_table: Option<TableBlock>,
    pub current_row: Option<TableRow>,
    pub current_cell: Option<TableCell>,
    pub pending_cellx: Vec<i32>,
    pub pending_cell_merges: Vec<Option<CellMerge>>,
    pub pending_cell_v_aligns: Vec<Option<CellVerticalAlign>>,
    pub pending_cell_cbpats: Vec<Option<i32>>,
    pub pending_cell_cfpats: Vec<Option<i32>>,
    pub pending_cell_shadings: Vec<Option<i32>>,
    pub pending_cell_merge: Option<CellMerge>,
    pub pending_cell_v_align: Option<CellVerticalAlign>,
    pub pending_cell_cbpat: Option<i32>,
    pub pending_cell_cfpat: Option<i32>,
    pub pending_cell_shading: Option<i32>,
    pub pending_row_props: RowProps,
    pub pending_row_cbpat: Option<i32>,
    pub pending_row_cfpat: Option<i32>,
    pub pending_row_shading: Option<i32>,
    pub pending_table_cbpat: Option<i32>,
    pub pending_table_cfpat: Option<i32>,
    pub pending_table_shading: Option<i32>,
    pub pending_table_fts_width: Option<i32>,
    pub pending_table_w_width: Option<i32>,
    pub pending_row_fts_width: Option<i32>,
    pub pending_row_w_width: Option<i32>,
    pub pending_cell_fts_widths: Vec<Option<i32>>,
    pub pending_cell_w_widths: Vec<Option<i32>>,
    pub pending_cell_fts_width: Option<i32>,
    pub pending_cell_w_width: Option<i32>,
    pub pending_row_height_raw: Option<i32>,
    pub pending_row_padding: BoxSpacingTwips,
    pub pending_row_padding_fmt: [Option<i32>; 4],
    pub pending_cell_padding: BoxSpacingTwips,
    pub pending_cell_padding_fmt: [Option<i32>; 4],
    pub pending_cell_padding_captures: Vec<BoxSpacingTwips>,
    pub pending_cell_padding_fmt_captures: Vec<[Option<i32>; 4]>,
    pub pending_border_target: Option<BorderTarget>,
    pub pending_border_style: Option<BorderStyle>,
    pub pending_border_width_hp: Option<u32>,
    pub pending_border_color_idx: Option<i32>,
    pub current_cell_borders: CellBorderCapture,
    pub pending_cell_border_captures: Vec<CellBorderCapture>,
    pub pending_row_borders: RowBorderCapture,
    pub seen_intbl_in_paragraph: bool,
}

impl TableState {
    /// Creates a new default table state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Commit the currently accumulated border descriptor into the appropriate
    /// side of `current_cell_borders` or `pending_row_borders`, then clear the
    /// pending accumulator.
    pub fn commit_pending_border(&mut self) {
        let Some(target) = self.pending_border_target.take() else {
            return;
        };
        let style = self
            .pending_border_style
            .take()
            .unwrap_or(BorderStyle::Single);
        let attrs = PendingBorderAttrs {
            style,
            width_half_pts: self.pending_border_width_hp.take(),
            color_idx: self.pending_border_color_idx.take(),
        };
        match target {
            BorderTarget::CellTop => self.current_cell_borders.top = Some(attrs),
            BorderTarget::CellLeft => self.current_cell_borders.left = Some(attrs),
            BorderTarget::CellBottom => self.current_cell_borders.bottom = Some(attrs),
            BorderTarget::CellRight => self.current_cell_borders.right = Some(attrs),
            BorderTarget::RowTop => self.pending_row_borders.top = Some(attrs),
            BorderTarget::RowLeft => self.pending_row_borders.left = Some(attrs),
            BorderTarget::RowBottom => self.pending_row_borders.bottom = Some(attrs),
            BorderTarget::RowRight => self.pending_row_borders.right = Some(attrs),
            BorderTarget::RowInsideH => self.pending_row_borders.inside_h = Some(attrs),
            BorderTarget::RowInsideV => self.pending_row_borders.inside_v = Some(attrs),
        }
    }

    /// Check if we're currently in a table context (have an active table).
    pub fn in_table(&self) -> bool {
        self.current_table.is_some()
    }

    /// Check if we're currently in a row context.
    pub fn in_row(&self) -> bool {
        self.current_row.is_some()
    }

    /// Check if we're currently in a cell context.
    pub fn in_cell(&self) -> bool {
        self.current_cell.is_some()
    }

    /// Current effective table nesting depth (0 when not in a table).
    pub fn nesting_depth(&self) -> usize {
        if self.in_table() {
            self.context_stack.len() + 1
        } else {
            self.context_stack.len()
        }
    }

    /// Returns true when there is a parent table context on the stack.
    pub fn has_parent_context(&self) -> bool {
        !self.context_stack.is_empty()
    }

    fn take_active_context(&mut self) -> SavedTableContext {
        SavedTableContext {
            current_table: self.current_table.take(),
            current_row: self.current_row.take(),
            current_cell: self.current_cell.take(),
            pending_cellx: std::mem::take(&mut self.pending_cellx),
            pending_cell_merges: std::mem::take(&mut self.pending_cell_merges),
            pending_cell_v_aligns: std::mem::take(&mut self.pending_cell_v_aligns),
            pending_cell_cbpats: std::mem::take(&mut self.pending_cell_cbpats),
            pending_cell_cfpats: std::mem::take(&mut self.pending_cell_cfpats),
            pending_cell_shadings: std::mem::take(&mut self.pending_cell_shadings),
            pending_cell_merge: self.pending_cell_merge.take(),
            pending_cell_v_align: self.pending_cell_v_align,
            pending_cell_cbpat: self.pending_cell_cbpat,
            pending_cell_cfpat: self.pending_cell_cfpat,
            pending_cell_shading: self.pending_cell_shading,
            pending_row_props: std::mem::take(&mut self.pending_row_props),
            pending_row_cbpat: self.pending_row_cbpat,
            pending_row_cfpat: self.pending_row_cfpat,
            pending_row_shading: self.pending_row_shading,
            pending_table_cbpat: self.pending_table_cbpat,
            pending_table_cfpat: self.pending_table_cfpat,
            pending_table_shading: self.pending_table_shading,
            pending_table_fts_width: self.pending_table_fts_width,
            pending_table_w_width: self.pending_table_w_width,
            pending_row_fts_width: self.pending_row_fts_width,
            pending_row_w_width: self.pending_row_w_width,
            pending_cell_fts_widths: std::mem::take(&mut self.pending_cell_fts_widths),
            pending_cell_w_widths: std::mem::take(&mut self.pending_cell_w_widths),
            pending_cell_fts_width: self.pending_cell_fts_width,
            pending_cell_w_width: self.pending_cell_w_width,
            pending_row_height_raw: self.pending_row_height_raw,
            pending_row_padding: std::mem::take(&mut self.pending_row_padding),
            pending_row_padding_fmt: self.pending_row_padding_fmt,
            pending_cell_padding: std::mem::take(&mut self.pending_cell_padding),
            pending_cell_padding_fmt: self.pending_cell_padding_fmt,
            pending_cell_padding_captures: std::mem::take(&mut self.pending_cell_padding_captures),
            pending_cell_padding_fmt_captures: std::mem::take(
                &mut self.pending_cell_padding_fmt_captures,
            ),
            pending_border_target: self.pending_border_target,
            pending_border_style: self.pending_border_style,
            pending_border_width_hp: self.pending_border_width_hp,
            pending_border_color_idx: self.pending_border_color_idx,
            current_cell_borders: std::mem::take(&mut self.current_cell_borders),
            pending_cell_border_captures: std::mem::take(&mut self.pending_cell_border_captures),
            pending_row_borders: std::mem::take(&mut self.pending_row_borders),
            seen_intbl_in_paragraph: self.seen_intbl_in_paragraph,
        }
    }

    fn restore_active_context(&mut self, ctx: SavedTableContext) {
        self.current_table = ctx.current_table;
        self.current_row = ctx.current_row;
        self.current_cell = ctx.current_cell;
        self.pending_cellx = ctx.pending_cellx;
        self.pending_cell_merges = ctx.pending_cell_merges;
        self.pending_cell_v_aligns = ctx.pending_cell_v_aligns;
        self.pending_cell_cbpats = ctx.pending_cell_cbpats;
        self.pending_cell_cfpats = ctx.pending_cell_cfpats;
        self.pending_cell_shadings = ctx.pending_cell_shadings;
        self.pending_cell_merge = ctx.pending_cell_merge;
        self.pending_cell_v_align = ctx.pending_cell_v_align;
        self.pending_cell_cbpat = ctx.pending_cell_cbpat;
        self.pending_cell_cfpat = ctx.pending_cell_cfpat;
        self.pending_cell_shading = ctx.pending_cell_shading;
        self.pending_row_props = ctx.pending_row_props;
        self.pending_row_cbpat = ctx.pending_row_cbpat;
        self.pending_row_cfpat = ctx.pending_row_cfpat;
        self.pending_row_shading = ctx.pending_row_shading;
        self.pending_table_cbpat = ctx.pending_table_cbpat;
        self.pending_table_cfpat = ctx.pending_table_cfpat;
        self.pending_table_shading = ctx.pending_table_shading;
        self.pending_table_fts_width = ctx.pending_table_fts_width;
        self.pending_table_w_width = ctx.pending_table_w_width;
        self.pending_row_fts_width = ctx.pending_row_fts_width;
        self.pending_row_w_width = ctx.pending_row_w_width;
        self.pending_cell_fts_widths = ctx.pending_cell_fts_widths;
        self.pending_cell_w_widths = ctx.pending_cell_w_widths;
        self.pending_cell_fts_width = ctx.pending_cell_fts_width;
        self.pending_cell_w_width = ctx.pending_cell_w_width;
        self.pending_row_height_raw = ctx.pending_row_height_raw;
        self.pending_row_padding = ctx.pending_row_padding;
        self.pending_row_padding_fmt = ctx.pending_row_padding_fmt;
        self.pending_cell_padding = ctx.pending_cell_padding;
        self.pending_cell_padding_fmt = ctx.pending_cell_padding_fmt;
        self.pending_cell_padding_captures = ctx.pending_cell_padding_captures;
        self.pending_cell_padding_fmt_captures = ctx.pending_cell_padding_fmt_captures;
        self.pending_border_target = ctx.pending_border_target;
        self.pending_border_style = ctx.pending_border_style;
        self.pending_border_width_hp = ctx.pending_border_width_hp;
        self.pending_border_color_idx = ctx.pending_border_color_idx;
        self.current_cell_borders = ctx.current_cell_borders;
        self.pending_cell_border_captures = ctx.pending_cell_border_captures;
        self.pending_row_borders = ctx.pending_row_borders;
        self.seen_intbl_in_paragraph = ctx.seen_intbl_in_paragraph;
    }

    /// Push current table context and start a fresh child context.
    pub fn enter_child_context(&mut self) {
        let parent = self.take_active_context();
        self.context_stack.push(parent);
        self.pending_itap = None;
        self.seen_nesttableprops = false;
        self.ensure_table();
    }

    /// Pop and restore the parent table context.
    pub fn restore_parent_context(&mut self) -> bool {
        let Some(parent) = self.context_stack.pop() else {
            return false;
        };
        self.restore_active_context(parent);
        true
    }

    /// Reset state for a new row (called on \trowd).
    pub fn reset_for_new_row(&mut self) {
        self.pending_cellx.clear();
        self.pending_cell_merges.clear();
        self.pending_cell_v_aligns.clear();
        self.pending_cell_cbpats.clear();
        self.pending_cell_cfpats.clear();
        self.pending_cell_shadings.clear();
        self.current_row = Some(TableRow::new());
        self.pending_row_props = RowProps::default();
        self.pending_row_cbpat = None;
        self.pending_row_cfpat = None;
        self.pending_row_shading = None;
        self.pending_cell_merge = None;
        self.pending_cell_v_align = None;
        self.pending_cell_cbpat = None;
        self.pending_cell_cfpat = None;
        self.pending_cell_shading = None;
        // Border accumulation reset
        self.pending_cell_border_captures.clear();
        self.current_cell_borders = CellBorderCapture::default();
        self.pending_row_borders = RowBorderCapture::default();
        self.pending_border_target = None;
        self.pending_border_style = None;
        self.pending_border_width_hp = None;
        self.pending_border_color_idx = None;
        self.seen_intbl_in_paragraph = false;
        // Width model reset
        self.pending_row_fts_width = None;
        self.pending_row_w_width = None;
        self.pending_cell_fts_widths.clear();
        self.pending_cell_w_widths.clear();
        self.pending_cell_fts_width = None;
        self.pending_cell_w_width = None;
        // Row height reset
        self.pending_row_height_raw = None;
        // Padding reset
        self.pending_row_padding = BoxSpacingTwips::default();
        self.pending_row_padding_fmt = [None; 4];
        self.pending_cell_padding = BoxSpacingTwips::default();
        self.pending_cell_padding_fmt = [None; 4];
        self.pending_cell_padding_captures.clear();
        self.pending_cell_padding_fmt_captures.clear();
    }

    /// Reset state for a new cell.
    pub fn reset_for_new_cell(&mut self) {
        self.pending_cell_merge = None;
        self.pending_cell_v_align = None;
        self.pending_cell_cbpat = None;
        self.pending_cell_cfpat = None;
        self.pending_cell_shading = None;
        self.pending_cell_fts_width = None;
        self.pending_cell_w_width = None;
        self.pending_cell_padding = BoxSpacingTwips::default();
        self.pending_cell_padding_fmt = [None; 4];
    }

    /// Reset paragraph-level table state.
    pub fn reset_paragraph_state(&mut self) {
        self.seen_intbl_in_paragraph = false;
    }

    /// Start a new table if not already in one.
    pub fn ensure_table(&mut self) {
        if self.current_table.is_none() {
            self.current_table = Some(TableBlock::new());
        }
    }

    /// Record cell boundary (called on \cellxN).
    pub fn record_cellx(&mut self, boundary: i32) {
        self.pending_cellx.push(boundary);
        // Store the current merge state and vertical alignment for this cell
        self.pending_cell_merges
            .push(self.pending_cell_merge.take());
        self.pending_cell_v_aligns
            .push(self.pending_cell_v_align.take());
        // Store the current cell shading state for this cell
        self.pending_cell_cbpats
            .push(self.pending_cell_cbpat.take());
        self.pending_cell_cfpats
            .push(self.pending_cell_cfpat.take());
        self.pending_cell_shadings
            .push(self.pending_cell_shading.take());
        // Commit any trailing border descriptor for this cell, then store
        self.commit_pending_border();
        self.pending_cell_border_captures
            .push(self.current_cell_borders.clone());
        self.current_cell_borders = CellBorderCapture::default();
        // Store width model state for this cell
        self.pending_cell_fts_widths
            .push(self.pending_cell_fts_width.take());
        self.pending_cell_w_widths
            .push(self.pending_cell_w_width.take());
        // Store padding state for this cell
        self.pending_cell_padding_captures
            .push(std::mem::take(&mut self.pending_cell_padding));
        self.pending_cell_padding_fmt_captures
            .push(self.pending_cell_padding_fmt);
        self.pending_cell_padding_fmt = [None; 4];
    }

    /// Clear state after table finalization.
    pub fn clear_table(&mut self) {
        self.current_table = None;
        self.current_row = None;
        self.current_cell = None;
        self.pending_cellx.clear();
        self.pending_cell_merges.clear();
        self.pending_cell_v_aligns.clear();
        self.pending_cell_cbpats.clear();
        self.pending_cell_cfpats.clear();
        self.pending_cell_shadings.clear();
        self.pending_cell_border_captures.clear();
        self.current_cell_borders = CellBorderCapture::default();
        self.pending_row_borders = RowBorderCapture::default();
        self.seen_intbl_in_paragraph = false;
        self.pending_table_cbpat = None;
        self.pending_table_cfpat = None;
        self.pending_table_shading = None;
        self.pending_table_fts_width = None;
        self.pending_table_w_width = None;
        self.pending_cell_fts_widths.clear();
        self.pending_cell_w_widths.clear();
        self.pending_cell_padding_captures.clear();
        self.pending_cell_padding_fmt_captures.clear();
        self.pending_itap = None;
        self.seen_nesttableprops = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_state_default() {
        let state = TableState::new();
        assert!(state.current_table.is_none());
        assert!(state.current_row.is_none());
        assert!(state.current_cell.is_none());
        assert!(state.pending_cellx.is_empty());
        assert!(!state.in_table());
        assert!(!state.in_row());
        assert!(!state.in_cell());
    }

    #[test]
    fn test_table_state_ensure_table() {
        let mut state = TableState::new();
        state.ensure_table();
        assert!(state.in_table());
        assert!(state.current_table.is_some());
    }

    #[test]
    fn test_table_state_reset_for_new_row() {
        let mut state = TableState::new();
        state.pending_cellx.push(1000);
        state
            .pending_cell_merges
            .push(Some(CellMerge::HorizontalStart { span: 1 }));
        state.pending_row_cbpat = Some(5);

        state.reset_for_new_row();

        assert!(state.pending_cellx.is_empty());
        assert!(state.pending_cell_merges.is_empty());
        assert!(state.current_row.is_some());
        assert!(state.pending_row_cbpat.is_none());
    }

    #[test]
    fn test_table_state_record_cellx() {
        let mut state = TableState::new();
        state.pending_cell_merge = Some(CellMerge::HorizontalStart { span: 1 });
        state.pending_cell_v_align = Some(CellVerticalAlign::Center);
        state.pending_cell_cbpat = Some(3);

        state.record_cellx(2880);

        assert_eq!(state.pending_cellx.len(), 1);
        assert_eq!(state.pending_cellx[0], 2880);
        assert_eq!(state.pending_cell_merges.len(), 1);
        assert_eq!(
            state.pending_cell_merges[0],
            Some(CellMerge::HorizontalStart { span: 1 })
        );
        assert_eq!(
            state.pending_cell_v_aligns[0],
            Some(CellVerticalAlign::Center)
        );
        assert_eq!(state.pending_cell_cbpats[0], Some(3));

        // Pending values should be cleared after recording
        assert!(state.pending_cell_merge.is_none());
        assert!(state.pending_cell_v_align.is_none());
        assert!(state.pending_cell_cbpat.is_none());
    }

    #[test]
    fn test_table_state_clear_table() {
        let mut state = TableState::new();
        state.current_table = Some(TableBlock::new());
        state.current_row = Some(TableRow::new());
        state.pending_cellx.push(1000);
        state.pending_table_cbpat = Some(5);

        state.clear_table();

        assert!(state.current_table.is_none());
        assert!(state.current_row.is_none());
        assert!(state.pending_cellx.is_empty());
        assert!(state.pending_table_cbpat.is_none());
    }
}
