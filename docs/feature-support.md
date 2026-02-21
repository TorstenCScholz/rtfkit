# Feature Support Matrix

This document provides a comprehensive overview of RTF feature support in rtfkit (Phase 6).

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
| Font family (`\fN`) | ❌ Not Supported | Parsed but not mapped |
| Font size (`\fsN`) | ❌ Not Supported | Parsed but not mapped |
| Text color (`\cfN`) | ❌ Not Supported | Parsed but not mapped |
| Background color (`\cbN`, `\highlightN`) | ❌ Not Supported | Parsed but not mapped |
| Strikethrough (`\strike`) | ❌ Not Supported | Warning emitted |
| Small caps (`\scaps`) | ❌ Not Supported | Warning emitted |
| All caps (`\caps`) | ❌ Not Supported | Warning emitted |

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
| Row alignment (`\trql`, `\trqc`, `\trqr`) | ⚠️ Partial | Parsed but not fully emitted by docx-rs |
| Row indent (`\trleft`) | ⚠️ Partial | Parsed but not fully emitted by docx-rs |
| Nested tables | ❌ Not Supported | Warning emitted |
| Table borders | ❌ Not Supported | Parsed but not mapped |
| Cell shading | ❌ Not Supported | Parsed but not mapped |

### Destinations

| Destination | Support | Notes |
|-------------|---------|-------|
| Document body | ✅ Supported | Main content |
| Font table (`\fonttbl`) | 🔸 Degraded | Skipped, fonts not mapped |
| Color table (`\colortbl`) | 🔸 Degraded | Skipped, colors not mapped |
| List table (`\listtable`) | ✅ Supported | Parsed for list definitions |
| List override table (`\listoverridetable`) | ✅ Supported | Parsed for list references |
| Header (`\header`) | 🔸 Degraded | Dropped with warning |
| Footer (`\footer`) | 🔸 Degraded | Dropped with warning |
| Picture (`\pict`) | 🔸 Degraded | Dropped with `DroppedContent` |
| Object (`\obj`) | 🔸 Degraded | Dropped with `DroppedContent` |
| Field (`\field`) | 🔸 Degraded | Dropped with `DroppedContent` |
| Unknown destinations (`\*\foo`) | 🔸 Degraded | Dropped with `DroppedContent` |

## Output Formats

| Format | Support | Notes |
|--------|---------|-------|
| DOCX | ✅ Supported | Primary output format (default) |
| HTML | ✅ Supported | Via `--to html` flag; semantic-first output |
| IR JSON | ✅ Supported | Via `--emit-ir` flag |
| Report JSON | ✅ Supported | Via `--format json` |
| Report Text | ✅ Supported | Default output |
| PDF | ❌ Not Supported | Planned for future phase |

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
| Font family/size | ❌ Not Supported | Semantic-first design |
| Colors | ❌ Not Supported | Semantic-first design |
| Borders | ❌ Not Supported | Semantic-first design |
| Images | ❌ Not Supported | No IR image blocks |

## Safety Features

| Feature | Support | Notes |
|---------|---------|-------|
| Input size limit | ✅ Supported | Default: 10 MB |
| Group depth limit | ✅ Supported | Default: 256 levels |
| Warning count limit | ✅ Supported | Default: 1000 warnings |
| Table row limit | ✅ Supported | Default: 10,000 rows |
| Table cell limit | ✅ Supported | Default: 1,000 cells/row |
| Merge span limit | ✅ Supported | Default: 1,000 cells |
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

1. **No image support** - Images are dropped with `DroppedContent` warning
2. **No hyperlink support** - Fields are dropped with `DroppedContent` warning
3. **Limited font/style fidelity** - Font family, size, and color are not mapped to output
4. **Row alignment cosmetic loss** - Row alignment is parsed but not fully emitted by docx-rs
5. **List nesting limit** - Maximum 8 levels due to DOCX compatibility
6. **No nested tables** - Tables inside cells are not supported
7. **HTML is semantic-first** - No font sizes, colors, or borders in HTML output

## Version History

| Version | Changes |
|---------|---------|
| 0.6.0 | Added HTML output support, feature support matrix documentation |
| 0.5.0 | Added table merge and alignment support |
| 0.4.0 | Added basic table support |
| 0.3.0 | Added list support |
| 0.2.0 | Added DOCX output |
| 0.1.0 | Initial text extraction |

## Related Documentation

- [RTF Feature Overview](rtf-feature-overview.md)
- [Warning Reference](warning-reference.md)
- [Limits Policy](limits-policy.md)
- [Architecture Overview](arch/README.md)
