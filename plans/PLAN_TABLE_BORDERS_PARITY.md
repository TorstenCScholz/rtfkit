# rtfkit Plan: Table Borders Parity

**Status: IMPLEMENTED**

This plan defines table-border support across the full pipeline with deterministic behavior and strict-mode compatibility.

## 1. Objective

Close the largest remaining table-formatting gap by implementing border fidelity across parser, IR, and all output writers.

Target flow:

```text
RTF table/cell border controls
  -> normalized border IR model
  -> DOCX / HTML / Typst mapping
```

Target outcomes:

- Cell border controls produce visible output borders.
- Row-level border controls are applied consistently where representable.
- Border precedence is deterministic and testable.
- Unsupported border variants degrade as cosmetic loss (warning-only).

## 2. Scope

### In scope

- Parse common RTF border controls used in table rows/cells:
  - cell side selectors: `\clbrdrt`, `\clbrdrl`, `\clbrdrb`, `\clbrdrr`
  - row selectors: `\trbrdrt`, `\trbrdrl`, `\trbrdrb`, `\trbrdrr`, `\trbrdrh`, `\trbrdrv`
  - border descriptors: `\brdrs`, `\brdrdb`, `\brdrdot`, `\brdrdash`, `\brdrnone`, `\brdrwN`, `\brdrcfN`
- Add IR types for per-side borders and row border defaults.
- Map border model in:
  - `rtfkit-docx` (table/cell border XML)
  - `rtfkit-html` (deterministic CSS borders)
  - `rtfkit-render-typst` (best-effort cell stroke mapping)
- Preserve warning and strict-mode contracts.
- Add fixtures, goldens, integration tests, and docs updates.

### Out of scope

- Full parity for every RTF border art variant and legacy aliases.
- Exact pixel parity with Microsoft Word in all producers.
- Arbitrary diagonal/3D border effects unless cleanly representable.
- Non-border table layout features (handled in separate plans).

## 3. Delivery Strategy

Implement in 3 slices to reduce risk and keep tests green.

### Slice A: Cell border MVP (highest value)

- Support per-cell side borders from `\clbrdr*` + `\brdr*` descriptors.
- Emit cell borders in DOCX/HTML/Typst.
- Keep style set intentionally small and deterministic.

### Slice B: Row border defaults and inside rules

- Parse `\trbrdr*` and apply row-level defaults.
- Support horizontal/vertical inside rules (`\trbrdrh`, `\trbrdrv`) where practical.
- Apply deterministic precedence against explicit cell borders.

### Slice C: Style-width-color parity hardening

- Expand supported style variants if present in fixture corpus.
- Validate width/color conversion consistency across writers.
- Lock down determinism and degradation contracts.

## 4. Design Decisions

### 4.1 IR model: explicit side-based borders

Extend core IR with reusable table border types:

```rust
pub enum BorderStyle {
    None,
    Single,
    Double,
    Dotted,
    Dashed,
}

pub struct Border {
    pub style: BorderStyle,
    pub width: Option<i32>,        // normalized writer-facing width
    pub color: Option<Color>,
}

pub struct BorderSet {
    pub top: Option<Border>,
    pub left: Option<Border>,
    pub bottom: Option<Border>,
    pub right: Option<Border>,
    pub inside_horizontal: Option<Border>,
    pub inside_vertical: Option<Border>,
}
```

Add:

- `TableCell.borders: Option<BorderSet>`
- `RowProps.borders: Option<BorderSet>`
- `TableProps.borders: Option<BorderSet>` (optional now; keeps model future-safe)

Use sparse serde fields with `skip_serializing_if`.

### 4.2 Border precedence (deterministic)

For an effective cell-side border:

1. explicit cell side border (`TableCell.borders.<side>`)
2. row side fallback (`RowProps.borders.<side>`) for outer edges
3. row inside fallback (`RowProps.borders.inside_*`) for shared boundaries
4. none

Conflict rule:

- when two neighboring cells both define the same shared edge, pick a stable winner:
  - explicit cell border wins over fallback
  - otherwise prefer left/top cell in traversal order

### 4.3 Strict-mode and warnings

- Border loss is cosmetic by default and should remain warning-only.
- Continue using `unsupported_table_control` for unsupported border controls/styles.
- Do not emit `DroppedContent` for border-only degradation.
- Keep warning reason strings stable for contract tests.

## 5. Architecture Changes

