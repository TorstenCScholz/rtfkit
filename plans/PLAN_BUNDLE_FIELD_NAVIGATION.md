# rtfkit Plan Bundle: Field Expansion + Internal Links and Bookmarks

**Status: DRAFT (implementation-ready)**

This is the canonical bundle plan for:
- `plans/PLAN_FIELD_SUPPORT_EXPANSION.md`
- `plans/PLAN_INTERNAL_LINKS_AND_BOOKMARKS.md`

## 1. Objective

Expand field support from "external hyperlinks only" to robust field/navigation handling, with first-class bookmark anchors and internal links across parser, IR, and all writers.

Target flow:

```text
RTF field + bookmark controls
  -> field/bookmark-aware IR
  -> DOCX / HTML / Typst mapping
```

Target outcomes:
- Internal links (`HYPERLINK \l "bookmark"`) work end-to-end.
- Bookmark anchors are preserved and addressable.
- External hyperlink parsing is more tolerant and deterministic.
- Unsupported fields degrade predictably without unnecessary strict-mode failures when visible text is preserved.

## 2. Baseline and Gaps

Current baseline:
- Hyperlinks are parsed in `crates/rtfkit-core/src/rtf/handlers_fields.rs`.
- Supported URL schemes are limited to `http://`, `https://`, and `mailto:`.
- Only quoted `HYPERLINK "url"` instruction form is accepted.
- Non-hyperlink fields degrade with `DroppedContent("Dropped unsupported field type")`.
- Bookmark destinations are not supported as first-class content.

Observed gaps:
- No internal navigation target model in IR.
- No parsing for bookmark start/end anchors.
- Field parser does not handle common switch forms (`\l`, `\o`, unquoted URL patterns).
- Strict mode can fail on fields whose visible result text is preserved, because degradation currently uses dropped-content warnings.

## 3. Scope

### 3.1 In scope

1. Bookmark anchors
- Parse bookmark start/end and retain stable bookmark anchors in IR.
- Preserve anchor positions in paragraph flow.

2. Internal hyperlinks
- Parse `HYPERLINK \l "bookmark"` and map to internal targets.
- Keep external hyperlink behavior fully backward compatible.

3. Hyperlink instruction parser hardening
- Support common instruction variants:
  - quoted URL
  - URL with field switches before/after target
  - escaped quotes and whitespace normalization
- Continue scheme allowlisting for external URLs.

4. Field degradation policy refinement
- If `fldrslt` text is preserved, degrade as non-strict warning (no dropped-content strict failure).
- Reserve dropped-content warnings for true semantic/text loss.

5. Writer support for bookmark anchors + internal links
- DOCX, HTML, Typst (best-effort where necessary).

6. Fixtures/tests/docs updates.

### 3.2 Out of scope

- Full parity for all RTF field types (`TOC`, `INDEX`, `INCLUDETEXT`, etc.).
- Live/dynamic field updates (Word-calculated behaviors) beyond stored results.
- Complex bookmark edge cases across section breaks and legacy producer bugs.

## 4. IR Design

## 4.1 Hyperlink target model

Replace a single `url: String` target with a typed target:

```rust
pub enum HyperlinkTarget {
    ExternalUrl(String),
    InternalBookmark(String),
}

pub struct Hyperlink {
    pub target: HyperlinkTarget,
    pub runs: Vec<Run>,
}
```

Rationale:
- Makes internal links explicit.
- Prevents scheme/anchor ambiguity in writers.

## 4.2 Bookmark anchor inline

Add zero-width inline anchor:

```rust
pub struct BookmarkAnchor {
    pub name: String,
}

pub enum Inline {
    Run(Run),
    Hyperlink(Hyperlink),
    BookmarkAnchor(BookmarkAnchor),
}
```

Rationale:
- Bookmark placement is inline-position sensitive.
- Avoids block-level anchor approximation errors.

## 5. Parser and Interpreter Changes

## 5.1 Destination classification

In `crates/rtfkit-core/src/rtf/handlers_destinations.rs`:
- Add destination behaviors for bookmark groups (`bkmkstart`, `bkmkend`).
- Parse destination text payload instead of dropping as unknown destination.

## 5.2 Field instruction parser

In `crates/rtfkit-core/src/rtf/handlers_fields.rs`:
- Replace `extract_hyperlink_url` with a small instruction parser that returns:
  - field kind
  - target (external/internal)
  - optional switches retained for future extension
- Keep nested-field behavior deterministic (current model can remain).

## 5.3 Field finalization matrix

For each finalized field:
1. If recognized and mappable -> emit typed inline.
2. If unrecognized but `fldrslt` preserved -> emit result text + non-strict warning.
3. If result content missing or irrecoverable -> emit dropped-content warning.

