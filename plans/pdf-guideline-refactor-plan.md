# PDF Guideline Realignment Refactoring Plan

## Status
- Plan only (no implementation in this document).
- Target: realign current PHASE_PDF work with the PDF guideline:
  - Single binary
  - RTF -> IR -> Typst renderer (in-process) -> PDF
  - No external tools / no PATH dependency
  - Deterministic output

## 1. Executive Summary

Current PDF changes are functionally broad but architecturally misaligned with the guideline:
- External `typst` CLI process invocation is used.
- Backend-neutral abstractions and extra surface area were introduced.
- List nesting semantics are at risk due to dropped IR list levels.
- Determinism claims are stronger than current guarantees.

This refactor will:
1. Introduce a focused `rtfkit-render-typst` crate with in-process rendering.
2. Remove backend-system complexity from the active code path.
3. Preserve IR as source of truth.
4. Keep HTML export optional and independent.
5. Keep de-scoped artifacts by moving them into `deprecated/` (never deleting files).

## 2. Guiding Design Principles

1. Strong cohesion:
   - One crate owns Typst rendering concerns.
   - CLI only orchestrates input/output and error mapping.
2. Loose coupling:
   - No plugin system and no generic backend layer.
   - Explicit dependency direction: core IR -> typst renderer -> CLI.
3. Determinism by construction:
   - Bundled default font for stable shaping/fallback behavior.
   - Fixed metadata strategy.
   - Stable serialization ordering.
4. Incremental, test-gated rollout:
   - Small PRs, each with objective gates.
5. No file deletions:
   - Superseded files are moved to `deprecated/`.
   - Active code and docs must not reference deprecated paths.

## 3. Target Architecture

## 3.1 Runtime Pipeline
`RTF -> IR (rtfkit-core) -> Typst source mapping (rtfkit-render-typst) -> Typst engine in-process -> PDF bytes`

## 3.2 Crate Responsibilities

1. `rtfkit-core`
   - Unchanged parser/interpreter/IR ownership.
2. `rtfkit-render-typst` (new active crate)
   - IR-to-Typst mapping for paragraph/list/table MVP.
   - In-process Typst world/engine integration.
   - Deterministic metadata + bundled default font handling.
   - Writer-level dropped reason emission for strict mode.
3. `rtfkit-cli`
   - `convert` command orchestration and exit code contract.
   - PDF output writes bytes returned by renderer.
   - No backend selection knobs.

## 3.3 CLI Simplicity Target

Primary user flow:
`rtfkit convert input.rtf -o output.pdf`

Compatibility policy to decide during implementation:
- Either keep `--to pdf` as compatibility alias, or infer target from output extension.
- In both cases: no backend flag and no external tool requirement.

## 4. Deprecation Strategy (No Deletion)

## 4.1 Deprecation Folder Convention

Use a top-level folder:
- `deprecated/`

Move superseded assets with preserved structure, for example:
- `deprecated/crates/rtfkit-pdf/...`
- `deprecated/docs/specs/PHASE_PDF.md` (if replaced)
- `deprecated/docs/adr/0003-pdf-backend-selection.md` (if superseded)
- `deprecated/docs/reference/pdf-output.md` (if superseded)

Add:
- `deprecated/README.md` explaining archival-only status.

## 4.2 Hard Guard: Ensure Deprecated Files Are Unused

Add CI/check guard that fails if active code references `deprecated/`:
1. Search imports/paths in Rust, tests, scripts, docs.
2. Fail build on matches except inside `deprecated/README.md` or explicit migration docs.
3. Guard `Cargo.toml` workspace members from including deprecated crates.

## 4.3 Candidate Moves in This Refactor

Move current backend-system PDF implementation to deprecated once replacement is ready:
1. `crates/rtfkit-pdf/` (existing backend abstraction + CLI shell-out implementation).
2. PDF docs that prescribe CLI dependency and backend abstraction model.
3. ADR/spec text that conflicts with the in-process-only guideline.

Note: exact file move list is finalized in Phase 1 inventory gate.

## 5. Refactoring Workstreams

## 5.1 Workstream A: Baseline and Inventory

Goal: freeze current behavior and establish migration map.

