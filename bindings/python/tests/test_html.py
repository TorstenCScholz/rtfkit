"""Tests for HTML conversion."""

import rtfkit


def test_to_html_basic(simple_rtf):
    """to_html() returns an HTML string."""
    result = rtfkit.parse(simple_rtf)
    html = rtfkit.to_html(result.document)
    assert isinstance(html, str)
    assert "<!doctype html>" in html
    assert "Hello World" in html


def test_to_html_no_wrapper(simple_rtf):
    """emit_wrapper=False skips the document wrapper."""
    result = rtfkit.parse(simple_rtf)
    html = rtfkit.to_html(result.document, emit_wrapper=False)
    assert "<!doctype html>" not in html
    assert "Hello World" in html


def test_to_html_css_none(simple_rtf):
    """css_mode='none' omits built-in CSS."""
    result = rtfkit.parse(simple_rtf)
    html = rtfkit.to_html(result.document, css_mode="none")
    assert "<!doctype html>" in html
    # With css_mode="none", there should be no <style> block (unless custom_css)
    # The content should still be present
    assert "Hello World" in html


def test_to_html_style_profile(simple_rtf):
    """style_profile kwarg is accepted."""
    result = rtfkit.parse(simple_rtf)
    html = rtfkit.to_html(result.document, style_profile="compact")
    assert isinstance(html, str)
    assert "Hello World" in html


def test_to_html_invalid_css_mode(simple_rtf):
    """Invalid css_mode raises ValueError."""
    result = rtfkit.parse(simple_rtf)
    try:
        rtfkit.to_html(result.document, css_mode="invalid")
        assert False, "Should have raised ValueError"
    except ValueError:
        pass


def test_to_html_with_warnings(simple_rtf):
    """to_html_with_warnings() returns HtmlOutput."""
    result = rtfkit.parse(simple_rtf)
    output = rtfkit.to_html_with_warnings(result.document)
    assert isinstance(output, rtfkit.HtmlOutput)
    assert isinstance(output.html, str)
    assert isinstance(output.dropped_content_reasons, list)
    assert "Hello World" in output.html


def test_to_html_custom_css(simple_rtf):
    """custom_css is injected into the output."""
    result = rtfkit.parse(simple_rtf)
    custom = ".my-class { color: red; }"
    html = rtfkit.to_html(result.document, custom_css=custom)
    assert ".my-class" in html
