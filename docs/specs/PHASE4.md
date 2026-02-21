# rtfkit Next Steps Plan (Phase 4: Tables)

This phase adds practical table support on top of the existing `RTF -> IR -> DOCX` pipeline.
Goal is to preserve table semantics for common business documents while keeping fallback deterministic and strict-mode safe.

## 1. Primary objective

Ship reliable conversion for common RTF tables:

`RTF table controls -> IR table model -> DOCX table XML`

without regressing parser safety, deterministic output, or warning contracts.

## 2. Scope

## In scope

- Table structure extraction for common row/cell patterns.
- IR extension for `table -> row -> cell -> blocks`.
- DOCX writer support for basic table structures.
- Deterministic fallback when table semantics cannot be fully reconstructed.
- Warning coverage for semantic loss and unsupported controls.
- Fixture/golden/integration tests for success and degradation paths.

## Out of scope

- Full RTF table spec fidelity.
- Complete border/style fidelity (`\clbrdr*`, full shading matrix, all padding variants).
- Complex merge behavior beyond a narrow initial subset.
- Advanced floating/positioned tables and anchoring.
- Pixel-perfect width/layout matching with MS Word.

## 3. Design constraints and principles

- Keep architecture boundaries:
  - `rtfkit-core`: parse/interpreter + IR + warnings
  - `rtfkit-docx`: table serialization and package output
  - `rtfkit-cli`: orchestration only
- Preserve visible text as highest priority when structure is ambiguous.
- Emit explicit semantic-loss warnings whenever conversion degrades.
- Preserve strict-mode contract: semantic loss should produce `DroppedContent` and fail with strict exit behavior.
- Keep output deterministic:
  - stable row/cell ordering
  - stable default decisions for malformed sources

## 4. Concrete implementation proposals

## 4.1 IR extension (recommended)

Add a dedicated table block:

```rust
pub enum Block {
    Paragraph(Paragraph),
    ListBlock(ListBlock),
    TableBlock(TableBlock),
}

pub struct TableBlock {
    pub rows: Vec<TableRow>,
}

pub struct TableRow {
    pub cells: Vec<TableCell>,
}

pub struct TableCell {
    pub blocks: Vec<Block>,      // Phase 4 minimum: Paragraph and ListBlock support
    pub width_twips: Option<i32> // from \cellx where available
}
```

Rationale:
- Keeps table semantics explicit (no paragraph annotations hack).
- Enables future nested content and merges in later phases.
- Provides clean writer API for DOCX mapping.

## 4.2 Phase-4 minimum control-word subset

Support directly:

- `\trowd` row start/reset
- `\cellxN` cell boundary/width markers
- `\intbl` paragraph is table-contained
- `\cell` cell terminator
- `\row` row terminator

Recognize but largely ignore in Phase 4 (warn only if semantic impact):

- `\trgaph`, `\trleft`, `\trql`, `\trqr`, `\trqc`
- `\clvertalt`, `\clvertalc`, `\clvertalb`
- merge markers (`\clmgf`, `\clmrg`) as degraded path (no merge fidelity yet)

## 4.3 Parser state model

Add explicit interpreter table state:

- `current_table: Option<TableBlockBuilder>`
- `current_row: Option<TableRowBuilder>`
- `current_cell: Option<TableCellBuilder>`
- `pending_cellx: Vec<i32>` (boundaries encountered in current row)
- `paragraph_in_table: bool` (driven by `\intbl`)

Builder behavior:

1. On `\trowd`:
   - finalize any dangling row/cell with warning
   - start fresh row context
2. On `\cellxN`:
   - append boundary in current row metadata
3. On `\intbl`:
   - mark subsequent paragraph as belonging to current cell
4. On paragraph finalization while `paragraph_in_table`:
   - route paragraph to current cell instead of top-level document
5. On `\cell`:
   - finalize current cell, attach to row
6. On `\row`:
   - finalize row, attach to table
   - finalize table block at boundary conditions (see below)

Table block finalization policy:

- finalize table when first non-table paragraph begins after a completed row
- finalize at document end if table context remains open
- if unclosed structures exist, degrade with warnings but preserve text