Tasks:
1. Capture current failing/passing PDF tests and exact failure modes.
2. Enumerate all active references to:
   - `rtfkit-pdf`
   - `--pdf-backend`
   - `--keep-intermediate`
   - external `typst` CLI assumptions
3. Build explicit file move matrix (source -> deprecated destination).

Deliverables:
1. Migration inventory section in this plan (updated during execution).
2. Test baseline snapshot in PR notes.

Gate:
1. No code changes yet; inventory reviewed.

## 5.2 Workstream B: New Renderer Crate Skeleton (`rtfkit-render-typst`)

Goal: introduce the new cohesive renderer boundary without behavior parity yet.

Tasks:
1. Create `crates/rtfkit-render-typst`.
2. Define minimal public API:
   - `document_to_pdf_with_warnings(doc, options) -> Result<Output, Error>`
3. Add focused options only needed for MVP determinism and page setup.
4. Add unit-test scaffold for API contract and error mapping.

Deliverables:
1. Compiling crate with stubbed in-process integration points.
2. CLI wired behind compile-safe placeholder (if needed).

Gate:
1. `cargo check --workspace` passes.

## 5.3 Workstream C: IR -> Typst Mapping (No Backend System)

Goal: deterministic mapping for MVP elements with no generic backend DSL.

Tasks:
1. Map Paragraph + runs:
   - bold/italic/underline
   - alignment
2. Map Lists:
   - preserve IR `ListItem.level` semantics
   - deterministic handling for malformed level transitions
3. Map Tables:
   - stable row/cell ordering
   - colspan/rowspan support
4. Emit explicit dropped reasons only for semantic loss.

Deliverables:
1. Mapping unit tests for representative fixtures.
2. Deterministic Typst source snapshots.

Gate:
1. Mapping tests cover nested list depth and merge-heavy tables.

## 5.4 Workstream D: In-Process Typst Engine Integration

Goal: remove shell-out and PATH dependency entirely.

Tasks:
1. Integrate Typst Rust library APIs in renderer crate.
2. Build in-process world/context with controlled inputs.
3. Bundle one default font in-repo or as embedded bytes.
4. Ensure offline behavior (no network/resource fetches).
5. Remove all runtime dependency on external `typst` binary.

Deliverables:
1. PDF bytes generated in-process.
2. Error variants mapped to writer-level failures.

Gate:
1. PATH-empty test still succeeds for core PDF fixtures.

## 5.5 Workstream E: Determinism Hardening

Goal: make deterministic claims true and testable.

Tasks:
1. Fix metadata strategy (creator/producer/timestamps).
2. Stabilize any generated IDs/order where possible.
3. Control font resolution order and fallback deterministically.
4. Add deterministic regression tests:
   - same input/options/environment -> byte-identical PDF (preferred)
   - if not feasible, define explicit normalized fingerprint fallback with maintainer sign-off.

Deliverables:
1. Determinism test suite in CLI + crate tests.
2. Documented determinism guarantees and limits.

Gate:
1. Determinism tests green and reproducible in CI.

## 5.6 Workstream F: CLI Simplification and Contract Preservation

Goal: preserve existing exit code contracts with simpler PDF surface.

Tasks:
1. Remove backend-system flags from active CLI path:
   - `--pdf-backend`
2. Decide fate of debug flags:
   - Either keep `--keep-intermediate` as deterministic debug feature, or remove for MVP simplicity.
3. Keep strict-mode integration for writer-level drops.
4. Ensure exit code mapping remains: `0/2/3/4`.

Deliverables:
1. Updated CLI contract tests.
2. Clean help text focused on user intent, not backend internals.

Gate:
1. Contract tests pass without external tool installation.

## 5.7 Workstream G: Deprecation Moves + Guard Rails

Goal: retire old implementation safely without deleting files.

Tasks:
1. Move superseded crate/docs/spec/ADR files to `deprecated/`.
2. Remove active references/imports/workspace memberships.
3. Add "no deprecated references" check in CI/scripts.

Deliverables:
1. Deprecated tree with provenance-preserving moves.
2. Active workspace free from deprecated dependencies.

Gate:
1. Search checks confirm no active usage of deprecated files.

## 5.8 Workstream H: Documentation Realignment

Goal: make docs match real architecture and operations.

