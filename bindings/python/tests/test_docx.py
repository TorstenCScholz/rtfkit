"""Tests for DOCX conversion."""

import tempfile
from pathlib import Path

import rtfkit


def test_to_docx_bytes(simple_rtf):
    """to_docx_bytes() returns bytes starting with PK (ZIP)."""
    result = rtfkit.parse(simple_rtf)
    data = rtfkit.to_docx_bytes(result.document)
    assert isinstance(data, bytes)
    assert data[:2] == b"PK"


def test_to_docx_file(simple_rtf):
    """to_docx_file() writes a valid DOCX file."""
    result = rtfkit.parse(simple_rtf)
    with tempfile.TemporaryDirectory() as tmpdir:
        path = str(Path(tmpdir) / "test.docx")
        rtfkit.to_docx_file(result.document, path)
        data = open(path, "rb").read()
        assert data[:2] == b"PK"


def test_to_docx_empty_doc():
    """Empty document produces valid DOCX."""
    rtf = r"{\rtf1\ansi }"
    result = rtfkit.parse(rtf)
    data = rtfkit.to_docx_bytes(result.document)
    assert isinstance(data, bytes)
    assert len(data) > 0
