# rtfkit HTML Output Track (Implementation Blueprint)

Status: planned follow-on track after Phase 6.

This document is intentionally implementation-oriented. It is meant to be handed directly to an engineer and executed with minimal ambiguity.

## 1. Objective

Deliver a production-grade `RTF -> IR -> HTML` path that is:

1. Deterministic for snapshot testing and reproducible builds.
2. Semantic-first rather than pixel-perfect.
3. Easy to extend without large rewrites.
4. Contract-consistent with existing CLI behavior, warnings, strict mode, and exit codes.

## 2. Non-goals

1. Word-level visual fidelity.
2. CSS layout engine or template framework integration.
3. Browser runtime features (JS, dynamic rendering).
4. PDF generation from HTML in this phase.
5. New parser capabilities that are not required for HTML emission.

## 3. Constraints and Existing Contracts

1. Parser/interpreter remains in `rtfkit-core`; HTML is a writer concern.
2. Existing exit code contract must hold:
   1. `0` success
   2. `2` parse/validation failure
   3. `3` writer/IO failure
   4. `4` strict-mode violation
3. Strict mode semantics remain based on `DroppedContent` warnings.
4. Output determinism is a hard requirement.
5. Safety limits remain enforced before writing (already handled by parser/interpreter).

## 4. Architecture and Component Boundaries

### 4.1 Proposed Workspace Change

Add a new crate:

- `crates/rtfkit-html/` - HTML writer implementation

Reasoning:

1. Keeps output-specific logic isolated from CLI orchestration.
2. Mirrors existing separation used by `rtfkit-docx`.
3. Reduces coupling and keeps feature growth localized.

### 4.2 Responsibilities by Crate

1. `rtfkit-core`
   1. Parse and interpret RTF into IR + report.
   2. No HTML concerns.
2. `rtfkit-html` (new)
   1. Convert IR `Document` to deterministic HTML string/bytes.
   2. Apply semantic mapping rules.
   3. Perform HTML escaping and normalization.
3. `rtfkit-cli`
   1. Parse new target option.
   2. Route IR to DOCX or HTML writer.
   3. Handle IO and preserve CLI/report contracts.

### 4.3 Internal Module Layout (`rtfkit-html`)

Keep modules small and cohesive:

1. `lib.rs`
   1. Public API only.
2. `writer.rs`
   1. Top-level orchestration from `Document` to HTML.
3. `serialize.rs`
   1. Deterministic string emission.
4. `escape.rs`
   1. HTML escaping (`&`, `<`, `>`, `"`, `'`).
5. `blocks/paragraph.rs`
6. `blocks/list.rs`
7. `blocks/table.rs`
8. `style.rs`
   1. Class/style mapping helpers.
9. `options.rs`
   1. Writer configuration.
10. `error.rs`
    1. Writer-level error type mapped by CLI to exit code `3`.

No heavy DOM framework. Use a small internal representation if needed.

## 5. Public API Contract (Writer)

### 5.1 Minimal API

```rust
pub struct HtmlWriterOptions {
    pub emit_document_wrapper: bool, // true => <!doctype html><html>...
    pub include_default_css: bool,   // true => include stable minimal stylesheet
}

impl Default for HtmlWriterOptions { ... }

pub fn document_to_html(
    doc: &rtfkit_core::Document,
    options: &HtmlWriterOptions,
) -> Result<String, HtmlWriterError>;
```

Design notes:

1. Return `String` for deterministic bytes and simple CLI IO.
2. Keep options minimal in this phase.
3. Avoid exposing internal node types publicly.

## 6. CLI Contract Changes

### 6.1 Target Selection

Add:

- `--to <docx|html>`

Behavior:

1. Default target remains `docx` when output writing is requested.
2. `--to html` selects HTML writer.
3. `--format <text|json>` continues to control report output only.
4. `--emit-ir` remains unchanged and orthogonal.

### 6.2 Output Rules

Recommended rule set for consistency and low surprise:

