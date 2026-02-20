# rtfkit Follow-on Plan (PDF Output Track)

This is an optional later track after Phase 6 and/or HTML maturity.

## 1. Primary objective

Deliver PDF generation without LibreOffice/Word by introducing a rendering backend.

## 2. Scope

- Choose and document backend (initial recommendation: Typst).
- Pipeline candidate:
  - `RTF -> IR -> backend intermediate -> PDF`
- Add debug option (for example `--keep-intermediate`) to aid issue triage.

## 3. Design principles

- Prioritize robustness and reproducibility over exact Word layout fidelity.
- Keep strict-mode/error/report contracts consistent with existing targets.
- Clearly document font/rendering variability by environment.

## 4. Testing

- Generate PDFs from representative fixture subset.
- Smoke-test output validity and non-crash guarantees in CI.
- Add baseline visual/manual checks for obvious regressions.

## 5. Acceptance criteria

- PDF generation works reliably for core fixtures.
- No crashes on malformed or unsupported inputs.
- Limits, warnings, and exit codes remain contract-consistent.

