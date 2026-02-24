# rtfkit Plan: Font and Foreground Color Mapping

**Status: REVISED (ready for implementation handoff)**

This plan replaces the previous draft with architecture-accurate steps for the current codebase.

## 1. Primary objective

Implement end-to-end mapping for these RTF controls:

- Font family selection: `\fN` (via `\fonttbl` + `\deffN`)
- Font size: `\fsN` (half-points)
- Foreground text color: `\cfN` (via `\colortbl`)
- Character reset: `\plain` (character formatting only)

Target flow:

```text
RTF \fonttbl + \colortbl + \deff + \f/\fs/\cf/\plain
  -> IR Run { font_family?, font_size?, color?, ... }
  -> DOCX / HTML / Typst(PDF) emitters
```

## 2. Reassessed scope

### In scope

- Parse `\fonttbl` and `\colortbl` destinations into interpreter lookup state.
- Parse and apply `\deffN`, `\fN`, `\fsN`, `\cfN`, `\plain`.
- Add `font_family: Option<String>` to `rtfkit_core::Run`.
- Populate existing `Run.font_size` and `Run.color` from style state.
- Emit mapped formatting in:
  - `crates/rtfkit-docx/src/writer.rs`
  - `crates/rtfkit-html/src/blocks/paragraph.rs`
  - `crates/rtfkit-render-typst/src/map/paragraph.rs`
- Preserve behavior for runs inside `Inline::Hyperlink`.
- Add fixtures + golden snapshots + contract tests.

### Out of scope

- Background/highlight color (`\cb`, `\highlight`).
- Strikethrough/superscript/subscript and advanced typography controls.
- Font embedding/substitution policy beyond emitting chosen family name.
- `\af` complex-script font routing.

## 3. Critical design decisions

### 3.1 Keep parser architecture stable

Do not rewrite parser/tokenizer flow. Extend existing destination handling in `Interpreter`:

- `DestinationBehavior` gains `FontTable` and `ColorTable`.
- Parsing remains in skipped-destination path (`process_skipped_destination_event`), same pattern as list table parsing.

### 3.2 Separate character style from paragraph/table/list state

`\plain` must only reset character formatting. It must not reset:

- paragraph alignment,
- list state,
- table state.

### 3.3 Strict-mode contract

Invalid/missing font/color references are cosmetic loss, not semantic loss.

- Never emit `DroppedContent` for unresolved `\f`/`\cf` indexes.
- Degrade to `None` values for unresolved references.

### 3.4 Deterministic output

- Deterministic style serialization order (especially HTML `style` attr keys).
- Deterministic table lookup behavior for same input.

## 4. Target architecture

### 4.1 IR changes (`crates/rtfkit-core/src/lib.rs`)

Add:

```rust
pub font_family: Option<String>
```

to `Run`, with `#[serde(skip_serializing_if = "Option::is_none")]`.

Keep `font_size` and `color` as-is (already present).

### 4.2 Interpreter style model (`crates/rtfkit-core/src/interpreter.rs`)

Extend `StyleState` with character properties:

- `font_index: Option<i32>`
- `font_size_half_points: Option<i32>`
- `color_index: Option<i32>`

Add interpreter-level document style resources:

- `default_font_index: Option<i32>`
- `font_table: HashMap<i32, String>`
- `color_table: Vec<Option<Color>>`

### 4.3 Destination parsers (small cohesive components)

Add dedicated parser helpers (private methods or small structs) for:

- `fonttbl` parsing
- `colortbl` parsing

Important: today skipped destinations ignore `Text` events. For font/color tables, `Text` must be consumed because semicolons and font names arrive as text tokens.

### 4.4 Control-word behavior

Implement:

- `\deffN`: set `default_font_index` and apply as default current font.
- `\fN`: set current `font_index`.
- `\fsN`: set current `font_size_half_points`.
- `\cfN`: set current `color_index`.
- `\plain`: reset only character-style fields (`bold`, `italic`, `underline`, font index/size/color) to defaults.

Do not change `\pard` semantics beyond existing behavior.

### 4.5 Run creation and style-change detection

Current style-change checks only bold/italic/underline. Expand to include new character fields.

Unify style-change comparison so both normal text flow and field result flow use the same logic (prevents hyperlink-specific drift).

`create_run()` resolves:

- `font_family` from `font_index` -> `font_table` (fallback to `default_font_index` if needed).
- `font_size` = `font_size_half_points as f32 / 2.0`.
- `color` from `color_index` -> `color_table`, with `\cf0 => None`.

## 5. Writer implementation strategy

### 5.1 DOCX (`crates/rtfkit-docx/src/writer.rs`)

Extend `convert_run` to include:

- font family (`w:rFonts`)
- size (`w:sz`, half-points)
- color (`w:color`, hex)

Requirements:

