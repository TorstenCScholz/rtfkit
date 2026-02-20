# rtfkit Next Steps Plan (Phase 4: Tables)

This phase introduces table support on top of list-capable conversion.

## 1. Primary objective

Preserve table structure from common RTF inputs so output is not flattened into plain flowing paragraphs.

## 2. In scope

- Implement a practical subset of table controls (for example `\trowd`, `\cell`, `\row`).
- Map parsed table structure into IR and DOCX table elements.
- Keep fallback deterministic when table structure cannot be resolved.
- Emit explicit warnings for degraded output paths.

## 3. Out of scope

- Full RTF table-spec fidelity.
- Complete support for all border, merge, and complex layout combinations.

## 4. Implementation direction

- Add or extend IR to represent table structure (`table -> rows -> cells`).
- Parse row/cell boundaries with robust error handling.
- DOCX writer maps to `w:tbl`, `w:tr`, `w:tc`.
- Fallback path:
  - render textual row/cell separators as plain paragraphs
  - emit `DroppedContent` when table semantics are lost

## 5. Testing and fixtures

- Add table-focused fixtures:
  - simple 2x2 table
  - multiple rows with uneven content
  - malformed/incomplete table controls
- Add XML-level assertions that table tags exist in DOCX output for supported cases.
- Verify strict mode failure on fallback fixtures that lose semantics.

## 6. Acceptance criteria

- Supported table fixtures remain structurally tables in DOCX.
- Unsupported patterns degrade predictably with warning coverage.
- Strict mode behavior remains contract-correct.