## 4.4 Malformed input policy (deterministic)

Cases and behavior:

- Missing `\cell` before `\row`:
  - auto-close current cell; warning
- Missing `\row` before next `\trowd`:
  - auto-close row; warning
- `\intbl` content without row/cell context:
  - emit warning, keep as paragraph output
- `\cellx` count mismatch with emitted cells:
  - keep available cells, assign widths best-effort, warning
- orphan `\cell`/`\row` outside table context:
  - warning, ignore control for structure, preserve text

Always prefer content preservation over silent drop.

## 5. Warning and strict-mode strategy

Add table-specific warning variants (names illustrative):

- `UnsupportedTableControl { control_word }`
- `MalformedTableStructure { reason }`
- `UnclosedTableCell`
- `UnclosedTableRow`

When table semantics are lost:

- emit table warning variant
- also emit `DroppedContent` with a deterministic reason string
- ensure strict mode fails consistently

When semantics are preserved but styling is ignored:

- emit non-fatal warning only (no `DroppedContent`)

## 6. DOCX writer plan

## 6.1 Mapping

Map `TableBlock` to DOCX:

- table: `w:tbl`
- row: `w:tr`
- cell: `w:tc`
- cell content: existing block writer path for paragraphs/lists inside cell

## 6.2 Width behavior

- if `width_twips` available from `\cellx`, map to `w:tcW` (`dxa`)
- if missing/invalid, omit explicit width and rely on Word auto layout

## 6.3 Unsupported semantics

- merges (`\clmgf`/`\clmrg`) Phase 4:
  - do not attempt full merge XML fidelity
  - keep separate cells with warnings
  - emit `DroppedContent` if merge loss changes semantic structure materially

## 6.4 Determinism

- preserve source row/cell order as encountered
- stable fallback decisions for malformed rows/cells
- avoid writer-side randomness in table IDs/properties

## 7. Testing plan

## 7.1 New fixtures

- `table_simple_2x2.rtf`
- `table_multirow_uneven_content.rtf`
- `table_with_list_in_cell.rtf`
- `table_missing_cell_terminator.rtf`
- `table_missing_row_terminator.rtf`
- `table_orphan_controls.rtf`
- `table_merge_controls_degraded.rtf`

## 7.2 Golden IR expectations

- success fixtures produce `TableBlock` with expected row/cell counts
- degraded fixtures preserve text and include expected warnings
- snapshots remain deterministic across reruns

## 7.3 DOCX integration checks

- for supported fixtures:
  - `word/document.xml` contains `w:tbl`, `w:tr`, `w:tc`
- for degraded fixtures:
  - strict mode fails where `DroppedContent` is present
- for mixed content fixtures:
  - table and non-table blocks remain in correct order

## 7.4 Contract tests

- strict mode fails on semantic-loss fixtures
- non-strict mode succeeds with warnings
- warning cap behavior still preserves strict-mode signal

## 8. Suggested implementation sequence (PR plan)

1. PR 1: IR model + serialization updates
2. PR 2: parser table state machine + warnings + malformed fallback
3. PR 3: DOCX writer table emission + width mapping
4. PR 4: fixtures/golden/integration/strict tests
5. PR 5: docs/changelog consistency pass

## 9. Risks and mitigations

- Risk: ambiguous boundary parsing causes unstable output
  - Mitigation: explicit state machine + deterministic auto-close rules
- Risk: nested list-in-cell regressions
  - Mitigation: add dedicated fixtures early and keep list path unchanged
- Risk: strict-mode noise from over-warning
  - Mitigation: classify warnings by semantic-loss vs cosmetic-loss
- Risk: docx-rs API limitations for table details
  - Mitigation: prioritize baseline `w:tbl` support and defer advanced styling

## 10. Acceptance criteria

- Common table fixtures are represented as `TableBlock` in IR.
- Supported fixtures render as real tables in DOCX (not flattened text).
- Degraded/malformed fixtures preserve visible text and emit deterministic warnings.
- Strict mode behavior remains contract-correct.
- Full workspace tests are green after fixture/golden updates.
