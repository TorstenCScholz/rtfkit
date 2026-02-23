# rtfkit

RTF parsing toolkit with a CLI-first workflow.

[![CI](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml)

## Current Status (Phase 6)

rtfkit provides a complete RTF-to-DOCX, RTF-to-HTML, and RTF-to-PDF conversion pipeline with:

- **Text extraction** with formatting preservation (bold, italic, underline, alignment)
- **List support** - bullet and decimal lists with nested levels (up to 8)
- **Table support** - rows, cells, horizontal/vertical merges, cell alignment
- **HTML output** - semantic-first HTML5 output with `--to html`
- **PDF output** - In-process PDF generation via embedded Typst renderer with `--to pdf`
- **Style profiles** - Consistent cross-format styling with `--style-profile` (classic, report, compact)
- **Conversion reports** - JSON or text format with warnings and statistics
- **IR emission** - `--emit-ir` for snapshot/debug workflows
- **Parser limits** - safety limits for input size, depth, and warnings
- **Strict mode** - fail on dropped content for quality assurance

See [RTF Feature Overview](docs/rtf-feature-overview.md) for supported vs. not-yet-supported features.

## Install

From source:

```sh
cargo install --path crates/rtfkit-cli
```

Or download a pre-built binary from [Releases](https://github.com/TorstenCScholz/rtfkit/releases).

## Usage

```sh
# Convert RTF to DOCX (default output format)
rtfkit convert input.rtf -o output.docx

# Convert RTF to HTML
rtfkit convert input.rtf --to html -o output.html

# Convert and overwrite existing output file
rtfkit convert input.rtf -o output.docx --force

# Human-readable report (stdout)
rtfkit convert fixtures/text_simple_paragraph.rtf

# JSON report (stdout)
rtfkit convert fixtures/text_simple_paragraph.rtf --format json

# Emit IR JSON to file
rtfkit convert fixtures/text_simple_paragraph.rtf --emit-ir out.json

# Strict mode: fail when dropped content is reported
rtfkit convert fixtures/mixed_complex.rtf --strict --format json
```

### HTML Output Options

Control CSS output with `--html-css`:

```bash
# Default: embed built-in CSS
rtfkit convert document.rtf --to html --output document.html

# No built-in CSS (for custom styling)
rtfkit convert document.rtf --to html --html-css none --output document.html

# Append custom CSS
rtfkit convert document.rtf --to html --html-css-file custom.css --output document.html
```

### Style Profiles

Use `--style-profile` for consistent styling across HTML and PDF outputs:

```bash
# Use the report profile (default) - optimized for long-form documents
rtfkit convert document.rtf --to html --style-profile report --output document.html

# Use the classic profile - conservative, neutral styling
rtfkit convert document.rtf --to pdf --style-profile classic --output document.pdf

# Use the compact profile - dense styling for enterprise output
rtfkit convert document.rtf --to pdf --style-profile compact --output document.pdf
```

Built-in profiles: `classic`, `report` (default), `compact`

See [HTML Styling Reference](docs/reference/html-styling.md) for CSS classes and customization options.

### PDF Output Options

Convert RTF to PDF with `--to pdf`:

```bash
# Basic PDF conversion
rtfkit convert document.rtf --to pdf --output document.pdf

# US Letter page size
rtfkit convert document.rtf --to pdf --pdf-page-size letter --output document.pdf
```

PDF output uses an embedded Typst renderer - no external dependencies required. The output is deterministic and works completely offline.

See [PDF Output Reference](docs/reference/pdf-output.md) for PDF-specific options and [PDF Determinism](docs/reference/pdf-determinism.md) for determinism guarantees.

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Parse/validation error (invalid RTF or limit violation) |
| 3 | Writer/IO failure (e.g., cannot write output file) |
| 4 | Strict-mode violation (dropped content detected) |

### Parser Limits

For safety, the parser enforces these limits (see [Limits Policy](docs/limits-policy.md)):

| Limit | Default | Purpose |
|-------|---------|---------|
| Maximum input size | 10 MB | Prevents memory exhaustion |
| Maximum group depth | 256 levels | Prevents stack overflow |
| Maximum warnings | 1000 | Prevents unbounded memory growth |
| Maximum rows per table | 10,000 | Prevents resource exhaustion |
| Maximum cells per row | 1,000 | Prevents resource exhaustion |
| Maximum merge span | 1,000 cells | Limits merged regions |

## Output Contract

### Report JSON

```json
{
  "warnings": [],
  "stats": {
    "paragraph_count": 1,
    "run_count": 1,
    "bytes_processed": 29,
    "duration_ms": 0
  }
}
```

### Warning Types

| Type | Meaning | Strict Mode |
|------|---------|-------------|
| `unsupported_control_word` | RTF control not yet implemented | No failure |
| `unknown_destination` | RTF destination skipped | No failure |
| `dropped_content` | Content could not be represented | **Fails** |
| `unsupported_list_control` | List control not fully supported | No failure |
| `unresolved_list_override` | List reference not found | **Fails** |
| `unsupported_nesting_level` | List level > 8 clamped | No failure |
| `unsupported_table_control` | Table control not mapped | No failure |
| `malformed_table_structure` | Table structure issue | May fail |
| `unclosed_table_cell` | Missing `\cell` terminator | May fail |
| `unclosed_table_row` | Missing `\row` terminator | May fail |
| `merge_conflict` | Merge semantics conflict | **Fails** |
| `table_geometry_conflict` | Invalid table geometry | **Fails** |

See [Warning Reference](docs/warning-reference.md) for detailed documentation.

### IR JSON (`--emit-ir`)

Paragraph example:
```json
{
  "blocks": [
    {
      "type": "paragraph",
      "alignment": "left",
      "runs": [
        {
          "text": "Hello World",
          "bold": false,
          "italic": false,
          "underline": false
        }
      ]
    }
  ]
}
```

List example:
```json
{
  "blocks": [
    {
      "type": "listblock",
      "list_id": 1,
      "kind": "bullet",
      "items": [
        {
          "level": 0,
          "blocks": [
            {
              "type": "paragraph",
              "alignment": "left",
              "runs": [{"text": "First item", "bold": false, "italic": false, "underline": false}]
            }
          ]
        }
      ]
    }
  ]
}
```

Table example:
```json
{
  "blocks": [
    {
      "type": "tableblock",
      "rows": [
        {
          "cells": [
            {
              "blocks": [{"type": "paragraph", "alignment": "left", "runs": [{"text": "Cell 1"}]}],
              "width_twips": 1440
            }
          ]
        }
      ]
    }
  ]
}
```

## Development

```sh
# Test workspace
cargo test --all

# Update golden snapshots
UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt --all
```

## Testing

rtfkit has comprehensive test coverage:

- **Contract tests** - Exit codes, strict mode, warning semantics
- **Determinism tests** - IR/report/DOCX stability verification
- **Limits tests** - Safety and resource protection
- **Golden tests** - IR snapshot validation
- **DOCX integration tests** - End-to-end conversion verification

See [CONTRIBUTING.md](CONTRIBUTING.md) for the fixture-first contribution workflow.

## Limitations (v0.6)

- Partial RTF coverage (focused on common text/style cases)
- No images as first-class IR blocks
- No hyperlinks/fields as first-class output
- DOCX output supports basic text formatting, lists, and tables
- HTML output is semantic-first, not pixel-perfect (no font sizes, colors, or borders)
- PDF output uses embedded fonts (no custom font support)
- List nesting limited to 8 levels (DOCX compatibility)
- Row alignment and indent not fully supported by docx-rs (cosmetic loss only)

For up-to-date support details, see [RTF Feature Overview](docs/rtf-feature-overview.md) and [Feature Support Matrix](docs/feature-support.md).

## Documentation

- [Architecture Overview](docs/arch/README.md)
- [RTF Feature Overview](docs/rtf-feature-overview.md)
- [Feature Support Matrix](docs/feature-support.md)
- [HTML Styling Reference](docs/reference/html-styling.md)
- [PDF Output Reference](docs/reference/pdf-output.md)
- [PDF Determinism Guarantees](docs/reference/pdf-determinism.md)
- [Warning Reference](docs/warning-reference.md)
- [Limits Policy](docs/limits-policy.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

### Migration Notes

- **PDF rendering** is now in-process via `rtfkit-render-typst` crate (no external CLI required)
- **`--pdf-backend` flag** has been removed (single backend now)
- **`--keep-intermediate` flag** has been removed (in-process rendering)

## License

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
