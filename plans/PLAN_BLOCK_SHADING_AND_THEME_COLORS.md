# rtfkit Plan: Block Shading and Theme Color Fidelity

**Status: DRAFT (implementation-ready)**

This plan defines the follow-up coloring work after run-level `\\highlight`/`\\cb` support.

## 1. Objective

Extend color fidelity beyond run-level text backgrounds to block/table shading and theme-aware color resolution.

Target outcomes:

- Paragraph shading support.
- Table cell shading support (with deterministic row/table fallback behavior).
- Patterned shading semantics (not just flat fill colors).
- Theme color references resolved to stable RGB output where possible.

Target flow:

```text
RTF color/shading controls (+ optional theme metadata)
  -> IR paragraph/table/cell shading model
  -> DOCX / HTML / Typst emitters
```

## 2. Scope

### In scope

- Add IR support for non-run shading (paragraph and table cell minimum).
- Parse and apply block/table shading controls used in real-world RTF:
  - paragraph shading controls (`\\cbpatN`, `\\cfpatN`, `\\shadingN`)
  - table/cell shading controls (`\\clcbpatN`, `\\clcfpatN`, `\\clshdngN`)
  - row/table fallback controls where available in input (`\\trcbpatN` etc.)
- Implement deterministic precedence rules for overlapping shading sources.
- Emit flat and patterned shading in DOCX where representable.
- Emit best-effort equivalent styling in HTML and Typst.
- Preserve strict-mode semantics: cosmetic shading loss must not trigger strict failure.
- Add fixtures, golden snapshots, and CLI contract coverage.

### Out of scope

- Full RTF theme package parsing parity with Word internals for every edge case.
- Gradients/textures and non-color pattern assets.
- Rewriting parser/tokenizer architecture.

## 3. Delivery strategy

Implement in 3 incremental slices so each lands safely and keeps tests green.

### Slice A (highest value): Flat block/cell shading

- Paragraph fill color (`\\cbpatN` as fill when resolvable).
- Table cell fill color (`\\clcbpatN` as fill when resolvable).
- Optional row/table fallback fill propagation when direct cell fill is missing.

### Slice B: Patterned shading semantics

- Capture pattern intent (`\\shadingN`, `\\cfpatN`, `\\clshdngN`, `\\clcfpatN`).
- DOCX: emit pattern-capable `w:shd` (`w:val`, `w:fill`, `w:color`).
- HTML/Typst: deterministic degradation policy when pattern cannot be represented.

### Slice C: Theme-aware color resolution

- Resolve theme-derived color references to concrete RGB before emission.
- Keep deterministic fallback when theme data is partial/missing.

## 4. Design decisions

### 4.1 IR model: promote from `Option<Color>` to a reusable shading object

Add a reusable type in `rtfkit-core`:

```rust
pub struct Shading {
    pub fill_color: Option<Color>,
    pub pattern_color: Option<Color>,
    pub pattern: Option<ShadingPattern>,
}
```

Add:

- `Paragraph.shading: Option<Shading>`
- `TableCell.shading: Option<Shading>`
- `RowProps.shading: Option<Shading>` (optional but recommended for row defaults)
- `TableProps.shading: Option<Shading>` (optional but recommended for table defaults)

Keep existing `Run.background_color` unchanged for backward compatibility.

Rationale: avoids another IR redesign when pattern support is added.

### 4.2 Precedence (deterministic)

For table cell effective shading:

1. explicit cell shading (`TableCell.shading`)
2. row shading fallback (`RowProps.shading`)
3. table shading fallback (`TableProps.shading`)
4. none

For paragraph text areas:

- paragraph shading is independent of run shading.
- run-level highlight/background remains glyph-local and may coexist inside a shaded paragraph.

### 4.3 Reset semantics

- `\\plain` remains character-only reset and must NOT reset paragraph/table shading.
- `\\pard` resets paragraph properties including paragraph shading state.
- row/cell shading resets follow existing table row-definition boundaries (`\\trowd` starts new row property scope).

### 4.4 Strict-mode contract

- Unresolved color indexes or unsupported pattern types are cosmetic loss.
- No new `DroppedContent` for shading-only loss.
- Use warning categories consistent with existing `UnsupportedControlWord` / `UnsupportedTableControl` behavior.

## 5. Architecture changes

## 5.1 Core IR (`crates/rtfkit-core/src/lib.rs`)

- Add `Shading` + `ShadingPattern` types.
- Extend `Paragraph`, `TableCell`, and optionally `RowProps` / `TableProps` with shading fields.
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for sparse JSON.

## 5.2 Interpreter (`crates/rtfkit-core/src/interpreter.rs`)

Add parser state for:

- paragraph shading indexes/percent (`cbpat`, `cfpat`, `shading`)
- pending cell shading indexes/percent (`clcbpat`, `clcfpat`, `clshdng`)
- optional row/table fallback shading indexes

Implementation notes:

