# rtfkit Plan: Table Layout Fidelity

**Status: IMPLEMENTED**

This plan closes the remaining table *layout* parity gap after structure/merge/shading/border support by adding deterministic mapping for alignment, indent, spacing, width, and row-height semantics.

## 1. Objective

Improve table layout fidelity across the full pipeline:

```text
RTF table layout controls
  -> normalized table layout IR
  -> DOCX / HTML / Typst mapping
```

Target outcomes:

- Row/table alignment and indent controls are emitted consistently.
- Cell spacing and padding controls are represented where output formats allow.
- Preferred width and row-height semantics are preserved deterministically.
- Unsupported layout controls degrade as warning-only cosmetic loss (no strict-mode regression).

## 2. Scope

### In scope

- Layout controls with high real-world impact:
  - alignment/indent: `\trql`, `\trqc`, `\trqr`, `\trleft`
  - inter-cell gap: `\trgaph`
  - row height: `\trrh`
  - preferred width model: `\trftsWidth` + `\trwWidth`, `\clftsWidth` + `\clwWidth`
  - row/cell padding: `\trpadd*` + `\trpaddf*`, `\clpad*` + `\clpadf*`
- Deterministic precedence for row defaults vs cell overrides.
- Writer mapping in:
  - `rtfkit-docx`
  - `rtfkit-html`
  - `rtfkit-render-typst` (best-effort)
- Fixture, golden, and contract coverage for both fidelity and degradation paths.

### Out of scope

- Full RTF table spec parity for all legacy/producer-specific controls.
- Floating/anchored table positioning parity with Word.
- Pagination controls such as repeating headers/keep-with-next (`\trhdr`, `\trkeep*`).
- Pixel-identical visual parity in every renderer.

## 3. Current Baseline and Gaps

- `\trql` / `\trqc` / `\trqr` / `\trleft` are parsed into IR but not fully emitted in writers (especially DOCX/Typst).
- `\trgaph` is currently warning-only despite frequent presence in fixture corpus.
- Width handling is mostly `\cellx`-derived; preferred width models are not normalized.
- Row height and explicit cell/row padding are not represented in IR.
- HTML defaults (`border-collapse`, tokenized padding) can mask source-intended layout when explicit layout controls are present.

## 4. IR Design

Add explicit table-layout types in `rtfkit-core`:

```rust
pub enum WidthUnit {
    Auto,
    Twips(i32),
    Percent(u16), // normalized 0..=10000 basis points
}

pub enum RowHeightRule {
    AtLeast,
    Exact,
}

pub struct BoxSpacingTwips {
    pub top: Option<i32>,
    pub right: Option<i32>,
    pub bottom: Option<i32>,
    pub left: Option<i32>,
}

pub struct TableLayoutProps {
    pub alignment: Option<RowAlignment>,
    pub left_indent_twips: Option<i32>,
    pub cell_gap_twips: Option<i32>, // normalized full gap
    pub preferred_width: Option<WidthUnit>,
}
```

Extend existing nodes:

- `TableProps.layout: Option<TableLayoutProps>`
- `RowProps.height_rule: Option<RowHeightRule>`
- `RowProps.height_twips: Option<i32>`
- `RowProps.default_padding: Option<BoxSpacingTwips>`
- `TableCell.preferred_width: Option<WidthUnit>`
- `TableCell.padding: Option<BoxSpacingTwips>`

Design rules:

- Keep all new fields optional and sparse (`skip_serializing_if`).
- Normalize units early (parser/finalize), not in writers.
- Clamp negative/overflow values and warn deterministically.

## 5. Control Normalization Rules

### 5.1 Alignment and indent

- Treat row alignment/indent controls as table-layout defaults in `TableProps.layout` when stable across rows.
- If rows conflict, keep per-row values in `RowProps` and apply deterministic fallback:
  - DOCX/HTML: prefer first row as table default, keep row variance warning-only.
  - Typst: table-level alignment only; emit partial-support warning for per-row variance.

### 5.2 `\trgaph` normalization

- Parse `\trgaphN` as half-gap and normalize to full gap in twips (`2 * N`) for IR.
- `N <= 0` maps to zero spacing.
- When spacing is set explicitly, writers must avoid double-applying style-token defaults.

### 5.3 Width model normalization

- Parse `ftsWidth` control families into `WidthUnit`.
- Supported canonical forms:
  - auto
  - absolute twips
  - percent
- Unsupported width unit variants degrade with `unsupported_table_control` warning.
- `\cellx` remains geometric fallback when preferred width is absent.

### 5.4 Row height normalization

- `\trrhN`:
  - `N > 0` => `AtLeast`, `height_twips = N`
  - `N < 0` => `Exact`, `height_twips = abs(N)`
  - `N == 0` => unset

### 5.5 Padding precedence

For each side:

1. explicit cell padding (`\clpad*`)
2. row default padding (`\trpadd*`)
3. writer/profile default

`padf` unit selectors are normalized; unsupported units degrade with warning-only fallback.

