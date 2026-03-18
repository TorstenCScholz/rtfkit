# RTF Feature Overview

This document summarizes the currently supported RTF feature set in `rtfkit`.

For the exhaustive matrix, see [Feature Support Matrix](feature-support.md).

## Supported

- **Text and paragraph structure**
  - plain text extraction
  - paragraph breaks and line breaks
  - Unicode text handling
  - escaped RTF symbols
- **Inline and paragraph formatting**
  - bold, italic, underline, strikethrough
  - all caps
  - small caps with DOCX fallback behavior
  - font family, font size, foreground color
  - background/highlight color
  - paragraph shading and common shading cases
  - paragraph alignment
- **Lists**
  - bullet lists
  - ordered lists
  - nested lists up to 8 levels
- **Tables**
  - rows, cells, widths
  - horizontal and vertical merges
  - cell vertical alignment
  - nested tables
  - borders and shading
  - mixed content inside cells, including text, lists, nested tables, and images
- **Fields and navigation**
  - hyperlinks
  - bookmark anchors
  - page-related fields with deterministic/static behavior
  - TOC markers
  - semantic references and fallback handling
- **Document structure**
  - headers and footers
  - footnotes and endnotes
- **Embedded images**
  - PNG
  - JPEG
  - image sizing and scaling controls

## Partially supported or degraded

- **Some field types** use deterministic fallback/result text rather than full dynamic evaluation.
- **Some shading patterns** degrade outside DOCX output.
- **Header/footer variants** are stronger in DOCX/HTML than in PDF output.
- **Row-level table layout differences** may be normalized to preserve stable output.
- **Malformed table/list/image input** may be repaired or degraded with warnings rather than rejected immediately.
- **DOCX small caps** degrade to caps because of writer limitations.

## Not supported

- **WMF and EMF images**
- **Inline/floating images**
- **Image crop controls**
- **Custom PDF font loading**
- **Full pixel-perfect parity with every historical RTF producer**

## Notes

- In `--strict` mode, dropped content causes conversion to fail with exit code `4`.
- Parser safety limits apply to input size, nesting depth, warnings, tables, and images.
- Unsupported or degraded content is surfaced through warnings and conversion reports.

## Related documentation

- [Feature Support Matrix](feature-support.md)
- [Warning Reference](warning-reference.md)
- [Limits Policy](limits-policy.md)
- [Architecture Overview](arch/README.md)
