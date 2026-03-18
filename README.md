# rtfkit

Open-source RTF converter for turning Rich Text Format documents into DOCX, HTML, PDF, or structured JSON.

`rtfkit` is built for developers and teams that need a reliable way to parse `.rtf` files and convert them into modern formats from the command line or from Python. It is designed for offline use, deterministic output, and practical handling of real-world RTF documents that include tables, lists, fields, images, and mixed formatting.

[![CI](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml)
[![Python Bindings](https://github.com/TorstenCScholz/rtfkit/actions/workflows/bindings-python.yml/badge.svg)](https://github.com/TorstenCScholz/rtfkit/actions/workflows/bindings-python.yml)

## What rtfkit does

`rtfkit` helps you:

- convert RTF to DOCX
- convert RTF to HTML
- convert RTF to PDF
- parse RTF into a structured intermediate representation for debugging, validation, or custom pipelines
- inspect warnings and conversion statistics for unsupported or degraded content

Typical use cases include:

- document migration from legacy RTF archives
- backend services that need an RTF to PDF converter
- RTF to HTML rendering for web applications
- RTF to DOCX export in desktop, enterprise, and batch-processing workflows
- validation pipelines for untrusted or malformed RTF input

## Why use it

- **One CLI for multiple outputs**: DOCX, HTML, PDF, and JSON-based reports
- **Offline PDF generation**: no external PDF CLI required
- **Deterministic behavior**: stable output and warning contracts for automation
- **Safety limits**: parser limits for input size, depth, warnings, and table complexity
- **Developer-friendly**: CLI workflow plus Python bindings
- **Real-world feature coverage**: formatting, lists, tables, fields, headers/footers, notes, and images

## Supported output formats

- **DOCX**: default output format for editable Word-compatible documents
- **HTML**: semantic HTML5 output for websites, previews, and pipelines
- **PDF**: in-process PDF generation for offline document conversion
- **IR JSON**: structured intermediate representation via `--emit-ir`
- **Reports**: text or JSON conversion reports with warnings and statistics

For the full support matrix, see [docs/feature-support.md](docs/feature-support.md).

## Supported feature set

`rtfkit` currently supports a broad practical subset of RTF, including:

- text extraction and paragraph structure
- bold, italic, underline, strikethrough, caps, and small-caps fallback behavior
- font families, font sizes, foreground colors, highlights, and shading
- bullet lists, ordered lists, and nested lists
- tables, merged cells, nested tables, borders, alignment, and cell shading
- hyperlinks, bookmark anchors, page fields, TOC markers, semantic references, and merge-field fallback rendering
- headers, footers, footnotes, and endnotes
- embedded PNG and JPEG images

Known limitations are documented in [docs/rtf-feature-overview.md](docs/rtf-feature-overview.md) and [docs/feature-support.md](docs/feature-support.md).

## Install

### Prebuilt binaries

Download a release artifact for your platform from [GitHub Releases](https://github.com/TorstenCScholz/rtfkit/releases).

### Build from source

```sh
cargo install --path crates/rtfkit-cli
```

### Python bindings

Install from source:

```sh
git clone https://github.com/TorstenCScholz/rtfkit.git
cd rtfkit/bindings/python
pip install .
```

See [bindings/python/README.md](bindings/python/README.md) for the Python API.

## Quick start

### Convert RTF to DOCX

```sh
rtfkit convert input.rtf -o output.docx
```

### Convert RTF to PDF

```sh
rtfkit convert input.rtf --to pdf -o output.pdf
```

### Convert RTF to HTML

```sh
rtfkit convert input.rtf --to html -o output.html
```

### Emit structured IR JSON

```sh
rtfkit convert input.rtf --emit-ir output.json
```

### Print a JSON conversion report

```sh
rtfkit convert input.rtf --format json
```

## CLI usage

### Common examples

```sh
# DOCX output
rtfkit convert document.rtf -o document.docx

# PDF output
rtfkit convert document.rtf --to pdf -o document.pdf

# HTML output
rtfkit convert document.rtf --to html -o document.html

# Overwrite an existing file
rtfkit convert document.rtf -o document.docx --force

# Strict mode: fail when content is dropped
rtfkit convert document.rtf --strict --format json

# Write IR JSON for debugging or snapshot workflows
rtfkit convert document.rtf --emit-ir document.ir.json
```

### HTML output options

```sh
# Default built-in CSS
rtfkit convert document.rtf --to html -o document.html

# No built-in CSS
rtfkit convert document.rtf --to html --html-css none -o document.html

# Append custom CSS
rtfkit convert document.rtf --to html --html-css-file custom.css -o document.html
```

### Style profiles

```sh
# Default long-form profile
rtfkit convert document.rtf --to pdf --style-profile report -o document.pdf

# Neutral styling
rtfkit convert document.rtf --to html --style-profile classic -o document.html

# Dense output
rtfkit convert document.rtf --to pdf --style-profile compact -o document.pdf
```

Built-in profiles: `classic`, `report`, `compact`

### PDF options

```sh
# A4 PDF
rtfkit convert document.rtf --to pdf --output document.pdf

# US Letter PDF
rtfkit convert document.rtf --to pdf --pdf-page-size letter --output document.pdf

# Deterministic timestamp for reproducible builds
rtfkit convert document.rtf --to pdf --fixed-timestamp "2024-01-01T00:00:00Z" --output document.pdf
```

## Python example

```python
from pathlib import Path
import rtfkit

rtf_content = Path("input.rtf").read_bytes().decode("latin-1")
result = rtfkit.parse(rtf_content)

html = rtfkit.to_html(result.document)
docx_bytes = rtfkit.to_docx_bytes(result.document)
pdf_bytes = rtfkit.to_pdf(result.document)

Path("output.html").write_text(html)
Path("output.docx").write_bytes(docx_bytes)
Path("output.pdf").write_bytes(pdf_bytes)
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Parse or validation failure |
| 3 | Writer or file output failure |
| 4 | Strict-mode failure caused by dropped content |

## Parser limits

Default parser limits:

| Limit | Default |
|------|---------|
| Maximum input size | 10 MB |
| Maximum group depth | 256 |
| Maximum warnings | 1000 |
| Maximum rows per table | 10,000 |
| Maximum cells per row | 1,000 |
| Maximum merge span | 1,000 |
| Maximum nested table depth | 16 |

See [docs/limits-policy.md](docs/limits-policy.md) for details.

## Current limitations

`rtfkit` is intended to cover practical RTF conversion needs, not full byte-for-byte visual parity with every historical RTF producer.

Current limitations include:

- advanced edge-case layout may degrade
- dynamic field evaluation is not executed; visible result text is preserved where possible
- page-related fields use deterministic fallback/static behavior rather than live pagination
- WMF and EMF images are not supported
- images are block-level only; floating placement and crop controls are not supported
- HTML output is semantic-first rather than pixel-perfect
- PDF output uses embedded fonts rather than custom font loading
- list nesting is capped at 8 levels for DOCX compatibility

## Documentation

- [Feature support matrix](docs/feature-support.md)
- [RTF feature overview](docs/rtf-feature-overview.md)
- [HTML styling reference](docs/reference/html-styling.md)
- [PDF output reference](docs/reference/pdf-output.md)
- [PDF determinism](docs/reference/pdf-determinism.md)
- [Warning reference](docs/warning-reference.md)
- [Limits policy](docs/limits-policy.md)
- [Architecture overview](docs/arch/README.md)
- [Contributing](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)
- [Python bindings](bindings/python/README.md)

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Update golden snapshots when intended:

```sh
UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests
```

## License

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
