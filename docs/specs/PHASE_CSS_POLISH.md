# rtfkit Follow-on Plan (HTML CSS Polish Track)

Status: planned follow-on track after `PHASE_HTML.md`, before `PHASE_PDF.md`.

This phase improves visual quality of HTML output while preserving existing parser, warning, strict-mode, and determinism contracts.

## 1. Objective

Deliver a professional, readable default HTML presentation for report-like documents without introducing layout fragility or backend lock-in.

Primary outcomes:

1. Better visual quality for prose, tables, lists, links, and headings.
2. Print-friendly CSS suitable for PDF experiments later.
3. Stable, deterministic CSS emission and class mapping.
4. Optional user-provided CSS overrides with safe local-file boundaries.

## 2. Non-goals

1. Pixel-perfect Word fidelity.
2. Interactive web application styling.
3. JavaScript runtime features.
4. Full design-system/theming engine.
5. PDF rendering implementation in this phase.

## 3. Constraints and Contracts

1. `rtfkit-core` remains unchanged in responsibilities.
2. Exit code contract must remain unchanged:
   1. `0` success
   2. `2` parse/validation failure
   3. `3` writer/IO failure
   4. `4` strict-mode violation
3. Strict mode remains driven by dropped semantic content, not cosmetics.
4. Deterministic output remains mandatory.
5. No external network access for styles, fonts, or assets.

## 4. Visual Direction

Define one built-in default profile aimed at "modern technical report" readability:

1. Improved typography scale (titles, body, table text, code-like spans).
2. Predictable spacing rhythm for paragraphs/lists/tables.
3. Better table readability (header background, row striping, borders, cell padding).
4. Clear link styling and accessible color contrast.
5. Print-aware defaults via `@media print` and `@page`.

## 5. CSS Architecture

### 5.1 Tokenized Stylesheet

Use CSS custom properties for core tokens:

1. Font families (system-safe defaults only).
2. Text/background colors.
3. Border/shadow colors.
4. Spacing scale.
5. Table sizing constants.

Keep tokens in a single deterministic order to preserve stable snapshots.

### 5.2 Stable Class Surface

Do not churn existing semantic class names unless required.
Allowed additive classes include:

1. `rtf-doc`
2. `rtf-content`
3. `rtf-p`, `rtf-h*` (if heading mapping is introduced later)
4. `rtf-table`, `rtf-table-row`, `rtf-table-cell`
5. `rtf-link`
6. `rtf-print-*` utility classes (optional)

Principle: styles evolve; structural classes remain stable.

### 5.3 Print Rules (No PDF Backend Yet)

Add print CSS focused on graceful degradation:

1. `@page` margins.
2. Avoid splitting table rows where possible (`break-inside` hints).
3. Conservative widow/orphan handling.
4. Optional link URL postfix in print mode.

These rules are preparation for future PDF backend experiments, not final PDF fidelity.

## 6. Writer and CLI Surface

### 6.1 Writer Options Extension

Extend `HtmlWriterOptions` with explicit CSS mode:

1. `default` (embed built-in stylesheet, new polished default)
2. `none` (emit semantic HTML with no built-in CSS)
3. `external` (link mode; requires href/path handling policy)

Recommended implementation order:

1. Add `default` and `none` first.
2. Add `external` only if a concrete need appears.

### 6.2 CLI Flags (Proposed)

Add HTML-specific flags:

1. `--html-css <default|none>`
2. `--html-css-file <FILE>` (optional local override appended after built-in CSS)

Rules:

1. `--html-css-file` is only valid with `--to html`.
2. File must be local and readable; IO errors map to exit code `3`.
3. CSS file size should be capped (reuse existing limits pattern).
4. CSS content is treated as trusted operator input, not RTF-derived input.

## 7. Security and Safety

1. Continue escaping all RTF-derived text/attributes.
2. Never inject RTF text into `<style>` blocks.
3. Disallow remote CSS fetch behavior.
4. Preserve existing malformed-input robustness and limit enforcement.
5. Avoid introducing parser-side style interpretation from raw RTF control words.

