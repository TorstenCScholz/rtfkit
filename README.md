# rtfkit

Open-source RTF toolkit and RTF-to-PDF converter with a CLI-first workflow.
Convert `.rtf` files to PDF, DOCX, or HTML with a single command.

[![CI](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml)

## Current Status (Phase 6)

rtfkit provides a complete RTF-to-DOCX, RTF-to-HTML, and RTF-to-PDF conversion pipeline with:

- **Text extraction** with formatting preservation (bold, italic, underline, alignment)
- **List support** - bullet and decimal lists with nested levels (up to 8)
- **Table support** - rows, cells, nested tables, horizontal/vertical merges, cell alignment
- **Hyperlink support** - external URLs and internal bookmark links
- **Field support (pragmatic subset)** - HYPERLINK, page fields (`PAGE`, `NUMPAGES`, `SECTIONPAGES`, `PAGEREF`), TOC markers, semantic refs (`REF`, `NOTEREF`, `SEQ`, `DOCPROPERTY`, built-ins), and `MERGEFIELD` fallback rendering
- **Embedded image support** - PNG/JPEG images with size/scaling controls
- **HTML output** - semantic-first HTML5 output with `--to html`
- **PDF output** - In-process PDF generation via embedded Typst renderer with `--to pdf`
- **Style profiles** - Consistent cross-format styling with `--style-profile` (classic, report, compact)
- **Conversion reports** - JSON or text format with warnings and statistics
- **IR emission** - `--emit-ir` for snapshot/debug workflows
- **Parser limits** - safety limits for input size, depth, and warnings
- **Strict mode** - fail on dropped content for quality assurance

See [RTF Feature Overview](docs/rtf-feature-overview.md) for supported vs. not-yet-supported features.

## RTF to PDF Converter (Quick Start)

If you are looking for an RTF to PDF converter, this repository provides one via the `rtfkit` CLI:

```sh
rtfkit convert input.rtf --to pdf -o output.pdf
```

PDF conversion is in-process and offline-capable (no external PDF CLI required).

## Install

### Rust CLI (Original)

From source:

```sh
cargo install --path crates/rtfkit-cli
```

Or download a pre-built binary from [Releases](https://github.com/TorstenCScholz/rtfkit/releases).

### Python Bindings

#### From PyPI (Planned, not published yet)

```bash
# Will work after first PyPI release
pip install rtfkit
```

#### From Source

```bash
# Clone the repository
git clone https://github.com/TorstenCScholz/rtfkit.git
cd rtfkit/bindings/python

# Install with pip
pip install .
```

#### Development Installation

```bash
# Clone the repository
git clone https://github.com/TorstenCScholz/rtfkit.git
cd rtfkit/bindings/python

# Install in editable mode
pip install -e .
```

## Usage

### Rust CLI (Original)

```sh
# Convert RTF to DOCX (default output format)
rtfkit convert input.rtf -o output.docx

# Convert RTF to PDF
rtfkit convert input.rtf --to pdf -o output.pdf

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

### Python Bindings

```python
import rtfkit

# Parse RTF content
rtf_content = r"{\rtf1\ansi Hello \b World!}"
result = rtfkit.parse(rtf_content)

# Convert to different formats
html = rtfkit.to_html(result.document)
docx_bytes = rtfkit.to_docx_bytes(result.document)
pdf_bytes = rtfkit.to_pdf(result.document)

# Save to files
with open("output.html", "w") as f:
    f.write(html)

with open("output.docx", "wb") as f:
    f.write(docx_bytes)

with open("output.pdf", "wb") as f:
    f.write(pdf_bytes)
```

See the [Python binding README](bindings/python/README.md) for comprehensive documentation.

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

# Deterministic metadata timestamp (for reproducible builds)
rtfkit convert document.rtf --to pdf --fixed-timestamp "2024-01-01T00:00:00Z" --output document.pdf
```

PDF output uses an embedded Typst renderer - no external dependencies required. The output is deterministic and works completely offline.

See [PDF Output Reference](docs/reference/pdf-output.md) for PDF-specific options and [PDF Determinism](docs/reference/pdf-determinism.md) for determinism guarantees.

### App Integration Examples (RTF to PDF)

Small examples for using `rtfkit` inside application code.

#### Java (web/enterprise app via CLI)

Install `rtfkit` on the host/container and call it from your service layer:

```java
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;

public final class RtfToPdfService {
    public static void convert(Path inputRtf, Path outputPdf) throws IOException, InterruptedException {
        Process process = new ProcessBuilder(
                "rtfkit", "convert",
                inputRtf.toString(),
                "--to", "pdf",
                "-o", outputPdf.toString(),
                "--force"
        ).redirectErrorStream(true).start();

        String logs = new String(process.getInputStream().readAllBytes(), StandardCharsets.UTF_8);
        int exitCode = process.waitFor();
        if (exitCode != 0) {
            throw new IOException("rtfkit failed (exit " + exitCode + "): " + logs);
        }
    }
}
```

#### Python (using bindings directly)

```python
from pathlib import Path
import rtfkit

def convert_rtf_to_pdf(input_rtf: str, output_pdf: str) -> None:
    # latin-1 keeps a 1:1 byte mapping for raw RTF content
    rtf_content = Path(input_rtf).read_bytes().decode("latin-1")
    result = rtfkit.parse(rtf_content)
    pdf_bytes = rtfkit.to_pdf(result.document, page_size="a4", style_profile="report")
    Path(output_pdf).write_bytes(pdf_bytes)
```

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
| Maximum table nesting depth | 16 levels | Prevents nested-table recursion abuse |

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
| `unsupported_field` | Field semantics not fully supported; `\fldrslt` preserved | No failure |
| `unsupported_page_field` | Page field rendered with best-effort/static behavior | No failure |
| `unsupported_toc_switch` | TOC switch parsed but not mapped | No failure |
| `unresolved_page_reference` | `PAGEREF` target not found; fallback text used | No failure |
| `section_numbering_fallback` | Section numbering semantics approximated | No failure |
| `unresolved_cross_reference` | `REF`/`NOTEREF` target not found; fallback text used | No failure |

See [Warning Reference](docs/warning-reference.md) for detailed documentation.

### Field Behavior

- Recognized field instructions are mapped to semantic IR where possible (hyperlinks, page fields, TOC markers, semantic refs, and merge-field fallback).
- Unknown/unsupported field instructions preserve visible `\fldrslt` text and emit `unsupported_field`.
- If a field has no usable result text, `dropped_content` is emitted (strict mode fails on this).
- `REF` and `NOTEREF` are resolved against bookmark anchors (`\bkmkstart`); unresolved targets stay visible as fallback text and emit `unresolved_cross_reference`.
- Non-run content inside semantic field results is downgraded deterministically to text runs with an `unsupported_field` warning.

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

### Python Bindings Development

```bash
# Install maturin
pip install maturin

# Build the package
cd bindings/python
maturin build

# Install the built package
pip install target/wheels/*.whl

# Run tests
cd bindings/python
pytest

# Type checking
cd bindings/python
mypy python/rtfkit/
```

## Testing

rtfkit has comprehensive test coverage:

- **Contract tests** - Exit codes, strict mode, warning semantics
- **Determinism tests** - IR/report/DOCX stability verification
- **Limits tests** - Safety and resource protection
- **Golden tests** - IR snapshot validation
- **DOCX integration tests** - End-to-end conversion verification

See [CONTRIBUTING.md](CONTRIBUTING.md) for the fixture-first contribution workflow.

## Current Limitations

- Full RTF parity is not complete yet (advanced/edge-case layout and styling can degrade)
- Field support is intentionally partial: dynamic evaluation (`DATE`/`TIME`, formulas, mail-merge execution, conditional logic) is not executed; visible fallback/result text is preserved instead
- Page-related fields are rendered with deterministic static placeholders/fallback text (not live-updating pagination in output)
- WMF/EMF image formats are not supported (PNG/JPEG are supported)
- Images are block-level only (no inline/floating placement, no crop controls)
- HTML output is semantic-first rather than pixel-perfect rendering
- PDF output uses embedded fonts (no custom font support)
- List nesting is limited to 8 levels (DOCX compatibility)
- Row alignment/indent controls are parsed, but docx-rs emission remains partially limited

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
- [Python Binding Documentation](bindings/python/README.md)

### Migration Notes

- **PDF rendering** is now in-process via `rtfkit-render-typst` crate (no external CLI required)
- **`--pdf-backend` flag** has been removed (single backend now)
- **`--keep-intermediate` flag** has been removed (in-process rendering)

## License

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
