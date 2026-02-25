"""Tests for IR type access and serialization."""

import json

import rtfkit


def test_paragraph_isinstance(simple_rtf):
    """Blocks are Paragraph instances."""
    result = rtfkit.parse(simple_rtf)
    blocks = result.document.blocks
    assert len(blocks) >= 1
    assert isinstance(blocks[0], rtfkit.Paragraph)


def test_paragraph_alignment(centered_rtf):
    """Paragraph alignment is a string."""
    result = rtfkit.parse(centered_rtf)
    para = result.document.blocks[0]
    assert isinstance(para, rtfkit.Paragraph)
    assert para.alignment == "center"


def test_paragraph_block_type(simple_rtf):
    """Paragraph has block_type property."""
    result = rtfkit.parse(simple_rtf)
    para = result.document.blocks[0]
    assert para.block_type == "paragraph"


def test_run_properties(bold_italic_rtf):
    """Run properties are accessible."""
    result = rtfkit.parse(bold_italic_rtf)
    para = result.document.blocks[0]
    assert isinstance(para, rtfkit.Paragraph)

    inlines = para.inlines
    assert len(inlines) >= 1

    # Find a bold run
    bold_runs = [i for i in inlines if isinstance(i, rtfkit.Run) and i.bold]
    assert len(bold_runs) >= 1

    run = bold_runs[0]
    assert run.text.strip() != ""
    assert run.inline_type == "run"


def test_run_italic(bold_italic_rtf):
    """Italic runs are accessible."""
    result = rtfkit.parse(bold_italic_rtf)
    para = result.document.blocks[0]
    italic_runs = [i for i in para.inlines if isinstance(i, rtfkit.Run) and i.italic]
    assert len(italic_runs) >= 1


def test_document_to_dict(simple_rtf):
    """Document.to_dict() returns a valid dict."""
    result = rtfkit.parse(simple_rtf)
    d = result.document.to_dict()
    assert isinstance(d, dict)
    assert "blocks" in d


def test_document_to_json(simple_rtf):
    """Document.to_json() returns valid JSON."""
    result = rtfkit.parse(simple_rtf)
    json_str = result.document.to_json()
    parsed = json.loads(json_str)
    assert isinstance(parsed, dict)
    assert "blocks" in parsed


def test_document_to_json_indent(simple_rtf):
    """Document.to_json(indent=2) returns indented JSON."""
    result = rtfkit.parse(simple_rtf)
    json_str = result.document.to_json(indent=2)
    assert "\n" in json_str
    parsed = json.loads(json_str)
    assert "blocks" in parsed


def test_document_len(multi_para_rtf):
    """Document supports len()."""
    result = rtfkit.parse(multi_para_rtf)
    assert len(result.document) == 3


def test_paragraph_len(bold_italic_rtf):
    """Paragraph supports len()."""
    result = rtfkit.parse(bold_italic_rtf)
    para = result.document.blocks[0]
    assert len(para) >= 1


def test_paragraph_to_dict(simple_rtf):
    """Paragraph.to_dict() returns a valid dict."""
    result = rtfkit.parse(simple_rtf)
    para = result.document.blocks[0]
    d = para.to_dict()
    assert isinstance(d, dict)
    assert d.get("type") == "paragraph"


def test_golden_match(fixtures_dir, golden_dir):
    """Python to_dict() matches the Rust-generated golden JSON."""
    fixture = fixtures_dir / "text_simple_paragraph.rtf"
    golden = golden_dir / "text_simple_paragraph.json"
    if not fixture.exists() or not golden.exists():
        return  # Skip if files not available
    rtf = fixture.read_text()
    expected = json.loads(golden.read_text())
    actual = rtfkit.parse(rtf).document.to_dict()
    assert actual == expected
