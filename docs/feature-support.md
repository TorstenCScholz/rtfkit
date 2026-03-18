# Feature Support Matrix

This document describes the current RTF feature support in `rtfkit` 1.0.0.

For a shorter summary, see [RTF Feature Overview](rtf-feature-overview.md).

## Support levels

| Level | Meaning |
|-------|---------|
| ✅ Supported | Implemented and covered by tests |
| ⚠️ Partial | Implemented with known limitations or normalized behavior |
| 🔸 Degraded | Preserved as best as possible with warnings or fallback output |
| ❌ Not supported | Not currently implemented |

## RTF content features

### Text and paragraphs

| Feature | Support | Notes |
|---------|---------|-------|
| Plain text extraction | ✅ Supported | Core parser behavior |
| Paragraph breaks (`\par`) | ✅ Supported | Creates new paragraph blocks |
| Line breaks (`\line`) | ✅ Supported | Creates line breaks inside paragraphs |
| Unicode text (`\uN`) | ✅ Supported | Includes `\ucN` skip-count handling |
| Escaped symbols (`\\`, `\{`, `\}`) | ✅ Supported | Preserved as text |

### Text formatting

| Feature | Support | Notes |
|---------|---------|-------|
| Bold (`\b`) | ✅ Supported | DOCX, HTML, PDF |
| Italic (`\i`) | ✅ Supported | DOCX, HTML, PDF |
| Underline (`\ul`, `\ulnone`) | ✅ Supported | DOCX, HTML, PDF |
| Paragraph alignment (`\ql`, `\qc`, `\qr`, `\qj`) | ✅ Supported | DOCX, HTML, PDF |
| Font family (`\fN`, `\fonttbl`, `\deffN`) | ✅ Supported | Parsed and emitted across outputs |
| Font size (`\fsN`) | ✅ Supported | Parsed and emitted across outputs |
| Text color (`\cfN`, `\colortbl`) | ✅ Supported | Parsed and emitted across outputs |
| Background/highlight color (`\cbN`, `\highlightN`) | ✅ Supported | `\highlight` takes precedence |
| Paragraph shading (`\cbpatN`) | ✅ Supported | Block-level background shading |
| Paragraph shading patterns (`\shadingN`, `\cfpatN`) | ⚠️ Partial | DOCX support is stronger; HTML/PDF use deterministic approximations for some patterns |
| Strikethrough (`\strike`) | ✅ Supported | DOCX, HTML, PDF |
| Small caps (`\scaps`) | ⚠️ Partial | DOCX degrades to caps due to writer limitations |
| All caps (`\caps`) | ✅ Supported | DOCX, HTML, PDF |
| Formatting reset (`\plain`) | ✅ Supported | Resets character formatting to defaults |

### Lists

| Feature | Support | Notes |
|---------|---------|-------|
| Bullet lists | ✅ Supported | |
| Ordered/decimal lists | ✅ Supported | |
| Nested lists | ✅ Supported | Up to 8 levels |
| List level (`\ilvlN`) | ✅ Supported | Clamped to the supported nesting range |
| List table parsing | ✅ Supported | `\listtable` and `\listoverridetable` |
| Mixed list kinds | ✅ Supported | |
| Legacy paragraph numbering (`\pn...`) | 🔸 Degraded | Dropped with warnings |

### Tables

| Feature | Support | Notes |
|---------|---------|-------|
| Basic table structure | ✅ Supported | `\trowd`, `\cellxN`, `\intbl`, `\cell`, `\row` |
| Multiple rows and columns | ✅ Supported | Subject to parser limits |
| Cell content | ✅ Supported | Paragraphs, lists, nested tables, images |
| Cell widths (`\cellxN`) | ✅ Supported | |
| Horizontal merge (`\clmgf`, `\clmrg`) | ✅ Supported | |
| Vertical merge (`\clvmgf`, `\clvmrg`) | ✅ Supported | |
| Cell vertical alignment | ✅ Supported | |
| Row alignment (`\trql`, `\trqc`, `\trqr`) | ⚠️ Partial | May be normalized when rows disagree |
| Row indent (`\trleft`) | ⚠️ Partial | May be normalized when rows disagree |
| Cell shading (`\clcbpatN`) | ✅ Supported | |
| Cell shading patterns (`\clshdngN`, `\clcfpatN`) | ⚠️ Partial | Some patterns degrade in HTML/PDF |
| Row shading (`\trcbpatN`) | ✅ Supported | |
| Table-level shading fallback | ✅ Supported | Derived from row shading fallback rules |
| Nested tables | ✅ Supported | |
| Table borders | ✅ Supported | DOCX, HTML, PDF |

### Embedded images

| Feature | Support | Notes |
|---------|---------|-------|
| PNG (`\pngblip`) | ✅ Supported | |
| JPEG (`\jpegblip`) | ✅ Supported | |
| WMF (`\wmetafile`) | ❌ Not supported | Dropped with warnings |
| EMF (`\emfblip`) | ❌ Not supported | Dropped with warnings |
| Image sizing controls | ✅ Supported | `\picwgoal`, `\pichgoal`, `\picw`, `\pich` |
| Image scaling controls | ✅ Supported | `\picscalex`, `\picscaley` |
| Shape/non-shape picture groups | ✅ Supported | `\shppict`, `\nonshppict` |