1. `--to html` requires `-o/--output <FILE>` in this phase.
2. If target is `html` and output path exists without `--force`, return writer/IO failure (`3`), consistent with DOCX flow.
3. Keep stdout for reports, not for HTML payload, to avoid mixed output streams.

### 6.3 File Extension Policy

1. Do not hard-require `.html`.
2. If extension is non-HTML, optionally warn via stderr in verbose mode only.

## 7. HTML Mapping Specification

### 7.1 Document Wrapper

When `emit_document_wrapper = true`, emit:

1. `<!doctype html>`
2. `<html lang="en">`
3. `<head>`
   1. `<meta charset="utf-8">`
   2. optional `<style>` for minimal defaults
4. `<body>`
   1. rendered content blocks

### 7.2 Paragraphs and Runs

| IR | HTML |
|---|---|
| `Paragraph` | `<p>` |
| `alignment = left/center/right/justify` | class on `<p>` (e.g. `rtf-align-center`) |
| `Run` plain | text node |
| `bold` | `<strong>` |
| `italic` | `<em>` |
| `underline` | `<span class="rtf-u">` |
| hard line break | `<br>` |

Rules:

1. Nest inline tags in stable order: `strong -> em -> span.rtf-u`.
2. Merge adjacent compatible runs to reduce noisy output.
3. Escape all text content.

### 7.3 Lists

| IR | HTML |
|---|---|
| `ListKind::Bullet` | `<ul>` |
| `ListKind::OrderedDecimal` | `<ol>` |
| `ListKind::Mixed` | choose stable fallback (default `<ul>`) + class `rtf-list-mixed` |
| `ListItem` | `<li>` |

Rules:

1. Preserve nesting via nested lists under `li`.
2. Keep item block order as in IR.
3. If an item has multiple blocks, emit them all inside `li` in order.

### 7.4 Tables

| IR | HTML |
|---|---|
| `TableBlock` | `<table class="rtf-table">` |
| row | `<tr>` |
| cell | `<td>` |
| `width_twips` | inline style `width: Npt` or class strategy (choose one and keep stable) |
| horizontal merge | `colspan` |
| vertical merge | `rowspan` |
| `CellVerticalAlign` | class `rtf-valign-top|middle|bottom` |

Rules:

1. Emit valid table grid after merge normalization from IR.
2. Do not emit placeholder continuation cells where HTML spans already encode coverage.
3. Preserve cell block order.

### 7.5 Unsupported/Degraded Features

1. If content already dropped at IR stage, strict-mode behavior remains unchanged.
2. If HTML writer cannot represent an IR feature that is semantically meaningful, emit `DroppedContent` warning with stable reason string and fail in strict mode.
3. Cosmetic-only differences should not emit `DroppedContent`.

### 7.6 Images

Current IR has no first-class image blocks. Phase HTML behavior:

1. No image emission in initial release.
2. Do not introduce ad hoc parser-side image handling here.
3. Add extension point in writer for future `Block::ImageBlock`.

## 8. Determinism Requirements

1. Stable tag ordering and attribute ordering.
2. Stable class ordering.
3. Stable whitespace policy.
4. Stable escaping strategy.
5. No timestamp metadata.
6. Same input IR must produce byte-identical HTML.

Suggested serializer conventions:

1. Use lowercase tags and attributes.
2. Use double quotes for attribute values.
3. Use `\n` line endings.
4. Emit a trailing newline exactly once.

## 9. Security Requirements

1. Escape all text nodes.
2. Escape all attribute values.
3. Never pass through raw RTF text as unescaped HTML.
4. No script/style injection from input content.
5. No remote fetches or URL dereferencing in writer.
6. Validate output path handling exactly as existing DOCX writer flow.

## 10. Implementation Plan (Workstreams)

### 10.1 Workstream A: Crate and Interfaces

1. Create `rtfkit-html` crate.
2. Add public API and options.
3. Add unit tests for empty/simple document emission.

Definition of done:

1. Crate compiles independently.
2. Public API documented and minimal.

### 10.2 Workstream B: Paragraph and Inline Formatting

1. Implement paragraph and run emission.
2. Implement escaping.
3. Implement alignment classes.