## 8. Implementation Workstreams

### 8.1 Workstream A: Stylesheet v2

1. Introduce tokenized default CSS block in `rtfkit-html`.
2. Keep deterministic ordering and normalized newlines.
3. Preserve current semantic mappings.

Definition of done:

1. Existing fixtures render with improved baseline readability.
2. No behavior regressions in structural HTML tests.

### 8.2 Workstream B: Print Profile

1. Add minimal print styles and page settings.
2. Validate output with representative long fixtures (`fixtures/realworld/`).

Definition of done:

1. Printed output remains readable with no clipping of core content.
2. No strict-mode or warning-model behavior changes.

### 8.3 Workstream C: CSS Mode/Overrides

1. Extend writer options and CLI flags.
2. Add local CSS file ingestion path with IO validation.
3. Define override order: built-in first, user CSS second.

Definition of done:

1. `--html-css none` suppresses built-in CSS.
2. `--html-css-file` deterministically appends custom CSS.
3. Invalid files fail with exit code `3`.

### 8.4 Workstream D: Docs and Examples

1. Update README HTML examples with CSS flags.
2. Add `docs/reference/html-styling.md` with class surface and token docs.
3. Add guidance for preparing HTML for later PDF pipeline tests.

Definition of done:

1. Contributors can style output without touching parser logic.
2. Doc examples match actual CLI behavior.

## 9. Test Strategy

### 9.1 Unit Tests (`rtfkit-html`)

1. Default CSS serialization is deterministic.
2. CSS mode switching works (`default` vs `none`).
3. User CSS append behavior is deterministic.

### 9.2 Integration Tests (`rtfkit-cli`)

1. `--to html --html-css default` succeeds.
2. `--to html --html-css none` succeeds and emits no built-in stylesheet.
3. `--html-css-file` valid path succeeds.
4. `--html-css-file` missing/unreadable path returns exit code `3`.
5. Non-html targets reject HTML-only flags with clear CLI errors.

### 9.3 Snapshot and Contract Tests

1. Keep structural snapshots stable (semantic HTML).
2. Add targeted CSS snapshots for built-in stylesheet bytes.
3. Ensure realworld fixtures still convert deterministically across runs.

### 9.4 Manual Verification

For a fixed representative set (`annual_report_10p`, `technical_spec_12p`, `policy_doc_15p`):

1. Open generated HTML in at least one Chromium-based browser and one WebKit/Gecko browser.
2. Verify readability, table legibility, and link visibility.
3. Verify print preview is acceptable and does not truncate content.

## 10. Risks and Mitigations

1. Risk: CSS churn breaks snapshot stability.
   1. Mitigation: isolate stylesheet snapshots and gate changes in PR review.
2. Risk: style complexity grows into a second rendering system.
   1. Mitigation: enforce strict non-goals; prioritize readability over fidelity.
3. Risk: override flags complicate CLI UX.
   1. Mitigation: keep default behavior simple; reserve advanced flags for explicit use.
4. Risk: backend variance across browsers.
   1. Mitigation: document expected variability and keep CSS conservative.

## 11. Acceptance Criteria

Phase is complete when all are true:

1. HTML output looks materially better on representative fixtures with no parser changes.
2. Built-in CSS is deterministic and covered by tests.
3. `--html-css` mode selection behaves as documented.
4. Optional local CSS override is supported with clear IO failures.
5. Exit codes, strict mode, warnings, and security contracts remain intact.
6. Documentation is updated with styling behavior and examples.

## 12. Follow-on Relationship to PDF

After this phase:

1. Start `PHASE_PDF.md` backend selection/implementation.
2. Reuse print-oriented CSS and realworld fixtures for PDF quality baselines.
3. Keep PDF-specific layout logic in the chosen backend; do not overload HTML polish with PDF-only hacks.
