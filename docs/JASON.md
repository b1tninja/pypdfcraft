# jason drop-in mapping

This document maps jason’s PDF seams to pypdfcraft. jason is not wired to pypdfcraft yet; use this when adding an optional extra (for example `jason[pypdfcraft]`).

## Renderer (`tasks/split_thumbs.py`)

```python
import pypdfcraft as pdf

def render_page_pypdfcraft(path: Path, page: int, size: int, rotate: int = 0) -> bytes:
    with pdf.open(path) as doc:
        bmp = doc[page].render(max_side=size, rotate=rotate)
        return bmp.data
```

Raises `pypdfcraft.RendererUnavailable` only if the extension failed to import; otherwise failures use `PageUnreadable` or `PdfError`.

## Text layer (`tasks/library.py`, `community/parsing.py`)

```python
with pdf.open(path) as doc:
    layer = doc.extract_text()
    per_page = [doc[i].extract_text() for i in range(len(doc))]
```

Empty or short text (`< 200` chars) should still trigger jason’s OCR chain.

## Split writer (`tasks/record_upload.py`)

```python
out = pdf.extract_pages(path, ranges=[(start, end)])  # 1-based inclusive
# or
with pdf.open(path) as doc:
    out = doc.extract(pages=list(range(a, b)))
```

## Vision page images (`community/model_extractor.py`)

```python
with pdf.open(path) as doc:
    png = doc[page_index].render(dpi=150).data
```

## Mailroom page count (`tasks/mailroom.py`)

```python
with pdf.open(path) as doc:
    n = len(doc)
```

## Preflight / splitter facts (partial)

`PageMedia` from `page.media()` or `doc.page_facts()` exposes size, rotation, embedded image count, and coarse codec hints (`has_jbig2`, `has_ccitt`). This does not replace full `pdf_preflight` until pdfcraft exposes the same facts as jason’s preflight module.

## Forms (`community/pdf_fields.py`)

```python
fields = doc.form_fields()
doc.form_fill({"Field1": "value"})
doc.flatten(forms=True)
```

## Backend order (recommended)

1. `pypdfcraft` for render + text when installed  
2. `pypdfium2` (`[split]`)  
3. `pymupdf` (`[pdf]`)  
4. `pypdf` for page surgery until byte-preserving extract is proven on archive scans  
