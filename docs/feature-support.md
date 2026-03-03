# Feature Support Matrix

This document provides a comprehensive overview of current RTF feature support in rtfkit.

## Support Levels

| Level | Description |
|-------|-------------|
| ✅ Supported | Fully implemented and tested |
| ⚠️ Partial | Implemented with limitations |
| 🔸 Degraded | Handled gracefully with warnings |
| ❌ Not Supported | Not yet implemented |

## RTF Content Features

### Text and Paragraphs

| Feature | Support | Notes |
|---------|---------|-------|
| Plain text extraction | ✅ Supported | Core functionality |
| Paragraph breaks (`\par`) | ✅ Supported | Creates new paragraph blocks |
| Line breaks (`\line`) | ✅ Supported | Creates line break within paragraph |
| Unicode text (`\uN`) | ✅ Supported | With `\ucN` skip count handling |
| Escaped symbols (`\\`, `\{`, `\}`) | ✅ Supported | Preserved as text |

### Text Formatting

| Feature | Support | Notes |
|---------|---------|-------|
| Bold (`\b`) | ✅ Supported | Mapped to DOCX `<w:b/>` |
| Italic (`\i`) | ✅ Supported | Mapped to DOCX `<w:i/>` |
| Underline (`\ul`, `\ulnone`) | ✅ Supported | Mapped to DOCX `<w:u/>` |
| Paragraph alignment (`\ql`, `\qc`, `\qr`, `\qj`) | ✅ Supported | Mapped to DOCX `<w:jc/>` |
| Font family (`\fN`, `\fonttbl`, `\deffN`) | ✅ Supported | Mapped to DOCX `<w:rFonts>`, HTML `font-family`, Typst `#text(font: ...)` |
| Font size (`\fsN`) | ✅ Supported | Mapped to DOCX `<w:sz>`, HTML `font-size`, Typst `#text(size: ...)` |
| Text color (`\cfN`, `\colortbl`) | ✅ Supported | Mapped to DOCX `<w:color>`, HTML `color`, Typst `#text(fill: ...)` |
| Background color (`\cbN`, `\highlightN`) | ✅ Supported | Mapped to DOCX `<w:shd>`, HTML `background-color`, Typst `#highlight(fill: ...)`; `\highlight` takes precedence over `\cb` |
| Paragraph shading (`\cbpatN`) | ✅ Supported | Block-level background for paragraphs; mapped to DOCX `<w:shd>`, HTML `background-color`; `\pard` resets, `\plain` does not |
| Paragraph shading pattern (`\shadingN`, `\cfpatN`) | ⚠️ Partial | Percentage patterns (0-10000) mapped to `ShadingPattern`; DOCX full support, HTML/Typst use deterministic blended approximation; non-percent patterns still degrade to flat fill |
| Formatting reset (`\plain`) | ✅ Supported | Resets character formatting to defaults (including background/highlight) |
| Strikethrough (`\strike`) | ✅ Supported | Mapped to DOCX `<w:strike>`, HTML `<span class="rtf-s">`, Typst `#strike[...]` |
| Small caps (`\scaps`) | ⚠️ Partial | HTML and Typst map small caps directly; DOCX degrades to `<w:caps>` (all caps) because `docx-rs` has no `w:smallCaps` setter |
| All caps (`\caps`) | ✅ Supported | Mapped to DOCX `<w:caps>`, HTML `<span class="rtf-ac">`, Typst `#upper[...]` |

### Lists

| Feature | Support | Notes |
|---------|---------|-------|
| Bullet lists | ✅ Supported | Via `\lsN` with list table |
| Decimal/ordered lists | ✅ Supported | Via `\lsN` with list table |
| Nested lists | ✅ Supported | Up to 8 levels (DOCX limit) |
| List level (`\ilvlN`) | ✅ Supported | Clamped to 0-8 |
| List table parsing | ✅ Supported | `\listtable` and `\listoverridetable` |
| Legacy paragraph numbering (`\pn...`) | 🔸 Degraded | Dropped with `DroppedContent` warning |
| Mixed list kinds | ✅ Supported | Fallback to `Mixed` kind |

### Tables

