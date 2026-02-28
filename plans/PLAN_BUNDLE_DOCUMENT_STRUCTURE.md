# rtfkit Plan Bundle: Headers and Footers + Footnotes and Endnotes

**Status: DRAFT (implementation-ready)**

This is the canonical bundle plan for:
- `plans/PLAN_HEADERS_FOOTERS.md`
- `plans/PLAN_FOOTNOTES_ENDNOTES.md`

## 1. Objective

Add first-class document-structure support for page decorators (headers/footers) and note systems (footnotes/endnotes) across parsing, IR, and output writers.

Target flow:

```text
RTF structure destinations (header/footer/footnote/endnote)
  -> structure-aware IR
  -> DOCX / HTML / Typst mapping
```

Target outcomes:
- Header and footer content is retained and rendered.
- Footnote/endnote references and note bodies are preserved.
- Strict-mode behavior reflects true loss only.
- Writer mappings are deterministic and testable.

## 2. Baseline and Gaps

Current baseline:
- `header*`, `footer*`, and `footnote` destinations are dropped in destination handling.
- Document IR only contains body blocks.
- Writers render body-only output (no page decorators, no notes).

Key gaps:
- No IR container for page decorators.
- No note model (reference + body).
- No parser context switch for non-body block sinks.
- No writer plumbing for DOCX header/footer parts and notes.

## 3. Scope

### 3.1 In scope

1. Header/footer destination capture
- Support common destinations:
  - `header`, `headerl`, `headerr`, `headerf`
  - `footer`, `footerl`, `footerr`, `footerf`
- Parse and retain their block content.

2. Footnote/endnote capture
- Support footnote and endnote destinations used in real-world input.
- Insert inline note references at source location.
- Retain note body blocks in IR.

3. Cross-writer mapping
- DOCX: section header/footer and note references/bodies.
- HTML: structural wrappers + linked notes.
- Typst/PDF: page header/footer best-effort + note mapping.

4. Degradation contract
- Distinguish representational approximation (non-strict) from true dropped content.

5. Tests/fixtures/docs updates.

### 3.2 Out of scope

- Full multi-section layout parity with independent header/footer sets per section break.
- Advanced note-numbering customizations (restart scope, custom symbols, mixed sequences).
- Annotation/comment systems.
- Full fidelity for all producer-specific destination variants in one pass.

## 4. IR Design

## 4.1 Document structure extensions

Extend `Document` with optional structure payload:

```rust
pub struct Document {
    pub blocks: Vec<Block>,
    pub structure: Option<DocumentStructure>,
}

pub struct DocumentStructure {
    pub headers: HeaderFooterSet,
    pub footers: HeaderFooterSet,
    pub notes: Vec<Note>,
}

pub struct HeaderFooterSet {
    pub default: Vec<Block>,
    pub first: Vec<Block>,
    pub even: Vec<Block>,
}
```

(Exact field names can vary, but keep this shape: default/first/even channels and block-level content.)

## 4.2 Note model

```rust
pub enum NoteKind {
    Footnote,
    Endnote,
}

pub struct Note {
    pub id: u32,
    pub kind: NoteKind,
    pub blocks: Vec<Block>,
}

pub struct NoteRef {
    pub id: u32,
    pub kind: NoteKind,
}

pub enum Inline {
    Run(Run),
    Hyperlink(Hyperlink),
    NoteRef(NoteRef),
}
```

Rationale:
- Keeps reference position inline in body text.
- Keeps note body reusable for all output formats.

## 5. Parser and Runtime Architecture

## 5.1 Destination handling

In `crates/rtfkit-core/src/rtf/handlers_destinations.rs`:
- Stop dropping supported structure destinations.
- Introduce explicit destination behaviors for header/footer/note capture.

## 5.2 Structure capture context

Introduce structure parsing sub-state, e.g. `state_structure.rs`:
- Active sink target (`Body`, `Header(kind)`, `Footer(kind)`, `Note(id, kind)`).
- Temporary block buffers for active destination.
- Deterministic finalization on group end.

## 5.3 Note reference insertion

When a note destination is encountered:
1. Flush pending run text.
2. Allocate deterministic note id.
3. Insert `Inline::NoteRef` at current paragraph position.
4. Switch block sink to note body until destination closes.

## 5.4 Finalization and ordering

- Ensure pending paragraphs are finalized before sink transitions.
- Preserve original order and avoid body/note interleaving bugs.
- Keep group depth and destination semantics compatible with current pipeline.

## 6. Writer Mapping

## 6.1 DOCX (`crates/rtfkit-docx`)

