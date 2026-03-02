# rtfkit Plan: Docs and Contract Sync Hardening

**Status: PLANNED**

## 1. Objective

Re-align public documentation with actual runtime behavior and add lightweight guardrails so docs do not drift from code-level contracts.

Primary outcomes:
- Users can trust support matrices and warning docs.
- Strict-mode and warning semantics are documented exactly as implemented.
- Future feature work has a repeatable "update docs + verify contract" workflow.

## 2. Why this is high priority

- Current docs contain stale feature statements in some areas (especially PDF and field/page-management evolution).
- Drift creates support overhead and incorrect implementation assumptions.
- Contract clarity is required before shipping additional major features.

## 3. Scope

### In scope

- Public contract docs:
  - `README.md`
  - `docs/feature-support.md`
  - `docs/rtf-feature-overview.md`
  - `docs/warning-reference.md`
  - `docs/reference/pdf-output.md`
- Warning contract alignment with `rtfkit-core` warning variants and strict-mode semantics.
- Feature-support alignment for DOCX/HTML/PDF and parser behavior.
- Add CI-level checks for basic doc/contract consistency.
- Add release checklist items that force contract-doc review before release.

### Out of scope

- Rewriting all architecture/spec docs.
- Large documentation IA redesign.
- New feature implementation (this plan documents and hardens existing behavior).

## 4. Workstreams

### Workstream A: Contract inventory (source of truth)

- Build a compact inventory table from code:
  - Warning variants and stable reason strings from `rtfkit-core`.
  - Strict-mode failure rules from CLI + contract tests.
  - Current output capabilities per renderer from tests and writer code.
- Mark each item as:
  - `accurate in docs`
  - `missing in docs`
  - `docs claim unsupported but code supports`

### Workstream B: Documentation realignment pass

- Update docs to match inventory without expanding feature claims.
- Ensure unsupported/partial/supported labels are consistent across:
  - README summary
  - feature matrix
  - reference pages
- Add concrete examples for:
  - page fields and TOC behavior
  - warning categories and strict/non-strict effects

### Workstream C: Drift-prevention guardrails

- Add a small CI script/check that:
  - verifies every warning type is documented in `docs/warning-reference.md`
  - fails if known stable warning names are removed from docs
- Add a release gate checklist:
  - "contract docs updated" and "support matrix re-validated against tests"

### Workstream D: Test and release integration

- Add/refresh contract tests where docs expose guarantees (warning names, strict behavior, exit codes).
- Add changelog entry documenting contract-sync pass and any externally visible doc corrections.

## 5. Proposed execution order

1. Inventory and gap report (single PR note/table).
2. Docs realignment edits (single docs-focused PR).
3. CI guardrail script + release checklist updates.
4. Contract-test hardening where gaps were uncovered.

## 6. File touch plan (expected)

- `/Users/torsten/Documents/Projects/rtfkit/README.md`
- `/Users/torsten/Documents/Projects/rtfkit/docs/feature-support.md`
- `/Users/torsten/Documents/Projects/rtfkit/docs/rtf-feature-overview.md`
- `/Users/torsten/Documents/Projects/rtfkit/docs/warning-reference.md`
- `/Users/torsten/Documents/Projects/rtfkit/docs/reference/pdf-output.md`
- `/Users/torsten/Documents/Projects/rtfkit/docs/release-checklist.md`
- `/Users/torsten/Documents/Projects/rtfkit/scripts/` (new docs/contract guard script)
- `/Users/torsten/Documents/Projects/rtfkit/.github/workflows/` (wire guard into CI if needed)

## 7. Risks and mitigations

- Risk: docs update claims behavior that is not covered by tests.
  - Mitigation: require contract-test reference for each "guaranteed" statement.
- Risk: guardrails become noisy and are ignored.
  - Mitigation: keep checks narrow (warning coverage + key contract strings only).
- Risk: ongoing feature PRs re-introduce drift.
  - Mitigation: add "docs contract impact" checklist item in PR template/release checklist.

## 8. Acceptance criteria

- Core public docs are consistent with current CLI/runtime behavior.
- Every public warning type is documented in warning reference.
- Strict-mode semantics described in docs match contract tests.
- CI guard detects basic doc/contract drift.
- No known contradictions remain across README, support matrix, and reference docs.
