# rtfkit Plan: Style Profiles for DOCX Output

**Status: IMPLEMENTED**

Extend the style profiles system (`classic`, `report`, `compact`) to the DOCX writer. HTML and PDF output already support style profiles via design tokens. DOCX currently ignores the `--style-profile` flag, which the CLI rejects with an error. This feature completes the cross-format styling story.

## 1. Primary objective

Apply style profile design tokens (typography, spacing, colors, component settings) to DOCX output, producing Word documents with consistent, professional formatting that matches the equivalent HTML and PDF output.

```
Style profile tokens (typography, spacing, colors)
  → DOCX default styles (<w:styles>)
  → Applied to paragraphs, runs, tables, lists
```

## 1.1 High-level implementation track

1. CLI unblocking and option plumbing
- Remove DOCX rejection for `--style-profile`.
- Pass resolved profile into DOCX writer options with stable defaults.

2. Writer baseline mapping (report parity)
- Map profile tokens into DOCX defaults (`docDefaults`, paragraph spacing, table defaults).
- Preserve current output for default profile as regression baseline.

3. Profile differentiation
- Implement clear deltas for `classic` and `compact` against `report`.
- Keep explicit RTF formatting precedence above profile defaults.

4. Verification hardening
- Add profile-specific XML assertions (defaults, list indentation, table properties).
- Add determinism checks and default-profile non-regression tests.

5. Documentation and rollout
- Remove "DOCX unsupported" statements.
- Document supported profile behavior and known intentional limits.

## 2. Scope

### In scope

- Remove the CLI restriction that rejects `--style-profile` for DOCX output.
- Apply `TypographyTokens` to DOCX:
  - Default body font (`font_body`) → `<w:rFonts>` on default paragraph style.
  - Default body size (`size_body`) → `<w:sz>` on default run properties.
  - Line height (`line_height_body`) → `<w:spacing w:line="...">` on default paragraph style.
- Apply `SpacingTokens` to DOCX:
  - Paragraph gap (`paragraph_gap`) → `<w:spacing w:after="...">` on default paragraph style.
  - List item gap (`list_item_gap`) → `<w:spacing w:after="...">` on list paragraph style.
  - Table cell padding (`table_cell_padding_x`, `table_cell_padding_y`) → `<w:tblCellMar>`.
- Apply `ComponentTokens` to DOCX:
  - Table border width (`table.border_width`) → `<w:tblBorders>` border size.
  - Table stripe mode (`table.stripe_mode`) — if `AlternateRows`, apply shading to even rows.
  - List indentation step (`list.indentation_step`) → `<w:ind w:left="...">` per level.
- Profile-specific DOCX behavior:
  - `classic` — Conservative: Georgia body font, generous spacing, standard borders.
  - `report` (default) — Professional: Arial headings, moderate spacing, clean borders.
  - `compact` — Dense: smaller fonts, tight spacing, minimal borders.
- Fixtures and integration tests verifying DOCX XML contains expected style values.
- Determinism: profile-applied DOCX output is byte-stable for same input and profile.

### Out of scope

