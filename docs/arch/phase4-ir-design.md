# Phase 4 IR Model Extension Design

## Overview

This document describes the IR model extension for Phase 4 table support, following the specification in [`PHASE4.md`](../specs/PHASE4.md). The design adds explicit table structures to the IR while maintaining consistency with existing patterns established in Phase 3 for list support.

## Current State Analysis

### Existing Block Enum

The current [`Block`](../../crates/rtfkit-core/src/lib.rs:282) enum supports paragraphs and lists:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    Paragraph(Paragraph),
    ListBlock(ListBlock),
}
```

### Existing Warning Pattern

The [`Warning`](../../crates/rtfkit-core/src/report.rs:50) enum uses a tagged union pattern with severity:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Warning {
    UnsupportedControlWord { word: String, parameter: Option<i32>, severity: WarningSeverity },
    UnsupportedListControl { control_word: String, severity: WarningSeverity },
    // ... other variants
}
```

---

## IR Model Extension

### New Types

Add the following types to [`crates/rtfkit-core/src/lib.rs`](../../crates/rtfkit-core/src/lib.rs):

#### TableBlock Struct

```rust
/// A table containing one or more rows.
///
/// Tables are block-level elements that contain rows of cells.
/// Each row contains cells that can hold block content.
///
/// # Example
/// ```
/// use rtfkit_core::{TableBlock, TableRow, TableCell, Paragraph, Run, Block};
/// let table = TableBlock {
///     rows: vec![
///         TableRow {
///             cells: vec![
///                 TableCell {
///                     blocks: vec![Block::Paragraph(Paragraph::from_runs(vec![Run::new("Cell 1")]))],
///                     width_twips: Some(1440), // 1 inch
///                 },
///             ],
///         },
///     ],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableBlock {
    /// The rows that make up this table
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<TableRow>,
}

impl TableBlock {
    /// Creates a new empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a table from a vector of rows.
    pub fn from_rows(rows: Vec<TableRow>) -> Self {
        Self { rows }
    }

    /// Adds a row to the table.
    pub fn add_row(&mut self, row: TableRow) {
        self.rows.push(row);
    }

    /// Returns true if the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the number of rows in the table.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}
```

#### TableRow Struct

```rust
/// A row within a table.
///
/// Each row contains a sequence of cells. The number of cells
/// may vary between rows in malformed RTF, though well-formed
/// tables have consistent column counts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableRow {
    /// The cells in this row
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<TableCell>,
}

impl TableRow {
    /// Creates a new empty row.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a row from a vector of cells.
    pub fn from_cells(cells: Vec<TableCell>) -> Self {
        Self { cells }
    }

    /// Adds a cell to the row.
    pub fn add_cell(&mut self, cell: TableCell) {
        self.cells.push(cell);
    }

    /// Returns true if the row has no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Returns the number of cells in this row.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}
```

#### TableCell Struct

```rust
/// A cell within a table row.
///
/// Cells contain block-level content (paragraphs and lists in Phase 4).
/// Width is stored as computed cell width in twips (1/20th of a point).
/// A `None` value indicates the width was not specified or could not be determined.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableCell {
    /// Block content within this cell (Paragraph and ListBlock in Phase 4)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
    
    /// Computed cell width in twips (1/20th point)
    /// None if width not specified or determinable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_twips: Option<i32>,
}

impl TableCell {
    /// Creates a new empty cell.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a cell from blocks with optional width.
    pub fn from_blocks(blocks: Vec<Block>, width_twips: Option<i32>) -> Self {
        Self { blocks, width_twips }
    }

    /// Creates a cell from a single paragraph.
    pub fn from_paragraph(paragraph: Paragraph) -> Self {
        Self {
            blocks: vec![Block::Paragraph(paragraph)],
            width_twips: None,
        }
    }

    /// Creates a cell from a paragraph with width.
    pub fn from_paragraph_with_width(paragraph: Paragraph, width_twips: i32) -> Self {
        Self {
            blocks: vec![Block::Paragraph(paragraph)],
            width_twips: Some(width_twips),
        }
    }

