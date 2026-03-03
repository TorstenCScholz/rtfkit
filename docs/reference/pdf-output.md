# PDF Output

rtfkit supports converting RTF documents to PDF using an embedded Typst renderer. This provides in-process PDF generation with no external dependencies.

## Architecture

PDF output uses the `rtfkit-render-typst` crate, which embeds the Typst typesetting system directly. This means:

- **No external CLI required** - Everything runs in-process
- **Offline capable** - No network access needed
- **Single binary** - No separate Typst installation
- **Deterministic output** - Same input always produces identical PDFs

## Basic Usage

```bash
rtfkit convert input.rtf --to pdf --output output.pdf
```

Or using the short flag:

```bash
rtfkit convert input.rtf --to pdf -o output.pdf
```

## Style Profiles

PDF output uses **style profiles** to control typography, spacing, and layout. Style profiles ensure consistent visual identity across HTML, DOCX, and PDF outputs from the same source document.

### Built-in Profiles

Three built-in profiles are available:

| Profile | Description |
|---------|-------------|
| `classic` | Conservative, neutral styling close to original RTF appearance |
| `report` | Strong visual hierarchy optimized for long-form documents (default) |
| `compact` | Dense styling for enterprise output with reduced whitespace |

### CLI Usage

Use the `--style-profile` flag to select a profile:

```bash
# Use the report profile (default)
rtfkit convert document.rtf --to pdf --output document.pdf

# Use the classic profile
rtfkit convert document.rtf --to pdf --style-profile classic --output document.pdf

# Use the compact profile
rtfkit convert document.rtf --to pdf --style-profile compact --output document.pdf
```

### Typst Preamble Generation

When generating PDF output, the style profile is converted to a Typst preamble that defines:

- **Typography**: Body text font, size, and line height; heading fonts and sizes
- **Heading hierarchy**: Spacing above and below headings, font weights
- **List styling**: Indentation steps, item spacing, marker styles
- **Table styling**: Border widths, cell padding, header emphasis, row striping
- **Page layout**: Margins, content width

The `report` profile provides particularly noticeable improvements for PDF output:
- Clear visual hierarchy with distinct heading levels
- Improved table readability with header styling
- Consistent list indentation and spacing
- Professional page margins

### Cross-Format Consistency

Using the same style profile across formats ensures visual consistency:

```bash
# Generate both outputs with the same profile
rtfkit convert document.rtf --to html --style-profile report --output document.html
rtfkit convert document.rtf --to docx --style-profile report --output document.docx
rtfkit convert document.rtf --to pdf --style-profile report --output document.pdf
```

## PDF-Specific Options

### `--pdf-page-size <size>`

Set the page size for the output PDF:

| Value | Description |
|-------|-------------|
| `a4` (default) | A4 paper size (210mm × 297mm) |
| `letter` | US Letter paper size (8.5" × 11") |

Example:

```bash
rtfkit convert document.rtf --to pdf --pdf-page-size letter -o output.pdf
```

### `--fixed-timestamp <RFC3339>`

Set a fixed timestamp for deterministic PDF metadata.

Example:

```bash
rtfkit convert document.rtf --to pdf --fixed-timestamp "2024-01-01T00:00:00Z" -o output.pdf
```

## Examples

### Basic conversion

```bash
rtfkit convert document.rtf --to pdf -o document.pdf
```

### US Letter page size

```bash
rtfkit convert document.rtf --to pdf --pdf-page-size letter -o document.pdf
```

### Strict mode

```bash
rtfkit convert document.rtf --to pdf --strict -o document.pdf
```

Exits with code 4 if any semantic content is dropped during conversion.

### Combined with IR emission

```bash
rtfkit convert document.rtf --to pdf --emit-ir document.json -o document.pdf
```

## Supported Content

| Feature | Support |
|---------|---------|
| Paragraphs with inline formatting | ✅ Supported |
| Bold text | ✅ Supported |
| Italic text | ✅ Supported |
| Underline text | ✅ Supported |
| Text alignment (left, center, right, justify) | ✅ Supported |
| Bullet lists | ✅ Supported |
| Decimal/ordered lists | ✅ Supported |
| Nested lists (up to 8 levels) | ✅ Supported |
| Tables | ✅ Supported |
| Horizontal cell merges | ✅ Supported |
| Vertical cell merges | ✅ Supported |
| Unicode text | ✅ Supported |
| Page size options | ✅ Supported |
| Images | ✅ Supported | PNG/JPEG images mapped via deterministic in-memory assets |
| Hyperlinks | ✅ Supported | Typst `#link()` syntax; external URLs and internal bookmarks |
| Custom fonts | ❌ Not Supported (uses embedded fonts) |

## Determinism

PDF output is designed to be deterministic. With the same input and options, the output PDF will be byte-identical across runs. This is critical for:

- **Reproducible builds** - Same source always produces same PDF
- **Version control** - PDFs can be diffed and tracked
- **CI/CD pipelines** - Consistent output across environments

For details on determinism guarantees and how to ensure reproducible output, see [PDF Determinism Guarantees](pdf-determinism.md).

## Limitations

- **Embedded fonts only** - Uses Typst's embedded fonts; custom fonts not supported
- **Advanced typography** - Kerning and ligatures use Typst defaults

## Troubleshooting

### PDF generation fails

1. Check the error message for details
2. Try with `--strict` to see if content is being dropped
3. Use `--emit-ir` to inspect the intermediate representation

### Content appears different from original RTF

PDF output prioritizes readability over pixel-perfect fidelity. Some formatting differences are expected:

- Font substitution may occur (embedded fonts used)
- Complex table layouts may be simplified
- Page breaks are determined by Typst's layout engine

### Strict mode exits with code 4

Semantic content was dropped during conversion. Check the warning messages for details. Common causes:

- Mixed list kinds (converted to bullet list)
- Unsupported formatting features
- Malformed RTF structures

### Different output on different machines

Ensure you're using the same version of rtfkit. The embedded Typst version is included in the PDF metadata, and output may differ between versions.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Parse/validation failure (invalid input or options) |
| 3 | Writer/IO failure (PDF generation error) |
| 4 | Strict-mode violation (semantic content dropped) |

## Related Documentation

- [PDF Determinism Guarantees](pdf-determinism.md) - Details on reproducible output
- [Feature Support Matrix](../feature-support.md) - Overview of all supported features
- [Warning Reference](../warning-reference.md) - Warning types and meanings
