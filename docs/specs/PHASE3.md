# rtfkit Next Steps Plan (Phase 3: Lists)

This phase focuses on list support after the current Phase 2 baseline (`RTF -> IR -> DOCX`).
It is intentionally the most detailed phase plan because list parsing and numbering behavior can easily drift without a strict contract.

## 1. Primary objective

Deliver "good enough" list conversion for common RTF documents:

`RTF lists -> IR list semantics -> DOCX numbering`

with deterministic output and strict-mode-safe fallback behavior.

## 2. In scope

- Parse and map common list control patterns:
  - `\pn...` style list markers
  - `\listtable`, `\listoverridetable`
  - paragraph references like `\lsN`, `\ilvlN`
  - `\pntext` and related inline list text forms
- Preserve list semantics in IR (not only literal bullet characters in text).
- Emit valid DOCX numbering (`w:numPr`, numbering definitions).
- Support:
  - unordered bullets
  - ordered decimal numbering
  - nested levels (minimum two levels)
- Deterministic numbering/id assignment.
- Fallback path for ambiguous/unsupported patterns with explicit warnings.

## 3. Out of scope

- Full RTF list spec coverage.
- Exact Word-fidelity spacing/layout for all list variants.
- Advanced restart/continuation behavior across all edge cases.

## 4. IR contract updates

Phase 3 requires explicit list semantics in IR. Use one of these approaches:

## 4.1 Preferred

- Add list-aware block model:
  - `Block::List(ListBlock)`
  - `ListBlock { ordered, items }`
  - `ListItem { level, blocks }`

## 4.2 Transitional alternative

- Add list metadata to paragraphs first:
  - `list_id`, `list_level`, `list_kind`
- Migrate to dedicated list blocks in a later cleanup.

Whichever option is selected, document it in architecture docs before merge.

## 5. Parser/interpreter behavior

- Parse list table definitions before applying paragraph-level references.
- Resolve list references (`\lsN`) to concrete list style/kind.
- Track nesting level from control words (for example `\ilvlN`).
- Keep list sequence continuity deterministic for adjacent list paragraphs.
- When parsing is ambiguous:
  - preserve text
  - degrade to paragraph output
  - add warning(s), including `DroppedContent` when list semantics are lost

## 6. DOCX writer behavior

- Write numbering definitions into DOCX package (`numbering.xml`).
- Map list paragraphs to:
  - `w:pPr/w:numPr/w:numId`
  - `w:pPr/w:numPr/w:ilvl`
- Use stable id assignment based on first encounter order.
- Keep output deterministic across repeated runs with same input.

## 7. Strict mode and warnings

- If list semantics cannot be represented, emit `DroppedContent`.
- `--strict` must fail with exit code `4` for those cases.
- Warning caps must not hide strict-mode violations.

## 8. Test strategy

## 8.1 Unit tests

- Interpreter tests for list-control detection and state transitions.
- IR construction tests for list structure and nesting.
- Writer tests for numbering id/level mapping.
- Determinism tests for stable id assignment.

## 8.2 Integration tests (CLI)

For each list fixture:
1. run `rtfkit convert ... -o out.docx`
2. unzip output
3. assert `word/document.xml` contains `w:numPr`
4. assert `word/numbering.xml` contains expected numbering definitions
5. verify strict-mode behavior on unsupported/malformed list fixtures

## 8.3 Fixture matrix (minimum)

- simple bullets (single level)
- simple numbering (single level)
- nested bullets (>= 2 levels)
- mixed bullet + numbering in one document
- malformed/unsupported list pattern (fallback + warning)

## 9. Acceptance criteria

- Generated list fixtures open in Word/LibreOffice with plausible list rendering.
- XML assertions pass for numbering references and definitions.
- Fallback behavior is deterministic and warning-driven.
- Strict mode reliably fails when list semantics are dropped.
- CI green on supported OS matrix.

## 10. Risks and mitigations

- Risk: RTF list controls are inconsistent across sources.
  - Mitigation: explicit fallback path and fixture-driven parser improvements.
- Risk: DOCX numbering id instability.
  - Mitigation: deterministic id allocator with dedicated regression tests.
- Risk: parser complexity regression.
  - Mitigation: keep parser state transitions small and test each control path.

