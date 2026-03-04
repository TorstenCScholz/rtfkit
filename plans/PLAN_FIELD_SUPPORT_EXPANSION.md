# rtfkit Plan: Field Support Expansion (Enterprise Reliability)

**Status: DRAFT (implementation-ready)**

This plan targets the highest-impact real-world gap: RTF field handling beyond the
currently supported subset (hyperlink + page fields + TOC marker).

## 1. Objective

Expand field support with deterministic behavior and predictable strict-mode semantics:

```text
RTF fields
  -> normalized field semantics in IR (where meaningful)
  -> DOCX / HTML / PDF mapping or explicit deterministic fallback
  -> stable warnings for pipeline triage
```

Target outcomes:

- Fewer semantic losses on common enterprise templates.
- Better compatibility with cross-references and document metadata fields.
- More actionable warnings (field type + reason) without strict-mode noise.
- No regression in determinism, limits, or exit-code contracts.

## 2. Why This First

Based on enterprise document patterns, fields fail more often than fonts and cause
higher business risk when degraded silently.

High-impact field families:

1. Cross-reference fields (`REF`, `NOTEREF`)
2. Numbering/sequence fields (`SEQ`)
3. Document property fields (`DOCPROPERTY`, common built-ins)
4. Template mail-merge fields (`MERGEFIELD`)

## 3. Baseline (Current)

Implemented today in `crates/rtfkit-core/src/rtf/handlers_fields.rs`:

- `HYPERLINK` (external + `\l` internal bookmark)
- Page fields: `PAGE`, `NUMPAGES`, `SECTIONPAGES`, `PAGEREF`
- `TOC` marker + partial switch handling
- Unsupported fields:
  - `UnsupportedField` warning when `\fldrslt` is preserved
  - `DroppedContent` when field has no usable result

This is a strong fallback baseline but still partial for common business field types.

## 4. Scope

### In scope

1. Add semantic parsing + IR support for:
   - `REF <bookmark>`
   - `NOTEREF <bookmark>`
   - `SEQ <identifier>`
2. Add typed handling for document property fields:
   - `DOCPROPERTY <name>`
   - common direct built-ins (`AUTHOR`, `TITLE`, `SUBJECT`, `KEYWORDS`)
3. Add deterministic mail-merge fallback policy:
   - `MERGEFIELD <name>` with preserved visible result and structured warning
4. Improve field instruction parser robustness:
   - quoted/unquoted tokens
   - escaped quotes
   - switch/value parsing
   - case-insensitive keywords
5. Expand warnings to be specific and stable for triage.
6. Full fixture-first coverage (IR + CLI contracts + writer integration).

### Out of scope

- Full Word formula engine (`IF`, `=`, nested expression evaluation)
- External content inclusion fields (`INCLUDETEXT`, `INCLUDEPICTURE`, `DDE*`)
- Active form behavior (legacy form fields as interactive controls)

## 5. Contracts to Preserve

1. Exit codes stay unchanged:
   - `0` success
   - `2` parse/validation failure
   - `3` writer/IO failure
   - `4` strict-mode violation (`DroppedContent` only)
2. Strict mode fails only on actual content/semantic loss.
3. Determinism is mandatory for IR/report/writer output.
4. Fail-closed behavior for malformed/hostile input remains intact.

## 6. IR Additions

Extend IR in `crates/rtfkit-core/src/lib.rs` with explicit field refs:

```rust
pub enum SemanticFieldRef {
    Ref { target: String, fallback_text: Option<String> },
    NoteRef { target: String, fallback_text: Option<String> },
    Sequence { identifier: String, fallback_text: Option<String> },
    DocProperty { name: String, fallback_text: Option<String> },
    MergeField { name: String, fallback_text: Option<String> },
}

// Inline addition:
Inline::SemanticField(SemanticFieldRef)
```

Design rule:

- Always carry `fallback_text` when available from `\fldrslt` to preserve visible output
  and avoid information loss when writers cannot emit dynamic behavior.

## 7. Parser Strategy

Implementation focus in `crates/rtfkit-core/src/rtf/handlers_fields.rs`:

1. Replace ad hoc instruction handling with a shared field-token parser:
   - keyword
   - switches (with/without values)
   - positional arguments
2. Parse recognized fields into typed instruction variants.
3. Finalization matrix:
   - recognized + mappable -> emit `Inline::SemanticField(...)`
   - recognized but partial -> emit semantic field + `UnsupportedField`/specific warning
   - unrecognized with result text -> emit result text + `UnsupportedField`
   - no recoverable result text -> emit `DroppedContent`

