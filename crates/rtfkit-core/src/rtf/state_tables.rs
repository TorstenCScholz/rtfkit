//! Table State Module
//!
//! This module contains table, merge, and shading state for handling
//! RTF table parsing.

use crate::{CellMerge, CellVerticalAlign, RowProps, TableBlock, TableCell, TableRow};

/// Table parsing state.
///
/// Tracks current table structure, cell boundaries, merge state, and shading.
#[derive(Debug, Clone, Default)]
pub struct TableState {
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

    // =============================================================================
    // Flags
    // =============================================================================
    /// Whether the current paragraph saw \intbl.
    ///
    /// This flag is scoped to the current paragraph and reset at paragraph boundaries.
    /// Table membership itself is derived from active row/cell state.
    pub seen_intbl_in_paragraph: bool,
}

impl TableState {
    /// Creates a new default table state.
    pub fn new() -> Self {
        Self::default()
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
        self.seen_intbl_in_paragraph = false;
    }

    /// Reset state for a new cell.
    pub fn reset_for_new_cell(&mut self) {
        self.pending_cell_merge = None;
        self.pending_cell_v_align = None;
        self.pending_cell_cbpat = None;
        self.pending_cell_cfpat = None;
        self.pending_cell_shading = None;
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
        self.seen_intbl_in_paragraph = false;
        self.pending_table_cbpat = None;
        self.pending_table_cfpat = None;
        self.pending_table_shading = None;
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
