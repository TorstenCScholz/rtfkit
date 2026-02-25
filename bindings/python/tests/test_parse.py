"""Tests for RTF parsing."""

import rtfkit


def test_parse_simple(simple_rtf):
    """parse() returns a ParseResult with document and report."""
    result = rtfkit.parse(simple_rtf)
    assert isinstance(result, rtfkit.ParseResult)
    assert isinstance(result.document, rtfkit.Document)
    assert isinstance(result.report, rtfkit.Report)


def test_parse_has_blocks(simple_rtf):
    """Parsed document has at least one block."""
    result = rtfkit.parse(simple_rtf)
    assert len(result.document) >= 1


def test_parse_report_stats(simple_rtf):
    """Report contains valid stats."""
    result = rtfkit.parse(simple_rtf)
    stats = result.report.stats
    assert isinstance(stats, rtfkit.Stats)
    assert stats.paragraph_count >= 1
    assert stats.bytes_processed > 0


def test_parse_multiple_paragraphs(multi_para_rtf):
    """Multiple \\par controls produce multiple blocks."""
    result = rtfkit.parse(multi_para_rtf)
    assert len(result.document) == 3
    assert result.report.stats.paragraph_count == 3


def test_parse_empty_raises():
    """Empty input raises ParseError."""
    try:
        rtfkit.parse("")
        assert False, "Should have raised ParseError"
    except rtfkit.ParseError:
        pass


def test_parse_invalid_rtf_raises():
    """Non-RTF input raises ParseError."""
    try:
        rtfkit.parse("not rtf at all")
        assert False, "Should have raised ParseError"
    except rtfkit.ParseError:
        pass


def test_parse_with_limits(simple_rtf):
    """parse_with_limits() works with custom limits."""
    limits = rtfkit.ParserLimits(max_input_bytes=10_000_000)
    result = rtfkit.parse_with_limits(simple_rtf, limits)
    assert len(result.document) >= 1


def test_parse_with_limits_too_small():
    """Input exceeding limits raises ParseError."""
    limits = rtfkit.ParserLimits(max_input_bytes=5)
    try:
        rtfkit.parse_with_limits(r"{\rtf1\ansi Hello World}", limits)
        assert False, "Should have raised ParseError"
    except rtfkit.ParseError as e:
        assert "bytes" in str(e).lower() or "large" in str(e).lower()


def test_parse_fixture_file(fixtures_dir):
    """Parse a real fixture file."""
    fixture = fixtures_dir / "text_simple_paragraph.rtf"
    if not fixture.exists():
        return  # Skip if fixture not available
    rtf = fixture.read_text()
    result = rtfkit.parse(rtf)
    assert len(result.document) >= 1
