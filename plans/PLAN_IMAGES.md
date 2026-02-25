# rtfkit Plan: Embedded Images (`\\pict`) - Unified Plan

**Status: PLANNED (Canonical)**

This file is the single source of truth for image support. It supersedes the previous draft reminder plan.

## 1. Executive Summary

Implement first-class embedded image support for RTF `\\pict` groups across the full pipeline:

`RTF -> parser/rtf state machine -> IR -> DOCX/HTML/PDF writers`

Current behavior drops image content with `DroppedContent` warnings. This is the largest remaining fidelity gap for real-world documents.

Primary deliverable:
- Parse supported `\\pict` payloads (PNG/JPEG) into IR `Block::Image`.
- Render those images in DOCX, HTML, and PDF outputs.
- Preserve strict-mode and determinism guarantees.

## 2. Product Goals and Non-Goals

### 2.1 Goals

1. Support embedded PNG and JPEG images from `\\pict`.
2. Preserve image display size when available (`\\picwgoal`, `\\pichgoal`, `\\picscalex`, `\\picscaley`).
3. Keep behavior deterministic and testable end-to-end.
4. Avoid large monolithic logic; implement in cohesive parser/writer components.
5. Maintain backward compatibility for existing non-image behavior.

### 2.2 Non-Goals (for this phase)

1. WMF/EMF vector conversion.
2. OLE object extraction (`\\object`, `\\objdata`).
3. Floating/anchored positioning and text-wrap fidelity.
4. Image cropping (`\\piccropl`, `\\piccropr`, `\\piccropt`, `\\piccropb`).
5. Externalized asset pipeline (file extraction mode) for HTML/PDF.

## 3. Scope and Contracts

### 3.1 In Scope

1. `\\pict` destination parsing.
2. Format controls: `\\pngblip`, `\\jpegblip`.
3. Size controls: `\\picw`, `\\pich`, `\\picwgoal`, `\\pichgoal`, `\\picscalex`, `\\picscaley`.
4. Shape picture handling: `\\shppict` preferred over `\\nonshppict` fallback.
5. IR model extension with an image block type.
6. Writer support:
- DOCX: embedded media + DrawingML reference.
- HTML: `img` data URI.
- PDF/Typst: embedded image rendering path.
7. Parser resource limit for cumulative image bytes.

### 3.2 Output Contract

1. Supported images produce real image output blocks, not dropped-content warnings.
2. Unsupported image formats are explicitly degraded with `DroppedContent` reason.
3. Strict mode fails when degradation occurs.
4. Stable warning strings and exit-code semantics remain intact.

## 4. Architecture and Design Decisions

### 4.1 IR Model

Add image domain types in `crates/rtfkit-core/src/lib.rs`:

```rust
pub enum ImageFormat {
    Png,
    Jpeg,
}

pub struct ImageBlock {
    pub format: ImageFormat,
    pub data: Vec<u8>,
    pub width_twips: Option<i32>,
    pub height_twips: Option<i32>,
}

pub enum Block {
    Paragraph(Paragraph),
    ListBlock(ListBlock),
    TableBlock(TableBlock),
    ImageBlock(ImageBlock),
}
```

Decision: image is block-level in this phase. This matches current block pipeline and minimizes cross-cutting complexity.

### 4.2 Parser State Design

Implement a dedicated image parsing sub-state within `rtf` runtime state.

Proposed internal state (in `rtf/state_resources.rs` or a new `rtf/state_images.rs` if it stays cohesive):

1. `parsing_pict: bool`
2. `pict_group_depth: usize`
3. `pict_format: Option<ImageFormat>`
4. `pict_hex_buffer: String`
5. `picw/pich/picwgoal/pichgoal/picscalex/picscaley`
6. `prefer_shppict: bool` + tracking for `\\nonshppict`
7. `total_image_bytes: usize` (for limits)

Decision: keep image decoding logic isolated behind helper functions; pipeline remains orchestration only.

### 4.3 Destination Handling

Update `rtf/handlers_destinations.rs` behavior classification:

1. `\\pict` no longer maps to unconditional dropped destination.
2. Enter image parsing mode when encountering `\\pict` destination.
3. `\\shppict` group content is eligible for image parsing.
4. `\\nonshppict` group is skipped when preferred content exists.

### 4.4 Hex Decode Strategy

Create a small utility helper in parser layer:

`decode_pict_hex(input: &str) -> Result<Vec<u8>, PictDecodeError>`

Rules:
1. Ignore ASCII whitespace.
2. Decode hex pairs.
3. Odd-length hex: degrade with warning and drop image.
4. Invalid hex chars: degrade with warning and drop image.

Decision: no streaming decoder for v1; linear decode into `Vec<u8>` is acceptable with enforced byte limit.

### 4.5 Dimension Resolution

Define one deterministic resolver:

`resolve_image_dimensions(...) -> (Option<i32>, Option<i32>) // twips`

Priority:
1. `picwgoal/pichgoal` when present.
2. Fallback `picw/pich`.
3. Apply scale (`picscalex`, `picscaley`, default 100).
4. Non-positive dimensions become `None` (degrade to intrinsic size behavior in writers).

### 4.6 Limits and Failure Policy

Extend `ParserLimits` with:

