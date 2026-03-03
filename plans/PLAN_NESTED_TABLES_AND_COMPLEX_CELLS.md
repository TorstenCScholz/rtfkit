# rtfkit Plan: Nested Tables and Complex Cells

**Status: PLANNED**

## 1. Objective

Add deterministic nested-table and complex-cell support across parser, IR, and writers so real-world enterprise tables no longer degrade to flattened or lossy output.

Primary outcomes:
- Tables inside cells are preserved as structured content.
- Mixed cell content (text, lists, tables, images) is represented consistently.
- Strict-mode behavior distinguishes cosmetic fallback from semantic loss.

## 2. Scope

### In scope

- Parser/runtime support for nested `\trowd...\row` regions inside cell context.
- IR updates to represent complex cell block trees.
- Writer support for DOCX, HTML, and Typst/PDF using a shared normalized model.
- Deterministic recovery rules for malformed nested structures.
- Fixtures, golden updates, integration tests, and contract tests.

### Out of scope (v1)

- Arbitrary floating layout within cells.
- Pixel-perfect parity for all producer-specific quirks.
- Full support for extreme recursion depths beyond safety limits.

## 3. Design principles

- Keep table semantics explicit in IR; avoid writer-specific hidden inference.
- Preserve source order within each cell block stream.
- Recover deterministically from malformed nested boundaries.
- Keep strict-mode failure tied to true semantic loss only.

## 4. High-level architecture plan

1. Parser/state model expansion
- Track a stack of active table contexts.
- Allow entering a child table context only when current sink is cell content.
- Finalize child table into parent cell block list at deterministic boundaries.

2. IR model extension
- Ensure table cells can contain heterogeneous `Vec<Block>` safely.
- Normalize nested tables as regular `Block::Table` entries within cell content.
- Add metadata needed for diagnostics (depth, recovered boundaries if any).

3. Finalization and normalization
- Validate nesting boundaries and close orphaned scopes predictably.
- Emit warnings for repaired structure; emit dropped-content only when semantics are lost.
- Enforce nesting/depth limits to protect memory and runtime.

4. Writer mapping
- DOCX: emit nested `<w:tbl>` within `<w:tc>` content without flattening.
- HTML: emit nested `<table>` trees with stable CSS classes.
- Typst: map nested tables in cell body flow with deterministic spacing/indent rules.

## 5. Delivery slices

### Slice A: Core parse + IR (highest risk)

- Runtime table-stack handling
- IR compatibility updates
- Parser/unit coverage

### Slice B: DOCX writer support

- Nested table emission inside cells
- Non-regression for existing table fixtures

### Slice C: HTML + Typst support

- Nested rendering parity and deterministic output
- Snapshot and integration coverage

### Slice D: Hardening and contracts

- Strict/non-strict contract validation
- Malformed nested input recovery fixtures
- Docs and changelog updates

## 6. Test strategy

- Unit tests (core):
  - nested table start/end in cell
  - orphan nested row recovery
  - depth limit enforcement
- Golden/snapshot tests:
  - 2-level and 3-level nested tables
  - mixed list + nested table cell content
  - malformed nested boundaries
- CLI integration/contract tests:
  - strict-mode pass for representable nested content
  - strict-mode fail on semantic loss cases
  - determinism checks across repeated runs

## 7. File touch plan (expected)

- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-core/src/rtf/state_tables.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-core/src/rtf/handlers_tables.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-core/src/rtf/finalize/tables.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-core/src/lib.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-docx/src/writer.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-html/src/blocks/table.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-render-typst/src/map/table.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/cli_contract_tests.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/golden_tests.rs`
- `/Users/torsten/Documents/Projects/rtfkit/docs/feature-support.md`

## 8. Risks and mitigations

- Risk: table IR complexity increases quickly with nesting.
  - Mitigation: stage rollout by depth and lock invariants with focused tests.
- Risk: writer parity diverges across formats.
  - Mitigation: shared normalization rules + cross-format fixture matrix.
- Risk: malformed producer output causes unstable recovery.
  - Mitigation: explicit deterministic repair rules and stable warning reasons.

## 9. Acceptance criteria

- Nested tables render across DOCX/HTML/PDF without flattening for supported cases.
- Existing non-nested table fixtures remain unchanged.
- Strict-mode behavior is deterministic and contract-tested.
- Documentation reflects supported depth/limitations and fallback behavior.
