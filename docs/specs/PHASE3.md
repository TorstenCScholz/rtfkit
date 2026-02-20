# rtfkit Next Steps Plan (Phase 3: Lists)

This phase adds robust, deterministic list support after Phase 2 (`RTF -> IR -> DOCX`).
It is intentionally more comprehensive than later phases because list parsing is the highest-risk parser/writer expansion in the current roadmap.

## 1. Primary objective

Ship practical list conversion for real-world RTF:

`RTF list controls -> IR list semantics -> DOCX numbering`

with explicit fallback behavior, stable output, and strict-mode correctness.

## 2. Scope

## In scope

- Support common list-related RTF patterns:
  - list tables (`\listtable`, `\listoverridetable`)
  - paragraph list references (`\lsN`, `\ilvlN`)
- Preserve list semantics in IR (not only literal bullet characters).
- Map lists to DOCX numbering definitions and paragraph numbering references.
- Support at minimum:
  - bullet lists
  - decimal ordered lists
  - nested levels (minimum two levels)
- Maintain deterministic ID assignment in writer output.
- Add fixture and integration coverage for success and fallback paths.

## Out of scope

- Full RTF list spec fidelity.
- All numbering styles (roman, alphabetic variants, locale-specific formats) in Phase 3.
- Pixel-perfect layout fidelity with Word for every source document.
- Legacy paragraph-numbering mode (`\pn...`) as a first-class conversion path.

## 3. Design constraints and principles

- Reuse current architecture boundaries:
  - `rtfkit-core`: parser/interpreter + IR + warnings
  - `rtfkit-docx`: DOCX serialization/mapping
  - `rtfkit-cli`: orchestration only
- Keep fallback explicit:
  - prefer preserving visible text over dropping content silently
  - emit `DroppedContent` when list semantics cannot be represented
- Preserve strict-mode contract:
  - unsupported semantic loss must surface as exit code `4` under `--strict`
- Keep deterministic output:
  - stable ordering of numbering definitions
  - stable `numId` allocation for same IR input

## 4. Concrete implementation proposals

## 4.1 Recommended path (A): explicit list block in IR

Add list semantics to IR via new types:

```rust
pub enum Block {
    Paragraph(Paragraph),
    List(ListBlock),
}

pub struct ListBlock {
    pub list_id: ListId,
    pub kind: ListKind,
    pub items: Vec<ListItem>,
}

pub enum ListKind {
    Bullet,
    OrderedDecimal,
    Mixed, // fallback/bridge for inconsistent sources
}

pub struct ListItem {
    pub level: u8, // 0..=8 clamp for DOCX
    pub blocks: Vec<Block>, // start with one Paragraph in Phase 3
}

pub type ListId = u32;
```

Benefits:
- Clean writer contract.
- Easier table/image coexistence later (list item can contain blocks).
- Better semantic fallback decisions.

Costs:
- Requires touching golden IR snapshots.
- Requires interpreter changes in paragraph finalization flow.

## 4.2 Transitional path (B): paragraph annotations

Add metadata to `Paragraph` first:

```rust
pub struct Paragraph {
    pub alignment: Alignment,
    pub runs: Vec<Run>,
    pub list: Option<ParagraphListMeta>,
}

pub struct ParagraphListMeta {
    pub list_id: u32,
    pub level: u8,
    pub kind: ListKind,
}
```

Benefits:
- Smaller initial diff.
- Faster rollout.

Costs:
- Writer becomes more heuristic-heavy.
- Follow-up migration to list blocks likely needed in later phases.

## 4.3 Decision recommendation

Use path A in Phase 3 unless timeline pressure is severe.
If path B is chosen, add migration TODOs and constraints in architecture docs immediately.

## 5. Parser/interpreter plan (concrete)

## 5.1 New internal parser state

Add list-related interpreter state:

- `list_table: HashMap<i32, ParsedListDefinition>`
- `list_overrides: HashMap<i32, ParsedListOverride>`
- `current_paragraph_list_ref: Option<ParagraphListRef>`

Definitions:

- `ParsedListDefinition`: abstract list template (kind per level).
- `ParsedListOverride`: mapping from `\lsN` to abstract list plus start overrides.
- `ParagraphListRef`: resolved list id + level + kind for current paragraph.

## 5.2 Control-word handling proposals

Add targeted handling for:

- destination groups:
  - `\listtable`
  - `\listoverridetable`
- paragraph-level controls:
  - `\lsN` -> select list override/reference
  - `\ilvlN` -> set level for current paragraph
  - legacy `\pn...` families -> emit `UnsupportedListControl` + `DroppedContent`

Unknown/partially supported list controls:
- emit `UnsupportedControlWord`
- if semantics are lost, also emit `DroppedContent`

## 5.3 Paragraph finalization algorithm

At paragraph finalize:

1. finalize paragraph runs as today
2. resolve paragraph list metadata from:
   - explicit `\lsN` + `\ilvlN` only
3. if resolved:
   - append into existing trailing `ListBlock` if compatible
   - otherwise create new `ListBlock`
4. if unresolved/ambiguous:
   - keep paragraph text
   - emit warning(s); include `DroppedContent` for semantic loss

