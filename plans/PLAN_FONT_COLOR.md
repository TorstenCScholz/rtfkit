# rtfkit Plan: Font and Color Mapping

**Status: PLANNED**

Wire the already-parsed font table (`\fonttbl`), color table (`\colortbl`), and per-run formatting controls (`\f`, `\fs`, `\cf`) through the interpreter into the IR and all output writers. The parser already encounters these control words — they are silently ignored today. This feature closes the gap between "parsed" and "output" for the most visible text formatting properties.

## 1. Primary objective

Map RTF font selection (`\f`), font size (`\fs`), and foreground color (`\cf`) to the existing IR `Run` fields (`font_size`, `color`) and add a new `font_family` field, then emit correct formatting in DOCX, HTML, and PDF.

```
RTF \fonttbl + \colortbl + \fN \fsN \cfN
  → IR Run { font_family, font_size, color, ... }
  → DOCX <w:rFonts>/<w:sz>/<w:color> / HTML style attrs / Typst #text()
```

## 2. Scope

### In scope

- Parse `\fonttbl` destination to build a font-index-to-name lookup table.
- Parse `\colortbl` destination to build a color-index-to-RGB lookup table.
- Handle `\f N` — set current font by index (lookup in font table).
- Handle `\fs N` — set current font size (N is in half-points; divide by 2 for points).
- Handle `\cf N` — set current foreground color by index (lookup in color table).
- Handle `\plain` — reset all character formatting to defaults.
- Add `font_family: Option<String>` to `Run` in the IR.
- Wire `font_size` and `color` (already present on `Run` but never populated).
- DOCX writer: emit `<w:rFonts w:ascii="..." w:hAnsi="...">`, `<w:sz w:val="N">`, `<w:color w:val="RRGGBB">`.
- HTML writer: emit inline `style` attributes or CSS classes for font-family, font-size, color.
- Typst/PDF writer: emit `#text(font: "...", size: Npt, fill: rgb("..."))`.
- Handle `\deff N` (default font) — apply default font to runs that don't specify one.
- Fixtures: at least 4 new RTF fixtures.
- Golden IR snapshots for all new fixtures.

### Out of scope