`max_image_bytes_total: usize` (default: 50 MiB)

Behavior:
1. If cumulative decoded image bytes exceed limit: hard parse failure (exit code 2).
2. Unsupported format/invalid hex: warning + dropped content (strict mode fails).

## 5. Writer Implementation Plan

### 5.1 DOCX (`crates/rtfkit-docx`)

1. Add `Block::ImageBlock` mapping in writer dispatch.
2. Store media as `word/media/imageN.(png|jpg)`.
3. Register relationships in `word/_rels/document.xml.rels`.
4. Emit drawing run with deterministic IDs/order.
5. Twips -> EMU conversion for extents.

Determinism rules:
1. Media file naming by encounter order.
2. Relationship IDs allocated in stable order.

### 5.2 HTML (`crates/rtfkit-html`)

1. Add block mapper for image blocks.
2. Emit:

```html
<figure class="rtf-image"><img src="data:image/...;base64,..." ...></figure>
```

3. Width/height emitted when resolved; omitted otherwise.
4. Stable attribute ordering.

### 5.3 PDF/Typst (`crates/rtfkit-render-typst`)

1. Add image block mapping in typst mapper.
2. Use deterministic embedded bytes path supported by current renderer.
3. Apply optional size mapping when twips dimensions available.

Decision: keep implementation aligned with existing in-process renderer constraints; no external binary dependency.

## 6. CLI, Reports, and Strict Mode

### 6.1 CLI

No new CLI flags required for v1.

### 6.2 Warnings / reasons

Stabilize and document new/updated dropped-content reasons:

1. `Dropped unsupported image format`
2. `Dropped malformed image hex payload`
3. `Dropped image with invalid dimensions` (only if needed)

These reasons must be added to docs and tested as stable contracts.

### 6.3 Strict mode

1. Supported PNG/JPEG path should pass strict mode.
2. Unsupported/malformed image path should fail strict mode via dropped-content.

## 7. Detailed Implementation Sequence

1. Add IR types and serde support.
2. Add parser limit field and defaults.
3. Add image parsing state + helpers.
4. Add destination behavior and event wiring for `\\pict` groups.
5. Add hex decode + dimension resolver utilities with unit tests.
6. Emit `Block::ImageBlock` from parser finalization path.
7. Update all writers with `Block::ImageBlock` handling.
8. Add fixtures and parser tests.
9. Add writer integration tests (docx/html/pdf).
10. Add CLI contract tests and strict-mode tests.
11. Update docs (`rtf-feature-overview`, warning reference, limits policy).

## 8. Test Plan (Decision Complete)

### 8.1 New fixtures

1. `image_png_simple.rtf`
2. `image_jpeg_simple.rtf`
3. `image_with_dimensions.rtf`
4. `image_multiple.rtf`
5. `image_in_table.rtf`
6. `image_unsupported_format.rtf`
7. `image_malformed_hex.rtf`
8. `image_limit_exceeded.rtf`
9. `image_shppict_nonshppict.rtf`

### 8.2 Core tests

1. Hex decode unit tests:
- valid payload
- whitespace-tolerant payload
- odd-length payload
- invalid char payload

2. Parser image tests:
- PNG/JPEG recognition
- dimension precedence and scaling
- shppict preference behavior
- unsupported format degradation
- cumulative byte limit failure

3. Writer tests:
- DOCX contains media entries + relationships + drawing nodes
- HTML emits `img` with correct MIME/base64 and dimensions
- PDF mapper includes image node with deterministic output

4. Contract tests:
- strict success on valid images
- strict failure on unsupported/malformed images
- determinism snapshots for all formats

### 8.3 Regression protection

Run required matrix:

1. `cargo test -p rtfkit-core`
2. `RUSTFLAGS='-D warnings' cargo check --workspace --all-targets`
3. `cargo test --workspace`

## 9. Acceptance Criteria

1. PNG/JPEG images from `\\pict` are rendered in DOCX/HTML/PDF.
2. Valid image fixtures pass strict mode.
3. Unsupported/malformed image fixtures fail strict mode with stable dropped-content reasons.
4. Image byte limits enforce parse failure deterministically.
5. Determinism tests pass across IR/docx/html/pdf.
6. No regressions in existing non-image tests.

## 10. Risks and Mitigations

1. `docx-rs` image API constraints:
- Mitigation: adapt using minimal raw XML fallback only where necessary.

2. Memory pressure from large payloads:
- Mitigation: cumulative byte limit + reuse buffers + fail fast.

3. Snapshot bloat:
- Mitigation: use tiny deterministic fixture images; prefer focused assertions for large payloads.

4. Cross-format size discrepancies:
- Mitigation: standardize twips-based source of truth and format-specific conversions with tests.

## 11. Explicit Defaults and Assumptions

1. Supported formats in v1: PNG and JPEG only.
2. Images are block-level IR elements in v1.
3. No external image extraction mode in v1 (inline/embed only).
4. Keep architecture pragmatic: small handlers, strong cohesion, no new framework layers.

## 12. Done Definition

1. This plan file is the only image plan file.
2. All implementation steps above completed.
3. All acceptance criteria and required test matrix pass.
4. Documentation updated to reflect supported/unsupported image behavior.