Definition of done:

1. Text fixtures render deterministically.
2. Snapshot tests pass for `text_*` fixtures.

### 10.3 Workstream C: Lists

1. Implement list and nested list emission.
2. Define stable fallback for mixed list kinds.

Definition of done:

1. List fixtures pass structural assertions.
2. No ordering regressions in determinism tests.

### 10.4 Workstream D: Tables

1. Implement table structure emission.
2. Implement merge-to-span mapping.
3. Implement cell alignment and width mapping policy.

Definition of done:

1. Table fixtures render valid HTML.
2. Malformed/degraded fixtures preserve report behavior.

### 10.5 Workstream E: CLI Integration

1. Add `--to` target selection in CLI.
2. Route HTML writes through common output path checks.
3. Preserve report and strict-mode behavior.

Definition of done:

1. CLI contract tests cover target selection and failures.
2. Exit code mapping unchanged.

### 10.6 Workstream F: Docs and Release Readiness

1. Update README usage examples.
2. Update feature matrix and architecture docs.
3. Add release checklist entries for HTML target.

Definition of done:

1. Documentation matches actual behavior.
2. CI includes HTML tests.

## 11. Test Strategy

### 11.1 Unit Tests (`rtfkit-html`)

1. Escaping edge cases.
2. Run nesting order.
3. List nesting output.
4. Table merge/span conversion.
5. Deterministic serializer behavior.

### 11.2 Snapshot Tests

Add new snapshot suite in CLI tests:

1. Source fixtures from existing `fixtures/`.
2. Output snapshots in `golden_html/` (new directory).
3. Include representative categories:
   1. `text_*`
   2. `list_*`
   3. `table_*`
   4. `mixed_*`
   5. selected `malformed_*`

### 11.3 Contract Tests

1. `--to html` success path.
2. `--to html` with existing output and no `--force` returns `3`.
3. Strict mode behavior unchanged.
4. Invalid CLI argument combinations fail clearly.

### 11.4 Determinism Tests

1. Run same conversion 3x and compare bytes.
2. Ensure no nondeterministic fields.
3. Validate stable attribute ordering.

### 11.5 Safety Tests

1. Reuse malformed fixture corpus.
2. Verify no panic/hang in HTML path.
3. Verify limit failures still fail before writing output.

### 11.6 Structural Validation

1. Parse generated HTML with an HTML parser in tests to confirm well-formed structure.
2. Assert required container presence for complex fixtures.

## 12. Migration and Backward Compatibility

1. DOCX remains default path for existing commands.
2. New behavior only activated when `--to html` is used.
3. Existing `--format`, `--emit-ir`, and `--strict` behavior remains stable.

## 13. Risks and Mitigations

1. Risk: drift between DOCX and HTML semantics.
   1. Mitigation: maintain shared mapping notes against IR types and add parity tests.
2. Risk: HTML serializer nondeterminism.
   1. Mitigation: centralized serializer with explicit ordering policy.
3. Risk: feature creep into a mini rendering engine.
   1. Mitigation: strict non-goals and milestone gating.
4. Risk: tight coupling to CLI.
   1. Mitigation: keep writer crate interface pure and file-IO free.

## 14. Detailed Acceptance Criteria

Phase is complete when all are true:

1. `rtfkit convert <input> --to html -o out.html` works for representative fixture corpus.
2. HTML output is deterministic (byte-identical across runs).
3. Output is valid HTML5 with correct escaping.
4. Unsupported cases degrade predictably with stable warnings.
5. Strict mode and exit codes remain contract-consistent.
6. CI includes HTML unit, contract, and determinism coverage.
7. Docs are updated and consistent with behavior.

## 15. Follow-on Extension Points

Not part of initial delivery, but design must leave room for:

1. `ImageBlock` support once IR adds first-class images.
2. Hyperlink/field mapping if IR adds explicit link nodes.
3. Optional output profile presets:
   1. semantic-minimal
   2. email-friendly
   3. print-friendly

Keep these as additive options. Do not redesign core writer APIs when adding them.
