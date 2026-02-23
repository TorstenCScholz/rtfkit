# rtfkit Plan: Hyperlink Support

**Status: PLANNED**

Add first-class hyperlink support to the RTF-to-IR-to-output pipeline. Hyperlinks (`\field{\fldinst HYPERLINK "..."}{...}`) are among the most common RTF features in real-world documents and are currently dropped with a `DroppedContent` warning.

## 1. Primary objective

Convert RTF `HYPERLINK` fields into a first-class IR element and emit correct output across all three writers (DOCX, HTML, PDF).

```
RTF \field{\fldinst HYPERLINK "url"}{…\fldrslt visible text…}
  → IR Hyperlink { url, runs }
  → DOCX <w:hyperlink> / HTML <a href> / Typst #link()
```

## 2. Scope

### In scope

- Parse `\field` groups containing `\fldinst HYPERLINK "url"` instructions.
- Extract the URL from `\fldinst` and the visible text/runs from `\fldrslt`.
- New IR inline element `Hyperlink` that wraps child `Run`s and carries a URL.
- DOCX writer: emit `<w:hyperlink r:id="...">` with relationship entry.
- HTML writer: emit `<a href="...">` with appropriate CSS class.
- Typst/PDF writer: emit `#link("url")[content]`.
- Preserve formatting (bold, italic, underline) on runs inside hyperlinks.
- Strict-mode: hyperlinks no longer emit `DroppedContent` (they are supported).
- Non-HYPERLINK fields continue to emit `DroppedContent` as before.
- Fixtures: at least 4 new RTF fixtures (`hyperlink_simple`, `hyperlink_formatted`, `hyperlink_multiple`, `hyperlink_nested_in_table`).
- Golden IR snapshots for all new fixtures.
- Contract tests for strict mode no longer failing on hyperlink-only documents.

### Out of scope

- Non-HYPERLINK field types (e.g., `\field{\fldinst PAGE}`, `\field{\fldinst TOC}`).
- Bookmark-based internal links (`\field{\fldinst HYPERLINK \\l "bookmark"}`). Can be added later.
- Mail-to links or other URI schemes beyond `http://`, `https://`, `mailto:`.
- Field nesting (field inside another field).
- `\datafield` binary payloads.

## 3. Design constraints and principles

- Hyperlinks are **inline** elements, not blocks. They appear inside paragraphs alongside other runs.
- A hyperlink groups one or more `Run`s under a single URL — the IR must model this nesting.
- The interpreter currently skips `\field`, `\fldinst`, `\fldrslt` as dropped destinations. This must become selective: `HYPERLINK` fields are parsed, all others remain dropped.
- Preserve text as a hard baseline — if hyperlink URL parsing fails, fall back to emitting the `\fldrslt` text as plain runs with a warning (no silent text loss).
- Output must remain deterministic for identical input.
- Strict-mode contract: supported hyperlinks do NOT emit `DroppedContent`. Unsupported field types continue to emit `DroppedContent`.

## 4. IR changes

### New inline element

The `Run` type today is a flat text span. Hyperlinks need an inline container. Two options:

**Option A — Inline enum wrapper (recommended):**

```rust
/// An inline element within a paragraph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    /// A plain text run with formatting.
    Run(Run),
    /// A hyperlink wrapping one or more formatted runs.
    Hyperlink(Hyperlink),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hyperlink {
    /// Target URL
    pub url: String,
    /// Visible content (formatted runs)
    pub runs: Vec<Run>,
}
```

`Paragraph.runs: Vec<Run>` becomes `Paragraph.inlines: Vec<Inline>`.

**Migration path:** Since `Paragraph.runs` is used across all writers and tests, this is a cross-cutting change. Rename in one pass, update all call sites.

**Option B — Run variant with optional URL:**

```rust
pub struct Run {
    pub text: String,
    pub bold: bool,
    // ...existing fields...
    pub hyperlink_url: Option<String>,
}
```

Simpler but loses grouping semantics (multiple runs sharing one URL). Not recommended.

**Decision: Option A.** It models the semantics correctly and aligns with how DOCX (`<w:hyperlink>` wrapping `<w:r>`) and HTML (`<a>` wrapping `<span>`s) represent links.

## 5. Interpreter changes

### Field parsing state machine

Currently `\field`, `\fldinst`, `\fldrslt` are all in the `Dropped` destination list. Replace with a targeted field-parsing mode:

1. When `\field` is encountered as a destination, enter **field-parsing mode** instead of dropping.
2. Inside the `\field` group, track two sub-destinations:
   - `\fldinst` — capture the instruction text (look for `HYPERLINK "url"`).
   - `\fldrslt` — capture the visible runs with formatting (reuse normal run-building logic).
3. On `\field` group close:
   - If instruction matches `HYPERLINK "url"`, emit `Inline::Hyperlink { url, runs }`.
   - Otherwise, emit `\fldrslt` runs as plain `Inline::Run`s and emit `DroppedContent` for the unsupported field type.

### Regex for URL extraction

```
HYPERLINK\s+"([^"]*)"
```

Keep it simple. Quoted URL extraction only. Optional switches like `\l` (bookmark) or `\t` (target) are out of scope for this phase.

