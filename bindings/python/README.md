# rtfkit Python Bindings

Python bindings for rtfkit - a high-performance RTF conversion library powered by Rust.

## Overview

This package provides Python bindings to the rtfkit Rust library, enabling you to convert RTF documents to HTML, DOCX, and PDF formats directly from Python.

## Installation

### From PyPI (Planned, not published yet)

```bash
# Will work after first PyPI release
pip install rtfkit
```

### From Source

```bash
# Clone the repository
git clone https://github.com/TorstenCScholz/rtfkit.git
cd rtfkit/bindings/python

# Install with pip
pip install .
```

### Development Installation

```bash
# Clone the repository
git clone https://github.com/TorstenCScholz/rtfkit.git
cd rtfkit/bindings/python

# Install in editable mode
pip install -e .
```

## Quick Start

```python
import rtfkit

# Parse RTF content
rtf_content = r"{\rtf1\ansi Hello \b World!}"
result = rtfkit.parse(rtf_content)

# Convert to different formats
html = rtfkit.to_html(result.document)
docx_bytes = rtfkit.to_docx_bytes(result.document)
pdf_bytes = rtfkit.to_pdf(result.document)
```

## API Reference

### Core Functions

#### `parse(rtf: str) -> ParseResult`
Parse RTF content and return a `ParseResult` containing the document and report.

```python
result = rtfkit.parse(rtf_content)
document = result.document
report = result.report
```

#### `parse_with_limits(rtf: str, limits: ParserLimits) -> ParseResult`
Parse RTF content with custom resource limits.

```python
limits = rtfkit.ParserLimits(
    max_input_bytes=1024 * 1024,  # 1MB
    max_group_depth=32,
    max_warning_count=10,
)
result = rtfkit.parse_with_limits(rtf_content, limits)
```

#### `to_html(document: Document, **kwargs) -> str`
Convert document to HTML.

```python
html = rtfkit.to_html(
    document,
    emit_wrapper=True,
    css_mode="default",
    style_profile="report",
    custom_css=".custom-class { color: red; }"
)
```

#### `to_html_with_warnings(document: Document, **kwargs) -> HtmlOutput`
Convert document to HTML with warnings.

```python
output = rtfkit.to_html_with_warnings(document)
html = output.html
dropped_reasons = output.dropped_content_reasons
```

#### `to_docx_bytes(document: Document) -> bytes`
Convert document to DOCX bytes.

```python
docx_data = rtfkit.to_docx_bytes(document)
with open("output.docx", "wb") as f:
    f.write(docx_data)
```

#### `to_docx_file(document: Document, path: str) -> None`
Convert document to DOCX and save to file.

```python
rtfkit.to_docx_file(document, "output.docx")
```

#### `to_pdf(document: Document, **kwargs) -> bytes`
Convert document to PDF bytes.

```python
pdf_data = rtfkit.to_pdf(
    document,
    page_size="a4",
    margin_top=10.0,
    margin_bottom=10.0,
    style_profile="classic"
)
with open("output.pdf", "wb") as f:
    f.write(pdf_data)
```

#### `to_pdf_with_warnings(document: Document, **kwargs) -> PdfOutput`
Convert document to PDF with warnings.

```python
output = rtfkit.to_pdf_with_warnings(document)
pdf_data = output.pdf_bytes
warnings = output.warnings
```

### Data Types

#### `Document`
Represents the parsed RTF document.

```python
# Access blocks
document = result.document
for block in document.blocks:
    if isinstance(block, rtfkit.Paragraph):
        # Process paragraph
        pass
    elif isinstance(block, rtfkit.ListBlock):
        # Process list
        pass
    elif isinstance(block, rtfkit.TableBlock):
        # Process table
        pass
    elif isinstance(block, rtfkit.ImageBlock):
        # Process image
        pass
```

#### `ParseResult`
Contains the parsed document and report.

```python
result = rtfkit.parse(rtf_content)
print(f"Blocks: {len(result.document)}")
print(f"Warnings: {len(result.report)}")
```

#### `Report`
Contains warnings and statistics about the conversion.

```python
report = result.report
print(f"Paragraphs: {report.stats.paragraph_count}")
print(f"Warnings: {len(report.warnings)}")
```

#### `ParserLimits`
Controls resource limits for parsing.

```python
# Create custom limits
limits = rtfkit.ParserLimits(
    max_input_bytes=1024 * 1024,
    max_group_depth=32,
    max_warning_count=10,
)

# Unlimited limits
unlimited = rtfkit.ParserLimits.unlimited()
```

### Error Types

All errors inherit from `RtfkitError`:

- `ParseError` - RTF parsing failed
- `ReportError` - Report generation failed
- `HtmlWriterError` - HTML generation failed
- `DocxWriterError` - DOCX generation failed
- `PdfRenderError` - PDF rendering failed

```python
try:
    result = rtfkit.parse(rtf_content)
except rtfkit.ParseError as e:
    print(f"Parsing failed: {e}")
```

## Examples

### Basic Conversion

```python
import rtfkit

# Read RTF file
with open("document.rtf", "r") as f:
    rtf_content = f.read()

# Parse and convert
result = rtfkit.parse(rtf_content)

# HTML output
html = rtfkit.to_html(result.document, style_profile="report")
with open("output.html", "w") as f:
    f.write(html)

# DOCX output
rtfkit.to_docx_file(result.document, "output.docx")

# PDF output
pdf_data = rtfkit.to_pdf(result.document, page_size="a4")
with open("output.pdf", "wb") as f:
    f.write(pdf_data)
```

### Custom Limits

```python
import rtfkit

# Restrictive limits for untrusted input
limits = rtfkit.ParserLimits(
    max_input_bytes=1024 * 1024,  # 1MB
    max_group_depth=32,
    max_warning_count=5,
)

try:
    result = rtfkit.parse_with_limits(rtf_content, limits)
    print(f"Successfully parsed: {len(result.document)} blocks")
except rtfkit.ParseError as e:
    print(f"Input rejected: {e}")
```

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
