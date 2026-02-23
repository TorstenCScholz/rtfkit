# Cross-Format Style Tokens Framework Specification

## Status
- **Implemented** (Phase D complete).
- Scope target: HTML and PDF outputs, with a forward path for DOCX styling decisions.

## 1. Problem Statement

Today, visual styling quality differs by output format:
- HTML has a polished stylesheet and tokenized CSS variables.
- PDF uses Typst defaults with limited style controls.

Result:
- Same IR document produces noticeably different visual personality across HTML vs PDF.
- Styling logic is duplicated per writer and will drift further as features grow.

This spec defines a single style-token framework so HTML and PDF can share one visual system without introducing architecture debt.

## 2. Product Goals

1. One source of truth for style primitives and component-level presentation decisions.
2. Consistent visual language across HTML and PDF for headings, body text, lists, tables, spacing, and page rhythm.
3. Deterministic output preserved in all formats.
4. Incremental migration with low risk and no large-bang refactor.
5. Strong boundaries so format writers remain cohesive and avoid generic over-abstraction.

## 3. Non-Goals

1. Pixel-perfect parity between HTML and PDF rendering engines.
2. Full RTF style fidelity in first iteration (font table/color table parity is separate work).
3. Runtime plugin ecosystem for themes.
4. Replacing current IR model.

## 4. Design Principles (Anti-Tech-Debt)

1. Single ownership:
- Token definitions live in one crate/module only.

2. Explicit renderer adapters:
- HTML and PDF have dedicated mappers from tokens to CSS/Typst.
- No generic "style engine" trait hierarchy.

3. Strong typing over stringly config:
- Internal APIs use typed structs/enums, not unvalidated `HashMap<String, String>`.

4. Deterministic serialization:
- Token-to-CSS and token-to-Typst preamble generation must be stable and order-controlled.

5. Progressive rollout:
- Existing behavior remains available while each format is migrated in small gated steps.

## 5. Proposed Architecture

## 5.1 New Module/Crate

Introduce a focused crate:
- `crates/rtfkit-style-tokens`

Responsibilities:
1. Define canonical style token schema.
2. Provide built-in style profiles (e.g., `Classic`, `Report`, `Compact`).
3. Validate tokens (`StyleValidationError`) with strict invariants.
4. Provide deterministic export helpers:
- CSS variable/value map (for HTML writer)
- Typst style preamble model (for PDF writer)

Non-responsibilities:
1. No dependency on HTML writer internals.
2. No dependency on Typst renderer internals.
3. No file I/O policy (CLI handles loading/merging external style files if enabled).

## 5.2 Dependency Direction

- `rtfkit-style-tokens` -> no renderer crates.
- `rtfkit-html` -> depends on `rtfkit-style-tokens`.
- `rtfkit-render-typst` -> depends on `rtfkit-style-tokens`.
- `rtfkit-cli` -> selects profile and passes resolved tokens to writers.

This keeps style policy centralized while renderers stay format-specific.

## 6. Canonical Token Model

## 6.1 Token Families (MVP)

1. Color tokens:
- `text_primary`, `text_muted`, `surface_page`, `surface_table_header`, `surface_table_stripe`, `border_default`, `link_default`, `link_hover`

2. Typography tokens:
- `font_body`, `font_heading`, `font_mono`
- `size_body`, `size_small`, `size_h1`, `size_h2`, `size_h3`
- `line_height_body`, `line_height_heading`
- `weight_regular`, `weight_semibold`, `weight_bold`

3. Spacing tokens:
- `space_xs`, `space_sm`, `space_md`, `space_lg`, `space_xl`
- `paragraph_gap`, `list_item_gap`, `table_cell_padding_x`, `table_cell_padding_y`

4. Layout/page tokens:
- `content_max_width_mm` (HTML wrapper + print friendliness)
- `page_margin_top_mm`, `page_margin_bottom_mm`, `page_margin_left_mm`, `page_margin_right_mm`

5. Component tokens:
- Table: border width, stripe mode, header emphasis
- List: indentation step, marker gap
- Heading: spacing above/below blocks

## 6.2 Example Internal Rust Types

```rust
pub struct StyleProfile {
    pub name: StyleProfileName,
    pub colors: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub layout: LayoutTokens,
    pub components: ComponentTokens,
}

pub enum StyleProfileName {
    Classic,
    Report,
    Compact,
    Custom(String),
}
```

Implementation note:
- Keep this model explicit, even if verbose.
- Avoid premature generic token graphs.

## 6.3 Units and Conversion Rules

Canonical units:
1. Typography sizes: points (`pt`) in model.
2. Spacing: points (`pt`) in model.
3. Page/layout physical dimensions: millimeters (`mm`).

Renderer conversion:
1. HTML converts `pt` to `rem` or `px` via one deterministic function.
2. PDF (Typst) maps `pt/mm` directly to Typst units.

This gives predictable cross-format scaling without ambiguous implicit unit behavior.

## 7. Profile Strategy

Built-in profiles for stable product defaults:
1. `classic` (conservative, neutral)
2. `report` (strong hierarchy for long-form docs)
3. `compact` (dense enterprise output)

MVP default recommendation:
- `report` for PDF and HTML unless explicitly overridden.

Future optional extension:
- `--style-file <path>` for custom profile overlay (strict schema, fail-fast validation).

## 8. Integration Plan

## 8.1 HTML (`rtfkit-html`)

1. Replace hardcoded CSS token constants with generated CSS variable block from `StyleProfile`.
2. Keep existing semantic classes (`rtf-align-*`, `rtf-table`, `rtf-u`) to preserve contract.
3. Update `HtmlWriterOptions`:
- add `style_profile: StyleProfileName` (or resolved profile object)
- keep `css_mode` + `custom_css`