Related state files:

- `crates/rtfkit-core/src/rtf/state_fields.rs`
- `crates/rtfkit-core/src/rtf/handlers_text.rs` (fallback text capture consistency)

## 8. Writer Mapping Plan

### DOCX (`crates/rtfkit-docx`)

1. `REF`/`NOTEREF`:
   - map to internal hyperlink (`w:anchor`) when target is resolvable
   - fallback to visible text when unresolved
2. `SEQ`, `DOCPROPERTY`, `MERGEFIELD`:
   - v1: emit fallback text only (deterministic)
   - preserve semantics via warning/report, not hard failure

### HTML (`crates/rtfkit-html`)

1. `REF`/`NOTEREF`:
   - `<a href="#...">...</a>` when bookmark target exists
2. Other semantic fields:
   - emit fallback text
   - optional stable class marker (non-functional) for traceability

### PDF / Typst (`crates/rtfkit-render-typst`)

1. `REF`/`NOTEREF`:
   - best-effort internal links if representable
2. Other semantic fields:
   - render fallback text
   - warning-only for semantic non-dynamic behavior

## 9. Warning Model Improvements

Extend warnings for clearer triage (still non-strict when text preserved):

1. `UnsupportedField` reason includes normalized field type
   - e.g. `Unsupported field type: MERGEFIELD (result preserved)`
2. Add focused warning for unresolved cross-reference target:
   - e.g. `unresolved_cross_reference`
3. Keep `DroppedContent` reserved for true loss:
   - missing/unrecoverable `\fldrslt`
   - malformed field with no visible fallback

Update `docs/warning-reference.md` and warning tests for stable strings.

## 10. Delivery Slices

### Slice A: Parser + IR foundation

1. Add `SemanticFieldRef` IR
2. Implement tokenizer/parser cleanup
3. Parse and emit `REF`, `NOTEREF`, `SEQ`
4. Add core unit tests

Definition of done:

- IR contains semantic field nodes for these types
- Existing hyperlink/page/TOC behavior unchanged

### Slice B: Metadata and template fields

1. Parse `DOCPROPERTY`, built-in doc properties, `MERGEFIELD`
2. Emit semantic field nodes with fallback text
3. Add structured warning reasons

Definition of done:

- Preserved text for all supported instructions with deterministic warnings
- No strict-mode false positives when fallback text exists

### Slice C: Writer and contract integration

1. DOCX/HTML/Typst mapping for semantic fields
2. CLI contract tests for strict/non-strict outcomes
3. Realworld fixture validation

Definition of done:

- Cross-reference links function where targets exist
- Unsupported semantics degrade predictably and visibly

### Slice D: Hardening + docs

1. Golden updates and determinism checks
2. Feature matrix/docs/changelog updates
3. Warning reference synchronization

Definition of done:

- All related tests pass
- Documentation matches actual behavior

## 11. Fixture and Test Matrix

Add fixtures:

- `fixtures/field_ref_simple.rtf`
- `fixtures/field_noteref_simple.rtf`
- `fixtures/field_seq_simple.rtf`
- `fixtures/field_docproperty_author.rtf`
- `fixtures/field_mergefield_preserve_result.rtf`
- `fixtures/field_ref_unresolved_target.rtf`
- `fixtures/field_nested_formula_fallback.rtf`

Coverage:

1. Core parser unit tests (`handlers_fields.rs`)
2. CLI contract tests:
   - strict passes when result preserved
   - strict fails only on `DroppedContent`
3. Golden IR snapshots (`golden/`)
4. HTML snapshots (`golden_html/`) where output changes
5. DOCX integration assertions for internal anchors
6. Determinism tests for newly added fixtures

## 12. Commands for Validation

From repository root:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
UPDATE_GOLDEN=1 cargo test -p rtfkit-cli --test golden_tests
```

If HTML changes:

```sh
UPDATE_GOLDEN=1 cargo test -p rtfkit-cli --test golden_tests -- html
```

## 13. Acceptance Criteria

1. `REF`/`NOTEREF`/`SEQ`/`DOCPROPERTY`/`MERGEFIELD` are handled deterministically.
2. Cross-reference fields are linked where possible (DOCX/HTML at minimum).
3. Fallback text is preserved for partial/unsupported semantics.
4. Strict mode fails only on actual dropped content.
5. Warning reasons are stable, specific, and documented.
6. Goldens and determinism tests remain stable across reruns.
