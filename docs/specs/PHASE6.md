# rtfkit Next Steps Plan (Phase 6: Consolidation and Release Hardening)

This phase consolidates capabilities from earlier phases and reduces operational risk before broader target expansion.

## 1. Primary objective

Stabilize quality, test coverage, CI reliability, and contributor workflow for list/table/image-capable conversion.

## 2. In scope

- Expand real-world fixture corpus (including noisy/legacy RTF samples).
- Increase regression coverage for parser limits, warnings, and strict-mode behavior.
- Add determinism checks where practical (same input -> same output).
- Tighten CI across Linux/macOS/Windows for fmt/clippy/test/release smoke.
- Align documentation and changelog with actual behavior.

## 3. Out of scope

- New major format capability (handled by follow-on HTML/PDF tracks).

## 4. Implementation direction

- Build a clear fixture taxonomy (lists, tables, images, malformed input, mixed docs).
- Add contract tests for:
  - exit code behavior
  - strict-mode invariants
  - warning semantics under warning caps
- Review release pipeline outputs and binary naming.
- Ensure contributor guidance stays "fixture-first" for feature additions.

## 5. Acceptance criteria

- CI is consistently green on supported matrix.
- Conversion behavior is deterministic and contract-tested for key paths.
- Docs/specs/architecture/changelog are synchronized.