## 6. Writer Mapping Plan

### 6.1 DOCX (`crates/rtfkit-docx`)

- Table alignment: map to `<w:tblPr><w:jc .../>`.
- Left indent: map to `<w:tblInd .../>` where supported.
- Cell spacing (`trgaph`): map to `<w:tblCellSpacing>`.
- Preferred table width: map to `<w:tblW>`.
- Row height: map to `<w:trHeight w:hRule="atLeast|exact">`.
- Cell preferred width: map to `<w:tcW>`.
- Row/cell padding: map to `<w:tblCellMar>` / `<w:tcMar>`.

If docx-rs API lacks specific setters, add a deterministic fallback path (or warning-only omission) without changing strict-mode semantics.

### 6.2 HTML (`crates/rtfkit-html`)

- Emit table-level layout styles on `<table>`:
  - alignment/indent via margins
  - spacing via `border-spacing` + `border-collapse: separate` when needed
- Emit row-height styles on `<tr>` or cell fallback where browser behavior requires.
- Emit padding styles per cell with row-default fallback.
- Emit explicit width styles using normalized width model.
- Keep style declaration order stable for deterministic snapshots.

### 6.3 Typst (`crates/rtfkit-render-typst`)

- Table alignment via `#align(...)` wrapper.
- Indent via wrapper offset/inset pattern.
- Preferred widths through column/table width mapping where representable.
- Cell padding via `table.cell(inset: ...)` when available.
- Row height exactness is likely partial in Typst; degrade with explicit `Pattern/PartialSupport` warning category.

## 7. Delivery Slices

### Slice A: Alignment/Indent/Gap (highest value)

- Implement `\trql/\trqc/\trqr/\trleft/\trgaph` normalization in core.
- Map in DOCX + HTML.
- Add warning contract tests for unsupported variants.

### Slice B: Width Model

- Implement `ftsWidth` parsing + normalized `WidthUnit`.
- Add table/cell preferred-width emission in DOCX + HTML + Typst best-effort.
- Add fixture matrix for auto vs twips vs percent.

### Slice C: Row Height and Padding

- Parse `\trrh`, `\trpadd*`, `\clpad*`.
- Apply precedence rules.
- Emit DOCX + HTML + Typst best-effort; add degradation warnings where needed.

### Slice D: Hardening and Real-World Validation

- Exercise real-world fixtures (including `policy_doc_15p.rtf`) and lock deterministic goldens.
- Reconcile feature-support docs and warning reference.
- Ensure `cargo fmt`, `cargo clippy --workspace --all-targets`, and test suites pass cleanly.

## 8. Test Plan

### 8.1 Core/parser tests

- Control-word parsing and normalization for each in-scope layout control.
- Unit conversion/clamping edge cases (negative, zero, overflow).
- Precedence tests for row defaults vs cell overrides.
- Warning stability for unsupported/ambiguous layout controls.

### 8.2 Writer tests

- DOCX XML assertions:
  - `w:tblPr/w:jc`, `w:tblInd`, `w:tblCellSpacing`, `w:tblW`
  - `w:trHeight`
  - `w:tcW`, `w:tcMar`
- HTML snapshot assertions:
  - deterministic table/cell style order
  - border-collapse and border-spacing behavior with/without gap
  - row-height/padding/width styles
- Typst mapping snapshots + warning assertions for degraded semantics.

### 8.3 Fixtures/goldens/contracts

Add fixtures:

- `table_layout_alignment_indent.rtf`
- `table_layout_gap_trgaph.rtf`
- `table_layout_preferred_widths.rtf`
- `table_layout_row_height_trrh.rtf`
- `table_layout_padding_row_cell.rtf`
- `table_layout_mixed_precedence.rtf`
- `table_layout_unsupported_units.rtf`

Update:

- `golden/*.json`
- `golden_html/*.html`
- DOCX XML assertions in writer tests
- Typst snapshots
- strict-mode contracts (layout-only loss remains warning-only)

## 9. Acceptance Criteria

1. In-scope layout controls are represented in IR with deterministic normalization.
2. DOCX and HTML emit alignment/indent/gap/width/height/padding where representable.
3. Typst emits best-effort mapping and explicit warnings for non-representable semantics.
4. No layout-only degradation emits `DroppedContent`.
5. Golden outputs are deterministic across repeated runs.
6. Feature matrix and warning docs accurately reflect implemented support.

## 10. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| RTF width/padding unit variants differ by producer | Support a constrained canonical subset first; add variants only from fixture evidence |
| DOCX API gaps in docx-rs | Add deterministic fallback/omission path with warning-only behavior |
| HTML default CSS conflicts with source layout | Gate token defaults behind explicit-source controls and enforce style precedence in tests |
| Typst cannot express exact row-height semantics | Use documented best-effort mapping and stable partial-support warnings |

## 11. Docs and Changelog Updates

- `docs/feature-support.md`
- `docs/rtf-feature-overview.md`
- `docs/warning-reference.md`
- `CHANGELOG.md`
