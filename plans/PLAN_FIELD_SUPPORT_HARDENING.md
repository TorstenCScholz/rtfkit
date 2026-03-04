# rtfkit Plan: Field Support Hardening and Design Cleanup

**Status: DRAFT (implementation-ready)**

This plan addresses four review findings in the current staged field expansion:

1. Semantic fields flatten `\fldrslt` to plain text and lose formatting.
2. Semantic field parsing is not truly switch-tolerant for flag switches before arguments.
3. Unresolved cross-reference handling is inconsistent across output formats.
4. IR JSON shape for nested tagged enums uses duplicate `type` keys.

The goal is to fix these as a cohesive design upgrade, not a sequence of local patches.

## 1. Objective

Deliver a maintainable field subsystem that is:

1. Correct for common RTF field variants.
2. Cohesive in parser/finalizer responsibilities.
3. Consistent in warning and strict-mode behavior across outputs.
4. Stable and explicit in IR JSON contract shape.

## 2. Design Principles

1. **Single-purpose components**:
   - field instruction parsing logic must be isolated from runtime state mutation.
2. **Semantic/presentation separation**:
   - field meaning and visible result rendering should both be represented without collapsing one into the other.
3. **Deterministic fallback policy**:
   - unresolved/unsupported semantics should preserve visible text and emit stable warnings.
4. **Contract-first evolution**:
   - IR schema changes need an explicit compatibility strategy.

## 3. Target Architecture

## 3.1 `field_instruction` module in core parser

Create a dedicated parser subsystem under:

- `crates/rtfkit-core/src/rtf/field_instruction/mod.rs`
- `crates/rtfkit-core/src/rtf/field_instruction/tokenize.rs`
- `crates/rtfkit-core/src/rtf/field_instruction/parse.rs`
- `crates/rtfkit-core/src/rtf/field_instruction/spec.rs`
- `crates/rtfkit-core/src/rtf/field_instruction/types.rs`

Responsibilities:

1. Parse `fldinst` text to a typed instruction model.
2. Handle quoted/unquoted args and escaped quotes.
3. Parse switches using an explicit switch spec table (flag vs value-carrying).
4. Return parser output independent of runtime state.

Non-responsibilities:

1. No warnings emission.
2. No report mutation.
3. No direct paragraph/inline mutation.

## 3.2 `SemanticField` inline representation

Introduce a dedicated inline payload to preserve visible formatting:

```rust
pub struct SemanticField {
    pub reference: SemanticFieldRef,
    pub runs: Vec<Run>,                 // visible field result with formatting
    pub has_non_run_content: bool,      // deterministic degradation signal
}

Inline::SemanticField(SemanticField)
```

Rationale:

1. Keeps semantic identity (`REF`, `SEQ`, etc.) explicit.
2. Preserves visible formatting (bold/italic/font/etc.) from `\fldrslt`.
3. Avoids dropping non-run content silently (record and warn).

## 3.3 Finalization pipeline split

Split current `finalize_field` flow into small functions:

1. `parse_instruction(...) -> ParsedFieldInstruction`
2. `extract_field_result_runs(...) -> FieldResultProjection`
3. `build_inline_from_instruction(...) -> InlineDecision`
4. `emit_field_warnings(...)`

This avoids one large `match` with mixed concerns and makes new field types low-risk.

## 3.4 Cross-reference resolution service

Add a post-parse semantic resolver in core:

- `crates/rtfkit-core/src/rtf/finalize/semantic_fields.rs`

Responsibilities:

1. Build known bookmark set.
2. Resolve `REF` / `NOTEREF` targets.
3. Emit `unresolved_cross_reference` warning in core report (output-agnostic).
4. Mark unresolved refs in semantic field data (`resolved: bool` or equivalent).

Result:

1. DOCX/HTML/PDF behavior can be consistent without each backend inventing its own resolution semantics.

## 4. IR Contract Cleanup

Current nested tagged-enum JSON emits duplicate `type` keys for `Inline::PageField` and `Inline::SemanticField`.

## 4.1 Schema v2 shape

Use disjoint keys for nested enums:

1. `Inline` stays tagged by `type`.
2. Nested field enums use `kind` (not `type`).

Example:

```json
{
  "type": "semantic_field",
  "kind": "ref",
  "target": "sec_intro",
  "runs": [...]
}
```

## 4.2 Compatibility strategy

1. Add `--ir-schema-version {1|2}` in CLI emission path.
2. Default to v1 for one release.
3. Emit deprecation note in docs/changelog.
4. Flip default to v2 in next minor release.

This avoids breaking existing consumers abruptly while fixing malformed JSON shape.

## 5. Writer Mapping Strategy (post-core cleanup)

## 5.1 DOCX (`crates/rtfkit-docx`)

1. `REF` / `NOTEREF`:
   - if resolved: emit anchor hyperlink with formatted runs.
   - if unresolved: emit formatted fallback runs (no broken hyperlink).
2. `SEQ`, `DOCPROPERTY`, `MERGEFIELD`:
   - emit formatted runs from semantic field payload.

