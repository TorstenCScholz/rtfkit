"""rtfkit - Convert RTF documents to HTML, DOCX, and PDF.

A high-performance RTF conversion library powered by Rust.

Quick start::

    import rtfkit

    result = rtfkit.parse(r\"{\\rtf1\\ansi Hello \\b World!}\")
    html = rtfkit.to_html(result.document)
    docx_bytes = rtfkit.to_docx_bytes(result.document)
    pdf_bytes = rtfkit.to_pdf(result.document)
"""

__version__ = "0.1.0"

from rtfkit._native import (
    # Exceptions
    RtfkitError,
    ParseError,
    ReportError,
    HtmlWriterError,
    DocxWriterError,
    PdfRenderError,
    # IR types
    Color,
    Shading,
    Run,
    Hyperlink,
    Paragraph,
    ListItem,
    ListBlock,
    TableCell,
    TableRow,
    TableBlock,
    ImageBlock,
    Document,
    # Report types
    Stats,
    Warning,
    Report,
    # Limits
    ParserLimits,
    # Parse
    ParseResult,
    parse,
    parse_with_limits,
    # HTML
    HtmlOutput,
    to_html,
    to_html_with_warnings,
    # DOCX
    to_docx_bytes,
    to_docx_file,
    # PDF
    PdfOutput,
    to_pdf,
    to_pdf_with_warnings,
)

__all__ = [
    # Exceptions
    "RtfkitError",
    "ParseError",
    "ReportError",
    "HtmlWriterError",
    "DocxWriterError",
    "PdfRenderError",
    # IR types
    "Color",
    "Shading",
    "Run",
    "Hyperlink",
    "Paragraph",
    "ListItem",
    "ListBlock",
    "TableCell",
    "TableRow",
    "TableBlock",
    "ImageBlock",
    "Document",
    # Report types
    "Stats",
    "Warning",
    "Report",
    # Limits
    "ParserLimits",
    # Parse
    "ParseResult",
    "parse",
    "parse_with_limits",
    # HTML
    "HtmlOutput",
    "to_html",
    "to_html_with_warnings",
    # DOCX
    "to_docx_bytes",
    "to_docx_file",
    # PDF
    "PdfOutput",
    "to_pdf",
    "to_pdf_with_warnings",
]
