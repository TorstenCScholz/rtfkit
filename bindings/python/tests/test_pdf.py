"""Tests for PDF conversion."""

import rtfkit


def test_to_pdf_basic(simple_rtf):
    """to_pdf() returns bytes starting with %PDF-."""
    result = rtfkit.parse(simple_rtf)
    data = rtfkit.to_pdf(result.document)
    assert isinstance(data, bytes)
    assert data[:5] == b"%PDF-"


def test_to_pdf_page_size(simple_rtf):
    """page_size kwarg is accepted."""
    result = rtfkit.parse(simple_rtf)
    data = rtfkit.to_pdf(result.document, page_size="letter")
    assert data[:5] == b"%PDF-"


def test_to_pdf_margins(simple_rtf):
    """Margin kwargs are accepted."""
    result = rtfkit.parse(simple_rtf)
    data = rtfkit.to_pdf(result.document, margin_top=20.0, margin_bottom=20.0)
    assert data[:5] == b"%PDF-"


def test_to_pdf_invalid_page_size(simple_rtf):
    """Invalid page_size raises ValueError."""
    result = rtfkit.parse(simple_rtf)
    try:
        rtfkit.to_pdf(result.document, page_size="invalid")
        assert False, "Should have raised ValueError"
    except ValueError:
        pass


def test_to_pdf_with_warnings(simple_rtf):
    """to_pdf_with_warnings() returns PdfOutput."""
    result = rtfkit.parse(simple_rtf)
    output = rtfkit.to_pdf_with_warnings(result.document)
    assert isinstance(output, rtfkit.PdfOutput)
    assert isinstance(output.pdf_bytes, bytes)
    assert output.pdf_bytes[:5] == b"%PDF-"
    assert isinstance(output.warnings, list)


def test_to_pdf_deterministic(simple_rtf):
    """Fixed timestamp produces deterministic output."""
    result = rtfkit.parse(simple_rtf)
    ts = "2021-01-01T00:00:00Z"
    data1 = rtfkit.to_pdf(result.document, fixed_timestamp=ts)
    data2 = rtfkit.to_pdf(result.document, fixed_timestamp=ts)
    assert data1 == data2