Compatibility rule for appending to trailing list:
- same resolved canonical list id
- contiguous block sequence (no non-list block in between)

## 5.4 Ambiguity policy

For ambiguous sources (incomplete tables, inconsistent overrides):

- Prefer text preservation.
- Downgrade to paragraph output.
- Emit deterministic warning message:
  - example: `"Dropped list semantics due to unresolved \\ls override"`

## 6. DOCX writer plan (concrete)

## 6.1 Numbering model

Introduce writer-side deterministic allocator:

- `abstractNumId`: assigned by first unique `(kind, level-pattern)` definition
- `numId`: assigned by first encountered concrete list instance
- both allocation maps should be insertion-order deterministic

## 6.2 XML mapping

For list paragraph mapping:

- `word/document.xml`:
  - `w:pPr/w:numPr/w:ilvl@w:val=<level>`
  - `w:pPr/w:numPr/w:numId@w:val=<numId>`
- `word/numbering.xml`:
  - one `w:abstractNum` per definition
  - one `w:num` per concrete list instance

Minimum level formatting in Phase 3:

- bullet list level text: bullet glyph
- decimal list level text: `%1.` style numbering

## 6.3 Determinism guarantees

- stable traversal order from IR blocks
- no random/clock-based identifiers
- repeated conversion of same IR yields byte-stable numbering parts (except compression metadata if library introduces variability; document this explicitly)

## 7. CLI and contract impacts

No new CLI flags are required for Phase 3.

Behavioral expectations:

- `--strict` continues to fail on `DroppedContent`.
- `--format text|json` reports list-related warnings clearly.
- Exit codes remain unchanged:
  - `2`: parse
  - `3`: writer/io
  - `4`: strict violation

## 8. Warning model updates

Add or standardize warning reasons for list paths:

- unsupported list control encountered
- unresolved list override (`\ls`)
- unsupported nested pattern (if level > supported range)
- fallback to paragraph due to malformed list destination

Guideline:
- `UnsupportedControlWord` for unknown control handling
- `DroppedContent` for semantic loss that affects strict mode

## 9. Test plan (detailed)

## 9.1 Unit tests (`rtfkit-core`)

- list table parse success/failure
- `\ls` + `\ilvl` resolution
- legacy `\pn...` degradation warnings
- list block construction and boundary splitting
- fallback behavior with expected warning type(s)
- strict-mode signal preserved under warning cap

## 9.2 Unit tests (`rtfkit-docx`)

- numbering definition generation from list IR
- `numId`/`abstractNumId` determinism
- paragraph-level `w:numPr` correctness
- mixed bullet/decimal document behavior

## 9.3 Integration tests (`rtfkit-cli`)

Per fixture:

1. run CLI conversion to DOCX
2. unzip DOCX
3. assert `word/document.xml` has expected list references (`w:numPr`)
4. assert `word/numbering.xml` has expected definitions
5. assert strict-mode pass/fail according to fixture expectation

## 9.4 Fixture matrix (minimum)

- `list_bullet_simple.rtf`
- `list_decimal_simple.rtf`
- `list_nested_two_levels.rtf`
- `list_mixed_kinds.rtf`
- `list_malformed_fallback.rtf`
- `list_unresolved_ls_strict_fail.rtf`

## 9.5 Determinism tests

- convert same fixture twice and compare:
  - `word/document.xml`
  - `word/numbering.xml`
- include at least one nested list fixture

## 10. Implementation slicing (recommended PR sequence)

1. IR model extension and serialization updates.
2. Parser/interpreter list-state scaffolding (no writer changes yet).
3. Core list parsing + fallback warnings.
4. DOCX numbering writer implementation.
5. CLI integration tests + fixtures.
6. Determinism and strict-mode regression tests.
7. Docs/changelog/architecture updates.

Each PR should keep tests green and avoid mixing multiple risk areas.

## 11. Migration and compatibility notes

- IR golden snapshots will require controlled regeneration.
- Add clear changelog entry for new list-related IR shape.
- If external users depend on previous IR schema, add migration note in README/docs.

## 12. Acceptance criteria

- List fixtures render plausibly in Word and LibreOffice.
- XML assertions pass for numbering references and definitions.
- Fallback behavior is deterministic and warning-driven.
- Strict mode fails reliably when list semantics are dropped.
- Existing non-list fixtures remain behaviorally stable.
- CI passes on supported matrix.

## 13. Risks and mitigations

- Risk: source RTF list patterns vary significantly.
  - Mitigation: prioritize fixture-driven iterations and explicit fallback.
- Risk: IR model churn delays downstream work.
  - Mitigation: choose path A early, or document path B migration milestone.
- Risk: writer complexity increases with nested lists.
  - Mitigation: keep a small deterministic numbering allocator and test heavily.
- Risk: strict-mode regressions under warning caps.
  - Mitigation: maintain explicit regression tests for capped-warning scenarios.

## 14. Definition of done

- End-to-end conversion supports bullet and decimal lists with nesting.
- Writer emits valid numbering XML with deterministic IDs.
- Strict mode and warning semantics behave by contract.
- Fixture/test matrix covers normal and fallback cases.
- Architecture and user docs reflect final behavior.
