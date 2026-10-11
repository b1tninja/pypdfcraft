"""Smoke tests for pypdfcraft (require a built extension and sibling pdfcraft)."""

import pytest

pypdfcraft = pytest.importorskip("pypdfcraft")


def test_version():
    assert pypdfcraft.__version__ == "0.1.0"


def test_tools_list():
    tools = pypdfcraft.tools()
    assert isinstance(tools, list)
    assert len(tools) > 50
    assert any(t["name"] == "text_extract" for t in tools)


def test_blank_document():
    doc = pypdfcraft.Document()
    assert len(doc) == 1
    doc.close()


def test_open_and_text():
    doc = pypdfcraft.Document()
    assert doc.page_count == 1
    text = doc.extract_text()
    assert isinstance(text, str)
    doc.close()


def test_extract_pages_blank():
    doc = pypdfcraft.Document()
    data = doc.to_bytes()
    doc.close()
    out = pypdfcraft.extract_pages(bytes=data, ranges=[(1, 1)])
    assert out.startswith(b"%PDF")
