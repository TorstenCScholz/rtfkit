# rtfkit Next Steps Plan (Phase 2+)

This document defines the next implementation phase for `rtfkit` after the current RTF -> IR/report baseline.
It is a planning artifact only and does not imply implementation is complete.

## 1. Current baseline

As of this plan:
- RTF parsing and interpretation to IR exists in `rtfkit-core`.
- CLI report output exists (`text`/`json`) and `--emit-ir` works.
- `--output` is intentionally rejected until a real writer exists.
- Golden tests currently validate IR snapshots.

## 2. Primary objective

Deliver true end-to-end conversion:

`RTF -> IR -> DOCX`

with deterministic output, explicit error reporting, and production-grade tests.

## 3. Scope for next delivery

### In scope
- DOCX writer implementation for current IR subset.
- Activation of `-o/--output` to write `.docx` files.
- End-to-end CLI behavior for conversion success/failure paths.
- DOCX-level golden/integration tests (`word/document.xml` assertions).
- Parser hardening limits (depth/size/resource protection).
- Documentation and release pipeline updates matching behavior.

### Out of scope (for this phase)
- Full RTF spec coverage.
- Lists/tables/images as first-class IR blocks (unless needed as safe fallback only).
- HTML/PDF targets.
- Pixel-perfect Word layout fidelity.

## 4. Architecture plan

## 4.1 Writer boundary

Introduce a dedicated writer module/crate (recommended: `crates/rtfkit-docx`) that:
- Accepts IR `Document` as input.
- Produces `.docx` bytes or writes to file.
- Owns all DOCX mapping and serialization details.

`rtfkit-core` remains parser/IR/report only.
`rtfkit` CLI orchestrates file I/O and exit codes.

## 4.2 Mapping contract (v1)

Map IR to DOCX as follows:
- `Document.blocks` -> sequence of `w:p` elements.
- `Paragraph.alignment` -> `w:pPr/w:jc` (`left`, `center`, `right`, `both` for justify).
- `Run.text` -> `w:r/w:t` (preserve spaces using `xml:space="preserve"` when needed).
- `Run.bold` -> `w:rPr/w:b`.
- `Run.italic` -> `w:rPr/w:i`.
- `Run.underline` -> `w:rPr/w:u w:val="single"`.

`font_size` and `color` may be no-op initially if not yet represented reliably in parser output.
If omitted, writer must be deterministic and documented.

## 4.3 Error handling contract

- Writer errors return typed failures to CLI.
- CLI maps errors to existing exit codes:
  - `2`: parse/validation failure
  - `3`: writer or I/O failure
  - `4`: strict mode violation
- Strict mode remains warning-driven (`DroppedContent`).

## 5. CLI behavior changes

When writer lands:
- `rtfkit convert input.rtf -o out.docx` becomes required for conversion mode.
- `--to docx` remains accepted (future-proofing for additional targets).
- `--emit-ir` remains optional and independent.
- stdout remains report-only (no binary output on stdout).

Suggested validation rules:
- Require `.docx` extension for output path or warn clearly.
- Fail early if output directory is missing/not writable.
- Refuse overwrite only if `--force` is not set (optional decision).

## 6. Test strategy

## 6.1 Unit tests

Add focused tests in writer module:
- paragraph/run mapping
- alignment mapping
- style mapping
- whitespace preservation in `w:t`
- deterministic XML ordering and generated package structure

## 6.2 Integration tests (CLI)

Add conversion tests that:
1. run CLI to generate `.docx` from fixture
2. unzip resulting file
3. parse/assert `word/document.xml`
4. verify expected tags/text (`w:p`, `w:r`, `w:t`, `w:b`, `w:i`, `w:u`, `w:jc`)

## 6.3 Golden tests

Maintain two golden layers:
- IR golden snapshots (existing)
- DOCX XML golden/assertions (new)

Do not allow blind golden regeneration in CI without semantic assertions.

## 6.4 Regression matrix

Minimum fixture categories for writer validation:
- simple paragraph
- multiple paragraphs
- mixed styles in one paragraph
- nested style scopes
- alignment variants
- unicode and escaped symbols
- unsupported destination content with strict mode behavior

## 7. Robustness and safety plan

Add parser/runtime safeguards:
- `max_input_bytes` (reject overly large inputs)
- `max_group_depth` (prevent pathological nesting)
- optional `max_warning_count` (cap report growth)

Behavior:
- limits should fail fast with explicit parse/conversion error.
- defaults should be conservative and documented.
- optional CLI flags for overrides can be considered later.

## 8. CI and release plan

- Ensure release workflow builds actual package `rtfkit` and archives produced binary.
- Add CI step for DOCX integration tests on at least Linux.
- Keep cross-platform smoke tests for CLI parse/report behavior.
- Ensure reproducible output where practical (stable ordering, deterministic writer).

## 9. Documentation plan

Update in lockstep with implementation:
- `README.md` usage examples with real `-o/--output` conversion.
- `docs/arch/README.md` pipeline diagram including DOCX writer stage.
- Add ADR for DOCX writer library choice (new `docs/adr/0002-...`).
- Update changelog with user-visible behavior changes.

## 10. Milestones and acceptance criteria

## Milestone A: Writer spike and decision
- Evaluate 1-2 DOCX crates/libraries.
- Record decision in ADR.
- Acceptance: ADR merged, proof-of-concept generates valid minimal docx.

## Milestone B: Functional writer
- Implement IR -> DOCX for text/alignment/basic inline styles.
- Wire CLI `-o/--output`.
- Acceptance:
  - generated files open in Word/LibreOffice
  - unit + integration tests pass
  - exit codes correct for failure paths

## Milestone C: Hardening and coverage
- Add limits and robust error messages.
- Expand fixtures and DOCX assertions.
- Acceptance:
  - CI green across supported platforms
  - strict mode and unsupported-content behavior deterministic

## Milestone D: Documentation and release readiness
- Update docs/README/examples.
- Verify release workflow artifacts and naming.
- Acceptance:
  - release dry-run succeeds
  - user-facing docs match actual CLI behavior exactly

## 11. Risks and mitigations

- Risk: DOCX library gaps or unstable APIs.
  - Mitigation: small abstraction boundary and ADR-backed selection.
- Risk: XML whitespace/escaping regressions.
  - Mitigation: dedicated tests around `w:t` semantics and unicode fixtures.
- Risk: parser/writer contract drift.
  - Mitigation: contract tests from fixture -> IR -> DOCX assertions in CI.
- Risk: scope creep into full RTF fidelity.
  - Mitigation: enforce out-of-scope list and warning-based fallbacks.

## 12. Execution order (recommended)

1. ADR for DOCX writer choice.
2. Implement writer crate/module with unit tests.
3. Wire CLI `--output` path and error handling.
4. Add DOCX integration tests and golden assertions.
5. Add robustness limits.
6. Update docs/changelog and run full CI/release checks.

## 13. Definition of done

- End-to-end `rtfkit convert input.rtf -o output.docx` works for fixture matrix.
- Strict mode and exit codes behave per contract.
- IR and DOCX tests both pass in CI.
- README and architecture docs are current and accurate.
- Release workflow is verified for package/binary naming and artifact generation.