4. Ensure deterministic CSS ordering:
- fixed property order
- fixed class block order

## 8.2 PDF (`rtfkit-render-typst`)

1. Generate a deterministic Typst style preamble from tokens.
2. Apply token-driven defaults for:
- body text size/line-height
- heading scale/weight/spacing
- list indentation/spacing
- table stroke/fill/padding
- page margins

3. Keep current content mapping logic separate from style preamble generation.
4. Preserve strict-mode warning semantics (styling changes must not alter dropped-content semantics).

## 8.3 CLI (`rtfkit-cli`)

MVP flags:
1. `--style-profile <classic|report|compact>`
2. `--html-css` / `--html-css-file` remain as is for compatibility.

Validation rules:
1. Unknown profile -> exit code 2.
2. Profile selection allowed for HTML and PDF.
3. For DOCX in MVP, ignore with warning or reject explicitly (choose one and document).

Recommendation:
- Reject for DOCX in MVP to avoid ambiguous behavior.

## 9. Rollout Phases

## Phase A: Foundation (schema + built-in profiles)

Deliverables:
1. `rtfkit-style-tokens` crate with typed schema and validators.
2. Built-in profiles with snapshot tests.
3. Deterministic serialization tests (stable string outputs).

Gate:
- `cargo test` and `clippy -D warnings` clean.

## Phase B: HTML migration

Deliverables:
1. HTML CSS generated from style profiles.
2. Existing HTML tests updated with deterministic snapshots.
3. No regression in semantic class coverage.

Gate:
- Existing HTML contract tests green.

## Phase C: PDF migration

Deliverables:
1. Typst preamble generated from same tokens.
2. PDF visual hierarchy improvements (headings/lists/tables).
3. Determinism tests updated and green.

Gate:
- Existing PDF determinism and integration tests green.

## Phase D: CLI + docs + hardening

Deliverables:
1. New CLI profile flag and validation.
2. Docs for style profiles and cross-format behavior.
3. CI guard against writer-local hardcoded style constants.

Gate:
- Full workspace CI green across platforms.

## 10. Testing Strategy

## 10.1 Unit Tests

1. Token validation rules.
2. Profile resolution and defaulting.
3. Deterministic token serialization.

## 10.2 Snapshot Tests

1. HTML generated CSS snapshot per profile.
2. Typst style preamble snapshot per profile.

## 10.3 Integration Tests

1. Same fixture rendered to HTML and PDF with same profile.
2. Assert key visual semantics are present in both generated artifacts:
- heading scale declarations
- table style declarations
- list indentation rules

3. Determinism checks over repeated runs.

## 10.4 Regression Tests

1. Existing strict-mode and warning semantics unchanged.
2. Existing output contracts (exit codes, CLI behavior) unchanged unless explicitly specified.

## 11. CI Guardrails (No Debt Reintroduction)

Add checks:
1. `scripts/check_style_token_usage.sh` to fail if writers introduce new hardcoded style constants outside token adapter modules.
2. `cargo deny`/workspace policy check to prevent accidental cyclic deps involving token crate.
3. Snapshot drift checks in CI for profile outputs.

## 12. API and Compatibility Policy

1. Style profile names are stable public contract.
2. Token schema changes require changelog entry and migration note.
3. Adding new tokens is backward-compatible.
4. Renaming/removing tokens requires a deprecation cycle if exposed via external style file schema.

## 13. Risks and Mitigations

Risk 1: Over-generalized abstraction layer.
- Mitigation: keep format-specific adapter modules and avoid trait plugin systems.

Risk 2: Determinism regressions from token maps.
- Mitigation: use ordered data structures and deterministic emit order.

Risk 3: Visual changes breaking existing expectations.
- Mitigation: profile-based rollout; keep `classic` close to current behavior.

Risk 4: Scope creep into full RTF style fidelity.
- Mitigation: keep this track focused on output styling framework, not parser fidelity expansion.

## 14. Acceptance Criteria

1. HTML and PDF both consume the same resolved `StyleProfile` object.
2. At least one polished profile (`report`) visibly improves PDF hierarchy and table/list styling.
3. No strict-mode semantic regressions.
4. Determinism tests remain green.
5. No duplicated style constants across writer crates outside adapter modules.
6. Documentation clearly states what is shared vs format-specific.

## 15. Follow-on Work (Post-MVP)

1. External style profile file support (`--style-file`) with strict TOML/JSON schema.
2. Optional profile inheritance (`base = "report"`) with deterministic merge rules.
3. DOCX mapping of selected tokens where writer API allows safe integration.
4. Advanced typography controls (open type features, orphan/widow policies) per format capabilities.

## 16. Initial File Touch Plan (for implementation phase)

New:
1. `crates/rtfkit-style-tokens/Cargo.toml`
2. `crates/rtfkit-style-tokens/src/lib.rs`
3. `crates/rtfkit-style-tokens/src/profile.rs`
4. `crates/rtfkit-style-tokens/src/validate.rs`

Modify:
1. `crates/rtfkit-html/src/options.rs`
2. `crates/rtfkit-html/src/style.rs`
3. `crates/rtfkit-render-typst/src/options.rs`
4. `crates/rtfkit-render-typst/src/map/mod.rs` (style preamble integration point)
5. `crates/rtfkit-cli/src/main.rs`
6. `docs/reference/pdf-output.md`
7. `docs/feature-support.md`

Optional CI script:
1. `scripts/check_style_token_usage.sh`

