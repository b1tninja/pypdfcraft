"""Open-path validation errors."""

import pytest

pypdfcraft = pytest.importorskip("pypdfcraft")


def test_open_invalid_bytes():
    with pytest.raises(pypdfcraft.PdfError, match="not a readable PDF"):
        pypdfcraft.open(bytes=b"not a pdf")