| Feature | Support | Notes |
|---------|---------|-------|
| Basic table structure | ✅ Supported | `\trowd`, `\cellxN`, `\intbl`, `\cell`, `\row` |
| Multiple rows | ✅ Supported | Unlimited rows (within limits) |
| Multiple columns | ✅ Supported | Unlimited cells (within limits) |
| Cell content | ✅ Supported | Paragraphs and lists in cells |
| Cell width (`\cellxN`) | ✅ Supported | Stored in `width_twips` |
| Horizontal merge (`\clmgf`, `\clmrg`) | ✅ Supported | Mapped to DOCX `gridSpan` |
| Vertical merge (`\clvmgf`, `\clvmrg`) | ✅ Supported | Mapped to DOCX `vMerge` |
| Cell vertical alignment (`\clvertalt`, etc.) | ✅ Supported | Mapped to DOCX `vAlign` |
| Row alignment (`\trql`, `\trqc`, `\trqr`) | ⚠️ Partial | Parsed and emitted as table-level layout defaults; per-row variance may be normalized |
| Row indent (`\trleft`) | ⚠️ Partial | Parsed and emitted as table-level indent defaults; per-row variance may be normalized |
| Cell shading (`\clcbpatN`) | ✅ Supported | Cell background color; precedence: cell > row > table |
| Cell shading pattern (`\clshdngN`, `\clcfpatN`) | ⚠️ Partial | Percentage patterns use deterministic blended approximation in HTML/Typst; DOCX full support; non-percent patterns still degrade to flat fill |
| Row shading (`\trcbpatN`) | ✅ Supported | Default shading for cells without explicit shading |
| Table shading | ✅ Supported | Fallback shading from first row's `\trcbpatN` |
| Nested tables | ❌ Not Supported | Warning emitted |
| Table borders | ✅ Supported | Cell, row, and table-level borders; styles (solid, double, dotted, dashed, none); DOCX, HTML (CSS), Typst (`stroke:`) |

### Embedded Images

rtfkit supports embedded PNG and JPEG images from RTF `\pict` groups.

#### Supported Formats

| Format | Control Word | Support |
|--------|-------------|---------|
| PNG | `\pngblip` | Full support |
| JPEG | `\jpegblip` | Full support |
| WMF | `\wmetafile` | Not supported (dropped) |
| EMF | `\emfblip` | Not supported (dropped) |

#### Supported Controls

| Control | Description | Support |
|---------|-------------|---------|
| `\picwgoal` | Goal width in twips | Full support |
| `\pichgoal` | Goal height in twips | Full support |
| `\picw` | Original width in twips | Fallback |
| `\pich` | Original height in twips | Fallback |
| `\picscalex` | Horizontal scale percentage | Full support |
| `\picscaley` | Vertical scale percentage | Full support |
| `\shppict` | Shape picture (preferred) | Handled |
| `\nonshppict` | Non-shape picture | Handled |

#### Output Formats

- **DOCX**: Images embedded as media files with DrawingML references
- **HTML**: Images as data URIs in `<figure class="rtf-image">` elements
- **PDF/Typst**: Images mapped to deterministic in-memory assets

#### Limitations

- Images are block-level elements (not inline)
- WMF/EMF vector formats are not supported
- Image cropping is not supported
- Floating/anchored positioning is not supported

### Destinations

| Destination | Support | Notes |
|-------------|---------|-------|
| Document body | ✅ Supported | Main content |
| Font table (`\fonttbl`) | ✅ Supported | Parsed for font family mapping |
| Color table (`\colortbl`) | ✅ Supported | Parsed for color mapping |
| List table (`\listtable`) | ✅ Supported | Parsed for list definitions |
| List override table (`\listoverridetable`) | ✅ Supported | Parsed for list references |
| Header (`\header`) | ✅ Supported | Default channel; DOCX, HTML full; Typst ⚠️ partial (default only, first/even emit `PartialSupport`) |
| Header variants (`\headerl`, `\headerr`, `\headerf`) | ⚠️ Partial | Parsed; DOCX/HTML full; Typst degrades to `PartialSupport` |
| Footer (`\footer`) | ✅ Supported | Default channel; DOCX, HTML full; Typst ⚠️ partial (default only, first/even emit `PartialSupport`) |
| Footer variants (`\footerl`, `\footerr`, `\footerf`) | ⚠️ Partial | Parsed; DOCX/HTML full; Typst degrades to `PartialSupport` |
| Footnote (`\footnote`) | ✅ Supported | Emitted as `NoteRef` inline + note body; DOCX, HTML, Typst |
| Endnote (`\endnote`) | ✅ Supported | Emitted as `NoteRef` inline + note body; DOCX, HTML, Typst |
| Picture (`\pict`) | ⚠️ Partial | PNG/JPEG supported; WMF/EMF dropped |
| Object (`\obj`) | 🔸 Degraded | Dropped with `DroppedContent` |
| Field (`\field`) | ⚠️ Partial | HYPERLINK (external + internal via `\l` switch), page fields (`PAGE`, `NUMPAGES`, `SECTIONPAGES`, `PAGEREF`), TOC markers, and `\bkmkstart` anchors supported; unsupported fields preserve result text with warnings |
| Unknown destinations (`\*\foo`) | 🔸 Degraded | Dropped with `DroppedContent` |

