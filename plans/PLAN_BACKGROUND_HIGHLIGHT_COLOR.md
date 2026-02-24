# rtfkit Plan: Background and Highlight Color Mapping

**Status: DRAFT (implementation-ready)**

## 1. Objective

Implement end-to-end support for RTF background/highlight text color:

- `\highlightN` (text highlight color index via `\colortbl`)
- `\cbN` (character background color index via `\colortbl`)

Target flow:

```text
RTF \colortbl + \highlight/\cb
  -> IR Run { background_color? }
  -> DOCX / HTML / Typst emitters
```

## 2. Scope

### In scope

- Parse/apply `\highlightN` and `\cbN` in interpreter character style state.
- Add `background_color: Option<Color>` to `rtfkit_core::Run`.
- Resolve color indexes using existing color table and graceful degradation behavior.
- Emit background/highlight in:
  - `crates/rtfkit-docx/src/writer.rs`
  - `crates/rtfkit-html/src/blocks/paragraph.rs`
  - `crates/rtfkit-render-typst/src/map/paragraph.rs`
- Preserve behavior for runs inside `Inline::Hyperlink`.
- Add fixtures, goldens, and CLI contract coverage.

### Out of scope

- Paragraph/table/cell shading (`\clcbpat`, row/table-level backgrounds).
- Patterned shading semantics beyond flat color mapping.
- Theme color mapping beyond RGB color-table values.

## 3. Design decisions

### 3.1 IR shape

Add a single run-level field:

- `Run.background_color: Option<Color>`

Rationale: writers consume one concrete background color regardless of whether source was `\cb` or `\highlight`.

### 3.2 Control-word precedence

- Keep separate interpreter state fields:
  - `highlight_color_index: Option<i32>`
  - `background_color_index: Option<i32>`
- Effective background color is computed as:
  1. `highlight_color_index` if set and resolvable
  2. otherwise `background_color_index` if set and resolvable
  3. otherwise `None`

This keeps behavior deterministic and preserves common RTF intent where `\highlight` is explicit text highlighting.

### 3.3 Reset semantics

- `\plain` resets both `highlight_color_index` and `background_color_index` (character-only reset).
- `\pard` behavior remains unchanged.

### 3.4 Strict mode contract

Unresolved/invalid color indexes are cosmetic loss only:

- no new `DroppedContent` warnings
- degrade to `None`

## 4. Architecture changes

## 4.1 Core IR (`crates/rtfkit-core/src/lib.rs`)

Extend `Run`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub background_color: Option<Color>
```

Initialize as `None` in `Run::new`.

## 4.2 Interpreter (`crates/rtfkit-core/src/interpreter.rs`)

Extend `StyleState` with:

- `highlight_color_index: Option<i32>`
- `background_color_index: Option<i32>`

Control words:

- `\highlightN` -> set `highlight_color_index`
- `\cbN` -> set `background_color_index`

Run creation:

- resolve `background_color` with precedence in section 3.2
- include both new fields in style-change comparison (normal + field result paths)

`reset_character_formatting()`:

- clear highlight/background indexes

## 4.3 DOCX writer (`crates/rtfkit-docx/src/writer.rs`)

In `convert_run`:

- emit run shading for arbitrary RGB background (`w:shd` with `w:fill`)
- keep existing run property order deterministic
- ensure hyperlink child runs pass through the same conversion path

Notes:

- Prefer `w:shd` over `w:highlight` to avoid limited named-color mapping and preserve full RGB fidelity.

## 4.4 HTML writer (`crates/rtfkit-html/src/blocks/paragraph.rs`)

Update run-style helper to emit:

- `background-color: #rrggbb`

Requirements:

- deterministic style key order (font-family, font-size, color, background-color)
- reuse existing attribute escaping and sanitization patterns

## 4.5 Typst mapper (`crates/rtfkit-render-typst/src/map/paragraph.rs`)

Add run background mapping in text wrapper:

- `#highlight(fill: rgb(r, g, b))[...]`

Composition:

- apply background wrapper alongside existing text/font/color wrappers in deterministic order
- include background in run merge style key equality

## 5. Test plan

## 5.1 Core interpreter tests

- `\highlightN` maps to `Run.background_color`
- `\cbN` maps to `Run.background_color`
- precedence when both are present
- `\highlight0` / `\cb0` => `None`
- unresolved indexes degrade to `None`
- `\plain` clears background/highlight but keeps paragraph alignment/list/table behavior
- hyperlink formatted field runs preserve background color

## 5.2 Writer tests

- DOCX XML contains run shading fill
- HTML contains `background-color` style
- Typst contains `#highlight(fill: rgb(...))`
- merge tests ensure background mismatch prevents merge

## 5.3 Fixtures/goldens/contracts

Add fixtures (minimum):

- `fixtures/text_highlight.rtf`
- `fixtures/text_background_cb.rtf`
- `fixtures/text_highlight_background_precedence.rtf`
- `fixtures/text_background_plain_reset.rtf`

Then update:

- `golden/*.json`
- `golden_html/*.html`
- strict-mode contract tests in `crates/rtfkit-cli/tests/cli_contract_tests.rs`

## 6. Implementation slices

1. IR + interpreter state/control handling + tests
2. DOCX writer support + tests
3. HTML writer support + tests
4. Typst mapping support + tests
5. Fixtures/goldens/contracts/docs

Each slice keeps `cargo test` green.

## 7. Acceptance criteria

1. IR includes `background_color` where expected.
2. `\highlight` and `\cb` both map correctly; precedence is deterministic.
3. `\plain` resets background/highlight as character formatting only.
4. DOCX/HTML/Typst outputs include background rendering.
5. Hyperlink child runs preserve background color.
6. No strict-mode regressions from unresolved background/highlight indexes.
7. Golden outputs are deterministic.

## 8. Docs updates (final PR)

- `docs/feature-support.md`
- `docs/rtf-feature-overview.md`
- `docs/warning-reference.md` (if warning semantics change)
- `CHANGELOG.md`