Tasks:
1. Replace PDF docs with in-process architecture and usage.
2. Update README and feature matrix:
   - remove Typst CLI install requirement
   - state offline single-binary behavior
3. Add troubleshooting for in-process renderer failures.
4. Add migration note for deprecated docs.

Deliverables:
1. New canonical PDF docs.
2. Deprecated docs moved under `deprecated/`.

Gate:
1. No conflicting statements across README/spec/reference docs.

## 6. Test Plan

## 6.1 Unit Tests

1. Paragraph/run mapping (formatting + escaping).
2. List-level preservation and malformed-level degradation.
3. Table merge mapping and ordering.
4. Deterministic metadata transformation.

## 6.2 Integration/CLI Tests

1. `convert input.rtf -o output.pdf` works on core fixtures.
2. No external Typst binary required.
3. Strict mode returns `4` only on semantic drops.
4. Invalid options/paths still map to correct exit codes.

## 6.3 Determinism Tests

1. Multi-run byte identity for representative fixtures.
2. Cross-run stability for nested list and merged table fixtures.
3. Optional cross-machine fingerprint validation in CI matrix.

## 6.4 Safety Tests

1. Offline rendering (no network).
2. Controlled resource loading only.
3. Output stability with bundled default font.

## 7. Rollout Strategy (Small PR Sequence)

1. PR1: Inventory + new crate skeleton + no behavior change.
2. PR2: Mapping implementation + unit tests.
3. PR3: In-process Typst integration + baseline PDF fixture pass.
4. PR4: Determinism hardening + determinism tests.
5. PR5: CLI simplification + contract test updates.
6. PR6: Deprecation moves + no-usage guard.
7. PR7: Docs realignment + final acceptance checklist.

Each PR must stay narrowly scoped and mergeable on its own.

## 8. Acceptance Criteria

1. Active PDF path uses in-process Typst library only.
2. No active dependency on external `typst` CLI or PATH.
3. CLI and strict-mode exit code contracts hold.
4. IR remains source of truth; no new backend-neutral framework in active path.
5. Determinism guarantees are verified by tests.
6. Superseded artifacts are moved to `deprecated/`, not deleted.
7. Active codebase has zero runtime/build references to deprecated files.

## 9. Risks and Mitigations

1. Typst library API complexity:
   - Mitigation: isolate in a small adapter module and add integration harness tests.
2. Determinism edge cases (fonts/metadata):
   - Mitigation: bundled font + explicit metadata normalization + snapshot/fingerprint tests.
3. Migration churn across CLI/tests/docs:
   - Mitigation: phased PR rollout and compatibility wrappers only where needed.
4. Hidden deprecated references:
   - Mitigation: automated reference guard in CI.

## 10. Open Decisions to Resolve Early

1. Keep `--to pdf` as explicit target vs output-extension inference default.
2. Keep or remove `--keep-intermediate` in active path.
3. Whether to keep a thin `rtfkit-pdf` facade (active) or fully standardize on `rtfkit-render-typst` for PDF generation API.

Decision deadline: before implementing Workstream D, to avoid rework.

## 11. Migration Inventory

### 11.1 Current Test Baseline Status

**Test Run Date:** 2026-02-22

**Summary:**
- Total tests: 103
- Passed: 95
- Failed: 8

**Failed Tests (all PDF-related, require external `typst` CLI):**

| Test Name | Failure Mode |
|-----------|--------------|
| `pdf_output_tests::pdf_output_empty_document_succeeds` | Exit code 3: "typst CLI not found in PATH" |
| `pdf_output_tests::pdf_output_refuses_overwrite_without_force` | Exit code 3: "typst CLI not found in PATH" |
| `pdf_output_tests::pdf_output_succeeds_on_simple_fixture` | Exit code 3: "typst CLI not found in PATH" |
| `pdf_output_tests::pdf_output_succeeds_on_mixed_content` | Exit code 3: "typst CLI not found in PATH" |
| `pdf_output_tests::pdf_output_succeeds_on_table_fixture` | Exit code 3: "typst CLI not found in PATH" |
| `pdf_output_tests::pdf_output_succeeds_on_list_fixture` | Exit code 3: "typst CLI not found in PATH" |
| `pdf_output_tests::pdf_output_unicode_succeeds` | Exit code 3: "typst CLI not found in PATH" |
| `pdf_output_tests::pdf_output_with_keep_intermediate_succeeds` | Exit code 3: "typst CLI not found in PATH" |

