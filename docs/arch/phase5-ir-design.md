# Phase 5 IR Design: Table Merge Semantics

## Overview

Phase 5 extends the table IR to support merge semantics and formatting properties. This document describes the IR extensions added for cell merging, vertical alignment, and row-level formatting.

## New Types

### CellMerge

The `CellMerge` enum represents the merge state of a table cell:

```rust
/// Represents the merge state of a table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CellMerge {
    /// Cell is not part of a merge (default)
    #[default]
    None,
    /// Cell is the start of a horizontal merge (span indicates width)
    HorizontalStart { span: u16 },
    /// Cell is a continuation of a horizontal merge (content omitted)
    HorizontalContinue,
    /// Cell is the start of a vertical merge
    VerticalStart,
    /// Cell is a continuation of a vertical merge (content omitted)
    VerticalContinue,
}
```

### CellVerticalAlign

The `CellVerticalAlign` enum represents vertical text alignment within a cell:

```rust
/// Vertical alignment of content within a table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellVerticalAlign {
    /// Align content to top of cell
    #[default]
    Top,
    /// Center content vertically in cell
    Center,
    /// Align content to bottom of cell
    Bottom,
}
```

### RowAlignment

The `RowAlignment` enum represents horizontal alignment for an entire row:

```rust
/// Horizontal alignment for a table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowAlignment {
    /// Left-aligned (default)
    #[default]
    Left,
    /// Center-aligned
    Center,
    /// Right-aligned
    Right,
}
```

### RowProps

The `RowProps` struct collects row-level formatting properties:

```rust
/// Row-level formatting properties.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RowProps {
    /// Horizontal alignment for the row
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<RowAlignment>,
    /// Left indent in twips (from `\trleft`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_indent: Option<i32>,
}
```

### TableProps

The `TableProps` struct is a placeholder for table-level properties:

```rust
/// Table-level formatting properties (placeholder for future expansion).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableProps {
    // Future: borders, shading, preferred width, etc.
}
```

## Extended Types

### TableCell Extensions

The `TableCell` struct is extended with merge and alignment fields:

```rust
pub struct TableCell {
    /// Block content within this cell
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
    
    /// Computed cell width in twips
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_twips: Option<i32>,
    
    /// Merge state for this cell (Phase 5)
    #[serde(default, skip_serializing_if = "is_default")]
    pub merge: CellMerge,
    
    /// Vertical alignment for cell content (Phase 5)
    #[serde(default, skip_serializing_if = "is_default")]
    pub v_align: CellVerticalAlign,
}
```

### TableRow Extensions

The `TableRow` struct is extended with row properties:

```rust
pub struct TableRow {
    /// The cells in this row
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<TableCell>,
    
    /// Row-level formatting properties (Phase 5)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_props: Option<RowProps>,
}
```

### TableBlock Extensions

The `TableBlock` struct is extended with table properties:

```rust
pub struct TableBlock {
    /// The rows that make up this table
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<TableRow>,
    
    /// Table-level formatting properties (Phase 5)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_props: Option<TableProps>,
}
```

## RTF Control Word Mapping

| RTF Control | IR Field | DOCX Output |
|-------------|----------|-------------|
| `\clmgf` | `CellMerge::HorizontalStart` | `w:gridSpan` |
| `\clmrg` | `CellMerge::HorizontalContinue` | (cell omitted) |
| `\clvmgf` | `CellMerge::VerticalStart` | `w:vMerge="restart"` |
| `\clvmrg` | `CellMerge::VerticalContinue` | `w:vMerge="continue"` |
| `\clvertalt` | `CellVerticalAlign::Top` | `w:vAlign="top"` |
| `\clvertalc` | `CellVerticalAlign::Center` | `w:vAlign="center"` |
| `\clvertalb` | `CellVerticalAlign::Bottom` | `w:vAlign="bottom"` |
| `\trql` | `RowAlignment::Left` | (default) |
| `\trqc` | `RowAlignment::Center` | (not supported by docx-rs) |
| `\trqr` | `RowAlignment::Right` | (not supported by docx-rs) |
| `\trleft` | `RowProps.left_indent` | (not supported by docx-rs) |

## Merge Normalization Algorithm

The interpreter processes merge controls in two phases:

### Phase 1: Collection

1. Track merge markers per cell position as control words are encountered
2. Store pending `\clmgf`, `\clmrg`, `\clvmgf`, `\clvmrg` flags
3. Wait for `\cellxN` to finalize cell descriptor

### Phase 2: Row Finalization

On `\row` (row terminator):

1. **Identify merge chains**: Group adjacent cells with merge markers
2. **Calculate horizontal spans**: Count continuation cells following a start
3. **Validate span bounds**: Ensure span doesn't exceed row cell count
4. **Resolve conflicts**: Handle orphan continuations and overlapping merges
5. **Emit warnings**: Report any semantic conflicts