Image limitations:

- images are block-level rather than inline/floating
- crop controls are not supported
- unsupported vector formats are dropped

### Destinations and structures

| Feature | Support | Notes |
|---------|---------|-------|
| Document body | ✅ Supported | |
| Font table (`\fonttbl`) | ✅ Supported | |
| Color table (`\colortbl`) | ✅ Supported | |
| List table (`\listtable`) | ✅ Supported | |
| List override table (`\listoverridetable`) | ✅ Supported | |
| Headers (`\header`) | ✅ Supported | |
| Header variants (`\headerl`, `\headerr`, `\headerf`) | ⚠️ Partial | PDF has more limitations than DOCX/HTML |
| Footers (`\footer`) | ✅ Supported | |
| Footer variants (`\footerl`, `\footerr`, `\footerf`) | ⚠️ Partial | PDF has more limitations than DOCX/HTML |
| Footnotes (`\footnote`) | ✅ Supported | |
| Endnotes (`\endnote`) | ✅ Supported | |
| Picture groups (`\pict`) | ⚠️ Partial | PNG/JPEG supported; WMF/EMF dropped |
| Objects (`\obj`) | 🔸 Degraded | Dropped with warnings |
| Fields (`\field`) | ⚠️ Partial | Practical subset with deterministic fallback behavior |
| Unknown destinations (`\*\foo`) | 🔸 Degraded | Skipped with warnings |

### Fields

| Field type | Support | Notes |
|-----------|---------|-------|
| `HYPERLINK` | ✅ Supported | External and internal links |
| `PAGE`, `NUMPAGES`, `SECTIONPAGES`, `PAGEREF` | ⚠️ Partial | Deterministic/static behavior rather than live pagination |
| TOC markers | ✅ Supported | |
| `REF`, `NOTEREF` | ✅ Supported | With fallback text for unresolved targets |
| `SEQ` | ⚠️ Partial | Deterministic fallback rendering |
| `DOCPROPERTY` and built-in properties | ⚠️ Partial | Deterministic fallback rendering |
| `MERGEFIELD` | ⚠️ Partial | Fallback/result text preserved |
| Unsupported field types | 🔸 Degraded | Visible result text preserved where possible |

## Output formats

| Format | Support | Notes |
|--------|---------|-------|
| DOCX | ✅ Supported | Default output format |
| HTML | ✅ Supported | Semantic HTML5 output |
| PDF | ✅ Supported | In-process renderer |
| IR JSON | ✅ Supported | `--emit-ir` |
| Report JSON | ✅ Supported | `--format json` |
| Report text | ✅ Supported | Default reporting output |

## Style profiles

| Output | Support | Notes |
|--------|---------|-------|
| HTML | ✅ Supported | CSS variables and built-in styles |
| PDF | ✅ Supported | Typst preamble generated from the profile |
| DOCX | ✅ Supported | Opt-in styling defaults via `--style-profile` |

Built-in profiles: `classic`, `report`, `compact`

## Safety features

| Feature | Support | Notes |
|---------|---------|-------|
| Input size limit | ✅ Supported | Default 10 MB |
| Group depth limit | ✅ Supported | Default 256 |
| Warning count limit | ✅ Supported | Default 1000 |
| Table row limit | ✅ Supported | Default 10,000 |
| Table cell limit | ✅ Supported | Default 1,000 |
| Merge span limit | ✅ Supported | Default 1,000 |
| Image byte limit | ✅ Supported | Default 50 MiB cumulative |
| Nested table depth limit | ✅ Supported | Default 16 |
| Strict mode | ✅ Supported | Exit code `4` on dropped content |

## Error handling

| Condition | Support | Notes |
|----------|---------|-------|
| Invalid RTF header | ✅ Supported | Exit code `2` |
| Unclosed groups | ✅ Supported | Exit code `2` |
| Malformed tables | ✅ Supported | Recovery where possible, warnings on degradation |
| Malformed lists | ✅ Supported | Recovery where possible, warnings on degradation |
| Unresolved list references | ✅ Supported | Warning; strict mode can fail |
| Merge conflicts | ✅ Supported | Deterministic handling |
| Limit violations | ✅ Supported | Exit code `2`, no partial output |

## Known limitations

1. Full visual parity with all historical RTF producers is not the goal.
2. WMF and EMF images are not supported.
3. Images are block-level only.
4. Dynamic field evaluation is intentionally not executed.
5. PDF output uses embedded fonts rather than custom font loading.
6. Some row-level table layout differences are normalized.
7. Some shading patterns degrade outside DOCX output.

## Related documentation

- [RTF Feature Overview](rtf-feature-overview.md)
- [HTML Styling Reference](reference/html-styling.md)
- [PDF Output Reference](reference/pdf-output.md)
- [PDF Determinism](reference/pdf-determinism.md)
- [Warning Reference](warning-reference.md)
- [Limits Policy](limits-policy.md)
- [Architecture Overview](arch/README.md)
