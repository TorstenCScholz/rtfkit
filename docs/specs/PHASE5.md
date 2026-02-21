# rtfkit Next Steps Plan (Phase 5: Table Fidelity and Robustness)

Phase 5 deepens table support after Phase 4 hardening.
Goal is to improve semantic fidelity for messy real-world RTF tables while preserving safety, determinism, and strict-mode correctness.

## 1. Primary objective

Ship high-confidence table conversion for common advanced cases:

`RTF table controls -> resilient table IR semantics -> high-fidelity DOCX table XML`

with explicit handling for malformed/hostile inputs and deterministic degradation.

## 2. Scope

## In scope

- Cell merge semantics for practical horizontal and vertical merge cases.
- Table and cell formatting fidelity for common business documents.
- Strong malformed-input recovery without silent semantic drift.
- Strict-mode signal refinement for semantic loss vs cosmetic loss.
- Expanded fixture, golden, integration, and contract coverage for adversarial table data.
- Performance and parser-limit hardening for large/noisy table documents.

## Out of scope

- Full RTF table-spec parity with all edge features.
- Pixel-perfect layout parity with Microsoft Word for every source document.
- Floating/anchored table behavior parity across all producers.
- Rich drawing objects and image payload parsing (handled in follow-on phases).

## 3. Design constraints and principles

- Keep architecture boundaries:
  - `rtfkit-core`: parser/interpreter, IR semantics, warnings, degradation policy
  - `rtfkit-docx`: XML mapping and package serialization
  - `rtfkit-cli`: orchestration and contract behavior
- Treat input as adversarial:
  - never trust control-word order
  - never assume balanced structure
  - never silently reinterpret semantics
- Prefer semantic transparency over speculative reconstruction.
- Preserve visible text as a hard baseline.
- Keep output deterministic for identical input and limits.
- Maintain strict-mode contract:
  - semantic loss must emit `DroppedContent`
  - strict mode must fail consistently (exit code `4`)

## 4. Phase 5 capability targets

## 4.1 Merge fidelity target

Implement practical support for merge-related controls:

- `\clmgf` (merge start)
- `\clmrg` (merge continuation)
- optional vertical merge controls when present in source corpus (`\clvmgf`, `\clvmrg`)

Expected behavior:

- Build explicit merge intent in IR.
- Emit DOCX merge primitives (`w:gridSpan`, `w:vMerge`) where structurally valid.
- If merge graph is invalid or ambiguous:
  - preserve cell text
  - emit deterministic warnings
  - emit `DroppedContent` when table semantics materially change

## 4.2 Formatting fidelity target

Support a practical subset of table formatting controls:

- Row-level alignment and indentation controls (for example `\trql`, `\trqc`, `\trqr`, `\trleft`).
- Cell vertical alignment (`\clvertalt`, `\clvertalc`, `\clvertalb`).
- Common borders and spacing where mapping is clear and deterministic.

Unsupported formatting controls:

- Keep warning-only path for cosmetic omissions.
- Avoid `DroppedContent` unless semantics (not appearance) are changed.

## 4.3 Structural resilience target

Strengthen recovery policy for malformed structures:

- orphan `\cell`, `\row`, `\cellx`, merge markers
- non-monotonic `\cellx` boundaries
- inconsistent merge spans
- rows with impossible cell geometry
- interleaved prose/table controls

Recovery policy:

- preserve text and stable ordering
- prefer local repair over global rewrite
- emit deterministic warning reasons

## 5. IR evolution proposal

Extend table IR to encode merge semantics and normalized geometry.

Recommended additions:

```rust
pub struct TableCell {
    pub blocks: Vec<Block>,
    pub width_twips: Option<i32>,
    pub merge: Option<CellMerge>,
    pub v_align: Option<CellVerticalAlign>,
}

pub enum CellMerge {
    None,
    HorizontalStart { span: u16 },
    HorizontalContinue,
    VerticalStart,
    VerticalContinue,
}

pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub row_props: Option<RowProps>,
}

pub struct TableBlock {
    pub rows: Vec<TableRow>,
    pub table_props: Option<TableProps>,
}
```

Guidelines:

- Keep optional fields sparse to preserve compact JSON.
- Keep merge semantics normalized so writer is straightforward and deterministic.
- Avoid storing contradictory representations of the same concept.

## 6. Warning and strict-mode model refinement

## 6.1 Warning categories

Use explicit categories:

- `UnsupportedTableControl { control_word }`
- `MalformedTableStructure { reason }`
- `MergeConflict { reason }` (new)
- `TableGeometryConflict { reason }` (new)
- `UnclosedTableCell`
- `UnclosedTableRow`

## 6.2 Strict-mode decision matrix

Semantic loss (strict-failing):

- lost or ambiguous merge semantics
- unrecoverable cell-to-row mapping changes
- row/cell reflow that changes reading structure

Cosmetic loss (warning-only):

- unmapped border style variant
- approximate spacing/alignment style fallback

Required behavior:

- semantic loss emits both a specific table warning and `DroppedContent`.
- warning cap must still preserve strict-mode signal.