- Background/highlight color (`\cb`, `\highlight`) — separate feature.
- Strikethrough, superscript, subscript (`\strike`, `\super`, `\sub`) — separate feature.
- Character spacing/kerning (`\expnd`, `\kerning`) — cosmetic, low priority.
- Full font substitution logic (if a font isn't available on the system).
- Font embedding in PDF (Typst uses its bundled fonts; custom font loading is separate).
- RTF font charset handling (`\fcharset`) — character encoding is handled by Unicode escapes.
- Multiple font families per run (`\af` — associated font for complex scripts).

## 3. Design constraints and principles

- `\fonttbl` and `\colortbl` are **document-level metadata** destinations. They must be parsed into lookup tables stored in the interpreter, not emitted as blocks.
- Font and color references (`\f`, `\cf`) are **run-level** properties that go onto `StyleState` and propagate into `Run`.
- `\fs` values are in **half-points** (RTF convention). The IR stores points. Convert: `points = half_points / 2.0`.
- Color index 0 in `\colortbl` means "auto" (default color, typically black). `\cf0` should result in `color: None` (let the output format decide the default).
- Font index 0 typically maps to the `\deff` default font.
- The IR already has `Run.font_size: Option<f32>` and `Run.color: Option<Color>` — these just need to be populated. Only `font_family` is genuinely new.
- Determinism: font table and color table parsing must produce stable lookup tables. The tables are ordered by their index in the RTF source.
- If `\f N` references an index not in the font table, emit `UnsupportedControlWord` warning and leave `font_family` as `None`.
- If `\cf N` references an index not in the color table, emit `UnsupportedControlWord` warning and leave `color` as `None`.

## 4. IR changes

### Add `font_family` to `Run`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub text: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    /// Font family name (e.g., "Times New Roman", "Arial")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,      // NEW
    /// Font size in points
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,           // EXISTING - now populated
    /// Text foreground color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,             // EXISTING - now populated
}
```

No new types needed. The `Color` struct already exists.

## 5. Interpreter changes

### Font table parsing

Currently `\fonttbl` is in the `Metadata` destination list (skipped entirely). Change to a new `DestinationBehavior::FontTable` that parses the content:

RTF font table structure:
```rtf
{\fonttbl
  {\f0\froman Times New Roman;}
  {\f1\fswiss Arial;}
  {\f2\fmodern Courier New;}
}
```

Parsing approach:
1. When `\fonttbl` destination is entered, switch to **font-table parsing mode**.
2. Each sub-group `{\fN\fFAMILY Name;}` provides an index and a font name.
3. Capture the `\f N` control word for the index.
4. Capture trailing text (before `;`) as the font name. Strip the trailing semicolon.
5. Ignore font family class control words (`\froman`, `\fswiss`, `\fmodern`, etc.) — they are informational.
6. Store result in `HashMap<i32, String>` on the interpreter.

### Color table parsing

Currently `\colortbl` is in the `Metadata` destination list. Change to `DestinationBehavior::ColorTable`:

RTF color table structure:
```rtf
{\colortbl;
  \red255\green0\blue0;
  \red0\green128\blue0;
}
```

Parsing approach:
1. When `\colortbl` destination is entered, switch to **color-table parsing mode**.
2. The color table is semicolon-delimited. The first entry (before the first `;`) is the "auto" color (index 0), which is typically empty.
3. For each entry, look for `\red N`, `\green N`, `\blue N` control words.
4. Store result in `Vec<Option<Color>>` on the interpreter. Index 0 = None (auto).

### StyleState changes

```rust
pub struct StyleState {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub alignment: Alignment,
    pub font_index: Option<i32>,    // NEW — current \f index
    pub font_size_half_points: Option<i32>,  // NEW — current \fs value (raw)
    pub color_index: Option<i32>,   // NEW — current \cf index
}
```

### Control word handling additions

```rust
// In handle_control_word match:
"f" => {
    if let Some(idx) = parameter {
        self.current_style.font_index = Some(idx);
    }
}
"fs" => {
    if let Some(size) = parameter {
        self.current_style.font_size_half_points = Some(size);
    }
}
"cf" => {
    if let Some(idx) = parameter {
        self.current_style.color_index = Some(idx);
    }
}
"plain" => {
    self.current_style = StyleState::default();
}
```

### Run construction

When finalizing a run, resolve indices to values:

```rust
fn build_run(&self, text: String) -> Run {
    let font_family = self.current_style.font_index
        .and_then(|idx| self.font_table.get(&idx).cloned());

    let font_size = self.current_style.font_size_half_points
        .map(|hp| hp as f32 / 2.0);

    let color = self.current_style.color_index
        .and_then(|idx| {
            if idx == 0 { None }  // auto color
            else { self.color_table.get(idx as usize).cloned().flatten() }
        });

    Run {
        text,
        bold: self.current_style.bold,
        italic: self.current_style.italic,
        underline: self.current_style.underline,
        font_family,
        font_size,
        color,
    }
}
```

### `\deff` handling

`\deff N` sets the default font index. Capture it during header parsing and apply as the initial value of `font_index` in `StyleState`.

## 6. Writer changes

### DOCX writer (`rtfkit-docx`)

For each `Run` with font/size/color set, emit run properties:

```xml
<w:rPr>
  <w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman"/>  <!-- if font_family -->
  <w:sz w:val="24"/>        <!-- if font_size (value in half-points: size * 2) -->
  <w:color w:val="FF0000"/> <!-- if color (hex RGB) -->
  <w:b/>                    <!-- if bold -->
</w:rPr>
```

Note: DOCX `<w:sz>` uses half-points, same as RTF `\fs`. Convert from IR points back to half-points: `half_points = (points * 2) as u32`.

Check `docx-rs` API:
- `RunProperty::fonts(...)` or similar.
- `RunProperty::size(...)`.
- `RunProperty::color(...)`.

### HTML writer (`rtfkit-html`)

For runs with font/size/color, emit inline styles on the `<span>`:

```html
<span style="font-family: 'Times New Roman', serif; font-size: 12pt; color: #ff0000;">text</span>
```

Design decisions:
- Only emit `style` attributes when values differ from the document default (avoid verbose output).
- Font family: quote names with spaces. Append generic family as fallback (serif/sans-serif/monospace based on name heuristic or RTF font family class).
- Font size: use `pt` units for consistency with RTF.
- Color: use `#rrggbb` hex notation.

### Typst/PDF writer (`rtfkit-render-typst`)

For runs with font/size/color, wrap content in `#text()`:

```typst
#text(font: "Times New Roman", size: 12pt, fill: rgb("#ff0000"))[content]
```

Avoid redundant wrapping — only emit `#text()` when the run has explicit font/size/color that differs from the default.