```
Example: Horizontal merge for 3 cells

RTF: \clmgf\cellx1440 \clmrg\cellx2880 \clmrg\cellx4320
IR:  [HorizontalStart{span:3}, HorizontalContinue, HorizontalContinue]
DOCX: <w:tc><w:tcPr><w:gridSpan w:val="3"/>...</w:tcPr></w:tc>
```

## Conflict Resolution

| Conflict | Resolution | Warning |
|----------|------------|---------|
| Orphan continuation (no start) | Convert to standalone cell | `MergeConflict` + `DroppedContent` |
| Span exceeds bounds | Clamp to available cells | `TableGeometryConflict` + `DroppedContent` |
| Both horizontal and vertical merge | Prefer horizontal, drop vertical | `MergeConflict` + `DroppedContent` |

### Orphan Continuation Handling

An orphan continuation is a `\clmrg` or `\clvmrg` without a preceding start marker:

```
RTF: \clmrg\cellx1440 \cellx2880
IR:  [None, None]  // First cell converted to standalone
Warning: MergeConflict + DroppedContent
```

### Span Bounds Validation

When a horizontal span would exceed the row boundary:

```
RTF: \clmgf\cellx1440 \clmrg\cellx2880
IR:  [HorizontalStart{span:2}]  // Clamped to 2 (available cells)
Warning: TableGeometryConflict + DroppedContent
```

## Strict Mode Behavior

Strict mode enforces semantic preservation:

| Scenario | Strict Mode | Non-Strict Mode |
|----------|-------------|-----------------|
| Orphan continuation | Exit code 4 | Warning only |
| Span exceeds bounds | Exit code 4 | Warning only |
| Conflicting merge types | Exit code 4 | Warning only |
| Unsupported formatting (row align) | Warning only | Warning only |

The `DroppedContent` signal is preserved through the warning cap mechanism, ensuring strict mode can detect semantic loss even when warnings are truncated.

## Warning Categories

### MergeConflict

Emitted when merge semantics cannot be fully preserved:

```rust
Warning::MergeConflict {
    description: String,
    severity: WarningSeverity,
}
```

### TableGeometryConflict

Emitted when table geometry is invalid:

```rust
Warning::TableGeometryConflict {
    description: String,
    severity: WarningSeverity,
}
```

## Parser Limits

Phase 5 adds table-specific limits:

| Limit | Default | Purpose |
|-------|---------|---------|
| `max_rows_per_table` | 10000 | Prevent memory exhaustion |
| `max_cells_per_row` | 1000 | Prevent memory exhaustion |
| `max_merge_span` | 1000 | Prevent invalid spans |

## DOCX Writer Mapping

### Horizontal Merge

```xml
<!-- HorizontalStart{span:3} -->
<w:tc>
  <w:tcPr>
    <w:gridSpan w:val="3"/>
  </w:tcPr>
  <w:p>...</w:p>
</w:tc>
<!-- HorizontalContinue cells are omitted -->
```

### Vertical Merge

```xml
<!-- VerticalStart -->
<w:tc>
  <w:tcPr>
    <w:vMerge w:val="restart"/>
  </w:tcPr>
  <w:p>...</w:p>
</w:tc>

<!-- VerticalContinue -->
<w:tc>
  <w:tcPr>
    <w:vMerge w:val="continue"/>
  </w:tcPr>
  <w:p/>  <!-- Empty paragraph required -->
</w:tc>
```

### Vertical Alignment

```xml
<!-- CellVerticalAlign::Center -->
<w:tc>
  <w:tcPr>
    <w:vAlign w:val="center"/>
  </w:tcPr>
  <w:p>...</w:p>
</w:tc>
```

## Test Coverage

Phase 5 adds comprehensive test coverage:

### RTF Fixtures

| Fixture | Purpose |
|---------|---------|
| `table_horizontal_merge_valid.rtf` | Valid horizontal merge |
| `table_vertical_merge_valid.rtf` | Valid vertical merge |
| `table_mixed_merge.rtf` | Mixed horizontal/vertical |
| `table_orphan_merge_continuation.rtf` | Orphan continuation |
| `table_conflicting_merge.rtf` | Conflicting merge types |
| `table_merge_controls_degraded.rtf` | Degraded merge handling |
| `table_non_monotonic_cellx.rtf` | Non-monotonic cellx |
| `table_large_stress.rtf` | Large table stress test |

### Integration Tests

- 10 new DOCX integration tests for merge output
- 11 new contract tests for strict mode behavior
- Golden IR snapshots for all fixtures

## References

- [Phase 4 IR Design](phase4-ir-design.md) - Base table IR
- [Phase 5 Specification](../specs/PHASE5.md) - Feature specification