## 5.4 Bookmark lifecycle

- Store open bookmark starts by name in parser state.
- Emit `Inline::BookmarkAnchor` at deterministic start position.
- Validate start/end pairing for diagnostics (warning-only if mismatched but recoverable).

## 6. Writer Mapping

## 6.1 DOCX (`crates/rtfkit-docx/src/writer.rs`)

- External links: existing behavior remains.
- Internal links: emit `<w:hyperlink w:anchor="...">`.
- Bookmark anchors: emit `<w:bookmarkStart>` / `<w:bookmarkEnd>` with deterministic ids.

## 6.2 HTML (`crates/rtfkit-html`)

- Anchor inline -> `<span id="..."></span>` (or `<a id="..."></a>`, choose one canonical form).
- Internal links -> `<a href="#anchor">...</a>`.
- External links unchanged.

## 6.3 Typst/PDF (`crates/rtfkit-render-typst`)

- External links unchanged.
- Internal links map to Typst label/ref best-effort.
- If an exact anchor/link mapping is not representable for a specific case, degrade with `PartialSupport` warning (not dropped content).

## 7. Strict-mode and Warning Contract

Policy:
- `DroppedContent` only when visible content or semantic target is lost.
- Unsupported-but-preserved field result text uses a non-strict warning path.
- Unknown external URL schemes still degrade safely; preserve visible text.

CLI impact:
- Fewer false-positive strict failures for documents that keep field result text.
- Strict mode still fails for true dropped content.

## 8. Delivery Slices

### Slice A: IR + parser support for bookmarks and typed hyperlink targets
- IR changes (`Inline`, `Hyperlink`, new bookmark type).
- Parser: bookmark destinations and internal hyperlink target parsing.
- Core tests for field parsing and bookmark placement.

### Slice B: Writer mapping for anchors/internal links
- DOCX internal anchors/links.
- HTML anchors/internal links.
- Typst best-effort mapping with explicit partial-support warnings.

### Slice C: Degradation policy hardening
- Refine warning categories and strict-mode behavior.
- Add contract tests for strict/non-strict behavior.
- Update fixture corpus and realworld checks.

## 9. File Touch Plan

Core:
- `crates/rtfkit-core/src/lib.rs`
- `crates/rtfkit-core/src/rtf/state_fields.rs`
- `crates/rtfkit-core/src/rtf/handlers_fields.rs`
- `crates/rtfkit-core/src/rtf/handlers_destinations.rs`
- `crates/rtfkit-core/src/rtf/handlers_text.rs`
- `crates/rtfkit-core/src/rtf/tests/fields.rs`
- `crates/rtfkit-core/src/rtf/tests/destinations.rs`

Writers:
- `crates/rtfkit-docx/src/writer.rs`
- `crates/rtfkit-html/src/blocks/paragraph.rs`
- `crates/rtfkit-render-typst/src/map/paragraph.rs`

CLI/contract tests:
- `crates/rtfkit-cli/tests/cli_contract_tests.rs`
- `crates/rtfkit-cli/tests/golden_tests.rs`
- fixtures + goldens under `fixtures/` and `golden*/`

## 10. Test Matrix

Fixtures to add:
- `field_internal_hyperlink_simple.rtf`
- `field_internal_hyperlink_formatted.rtf`
- `field_bookmark_anchor_only.rtf`
- `field_mixed_external_internal.rtf`
- `field_unsupported_type_preserve_result.rtf`
- `field_malformed_target_preserve_result.rtf`

Assertions:
- IR snapshots include typed targets + bookmark anchors.
- DOCX XML contains `w:anchor` and bookmark start/end tags.
- HTML contains stable `id` anchors and `href="#..."`.
- Typst source contains deterministic internal-link mapping or explicit partial-support warnings.
- Strict mode only fails when dropped-content warnings are present.

## 11. Risks and Mitigations

1. IR breakage risk
- Mitigation: migrate all writers/tests in one bundle PR series.

2. Bookmark naming collisions/invalid identifiers
- Mitigation: canonicalize and escape per backend with deterministic mapping table.

3. Writer capability mismatch
- Mitigation: backend-specific best-effort mapping + explicit warning policy.

4. Field grammar variability
- Mitigation: parser supports common canonical forms first, retains deterministic fallback to `fldrslt`.

## 12. Acceptance Criteria

1. Internal bookmark links round-trip through IR and render in DOCX and HTML.
2. Bookmark anchors are emitted at stable positions and are referenceable.
3. Field result text is preserved when instruction parsing fails.
4. Strict mode behavior matches the updated dropped-content policy.
5. Added fixtures/goldens/contracts pass in CI.
6. Determinism tests remain green across all targets.