## Output Formats

| Format | Support | Notes |
|--------|---------|-------|
| DOCX | ✅ Supported | Primary output format (default) |
| HTML | ✅ Supported | Via `--to html` flag; semantic-first output |
| PDF | ✅ Supported | Via `--to pdf` flag; in-process rendering (no external dependencies) |
| IR JSON | ✅ Supported | Via `--emit-ir` flag |
| Report JSON | ✅ Supported | Via `--format json` |
| Report Text | ✅ Supported | Default output |

### Style Profiles

Style profiles provide consistent visual styling across output formats:

| Format | Style Profiles | Notes |
|--------|----------------|-------|
| HTML | ✅ Supported | CSS variables generated from profile |
| PDF | ✅ Supported | Typst preamble generated from profile |
| DOCX | ✅ Supported | `--style-profile` is supported for DOCX output (opt-in) |

Built-in profiles: `classic`, `report` (default), `compact`

### HTML Output Details

HTML output is selected with `--to html` and produces semantic HTML5:

| Feature | HTML Support | Notes |
|---------|--------------|-------|
| Paragraphs | ✅ Supported | `<p>` elements |
| Bold text | ✅ Supported | `<strong>` elements |
| Italic text | ✅ Supported | `<em>` elements |
| Underline text | ✅ Supported | `<span class="rtf-u">` |
| Text alignment | ✅ Supported | CSS classes (e.g., `rtf-align-center`) |
| Bullet lists | ✅ Supported | `<ul>` elements |
| Ordered lists | ✅ Supported | `<ol>` elements |
| Nested lists | ✅ Supported | Nested `<ul>`/`<ol>` |
| Tables | ✅ Supported | `<table>` with proper structure |
| Horizontal merges | ✅ Supported | `colspan` attribute |
| Vertical merges | ✅ Supported | `rowspan` attribute |
| Cell alignment | ✅ Supported | CSS classes |
| Hyperlinks | ✅ Supported | `<a href>` with `rtf-link` class |
| Font family | ✅ Supported | Inline `font-family` style (sanitized) |
| Font size | ✅ Supported | Inline `font-size` style (pt) |
| Colors | ✅ Supported | Inline `color` style (hex) |
| Background color | ✅ Supported | Inline `background-color` style (hex) |
| Borders | ✅ Supported | Table/cell border CSS emitted from IR border model |
| Images | ✅ Supported | `<figure class="rtf-image">` with data URI |
| Style Profiles | ✅ Supported | `--style-profile` flag (classic, report, compact) |

### PDF Output Details

PDF output is selected with `--to pdf` and produces PDF via the embedded Typst renderer:

| Feature | PDF Support | Notes |
|---------|-------------|-------|
| Paragraphs | ✅ Supported | With inline formatting |
| Bold text | ✅ Supported | Mapped to Typst emphasis |
| Italic text | ✅ Supported | Mapped to Typst emphasis |
| Underline text | ✅ Supported | Mapped to Typst underline |
| Text alignment | ✅ Supported | Left, center, right, justify |
| Bullet lists | ✅ Supported | Typst list syntax |
| Ordered lists | ✅ Supported | Typst numbered list syntax |
| Nested lists | ✅ Supported | Up to 8 levels |
| Tables | ✅ Supported | With cell merging |
| Horizontal merges | ✅ Supported | Colspan in table cells |
| Vertical merges | ✅ Supported | Rowspan in table cells |
| Unicode text | ✅ Supported | Full Unicode support |
| Page size options | ✅ Supported | A4 (default) and US Letter |
| Deterministic output | ✅ Supported | Byte-identical for same input |
| Hyperlinks | ✅ Supported | Typst `#link()` syntax |
| Font family | ✅ Supported | Typst `#text(font: ...)` wrapper |
| Font size | ✅ Supported | Typst `#text(size: ...)` wrapper |
| Text color | ✅ Supported | Typst `#text(fill: ...)` wrapper |
| Background color | ✅ Supported | Typst `#highlight(fill: ...)` wrapper |
| Images | ✅ Supported | PNG/JPEG mapped via deterministic in-memory assets; malformed payloads are dropped with warnings |
| Custom fonts | ❌ Not Supported | Uses embedded fonts |
| Style Profiles | ✅ Supported | `--style-profile` flag (classic, report, compact) |

