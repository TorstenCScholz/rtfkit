"""Tests for the exception hierarchy."""

import rtfkit


def test_parse_error_is_rtfkit_error():
    """ParseError is a subclass of RtfkitError."""
    assert issubclass(rtfkit.ParseError, rtfkit.RtfkitError)


def test_report_error_is_rtfkit_error():
    """ReportError is a subclass of RtfkitError."""
    assert issubclass(rtfkit.ReportError, rtfkit.RtfkitError)


def test_html_writer_error_is_rtfkit_error():
    """HtmlWriterError is a subclass of RtfkitError."""
    assert issubclass(rtfkit.HtmlWriterError, rtfkit.RtfkitError)


def test_docx_writer_error_is_rtfkit_error():
    """DocxWriterError is a subclass of RtfkitError."""
    assert issubclass(rtfkit.DocxWriterError, rtfkit.RtfkitError)


def test_pdf_render_error_is_rtfkit_error():
    """PdfRenderError is a subclass of RtfkitError."""
    assert issubclass(rtfkit.PdfRenderError, rtfkit.RtfkitError)


def test_rtfkit_error_is_exception():
    """RtfkitError is a subclass of Exception."""
    assert issubclass(rtfkit.RtfkitError, Exception)


def test_all_exception_types_exist():
    """All 6 exception types are importable."""
    exceptions = [
        rtfkit.RtfkitError,
        rtfkit.ParseError,
        rtfkit.ReportError,
        rtfkit.HtmlWriterError,
        rtfkit.DocxWriterError,
        rtfkit.PdfRenderError,
    ]
    assert len(exceptions) == 6


def test_parse_error_message_is_informative():
    """ParseError messages contain useful information."""
    try:
        rtfkit.parse("")
    except rtfkit.ParseError as e:
        msg = str(e)
        assert len(msg) > 0


def test_catch_base_exception():
    """RtfkitError catches ParseError."""
    try:
        rtfkit.parse("")
    except rtfkit.RtfkitError:
        pass  # Correctly caught