## 5.2 HTML (`crates/rtfkit-html`)

1. `REF` / `NOTEREF`:
   - if resolved: `<a href="#...">` with formatted run content.
   - if unresolved: formatted fallback content without `href`.
2. other semantic fields:
   - render formatted runs in stable wrapper span for traceability.

## 5.3 Typst (`crates/rtfkit-render-typst`)

1. Reuse core resolution state instead of independent resolution checks where possible.
2. Keep `PartialSupport` warnings for non-dynamic semantics (`MERGEFIELD`, etc.).

## 6. Warning and Strict-Mode Policy

1. `DroppedContent` only for true visible-content loss.
2. `UnsupportedField` for semantics not fully represented but text preserved.
3. `UnresolvedCrossReference` (new, warning severity) when target missing.
4. Strict mode remains tied to `DroppedContent` only.

## 7. Implementation Slices

## Slice A: Parser extraction and switch semantics

Deliverables:

1. new `field_instruction` module + switch spec table.
2. replace ad hoc semantic parser path with typed parser output.
3. tests for switch-before-arg, switch-after-arg, quoted/unquoted variants.

Acceptance:

1. `REF \h target` and `REF target \h` both parse target correctly.
2. parser module has no direct runtime/report mutation.

## Slice B: Semantic field payload redesign

Deliverables:

1. `SemanticField` inline payload with formatted runs.
2. field finalizer projection helper that preserves run formatting.
3. warning when non-run content appears in field result and cannot be represented inline.

Acceptance:

1. formatted `\fldrslt` text remains formatted in all outputs.
2. no silent downgrade from formatted result to plain text.

## Slice C: Core cross-reference resolver

Deliverables:

1. bookmark target resolver in core finalize.
2. consistent unresolved warning in report.
3. resolved/unresolved marker available to writers.

Acceptance:

1. unresolved refs produce one stable warning per unique occurrence policy.
2. all writers consume same resolution outcome.

## Slice D: Writer parity pass

Deliverables:

1. DOCX/HTML/Typst mappings updated to use `SemanticField` runs and resolution state.
2. broken links avoided for unresolved refs.

Acceptance:

1. no unresolved internal link emitted as clickable in DOCX/HTML.
2. output remains deterministic.

## Slice E: IR schema v2 and migration

Deliverables:

1. schema-v2 emission path.
2. CLI flag + docs + changelog migration notes.
3. golden test coverage for schema v1 and v2 where required.

Acceptance:

1. no duplicate JSON keys in v2.
2. migration guidance documented and tested.

## 8. Test Strategy

## 8.1 Unit tests (core)

1. instruction parser matrix for each field family.
2. switch handling matrix (flag/value switch permutations).
3. field result projection preserving style attributes.

## 8.2 Contract tests (CLI)

1. strict mode:
   - passes with preserved result text.
   - fails only on dropped-content cases.
2. warnings:
   - stable reason strings for unsupported and unresolved cases.

## 8.3 Writer tests

1. DOCX XML assertions for resolved/unresolved refs.
2. HTML snapshots including formatted semantic field runs.
3. Typst mapping snapshots for partial support and unresolved refs.

## 8.4 Regression fixtures

Add fixtures focused on previously missed cases:

1. `field_ref_switch_before_target.rtf`
2. `field_ref_switch_after_target.rtf`
3. `field_ref_formatted_fldrslt.rtf`
4. `field_noteref_unresolved_target.rtf`
5. `field_mergefield_formatted_fldrslt.rtf`

## 9. File Touch Map (expected)

Core:

- `crates/rtfkit-core/src/lib.rs`
- `crates/rtfkit-core/src/rtf/handlers_fields.rs`
- `crates/rtfkit-core/src/rtf/state_fields.rs`
- `crates/rtfkit-core/src/rtf/finalize/semantic_fields.rs` (new)
- `crates/rtfkit-core/src/rtf/field_instruction/*` (new)
- `crates/rtfkit-core/src/rtf/tests/fields.rs`
- `crates/rtfkit-core/src/report.rs`

Writers:

- `crates/rtfkit-docx/src/paragraph.rs`
- `crates/rtfkit-html/src/blocks/paragraph.rs`
- `crates/rtfkit-render-typst/src/map/paragraph.rs`

CLI/tests/docs:

- `crates/rtfkit-cli/tests/cli_contract/*`
- `crates/rtfkit-cli/tests/golden_tests.rs`
- `docs/warning-reference.md`
- `docs/feature-support.md`
- `CHANGELOG.md`

## 10. Rollout and Risk Management

1. Merge by slice (A -> E), each slice independently testable.
2. Keep each PR small and scoped to one concern.
3. Run full suite after each slice:
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test --all`
4. Regenerate and review golden snapshots only when behavior changes are intentional.

## 11. Definition of Done

1. Semantic field parsing is switch-robust and modular.
2. Field result formatting is preserved for semantic fields.
3. Unresolved cross-reference diagnostics are consistent across outputs.
4. IR schema v2 eliminates duplicate-key ambiguity with documented migration.
5. Strict-mode semantics remain unchanged and fully covered by tests.
