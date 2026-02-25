"""Shared pytest fixtures for rtfkit tests."""

import os
from pathlib import Path

import pytest

# Project root (bindings/python -> project root)
PROJECT_ROOT = Path(__file__).parent.parent.parent.parent

FIXTURES_DIR = PROJECT_ROOT / "fixtures"
GOLDEN_DIR = PROJECT_ROOT / "golden"


@pytest.fixture
def simple_rtf():
    """A minimal RTF document with a single paragraph."""
    return r"{\rtf1\ansi Hello World}"


@pytest.fixture
def bold_italic_rtf():
    """RTF with bold and italic text."""
    return r"{\rtf1\ansi {\b Bold text} and {\i italic text}.}"


@pytest.fixture
def multi_para_rtf():
    """RTF with multiple paragraphs."""
    return r"{\rtf1\ansi First\par Second\par Third}"


@pytest.fixture
def centered_rtf():
    """RTF with centered alignment."""
    return r"{\rtf1\ansi \qc Centered text}"


@pytest.fixture
def fixtures_dir():
    """Path to the fixtures directory."""
    return FIXTURES_DIR


@pytest.fixture
def golden_dir():
    """Path to the golden test directory."""
    return GOLDEN_DIR