### 5.1 Core IR (`crates/rtfkit-core/src/lib.rs`)

- Add `BorderStyle`, `Border`, `BorderSet`.
- Extend `TableCell`, `RowProps`, `TableProps` with optional border fields.
- Keep backward compatibility by defaulting to `None`.

### 5.2 RTF parser/handlers (`crates/rtfkit-core/src/rtf/*`)

- Extend table state with:
  - pending border target (row/cell + side selector)
  - pending border descriptor buffer (style/width/color)
  - row default borders and inside borders
- Update `handlers_tables.rs` to parse:
  - border side selectors (`clbrdr*`, `trbrdr*`)
  - border descriptors (`brdr*`)
- Finalization updates:
  - attach parsed border sets to current cell/row/table IR nodes
  - clear pending border descriptor state at safe boundaries (`\cell`, `\row`, `\trowd`)

### 5.3 DOCX writer (`crates/rtfkit-docx/src/writer.rs`)

- Map effective borders to DOCX table/cell border elements.
- Emit deterministic element/attribute order.
- Unsupported style variants degrade to `single` or omitted border with warning.

### 5.4 HTML writer (`crates/rtfkit-html/src/blocks/table.rs`)

- Generate deterministic per-cell border CSS (`border-top`, etc.).
- Merge border declarations with existing style generation in stable key order.
- Degrade unsupported styles to `solid` or omitted border, warning-only.

### 5.5 Typst renderer (`crates/rtfkit-render-typst/src/map/table.rs`)

- Map borders to available table cell stroke options.
- Use best-effort style mapping:
  - `single`/`double` when representable
  - fallback to solid stroke for unsupported variants
- Keep output deterministic and avoid renderer-specific heuristics.

## 6. Test Plan

### 6.1 Core/parser tests

- `\clbrdr*` side mapping into cell border set.
- `\trbrdr*` row defaults and inside border mapping.
- Descriptor parsing:
  - style (`\brdrs`, `\brdrdb`, `\brdrdot`, `\brdrdash`, `\brdrnone`)
  - width (`\brdrwN`)
  - color (`\brdrcfN`)
- Precedence and shared-edge conflict rules.
- Unsupported control/style warnings are stable.

### 6.2 Writer tests

- DOCX XML contains expected border elements/values.
- HTML output has deterministic border CSS ordering.
- Typst mapping emits stable border parameters.
- Cross-format consistency tests for a representative matrix.

### 6.3 Fixtures and contracts

Add minimum fixtures:

- `table_borders_cell_basic.rtf`
- `table_borders_row_outer.rtf`
- `table_borders_inside_hv.rtf`
- `table_borders_cell_overrides_row.rtf`
- `table_borders_style_variants.rtf`
- `table_borders_color_width.rtf`
- `table_borders_unsupported_variant.rtf`

Update:

- `golden/*.json`
- `golden_html/*.html`
- Typst snapshot tests
- CLI contract tests for warning behavior and strict mode (no strict failure on cosmetic border loss)

## 7. Implementation Order

1. Add IR border types and optional fields.
2. Parse and attach cell borders (`\clbrdr*`) with tests.
3. Emit cell borders in DOCX/HTML/Typst.
4. Add row border defaults/inside rules (`\trbrdr*`) and precedence logic.
5. Expand style/color/width handling and degradation behavior.
6. Add fixtures, goldens, contract tests, and docs/changelog updates.

## 8. Acceptance Criteria

1. Table/cell borders are represented in IR and emitted across DOCX/HTML/Typst.
2. Border precedence is deterministic and documented.
3. Unsupported border variants produce warning-only degradation (no `DroppedContent`).
4. Strict mode behavior is unchanged for border-only loss.
5. Golden outputs are deterministic across repeated runs.
6. Feature matrix and warning docs reflect implemented behavior.

## 9. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| RTF border control variants differ by producer | Start with a constrained style subset and expand from fixture evidence |
| Shared-edge conflict ambiguity | Enforce one fixed winner rule and test it explicitly |
| Cross-writer style mismatches | Normalize in IR first, then keep writer mapping tables small and explicit |
| HTML style churn | Centralize table style assembly and assert key ordering in tests |

## 10. Docs Updates (final PR)

- `docs/feature-support.md`
- `docs/rtf-feature-overview.md`
- `docs/warning-reference.md`
- `CHANGELOG.md`