## 7. Fixtures and tests

### New fixtures

| Fixture | Content |
|---------|---------|
| `text_font_family.rtf` | Paragraphs using different fonts from `\fonttbl` |
| `text_font_size.rtf` | Text with varying `\fs` sizes (small, normal, large) |
| `text_color.rtf` | Text with `\cf` referencing `\colortbl` entries |
| `text_font_color_combined.rtf` | Mixed font, size, and color in one document |
| `text_plain_reset.rtf` | `\plain` resets all formatting mid-paragraph |
| `text_default_font.rtf` | `\deff1` sets non-zero default font |

### Test categories

| Category | Tests |
|----------|-------|
| Font table parsing unit tests | 4 (simple table, multiple fonts, empty table, missing font) |
| Color table parsing unit tests | 4 (simple table, auto color, multiple colors, missing color) |
| Golden IR snapshots | 6 (one per fixture) |
| DOCX integration | 4 (verify `<w:rFonts>`, `<w:sz>`, `<w:color>` in XML) |
| HTML integration | 3 (verify inline styles) |
| PDF integration | 2 (verify Typst source `#text()` params) |
| `\plain` reset behavior | 2 (verify formatting resets mid-paragraph) |
| Invalid index handling | 2 (out-of-range `\f` and `\cf` emit warnings) |
| Determinism | 2 (IR stability for font/color fixtures) |
| **Total** | **~29** |

## 8. Implementation order

1. **IR change** — Add `font_family: Option<String>` to `Run`. Update `Run::new()`.
2. **Update all existing tests** — Existing golden snapshots won't break (new field is `skip_serializing_if = None`), but `Run` construction sites need the new field.
3. **Font table parser** — Parse `\fonttbl` destination into `HashMap<i32, String>`.
4. **Color table parser** — Parse `\colortbl` destination into `Vec<Option<Color>>`.
5. **StyleState extension** — Add `font_index`, `font_size_half_points`, `color_index`.
6. **Control word handling** — Wire `\f`, `\fs`, `\cf`, `\plain`, `\deff`.
7. **Run construction** — Resolve indices to values when building runs.
8. **Fixtures** — Create RTF test files.
9. **Golden snapshots** — Generate and verify IR JSON.
10. **DOCX writer** — Emit `<w:rFonts>`, `<w:sz>`, `<w:color>`.
11. **HTML writer** — Emit inline styles for font, size, color.
12. **Typst writer** — Emit `#text()` with font/size/fill parameters.
13. **Contract tests** — Determinism, integration, edge cases.
14. **Documentation** — Update feature-support.md, CHANGELOG.

## 9. Acceptance criteria

1. `rtfkit convert text_font_family.rtf --emit-ir out.json` produces IR with `font_family` values matching the font table.
2. `rtfkit convert text_color.rtf --emit-ir out.json` produces IR with `color` values matching the color table.
3. `rtfkit convert text_font_size.rtf -o out.docx` produces a DOCX with correct `<w:sz>` values.
4. `rtfkit convert text_color.rtf --to html -o out.html` produces HTML with `color: #rrggbb` styles.
5. `rtfkit convert text_font_color_combined.rtf --to pdf -o out.pdf` produces a PDF with visible font/size/color variation.
6. `\cf0` (auto color) results in `color: None` in IR (no explicit color emitted).
7. `\plain` resets font, size, and color to defaults.
8. Out-of-range font/color indices produce warnings but don't crash.
9. Golden IR snapshots pass for all new fixtures.
10. All existing tests continue to pass.

## 10. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Font table parsing is complex (nested groups, charset info) | Keep it simple: extract index and name only. Ignore `\fcharset`, font family classes |
| Color table semicolon parsing is tricky | Handle carefully: track `\red`/`\green`/`\blue` per entry, split on `;` |
| Existing golden snapshots may break if `Run` serialization changes | `skip_serializing_if = None` prevents this. Existing runs have `None` for new fields |
| Font names may not exist on the user's system | This is expected — DOCX/HTML/PDF all have fallback font mechanisms. We just emit the name |
| `\plain` interaction with list/table state | `\plain` only resets character formatting, not paragraph or table state. Already scoped correctly |
| Too many inline styles in HTML | Only emit styles when values are non-default. Consider a CSS class approach for common combinations (future optimization) |

## 11. Dependencies

No new crate dependencies. All parsing is string manipulation. `Color` type already exists in the IR.