### Fallback behavior

- `\field` with no `\fldrslt`: emit nothing, emit `DroppedContent("Field with no result text")`.
- `\field` with `\fldrslt` but no `\fldinst`: emit runs as plain text, emit warning.
- `\field` with `\fldinst` that isn't HYPERLINK: emit `\fldrslt` as plain text, emit `DroppedContent`.

## 6. Writer changes

### DOCX writer (`rtfkit-docx`)

- Add relationship ID allocation for hyperlinks (similar to `NumberingAllocator`).
- For each `Inline::Hyperlink`, emit:
  ```xml
  <w:hyperlink r:id="rId{N}">
    <w:r>
      <w:rPr>...</w:rPr>
      <w:t>visible text</w:t>
    </w:r>
  </w:hyperlink>
  ```
- Add corresponding relationship entry in `word/_rels/document.xml.rels`:
  ```xml
  <Relationship Id="rId{N}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
  ```
- Check `docx-rs` crate API for hyperlink support — if insufficient, may need to extend or use raw XML.

### HTML writer (`rtfkit-html`)

- For `Inline::Hyperlink`, emit `<a href="url" class="rtf-link">` wrapping the child run spans.
- Sanitize URL: reject `javascript:` and `data:` schemes (security).
- Add `.rtf-link` class to CSS with styling from `ColorTokens.link_default` / `link_hover`.

### Typst/PDF writer (`rtfkit-render-typst`)

- For `Inline::Hyperlink`, emit `#link("url")[content]`.
- Apply `ColorTokens.link_default` as text fill color.
- Typst handles PDF link annotations automatically.

## 7. Fixtures and tests

### New fixtures

| Fixture | Content |
|---------|---------|
| `hyperlink_simple.rtf` | Single paragraph with one hyperlink |
| `hyperlink_formatted.rtf` | Hyperlink with bold/italic runs inside |
| `hyperlink_multiple.rtf` | Paragraph with text, link, text, link pattern |
| `hyperlink_in_table.rtf` | Hyperlink inside a table cell |
| `hyperlink_unsupported_field.rtf` | Non-HYPERLINK field (PAGE, TOC) — still dropped |
| `hyperlink_missing_fldrslt.rtf` | Malformed field with no result text |

### Test categories

| Category | Tests |
|----------|-------|
| Golden IR snapshots | 6 (one per fixture) |
| DOCX integration | 4 (verify `<w:hyperlink>` in XML, relationship entries) |
| HTML integration | 3 (verify `<a href>` output, URL sanitization) |
| PDF integration | 2 (verify Typst source contains `#link`) |
| Contract/strict-mode | 3 (hyperlink docs pass strict; unsupported fields still fail) |
| Determinism | 2 (IR and DOCX stability for hyperlink fixtures) |
| **Total** | **~20** |

## 8. Implementation order

1. **IR types** — Add `Inline` enum, `Hyperlink` struct. Migrate `Paragraph.runs` to `Paragraph.inlines`.
2. **Update all existing writers** — Mechanical update to iterate `Inline::Run` where they currently iterate `Run`.
3. **Interpreter** — Add field-parsing state machine. Emit `Inline::Hyperlink` for HYPERLINK fields.
4. **Fixtures** — Create RTF test files.
5. **Golden snapshots** — Generate and verify IR JSON.
6. **DOCX writer** — Hyperlink + relationship emission.
7. **HTML writer** — `<a href>` emission with URL sanitization.
8. **Typst writer** — `#link()` emission.
9. **Contract tests** — Strict mode, determinism, integration.
10. **Documentation** — Update feature-support.md, warning-reference.md, CHANGELOG.

## 9. Acceptance criteria

1. `rtfkit convert hyperlink_simple.rtf -o out.docx` produces a DOCX where the link is clickable in Word.
2. `rtfkit convert hyperlink_simple.rtf --to html -o out.html` produces `<a href="...">` in output.
3. `rtfkit convert hyperlink_simple.rtf --to pdf -o out.pdf` produces a PDF with a clickable link.
4. `rtfkit convert hyperlink_simple.rtf --strict` exits 0 (no `DroppedContent`).
5. `rtfkit convert hyperlink_unsupported_field.rtf --strict` exits 4 (non-HYPERLINK field still dropped).
6. Hyperlink formatting (bold, italic inside link) is preserved in all output formats.
7. Golden IR snapshots pass for all new fixtures.
8. All existing tests continue to pass (no regressions).
9. URL sanitization rejects `javascript:` schemes in HTML output.

## 10. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| `Paragraph.runs → inlines` is a large mechanical refactor | Do it as step 1-2 before any semantic changes; all tests must pass after rename |
| `docx-rs` may lack hyperlink API | Check API surface first; fall back to raw XML if needed |
| Field instructions may have complex syntax | Scope to simple `HYPERLINK "url"` only; everything else stays dropped |
| Nested fields (`\field` inside `\fldrslt`) | Treat inner fields as plain text; emit warning |
| Malicious URLs in HTML output | Sanitize: allowlist `http:`, `https:`, `mailto:` schemes |