## Safety Features

| Feature | Support | Notes |
|---------|---------|-------|
| Input size limit | ✅ Supported | Default: 10 MB |
| Group depth limit | ✅ Supported | Default: 256 levels |
| Warning count limit | ✅ Supported | Default: 1000 warnings |
| Table row limit | ✅ Supported | Default: 10,000 rows |
| Table cell limit | ✅ Supported | Default: 1,000 cells/row |
| Merge span limit | ✅ Supported | Default: 1,000 cells |
| Image byte limit | ✅ Supported | Default: 50 MiB cumulative |
| Strict mode | ✅ Supported | Exit code 4 on dropped content |

## Error Handling

| Feature | Support | Notes |
|---------|---------|-------|
| Invalid RTF header | ✅ Supported | Exit code 2 |
| Unclosed groups | ✅ Supported | Exit code 2 |
| Malformed tables | ✅ Supported | Recovery with warnings |
| Malformed lists | ✅ Supported | Recovery with warnings |
| Unresolved list references | ✅ Supported | Warning + strict mode failure |
| Merge conflicts | ✅ Supported | Deterministic resolution |
| Limit violations | ✅ Supported | Exit code 2, no partial output |

## Known Limitations

1. **Limited image format support** - PNG and JPEG are supported; WMF/EMF are dropped
2. **List nesting limit** - Maximum 8 levels due to DOCX compatibility
3. **No nested tables in RTF parser path** - Tables inside cells are not yet parsed from RTF input
4. **PDF uses embedded fonts** - Custom fonts are not supported
5. **DOCX small caps degradation** - `\scaps` degrades to all caps in DOCX output due to `docx-rs` API limitations
6. **Row layout variance normalization** - Row alignment/indent controls are represented with table-level defaults when rows disagree

## Version History

| Version | Changes |
|---------|---------|
| 0.16.0 | Added strikethrough (`\strike`), small caps (`\scaps`), and all caps (`\caps`) mapping across outputs (DOCX small-caps fallback); added page-management fields and section numbering |
| 0.15.0 | Added table borders (cell, row, and table-level; solid/double/dotted/dashed/none styles) for DOCX, HTML, Typst |
| 0.14.0 | Added document structure: headers, footers (default/first/even channels), footnotes, endnotes for DOCX, HTML, Typst |
| 0.13.0 | Added bookmark anchors (`\bkmkstart`) and internal hyperlinks (`\l` switch) for DOCX, HTML, Typst; added `UnsupportedField` warning for unrecognized field types |
| 0.12.0 | Added table layout fidelity: alignment, indent, gap (`\trgaph`), preferred widths, row height, cell and row padding for DOCX, HTML, Typst |
| 0.11.0 | Added embedded image support (PNG/JPEG) for DOCX, HTML, PDF output; image byte limit |
| 0.10.0 | Added block shading support (`\cbpatN`, `\clcbpatN`, `\trcbpatN`) for paragraphs and tables; theme color resolution; pattern support with degradation |
| 0.9.0 | Added background/highlight color support (`\cbN`, `\highlightN`) for DOCX, HTML, PDF; `\plain` now resets background/highlight |
| 0.8.0 | Added font family, font size, and foreground color support (DOCX, HTML, PDF); added `\plain` reset support |
| 0.7.0 | Added hyperlink support (DOCX, HTML, PDF), URL sanitization for HTML output |
| 0.6.0 | Added HTML output support, PDF output support, feature support matrix documentation |
| 0.5.0 | Added table merge and alignment support |
| 0.4.0 | Added basic table support |
| 0.3.0 | Added list support |
| 0.2.0 | Added DOCX output |
| 0.1.0 | Initial text extraction |

## Related Documentation

- [RTF Feature Overview](rtf-feature-overview.md)
- [HTML Styling Reference](reference/html-styling.md)
- [PDF Output Reference](reference/pdf-output.md)
- [PDF Determinism Guarantees](reference/pdf-determinism.md)
- [Warning Reference](warning-reference.md)
- [Limits Policy](limits-policy.md)
- [Architecture Overview](arch/README.md)