Headers/Footers:
- Emit header/footer parts for available channels (default/first/even).
- Attach section properties deterministically.

Notes:
- Emit note references in document runs.
- Emit note bodies in note parts with matching ids.

## 6.2 HTML (`crates/rtfkit-html`)

Headers/Footers:
- Render within semantic wrappers:
  - `<header class="rtf-header ...">`
  - `<footer class="rtf-footer ...">`

Notes:
- `Inline::NoteRef` -> linked superscript reference.
- Append footnote/endnote sections after body in deterministic order.

## 6.3 Typst/PDF (`crates/rtfkit-render-typst`)

Headers/Footers:
- Map to page header/footer best-effort in document preamble.

Notes:
- Footnotes map to native Typst footnotes where possible.
- Endnotes map to endnote section fallback when no direct native parity exists.
- Emit `PartialSupport` warnings for representational degradation.

## 7. Strict-mode and Warning Contract

Policy:
- Parser `DroppedContent` only when destination content cannot be retained.
- Approximate mappings in writers use non-dropped warnings (`PartialSupport` in Typst, writer-level non-strict warnings in HTML when needed).
- Strict mode should fail only for real content loss.

## 8. Delivery Slices

### Slice A: IR + parser infrastructure
- Add structure/note IR.
- Add destination behaviors and structure capture state.
- Capture headers/footers and notes in core parse output.

### Slice B: HTML + Typst mapping
- Render headers/footers in HTML wrappers.
- Render note references and note sections in HTML.
- Add Typst best-effort header/footer/note mapping with warnings.

### Slice C: DOCX mapping
- Add DOCX header/footer part generation.
- Add DOCX note references and note body generation.
- Harden deterministic id and part ordering.

### Slice D: Contract hardening
- Strict-mode tests and warning reason stabilization.
- Realworld fixture additions and regression coverage.

## 9. File Touch Plan

Core:
- `crates/rtfkit-core/src/lib.rs`
- `crates/rtfkit-core/src/rtf/state.rs`
- `crates/rtfkit-core/src/rtf/handlers_destinations.rs`
- `crates/rtfkit-core/src/rtf/pipeline.rs`
- `crates/rtfkit-core/src/rtf/finalize/mod.rs`
- new structure state/handler/finalize modules under `crates/rtfkit-core/src/rtf/`
- tests under `crates/rtfkit-core/src/rtf/tests/`

DOCX:
- `crates/rtfkit-docx/src/writer.rs`

HTML:
- `crates/rtfkit-html/src/writer.rs`
- `crates/rtfkit-html/src/blocks/paragraph.rs`
- add note/header/footer block helpers under `crates/rtfkit-html/src/blocks/`

Typst:
- `crates/rtfkit-render-typst/src/map/mod.rs`
- `crates/rtfkit-render-typst/src/map/paragraph.rs`
- optional new note/page-decorator map modules

CLI tests:
- `crates/rtfkit-cli/tests/cli_contract_tests.rs`
- `crates/rtfkit-cli/tests/realworld_tests.rs`
- `crates/rtfkit-cli/tests/golden_tests.rs`

## 10. Test Matrix

Fixtures to add:
- `structure_header_footer_default.rtf`
- `structure_header_footer_first_even.rtf`
- `notes_footnote_simple.rtf`
- `notes_footnote_multiple.rtf`
- `notes_endnote_simple.rtf`
- `notes_mixed_footnote_endnote.rtf`
- `structure_notes_combined.rtf`

Assertions:
- IR snapshot contains structure payload and note refs.
- Body text retains note reference order.
- DOCX contains expected header/footer and note reference/body XML markers.
- HTML contains semantic wrappers and working note links.
- Typst output includes expected mapping or explicit partial-support warnings.
- Strict mode behavior matches dropped-content contract.

## 11. Risks and Mitigations

1. Complexity in destination context switching
- Mitigation: explicit sink state machine with exhaustive tests for enter/exit transitions.

2. Writer capability mismatch (especially Typst endnotes)
- Mitigation: define fallback policy up front and enforce warning semantics.

3. DOCX part relationship ordering bugs
- Mitigation: deterministic allocators and snapshot tests for XML parts.

4. IR migration churn
- Mitigation: introduce optional structure fields first, then incrementally wire writers.

## 12. Acceptance Criteria

1. Header/footer destination content is no longer dropped by parser.
2. Footnote/endnote references and bodies are preserved in IR.
3. HTML and DOCX render structure and notes with deterministic output.
4. Typst renders best-effort equivalents with explicit partial-support warnings where needed.
5. Strict mode fails only for true dropped content.
6. Added fixtures and contract tests pass in CI.