- Same mapping path for runs and hyperlink child runs (already routed through `convert_run`).
- No unsafe casts/panics on invalid values.

### 5.2 HTML (`crates/rtfkit-html/src/blocks/paragraph.rs`)

Add deterministic style emission helper for runs:

- `font-family`
- `font-size: Npt`
- `color: #rrggbb`

Security requirements:

- Escape/sanitize font family for CSS-string context (not just HTML escaping).
- Keep URL allowlist behavior for hyperlinks unchanged.
- Do not emit raw untrusted style fragments.

Maintainability requirement:

- Centralize run-style string building in one helper; do not duplicate across paragraph/hyperlink paths.

### 5.3 Typst/PDF (`crates/rtfkit-render-typst/src/map/paragraph.rs`)

Replace current "drop font_size/color with warnings" behavior.

- Map character style via `#text(...)` wrapper when needed.
- Compose with existing bold/italic/underline mapping.
- Merge adjacent runs only when all relevant style fields match (bold/italic/underline/font/color/size/family).

Cleanup:

- Remove obsolete mapping warnings for dropped font size/color from `MappingWarning` and tests.

## 6. Security and robustness requirements

- No panics on malformed `\fonttbl`/`\colortbl` input.
- Clamp or ignore out-of-range RGB components safely.
- Ignore invalid indexes (`\f-1`, unresolved `\cf999`) without semantic-failure warnings.
- HTML output must remain XSS-safe and attribute-safe with malicious font names.
- Keep parser limits behavior unchanged.

## 7. Test plan (phase-gated)

### Phase A: Core parser + IR

- Interpreter unit tests for:
  - `\fonttbl` extraction
  - `\colortbl` extraction
  - `\deff` default font behavior
  - `\plain` character reset behavior
  - `\cf0` -> `Run.color == None`
  - unresolved indexes degrade safely
- Extend existing hyperlink tests with font/color styled link runs.

### Phase B: Writer unit/integration

- DOCX XML assertions for `w:rFonts`, `w:sz`, `w:color` (including hyperlink runs).
- HTML rendering tests for style attributes and sanitization.
- Typst mapping tests for `#text(...)` output and merge behavior.

### Phase C: Fixtures + snapshots + contracts

Add fixtures (minimum):

- `fixtures/text_font_family.rtf`
- `fixtures/text_font_size.rtf`
- `fixtures/text_color.rtf`
- `fixtures/text_font_color_combined.rtf`
- `fixtures/text_plain_reset.rtf`
- `fixtures/text_default_font_deff.rtf`

Then:

- update `golden/*.json`
- update `golden_html/*.html`
- add strict-mode contract cases in `crates/rtfkit-cli/tests/cli_contract_tests.rs`

## 8. Implementation order (recommended PR slicing)

1. **PR1: IR + interpreter core wiring**
   - `Run.font_family`
   - `StyleState` extensions
   - style-change/run creation updates
   - `\deff/\f/\fs/\cf/\plain`
2. **PR2: destination parsers**
   - `fonttbl` + `colortbl` parser states in skipped-destination flow
   - parser-focused unit tests
3. **PR3: DOCX writer**
   - run property emission + tests
4. **PR4: HTML writer + sanitization**
   - style helper + tests
5. **PR5: Typst writer cleanup**
   - actual mapping + warning enum cleanup + tests
6. **PR6: fixtures, goldens, contracts, docs**
   - snapshots, strict-mode tests, docs/changelog refresh

Each PR must keep `cargo test` green.

## 9. Acceptance criteria

1. IR includes `font_family`, `font_size`, `color` where expected for new fixtures.
2. `\plain` resets character formatting only (alignment/list/table semantics unchanged).
3. DOCX output contains expected run font/size/color XML.
4. HTML output contains deterministic, sanitized run style attributes.
5. Typst output renders font/size/color without `font_size_dropped`/`color_dropped` warnings.
6. Hyperlink child runs preserve font/size/color formatting.
7. Strict mode behavior is unchanged for semantic-loss categories; new font/color handling does not introduce `DroppedContent` regressions.
8. Golden IR and HTML snapshots pass deterministically.

## 10. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Destination parser complexity growth in `Interpreter` | Keep font/color parsing in isolated helpers with minimal shared state |
| `\plain` accidental reset of non-character state | Add explicit character-reset helper and targeted regression tests |
| HTML style injection via font names | Dedicated CSS-string sanitizer + tests with malicious font names |
| Typst formatting composition bugs | Add focused tests for combined bold/italic/underline + font/color/size |
| Snapshot churn | Land in PR slices and refresh goldens only after each stable phase |

## 11. Docs to update in final PR

- `docs/feature-support.md`
- `docs/warning-reference.md` (only if warning behavior changes)
- `docs/rtf-feature-overview.md`
- `CHANGELOG.md`