    /// Adds a block to the cell.
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Returns true if the cell has no content.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Sets the cell width in twips.
    pub fn with_width(mut self, width_twips: i32) -> Self {
        self.width_twips = Some(width_twips);
        self
    }
}
```

### Modified Block Enum

Update the [`Block`](../../crates/rtfkit-core/src/lib.rs:282) enum to include `TableBlock`:

```rust
/// A block-level element in the document.
///
/// `Block` represents the top-level structural elements of a document.
/// Currently supports paragraphs, lists, and tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    /// A paragraph block containing text
    Paragraph(Paragraph),
    /// A list block containing items
    ListBlock(ListBlock),
    /// A table block containing rows and cells
    TableBlock(TableBlock),
}
```

---

## Warning Enum Extensions

### New Warning Variants

Add the following variants to [`Warning`](../../crates/rtfkit-core/src/report.rs:50) enum:

```rust
/// A table-related control word was encountered but not fully supported.
///
/// This indicates table functionality that is recognized but partially implemented
/// or intentionally deferred to a later phase.
UnsupportedTableControl {
    /// The control word that was encountered (without leading backslash)
    control_word: String,
    /// Severity of this warning
    severity: WarningSeverity,
},

/// The table structure is malformed or incomplete.
///
/// This indicates structural issues like mismatched cell counts,
/// missing terminators, or invalid nesting.
MalformedTableStructure {
    /// Human-readable description of the issue
    reason: String,
    /// Severity of this warning
    severity: WarningSeverity,
},

/// A table cell was not properly closed before row/document end.
///
/// This indicates a missing \cell control word. The interpreter
/// auto-closes the cell to preserve content.
UnclosedTableCell {
    /// Severity of this warning
    severity: WarningSeverity,
},

/// A table row was not properly closed before next row/document end.
///
/// This indicates a missing \row control word. The interpreter
/// auto-closes the row to preserve content.
UnclosedTableRow {
    /// Severity of this warning
    severity: WarningSeverity,
},
```

### Warning Helper Methods

Add to the `Warning` impl block in [`report.rs`](../../crates/rtfkit-core/src/report.rs:121):

```rust
/// Creates a new `UnsupportedTableControl` warning.
pub fn unsupported_table_control(control_word: impl Into<String>) -> Self {
    Warning::UnsupportedTableControl {
        control_word: control_word.into(),
        severity: WarningSeverity::Warning,
    }
}

/// Creates a new `MalformedTableStructure` warning.
pub fn malformed_table_structure(reason: impl Into<String>) -> Self {
    Warning::MalformedTableStructure {
        reason: reason.into(),
        severity: WarningSeverity::Warning,
    }
}

/// Creates a new `UnclosedTableCell` warning.
pub fn unclosed_table_cell() -> Self {
    Warning::UnclosedTableCell {
        severity: WarningSeverity::Warning,
    }
}

/// Creates a new `UnclosedTableRow` warning.
pub fn unclosed_table_row() -> Self {
    Warning::UnclosedTableRow {
        severity: WarningSeverity::Warning,
    }
}
```

### Update severity() Method

Update the [`severity()`](../../crates/rtfkit-core/src/report.rs:174) method to handle new variants:

```rust
/// Returns the severity of this warning.
pub fn severity(&self) -> WarningSeverity {
    match self {
        Warning::UnsupportedControlWord { severity, .. } => *severity,
        Warning::UnknownDestination { severity, .. } => *severity,
        Warning::DroppedContent { severity, .. } => *severity,
        Warning::UnsupportedListControl { severity, .. } => *severity,
        Warning::UnresolvedListOverride { severity, .. } => *severity,
        Warning::UnsupportedNestingLevel { severity, .. } => *severity,
        // Phase 4 additions
        Warning::UnsupportedTableControl { severity, .. } => *severity,
        Warning::MalformedTableStructure { severity, .. } => *severity,
        Warning::UnclosedTableCell { severity } => *severity,
        Warning::UnclosedTableRow { severity } => *severity,
    }
}
```

### ReportBuilder Extensions

Add to [`ReportBuilder`](../../crates/rtfkit-core/src/report.rs:294) impl block:

```rust
/// Records an unsupported table control word.
///
/// If the warning count limit has been reached, this is a no-op.
pub fn unsupported_table_control(&mut self, control_word: &str) {
    if self.can_add_warning() {
        self.warnings
            .push(Warning::unsupported_table_control(control_word));
    }
}