**Root Cause:** All PDF tests fail because they require the external `typst` CLI to be installed and available in PATH. This is the core issue the refactor aims to address.

**Passing Test Categories:**
- Exit code tests (all pass)
- Determinism tests (all pass)
- HTML CSS tests (all pass)
- Limits tests (all pass)
- Recovery behavior tests (all pass)
- Regression tests (all pass)
- Strict mode tests (all pass)
- Warning semantics tests (all pass)

### 11.2 Active References to Deprecated Patterns

#### 11.2.1 `rtfkit-pdf` Crate References

| Location | Type | Description |
|----------|------|-------------|
| `Cargo.toml:3` | Workspace | `members = [..., "crates/rtfkit-pdf"]` |
| `Cargo.lock:962-1004` | Lockfile | Package metadata for `rtfkit-pdf` |
| `crates/rtfkit-cli/Cargo.toml:17` | Dependency | `rtfkit-pdf = { path = "../rtfkit-pdf" }` |
| `crates/rtfkit-cli/src/main.rs:10-12` | Import | `use rtfkit_pdf::{document_to_pdf_with_warnings, ...}` |
| `docs/specs/PHASE_PDF.md:60,72-73,83,136,272,327` | Spec | References to `rtfkit-pdf` crate |
| `docs/adr/0003-pdf-backend-selection.md:18,227,306-307` | ADR | References to `rtfkit-pdf` crate |

#### 11.2.2 `--pdf-backend` CLI Flag References

| Location | Line(s) | Description |
|----------|---------|-------------|
| `crates/rtfkit-cli/src/main.rs` | 114-115 | CLI argument definition |
| `crates/rtfkit-cli/src/main.rs` | 148-149 | Field in `ConvertRequest` struct |
| `crates/rtfkit-cli/src/main.rs` | 176-178, 191-193, 211-213 | Argument passing |
| `crates/rtfkit-cli/src/main.rs` | 227-229 | Validation check |
| `crates/rtfkit-cli/src/main.rs` | 233-234 | Error message |
| `crates/rtfkit-cli/src/main.rs` | 296-298, 527-529, 591-593 | Usage in `handle_pdf_output` |
| `docs/reference/pdf-output.md` | 13-16 | Documentation for flag |
| `docs/specs/PHASE_PDF.md` | 181 | Spec for flag |
| `crates/rtfkit-cli/tests/cli_contract_tests.rs` | 2214-2223 | Test for flag rejection on non-PDF target |

#### 11.2.3 `--keep-intermediate` CLI Flag References

| Location | Line(s) | Description |
|----------|---------|-------------|
| `README.md` | 86-87 | Usage example |
| `docs/reference/pdf-output.md` | 24, 30, 45-46, 95 | Documentation |
| `docs/adr/0003-pdf-backend-selection.md` | 70-71, 216-217, 243-245 | ADR discussion |
| `docs/specs/INITIAL_DESCRIPTION.md` | 234-235 | Initial spec |
| `docs/specs/PHASE_PDF.md` | 80, 146-147, 181-182, 305-306, 372-373 | Phase spec |
| `crates/rtfkit-cli/src/main.rs` | 118-119, 150, 178, 193, 213, 229, 234, 298, 530, 598 | CLI implementation |
| `crates/rtfkit-cli/tests/cli_contract_tests.rs` | 2284-2303 | Test for flag |
| `crates/rtfkit-pdf/src/writer.rs` | 30-32 | Output struct field |
| `crates/rtfkit-pdf/src/options.rs` | 73-75, 93-99, 114-116, 130-131, 174-181 | Options struct |
| `crates/rtfkit-pdf/src/lib.rs` | 94-95 | Default options |
| `crates/rtfkit-pdf/src/backend/typst.rs` | 304-306 | Backend implementation |
| `crates/rtfkit-pdf/src/backend/mod.rs` | 21-23 | Backend output struct |

#### 11.2.4 External `typst` CLI Assumptions (Shell-Out Code)