- Resolve color indexes through existing color table logic.
- Build `Shading` objects only when values are valid/resolved.
- Include shading fields in style/flush comparisons where relevant.
- Ensure table row/cell lifecycle (`\\trowd`, `\\cellx`, `\\cell`, `\\row`) carries pending shading correctly.

## 5.3 DOCX writer (`crates/rtfkit-docx/src/writer.rs`)

- Paragraph shading -> paragraph properties shading.
- Cell shading -> `TableCell::shading(Shading { ... })`.
- Pattern mapping:
  - map supported IR patterns to DOCX `w:shd/@w:val`
  - always emit deterministic `w:fill` / `w:color` when present

Fallback behavior:

- if pattern unsupported, emit flat fill only (when available), no hard failure.

## 5.4 HTML writer (`crates/rtfkit-html/src/blocks/*.rs`)

- Paragraph shading -> `<p style="background-color: #rrggbb;">` (merged with alignment class behavior).
- Table cell shading -> merge with existing `td` style generation (`width` + background styles in deterministic key order).
- Pattern degradation policy:
  - flat fill always emitted when available.
  - pattern intent not directly representable should be omitted (or optionally approximated with CSS hatch only if deterministic and safe).

Security:

- no untrusted raw CSS fragments.
- continue using escaped attributes and deterministic style assembly helpers.

## 5.5 Typst mapper (`crates/rtfkit-render-typst/src/map/*.rs`)

- Paragraph shading -> wrapper-level highlight for full paragraph content.
- Table cell shading -> `table.cell(fill: rgb(...), ...)` when available.
- Pattern degradation policy:
  - map fill color only when Typst pattern parity is unavailable.
  - emit partial-support warnings (not dropped-content) for pattern-only loss.

## 5.6 Theme color resolution

- Extend color-table representation to optionally retain theme metadata when present.
- Resolve theme references to concrete RGB before constructing IR colors.
- Deterministic fallback chain:
  1. explicit RGB in control/table entry
  2. resolved theme RGB
  3. `None` (cosmetic loss)

## 6. Test plan

## 6.1 Interpreter unit tests

- paragraph fill from `\\cbpatN`
- cell fill from `\\clcbpatN`
- row/table fallback precedence into cells
- `\\pard` resets paragraph shading; `\\plain` does not
- unresolved shading indexes degrade to `None`
- pattern fields captured and serialized when present

## 6.2 Writer tests

- DOCX paragraph/cell XML contains expected `w:shd` attributes.
- HTML paragraph/cell styles contain deterministic `background-color` ordering.
- Typst paragraph/cell mapping contains fill wrappers/cell params.
- Pattern degradation warnings are stable and non-strict-breaking.

## 6.3 Fixtures/goldens/contracts

Add fixtures (minimum):

- `fixtures/paragraph_shading_basic.rtf`
- `fixtures/table_cell_shading_basic.rtf`
- `fixtures/table_row_cell_shading_precedence.rtf`
- `fixtures/paragraph_shading_plain_pard_reset.rtf`
- `fixtures/shading_pattern_basic.rtf`
- `fixtures/shading_theme_color_reference.rtf`

Update:

- `golden/*.json`
- `golden_html/*.html`
- Typst snapshot tests
- CLI strict-mode contracts in `crates/rtfkit-cli/tests/cli_contract_tests.rs`

## 7. Implementation order

1. IR shading types + paragraph/cell flat fill fields.
2. Interpreter flat fill parsing (`cbpat`, `clcbpat`) + tests.
3. DOCX/HTML/Typst flat fill emission + tests.
4. Row/table fallback precedence + tests.
5. Pattern capture/mapping/degradation + tests.
6. Theme resolution + tests.
7. Fixtures/goldens/contracts/docs/changelog.

Each step should keep `cargo test` green.

## 8. Acceptance criteria

1. Paragraph and cell shading are represented in IR and emitted in DOCX/HTML/Typst.
2. Table shading precedence is deterministic (`cell > row > table`).
3. `\\plain` does not reset block/table shading; `\\pard` does for paragraph scope.
4. Patterned shading is preserved in DOCX when representable and degraded deterministically elsewhere.
5. Theme-based colors resolve to stable RGB when sufficient source data exists.
6. No strict-mode regressions: shading-only loss stays cosmetic.
7. Golden outputs are deterministic across repeated runs.

## 9. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| RTF shading controls vary by producer | Use fixture matrix from multiple producers; keep graceful fallback |
| Pattern mapping mismatch across formats | Keep IR explicit; document per-format degradation policy |
| Theme metadata incomplete | Deterministic fallback to explicit RGB/None; no hard failure |
| Style-attribute churn in HTML | Centralize style builders and enforce key-order tests |

## 10. Docs updates (final PR)

- `docs/feature-support.md`
- `docs/rtf-feature-overview.md`
- `docs/warning-reference.md`
- `CHANGELOG.md`
