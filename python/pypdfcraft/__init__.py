"""Python bindings for the pdfcraft PDF engine."""

from pypdfcraft._native import (
    Bitmap,
    Document,
    Page,
    PageMedia,
    PageUnreadable,
    PasswordError,
    PdfError,
    RendererUnavailable,
    Session,
    Word,
    __version__,
    extract_pages,
    open,
    tools,
)

__all__ = [
    "Bitmap",
    "Document",
    "Page",
    "PageMedia",
    "PageUnreadable",
    "PasswordError",
    "PdfError",
    "RendererUnavailable",
    "Session",
    "Word",
    "__version__",
    "extract_pages",
    "open",
    "tools",
]