/// Records a malformed table structure issue.
///
/// If the warning count limit has been reached, this is a no-op.
pub fn malformed_table_structure(&mut self, reason: &str) {
    if self.can_add_warning() {
        self.warnings
            .push(Warning::malformed_table_structure(reason));
    }
}

/// Records an unclosed table cell warning.
///
/// If the warning count limit has been reached, this is a no-op.
pub fn unclosed_table_cell(&mut self) {
    if self.can_add_warning() {
        self.warnings.push(Warning::unclosed_table_cell());
    }
}

/// Records an unclosed table row warning.
///
/// If the warning count limit has been reached, this is a no-op.
pub fn unclosed_table_row(&mut self) {
    if self.can_add_warning() {
        self.warnings.push(Warning::unclosed_table_row());
    }
}
```

---

## Serialization Notes

### JSON Format

The IR JSON format will include table blocks. Example:

```json
{
  "blocks": [
    {
      "type": "tableblock",
      "rows": [
        {
          "cells": [
            {
              "blocks": [
                {
                  "type": "paragraph",
                  "alignment": "left",
                  "runs": [{"text": "Cell 1,1", "bold": false, "italic": false, "underline": false}]
                }
              ],
              "width_twips": 2880
            },
            {
              "blocks": [
                {
                  "type": "paragraph",
                  "alignment": "left",
                  "runs": [{"text": "Cell 1,2", "bold": false, "italic": false, "underline": false}]
                }
              ],
              "width_twips": 2880
            }
          ]
        }
      ]
    }
  ]
}
```

### Serialization Attributes

The types use the following serde attributes for clean JSON output:

| Type | Attribute | Purpose |
|------|-----------|---------|
| `TableBlock.rows` | `#[serde(default, skip_serializing_if = "Vec::is_empty")]` | Omit empty rows array |
| `TableRow.cells` | `#[serde(default, skip_serializing_if = "Vec::is_empty")]` | Omit empty cells array |
| `TableCell.blocks` | `#[serde(default, skip_serializing_if = "Vec::is_empty")]` | Omit empty blocks array |
| `TableCell.width_twips` | `#[serde(skip_serializing_if = "Option::is_none")]` | Omit unspecified width |

### Backward Compatibility

The addition of `TableBlock` to the `Block` enum is a **breaking change** for the IR JSON format:
- Existing JSON files without table blocks remain valid
- Consumers that only handle `Paragraph` and `ListBlock` will need updates
- Golden tests will need regeneration

---

## Design Decisions and Trade-offs

### Decision 1: Explicit Table Block vs Paragraph Annotation

**Chosen**: Explicit `TableBlock` type in the IR.

**Alternatives Considered**:
- Adding `in_table: bool` flag to `Paragraph`
- Using nested `Block` references

**Rationale**:
- Clean separation of concerns
- Enables future nested content (lists in cells)
- Provides clear writer API for DOCX mapping
- Follows the pattern established by `ListBlock` in Phase 3

### Decision 2: Width Storage as Computed Cell Width

**Chosen**: Store `width_twips` as computed cell width (delta between adjacent `\cellxN` boundaries).

**Alternatives Considered**:
- Store right-boundary value directly (`\cellxN`)
- Store both boundary and width

**Rationale**:
- Matches DOCX `w:tcW` semantics directly
- Avoids duplicate width math in writer implementations
- Keeps IR values intuitive for downstream consumers
- Still derives deterministically from source `\cellx` boundaries

### Decision 3: Recursive Block Content in Cells

**Chosen**: `TableCell.blocks: Vec<Block>` allowing any block type.

**Alternatives Considered**:
- `TableCell.paragraphs: Vec<Paragraph>` (Phase 4 minimum only)
- Separate `CellContent` enum

**Rationale**:
- Future-proof for nested tables (Phase 5+)
- Supports lists in cells immediately
- Consistent with `ListItem.blocks` pattern from Phase 3
- Empty cells are naturally represented as empty `Vec`

### Decision 4: Warning Granularity

**Chosen**: Four specific warning variants for table issues.

**Alternatives Considered**:
- Single `TableWarning` with reason field
- Reusing `MalformedTableStructure` for all issues

**Rationale**:
- Enables specific programmatic handling
- Clear semantics for each issue type
- Follows existing warning pattern (e.g., `UnresolvedListOverride`)
- Allows severity differentiation per issue type

### Decision 5: No Default Width

