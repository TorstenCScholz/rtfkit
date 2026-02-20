# rtfkit Follow-on Plan (HTML Output Track)

This is an optional follow-on track after Phase 6 stabilization.

## 1. Primary objective

Add `RTF -> IR -> HTML` conversion for a diff-friendly, web-native target and to support potential downstream rendering workflows.

## 2. Scope

- New CLI target: `--to html`
- Writer mapping for:
  - paragraphs and inline styles
  - lists
  - tables
  - images (where supported by IR)
- Deterministic HTML output suitable for snapshots.

## 3. Design principles

- Prefer semantic HTML over pixel-fidelity.
- Keep fallback and warning model consistent with DOCX target.
- Reuse shared IR contract and strict-mode semantics.

## 4. Testing

- HTML fixture snapshots with structural assertions.
- CLI integration tests for target selection, output behavior, and strict mode.

## 5. Acceptance criteria

- `rtfkit convert --to html` produces valid HTML5 for supported fixtures.
- Unsupported features degrade predictably with warnings.