## 7. Parser/interpreter plan

## 7.1 State machine hardening

Build on Phase 4 state discipline:

- derive table membership from active row/cell context
- keep paragraph-local control markers local
- never let table context leak into prose

Additional state for Phase 5:

- pending merge markers per cell
- normalized row geometry builder
- row/table formatting accumulators

## 7.2 Merge normalization algorithm

Per row:

1. collect raw cell descriptors and boundaries
2. resolve merge start/continue chain
3. validate continuity and span
4. emit normalized cell merge model
5. degrade deterministically on conflicts

Determinism requirements:

- left-to-right stable traversal
- conflict resolution policy fixed and documented
- no heuristic branch based on runtime/environment

## 7.3 Degradation policy examples

- Continuation merge without valid start:
  - keep content as standalone cell
  - `MergeConflict` + `DroppedContent`
- Span exceeds row bounds:
  - clamp or split via documented rule
  - `TableGeometryConflict` + `DroppedContent`
- Incomplete vertical merge chain:
  - preserve text in visible cells
  - warning; strict-fail only if structure semantics changed

## 8. DOCX writer plan

## 8.1 Merge mapping

Map normalized IR merge semantics to DOCX:

- horizontal merge start -> `w:gridSpan`
- horizontal continuation -> collapsed/continuation representation per docx-rs capability
- vertical start/continue -> `w:vMerge` start/continue

If docx-rs has limitations:

- keep deterministic fallback representation
- emit semantic-loss signal in report when merge cannot be represented faithfully

## 8.2 Table property mapping

Map supported properties where deterministic:

- table justification/alignment
- row/cell vertical alignment
- selected borders/spacing controls

Fallback policy:

- omit unsupported properties (warning-only unless semantic impact)

## 8.3 Determinism constraints

- no random IDs
- stable element ordering
- stable defaults for omitted properties
- identical IR should yield stable DOCX XML (modulo package metadata outside contract)

## 9. Security and robustness requirements

- Keep parser limits enforced for table-heavy docs:
  - max input bytes
  - max group depth
  - max warning count
- Add table-specific defensive limits:
  - max rows per table
  - max cells per row
  - max merge span
- Ensure malformed merge graphs cannot trigger unbounded loops or quadratic explosions.
- Avoid integer overflow in width/geometry arithmetic.
- Validate negative/overflowing twip values before conversion.

## 10. Maintenance and code quality requirements

- Reduce duplicated finalization paths via shared helpers.
- Keep warning reason strings stable and documented for contracts.
- Document all deterministic fallback rules in one place.
- Add regression tests before refactors for high-risk parser transitions.
- Keep JSON IR changes backward-visible in changelog and migration notes.

## 11. Test plan

## 11.1 New fixture set

Add targeted fixtures:

- valid horizontal merge row
- valid vertical merge across rows
- mixed merge + non-merge row
- orphan merge continuation
- conflicting merge chains
- non-monotonic `\cellx`
- large table stress fixture
- prose/table interleave edge cases

## 11.2 Golden IR expectations

Validate:

- merge semantics encoded correctly in IR
- degraded fixtures preserve visible text
- semantic-loss fixtures include deterministic warnings and `DroppedContent`
- snapshots remain deterministic across reruns

## 11.3 DOCX integration assertions

For supported fixtures:

- `word/document.xml` contains correct merge tags (`w:gridSpan`, `w:vMerge`)
- table geometry and cell counts align with normalized IR

For degraded fixtures:

- strict mode fails when semantic loss is present
- non-strict mode succeeds with warnings

## 11.4 Contract tests

- strict-mode failure matrix for all semantic-loss categories
- warning-cap invariants (strict signal survives cap)
- deterministic output checks for same input repeated runs

## 12. Suggested implementation sequence (PR plan)

1. PR 1: IR extensions for merge and table props + serialization updates
2. PR 2: parser merge/state normalization with deterministic degradation
3. PR 3: DOCX merge/property writer mapping
4. PR 4: strict-mode warning/category refinements
5. PR 5: fixture/golden/integration/contract stress coverage
6. PR 6: docs/changelog/architecture sync

## 13. Risks and mitigations

- Risk: merge logic introduces non-deterministic repair paths
  - Mitigation: explicit conflict resolution rules + deterministic tests
- Risk: writer capability gaps in docx-rs for complex merge layouts
  - Mitigation: constrain supported subset and surface semantic-loss clearly
- Risk: performance regressions on large tables
  - Mitigation: stress fixtures, profiling, and defensive limits
- Risk: strict-mode over-failure due to noisy warnings
  - Mitigation: clear semantic-vs-cosmetic classification and contract tests

## 14. Acceptance criteria

- Common merge fixtures round-trip as real merged tables in DOCX.
- Unsupported/invalid merges degrade deterministically with explicit warnings.
- Strict mode fails only on semantic loss and remains stable under warning caps.
- No regressions in Phase 4 table behaviors (text preservation, ordering, width determinism).
- Full workspace tests and table stress tests pass in CI.
