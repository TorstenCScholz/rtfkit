# rtfkit Python Bindings

Python bindings for `rtfkit`, the Rust-powered RTF conversion toolkit.

Use the bindings when you want to parse RTF or convert RTF to HTML, DOCX, or PDF directly from Python without shelling out to the CLI.

## Installation

### From source

```bash
git clone https://github.com/TorstenCScholz/rtfkit.git
cd rtfkit/bindings/python
pip install .
```

### Development install

```bash
git clone https://github.com/TorstenCScholz/rtfkit.git
cd rtfkit/bindings/python
pip install -e .
```

## Quick start

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

## What the bindings support

- parse RTF content into a structured document model
- convert to HTML
- convert to DOCX
- convert to PDF
- inspect warnings and conversion statistics
- configure parser limits for untrusted input

## Core functions

### `parse(rtf: str) -> ParseResult`

Parses RTF content and returns a parsed document plus a conversion report.

```python
result = rtfkit.parse(rtf_content)
document = result.document
report = result.report
```

### `parse_with_limits(rtf: str, limits: ParserLimits) -> ParseResult`

Parses RTF using custom safety limits.

```python
limits = rtfkit.ParserLimits(
    max_input_bytes=1024 * 1024,
    max_group_depth=32,
    max_warning_count=10,
)
result = rtfkit.parse_with_limits(rtf_content, limits)
```

### `to_html(document: Document, **kwargs) -> str`

Converts a parsed document to HTML.

```python
html = rtfkit.to_html(
    document,
    emit_wrapper=True,
    css_mode="default",
    style_profile="report",
)
```

### `to_html_with_warnings(document: Document, **kwargs) -> HtmlOutput`

Returns HTML plus output warnings.

```python
output = rtfkit.to_html_with_warnings(document)
html = output.html
dropped_reasons = output.dropped_content_reasons
```

### `to_docx_bytes(document: Document) -> bytes`

Converts a parsed document to DOCX bytes.

```python
docx_bytes = rtfkit.to_docx_bytes(document)
```

### `to_docx_file(document: Document, path: str) -> None`

Writes DOCX output directly to a file.

```python
rtfkit.to_docx_file(document, "output.docx")
```

### `to_pdf(document: Document, **kwargs) -> bytes`

Converts a parsed document to PDF bytes.

```python
pdf_bytes = rtfkit.to_pdf(
    document,
    page_size="a4",
    style_profile="classic",
)
```

### `to_pdf_with_warnings(document: Document, **kwargs) -> PdfOutput`

Returns PDF bytes plus output warnings.

```python
output = rtfkit.to_pdf_with_warnings(document)
pdf_bytes = output.pdf_bytes
warnings = output.warnings
```

## Data types

### `Document`

Represents the parsed RTF document.

```python
document = result.document
for block in document.blocks:
    print(type(block).__name__)
```

### `ParseResult`

Contains the parsed document and report.

```python
result = rtfkit.parse(rtf_content)
print(len(result.document))
print(len(result.report.warnings))
```

### `Report`

Contains warnings and conversion statistics.

```python
report = result.report
print(report.stats.paragraph_count)
print(len(report.warnings))
```

### `ParserLimits`

Controls parser resource limits.

```python
limits = rtfkit.ParserLimits(
    max_input_bytes=1024 * 1024,
    max_group_depth=32,
    max_warning_count=10,
)
```

## Errors

All exposed errors inherit from `RtfkitError`.

- `ParseError`
- `ReportError`
- `HtmlWriterError`
- `DocxWriterError`
- `PdfRenderError`

```python
try:
    result = rtfkit.parse(rtf_content)
except rtfkit.ParseError as exc:
    print(f"Parsing failed: {exc}")
```

## Notes

- Read RTF files as bytes and decode with a one-byte codec such as `latin-1` when you need to preserve raw source bytes.
- The bindings follow the same support profile and limitations as the core Rust library.
- For the current supported feature set, see `../../docs/feature-support.md`.

### Document Inspection

```python
import rtfkit

result = rtfkit.parse(rtf_content)

# Walk the document tree
for block in result.document.blocks:
    if isinstance(block, rtfkit.Paragraph):
        print(f"Paragraph: {len(block.inlines)} inlines")
        for inline in block.inlines:
            if isinstance(inline, rtfkit.Run):
                print(f"  Run: {inline.text}")
    elif isinstance(block, rtfkit.ListBlock):
        print(f"List: {len(block.items)} items")
```

### HTML with Custom CSS

```python
import rtfkit

result = rtfkit.parse(rtf_content)

# Custom styling
custom_css = """
.custom-class {
    font-family: Arial, sans-serif;
    line-height: 1.6;
}
"""

html = rtfkit.to_html(
    result.document,
    css_mode="none",  # Don't include built-in CSS
    custom_css=custom_css,
)
```

### PDF with Custom Margins

```python
import rtfkit

result = rtfkit.parse(rtf_content)

pdf_data = rtfkit.to_pdf(
    result.document,
    page_size="letter",
    margin_top=15.0,
    margin_bottom=15.0,
    margin_left=10.0,
    margin_right=10.0,
    style_profile="compact",
)
```

## Development

### Building from Source

```bash
# Install maturin
pip install maturin

# Build the package
cd bindings/python
maturin build

# Install the built package
pip install target/wheels/*.whl
```

### Running Tests

```bash
# Run tests
cd bindings/python
pytest
```

### Type Checking

```bash
# Install mypy
pip install mypy

# Type check
cd bindings/python
mypy python/rtfkit/
```

## License

This package is licensed under either the [Apache License, Version 2.0](../../LICENSE-APACHE) or the [MIT License](../../LICENSE-MIT), at your option.

## Contributing

See the main project's [CONTRIBUTING.md](https://github.com/TorstenCScholz/rtfkit/blob/main/CONTRIBUTING.md) for contribution guidelines.

## Support

For issues and questions, please open an issue on the [GitHub repository](https://github.com/TorstenCScholz/rtfkit/issues).