**Chosen**: `width_twips: Option<i32>` with `None` default.

**Alternatives Considered**:
- Default to 0
- Compute default from table width / cell count

**Rationale**:
- `None` clearly indicates "unspecified" vs "zero width"
- DOCX can use auto-layout when width is omitted
- Avoids misleading width values in output
- Consistent with RTF semantics (missing `\cellx` = no width specified)

---

## Edge Cases

### Empty Tables

An empty `TableBlock` (no rows) is valid and serializes as:
```json
{"type": "tableblock"}
```

### Empty Rows

A `TableRow` with no cells is valid and serializes as:
```json
{"cells": []}
```
or simply `{}` with `skip_serializing_if`.

**Note**: The DOCX writer should handle empty rows gracefully (emit empty `w:tr`).

### Empty Cells

A `TableCell` with no blocks is valid and represents an empty cell:
```json
{"blocks": []}
```
or with width only:
```json
{"width_twips": 1440}
```

### Inconsistent Column Counts

RTF allows rows with different cell counts. The IR preserves this:
```rust
TableBlock {
    rows: vec![
        TableRow { cells: vec![cell1, cell2] },      // 2 columns
        TableRow { cells: vec![cell1, cell2, cell3] }, // 3 columns
    ],
}
```

**Warning**: `MalformedTableStructure` should be emitted when column counts differ.

### Nested Blocks in Cells

Phase 4 supports `Paragraph` and `ListBlock` in cells:
```rust
TableCell {
    blocks: vec![
        Block::Paragraph(Paragraph::from_runs(vec![Run::new("Intro")])),
        Block::ListBlock(ListBlock::new(1, ListKind::Bullet)),
    ],
    width_twips: Some(2880),
}
```

**Note**: Nested `TableBlock` in cells is structurally allowed but not produced by the Phase 4 interpreter.

---

## Summary of Changes

### Files Modified

| File | Changes |
|------|---------|
| [`crates/rtfkit-core/src/lib.rs`](../../crates/rtfkit-core/src/lib.rs) | Add `TableBlock`, `TableRow`, `TableCell` types; update `Block` enum |
| [`crates/rtfkit-core/src/report.rs`](../../crates/rtfkit-core/src/report.rs) | Add `UnsupportedTableControl`, `MalformedTableStructure`, `UnclosedTableCell`, `UnclosedTableRow` warnings; update `ReportBuilder` |

### Type Summary

| Type | Derives | Key Fields |
|------|---------|------------|
| `TableBlock` | Debug, Clone, PartialEq, Default, Serialize, Deserialize | `rows: Vec<TableRow>` |
| `TableRow` | Debug, Clone, PartialEq, Default, Serialize, Deserialize | `cells: Vec<TableCell>` |
| `TableCell` | Debug, Clone, PartialEq, Default, Serialize, Deserialize | `blocks: Vec<Block>`, `width_twips: Option<i32>` |

### Warning Summary

| Variant | Severity | Purpose |
|---------|----------|---------|
| `UnsupportedTableControl` | Warning | Recognized but unimplemented control word |
| `MalformedTableStructure` | Warning | Structural issues in table |
| `UnclosedTableCell` | Warning | Missing `\cell` terminator |
| `UnclosedTableRow` | Warning | Missing `\row` terminator |

---

## Implementation Checklist

- [ ] Add `TableBlock`, `TableRow`, `TableCell` structs to `lib.rs`
- [ ] Update `Block` enum with `TableBlock` variant
- [ ] Add table warning variants to `Warning` enum in `report.rs`
- [ ] Add warning helper methods to `Warning` impl
- [ ] Update `Warning::severity()` match expression
- [ ] Add `ReportBuilder` methods for table warnings
- [ ] Update re-exports in `lib.rs` if needed
- [ ] Regenerate golden test files after IR changes

---

## Phase 5 Extensions

Phase 5 extends the table IR defined here with merge semantics. See [phase5-ir-design.md](phase5-ir-design.md) for details on:

- `CellMerge` enum for horizontal and vertical merge semantics
- `CellVerticalAlign` enum for cell content alignment
- `RowAlignment` and `RowProps` for row-level formatting
- Merge normalization algorithm and conflict resolution
- DOCX merge mapping (`w:gridSpan`, `w:vMerge`, `w:vAlign`)