| Location | Line(s) | Description |
|----------|---------|-------------|
| `crates/rtfkit-pdf/src/backend/typst.rs` | 282-292 | `check_typst_available()` - `Command::new("typst")` |
| `crates/rtfkit-pdf/src/backend/typst.rs` | 321-327 | `Command::new("typst").arg("compile")` (keep_intermediate path) |
| `crates/rtfkit-pdf/src/backend/typst.rs` | 349-355 | `Command::new("typst").arg("compile")` (temp file path) |
| `docs/reference/pdf-output.md` | 72-85, 95-96 | Installation instructions and troubleshooting |
| `docs/specs/INITIAL_DESCRIPTION.md` | 233-235 | Pipeline description with `typst compile` |
| `docs/adr/0003-pdf-backend-selection.md` | 230-248 | Implementation pattern showing CLI invocation |

### 11.3 File Move Matrix

#### 11.3.1 Files to Move to `deprecated/`

| Source | Destination | Reason |
|--------|-------------|--------|
| `crates/rtfkit-pdf/` | `deprecated/crates/rtfkit-pdf/` | Entire crate with backend abstraction + CLI shell-out implementation |
| `docs/adr/0003-pdf-backend-selection.md` | `deprecated/docs/adr/0003-pdf-backend-selection.md` | ADR prescribes CLI shell-out approach and backend abstraction pattern |
| `docs/reference/pdf-output.md` | `deprecated/docs/reference/pdf-output.md` | PDF docs prescribe CLI dependency and backend abstraction model |
| `docs/specs/PHASE_PDF.md` | `deprecated/docs/specs/PHASE_PDF.md` | PDF spec conflicts with in-process-only guideline |

#### 11.3.2 Files to Modify (Not Move)

| File | Modifications Required |
|------|------------------------|
| `Cargo.toml` (workspace) | Remove `crates/rtfkit-pdf` from workspace members |
| `crates/rtfkit-cli/Cargo.toml` | Remove `rtfkit-pdf` dependency; add `rtfkit-render-typst` dependency |
| `crates/rtfkit-cli/src/main.rs` | Remove `--pdf-backend` flag; update imports; simplify `handle_pdf_output()` |
| `README.md` | Remove Typst CLI install requirement; update usage examples |
| `Cargo.lock` | Will be regenerated automatically |

#### 11.3.3 Files to Create

| File | Purpose |
|------|---------|
| `deprecated/README.md` | Explain archival-only status of deprecated directory |
| `crates/rtfkit-render-typst/` | New crate for in-process Typst rendering |
| `docs/reference/pdf-output.md` (new) | Updated PDF docs for in-process architecture |

### 11.4 rtfkit-pdf Crate Structure (to be deprecated)

```
crates/rtfkit-pdf/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public API facade
    ├── options.rs       # PdfWriterOptions, PdfBackend enum
    ├── error.rs         # PdfWriterError
    ├── writer.rs        # document_to_pdf_with_warnings()
    ├── determinism.rs   # Metadata normalization (unused)
    ├── backend/
    │   ├── mod.rs       # Backend trait + dispatch
    │   └── typst.rs     # Typst CLI shell-out implementation
    ├── intermediate/
    │   └── mod.rs       # Backend-neutral intermediate document model
    └── map/
        ├── mod.rs       # IR to intermediate mapping
        ├── paragraph.rs # Paragraph mapping
        ├── list.rs      # List mapping
        └── table.rs     # Table mapping
```

**Key Observations:**
1. The `backend/` module contains the abstraction layer that will be removed
2. The `intermediate/` and `map/` modules contain IR-to-Typst mapping logic that may be reusable
3. The `determinism.rs` module exists but is not fully utilized
4. All shell-out code is isolated in `backend/typst.rs`

### 11.5 Migration Dependencies

The following order must be maintained:

1. **Phase 1:** Create new `rtfkit-render-typst` crate with in-process implementation
2. **Phase 2:** Update CLI to use new crate
3. **Phase 3:** Move `rtfkit-pdf` to deprecated (only after new crate works)
4. **Phase 4:** Move conflicting docs to deprecated
5. **Phase 5:** Update remaining active docs

**Blocking Issues:**
- Cannot move `rtfkit-pdf` to deprecated until `rtfkit-render-typst` is functional
- Cannot remove CLI flags until backend is replaced
- Tests will remain failing until in-process implementation is complete