- Custom user-defined style profiles (the `Custom(String)` variant exists but loading from file is separate).
- Heading styles (no heading detection in RTF — all content is paragraphs, lists, or tables).
- Color tokens applied to DOCX body text (text color comes from RTF source, not profile).
- Page layout tokens (`LayoutTokens`) for DOCX (Word page margins are typically set by the user).
- Table header emphasis (requires row-role detection, which isn't in the IR).

## 3. Design constraints and principles

- DOCX styling works via `<w:styles>` (document-level style definitions) and per-element overrides.
- The preferred approach is to set **document defaults** (`<w:docDefaults>`) for body font, size, and spacing, so individual paragraphs and runs inherit them without explicit properties.
- Profile styles are **defaults** — explicit formatting from the RTF source (via font/color mapping) should override profile values.
- The `docx-rs` crate provides an API for setting default styles. Verify the API surface before committing to an approach.
- Keep the implementation simple: apply tokens to document defaults and table properties. Avoid complex conditional formatting.
- Determinism: the same profile must always produce the same DOCX XML. Token values are static per profile, so this is straightforward.

## 4. IR changes

None. Style profiles are a **writer concern** — they affect how IR is rendered, not the IR itself. The `StyleProfileName` is passed to the writer via options.

## 5. CLI changes

### Remove DOCX restriction

Currently in `main.rs` (around line 270-309), there's a validation that rejects `--style-profile` when the target is DOCX:

```rust
// Current: style profiles not supported for DOCX
if style_profile.is_some() && target == Target::Docx {
    // error
}
```

Remove this check. Pass the `StyleProfileName` to the DOCX writer.

### DOCX writer options

Create a `DocxWriterOptions` struct (similar to `HtmlWriterOptions` and `RenderOptions`):

```rust
pub struct DocxWriterOptions {
    pub style_profile: StyleProfileName,
}
```

Update `write_docx` and `write_docx_to_bytes` signatures:

```rust
pub fn write_docx(document: &Document, path: &Path, options: &DocxWriterOptions) -> Result<(), DocxError>
pub fn write_docx_to_bytes(document: &Document, options: &DocxWriterOptions) -> Result<Vec<u8>, DocxError>
```

Default behavior (no profile or `report` profile) should produce output identical to current behavior to avoid breaking existing tests. This means applying `report` profile values that match the current hardcoded defaults.

## 6. DOCX writer changes

### Document defaults (`<w:docDefaults>`)

Set via docx-rs API (or raw XML) at the document level:

```xml
<w:docDefaults>
  <w:rPrDefault>
    <w:rPr>
      <w:rFonts w:ascii="{font_body}" w:hAnsi="{font_body}"/>
      <w:sz w:val="{size_body * 2}"/>  <!-- half-points -->
    </w:rPr>
  </w:rPrDefault>
  <w:pPrDefault>
    <w:pPr>
      <w:spacing w:after="{paragraph_gap_twips}" w:line="{line_height_value}"/>
    </w:pPr>
  </w:pPrDefault>
</w:docDefaults>
```

Conversions:
- Font size: IR points → DOCX half-points (`size * 2`).
- Paragraph gap: IR points → DOCX twips (`points * 20`).
- Line height: unitless multiplier → DOCX line spacing value (`multiplier * 240` for proportional spacing).

### Table formatting

Apply table tokens when emitting `<w:tbl>`:

```xml
<w:tblPr>
  <w:tblBorders>
    <w:top w:val="single" w:sz="{border_width * 8}" w:color="{border_default}"/>
    <!-- same for bottom, left, right, insideH, insideV -->
  </w:tblBorders>
  <w:tblCellMar>
    <w:top w:w="{cell_padding_y_twips}" w:type="dxa"/>
    <w:bottom w:w="{cell_padding_y_twips}" w:type="dxa"/>
    <w:start w:w="{cell_padding_x_twips}" w:type="dxa"/>
    <w:end w:w="{cell_padding_x_twips}" w:type="dxa"/>
  </w:tblCellMar>
</w:tblPr>
```

Border width conversion: profile points → DOCX eighth-points (`points * 8`).

### Row striping (optional)

If `table.stripe_mode == AlternateRows`:
- Apply `<w:shd w:val="clear" w:fill="{surface_table_stripe}">` to cells in even-numbered rows.
- This is applied per-cell, not per-row, in DOCX.

### List indentation

Apply `list.indentation_step` to the numbering abstract definitions:

```xml
<w:lvl w:ilvl="0">
  <w:pPr>
    <w:ind w:left="{indentation_step * (level + 1)}" w:hanging="{marker_gap}"/>
  </w:pPr>
</w:lvl>
```

Conversion: profile points → DOCX twips (`points * 20`).

## 7. Profile-specific values

### Classic profile

| Token | Value | DOCX Mapping |
|-------|-------|--------------|
| `font_body` | Georgia, serif | `<w:rFonts w:ascii="Georgia">` |
| `size_body` | 12.0 pt | `<w:sz w:val="24">` |
| `line_height_body` | 1.6 | `<w:spacing w:line="384">` |
| `paragraph_gap` | 12.0 pt | `<w:spacing w:after="240">` |
| `table.border_width` | 0.75 pt | `<w:sz="6">` |
| `list.indentation_step` | 24.0 pt | `<w:ind w:left="480">` per level |

### Report profile (default)

| Token | Value | DOCX Mapping |
|-------|-------|--------------|
| `font_body` | Georgia, serif | `<w:rFonts w:ascii="Georgia">` |
| `size_body` | 11.0 pt | `<w:sz w:val="22">` |
| `line_height_body` | 1.5 | `<w:spacing w:line="360">` |
| `paragraph_gap` | 10.0 pt | `<w:spacing w:after="200">` |
| `table.border_width` | 0.5 pt | `<w:sz="4">` |
| `list.indentation_step` | 18.0 pt | `<w:ind w:left="360">` per level |

### Compact profile

| Token | Value | DOCX Mapping |
|-------|-------|--------------|
| `font_body` | Arial, sans-serif | `<w:rFonts w:ascii="Arial">` |
| `size_body` | 10.0 pt | `<w:sz w:val="20">` |
| `line_height_body` | 1.3 | `<w:spacing w:line="312">` |
| `paragraph_gap` | 6.0 pt | `<w:spacing w:after="120">` |
| `table.border_width` | 0.25 pt | `<w:sz="2">` |
| `list.indentation_step` | 14.0 pt | `<w:ind w:left="280">` per level |

## 8. Fixtures and tests

### New fixtures

No new RTF fixtures needed — style profiles use existing fixtures. Testing is done by converting existing fixtures with different profiles and inspecting the DOCX XML.

### Test categories

| Category | Tests |
|----------|-------|
| DOCX document defaults | 3 (one per profile — verify `<w:docDefaults>` XML) |
| DOCX table formatting | 3 (one per profile — verify `<w:tblBorders>`, `<w:tblCellMar>`) |
| DOCX list indentation | 3 (one per profile — verify `<w:ind>` values) |
| CLI integration | 2 (`--style-profile classic` accepted; `--style-profile invalid` rejected) |
| Regression | 3 (existing fixtures produce identical output with default `report` profile) |
| Determinism | 2 (same profile → identical DOCX XML) |
| Row striping | 1 (verify shading on even rows with compact profile if stripe_mode is set) |
| **Total** | **~17** |

### Regression safety

The critical constraint: existing DOCX output (without `--style-profile`) must not change. This means:
- The `report` profile must match current hardcoded behavior exactly, OR
- When no profile is specified, skip applying profile defaults entirely (preserve current behavior).

Recommended approach: **no-profile = current behavior** (no document defaults applied). When `--style-profile report` is explicitly set, apply report defaults. This avoids any regression risk.

## 9. Implementation order

1. **DocxWriterOptions** — Create options struct with `style_profile` field.
2. **Update writer signatures** — `write_docx()` and `write_docx_to_bytes()` accept options.
3. **Update CLI** — Remove DOCX profile restriction, pass profile to writer.
4. **Update all call sites** — Tests, CLI, any direct writer calls. Use `Default` options to preserve current behavior.
5. **Document defaults** — When profile is set, emit `<w:docDefaults>` with typography/spacing tokens.
6. **Table formatting** — Apply border and cell margin tokens to table output.
7. **List indentation** — Apply indentation step to numbering definitions.
8. **Row striping** — Apply alternating shading if configured.
9. **Integration tests** — Verify DOCX XML for each profile.
10. **Regression tests** — Verify existing output unchanged.
11. **Documentation** — Update feature-support.md, reference docs, CHANGELOG.

## 10. Acceptance criteria

1. `rtfkit convert input.rtf -o out.docx --style-profile classic` succeeds (no CLI error).
2. `rtfkit convert input.rtf -o out.docx --style-profile compact` produces a DOCX with smaller fonts and tighter spacing.
3. `rtfkit convert input.rtf -o out.docx` (no profile) produces identical output to current behavior.
4. DOCX XML contains `<w:docDefaults>` with correct font and spacing values when profile is set.
5. Table borders and cell margins reflect profile token values.
6. List indentation scales with profile's `indentation_step`.
7. All existing tests pass without modification (regression-safe).
8. Determinism: same input + same profile → byte-identical DOCX XML.

## 11. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| `docx-rs` may not support `<w:docDefaults>` | Check API; fall back to applying styles per-paragraph if needed |
| Existing test regression from new document defaults | Only apply defaults when profile is explicitly set. No profile = current behavior |
| Font name extraction from profile (e.g., "Georgia, serif") | Strip generic fallback; use first font name only for DOCX (`w:ascii` expects single name) |
| Line height calculation varies between proportional and exact | Use proportional (`w:lineRule="auto"`) consistently. Value = multiplier * 240 |
| Profile values don't match current hardcoded DOCX behavior | Measure current behavior first, set `report` profile to match. Or keep no-profile = no-defaults approach |
| Row striping may conflict with merge cells | Apply shading only to non-continuation cells. Skip cells with `VerticalContinue` merge |

## 12. Dependencies

- `rtfkit-style-tokens` crate (already a dependency of HTML and Typst writers). Add as dependency of `rtfkit-docx`.
- No new external crate dependencies.
