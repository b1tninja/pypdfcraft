# pypdfcraft

Python bindings for [pdfcraft](https://github.com/storytold/pdfcraft): a permissive (MIT/Apache-2.0) native PDF library with Document/Page ergonomics similar to PyMuPDF and pypdfium2, plus an escape hatch to pdfcraft’s full automation tool table.

## Layout

Clone **pdfcraft** beside this repository (CI and local builds expect siblings):

```text
parent/
  pdfcraft/      # storytold/pdfcraft
  pypdfcraft/    # this repo
```

On Windows you can use a directory junction if pdfcraft lives elsewhere:

```bat
mklink /J D:\code\pdfcraft M:\path\to\pdfcraft
```

## Install (development)

```bash
python -m venv .venv
.venv\Scripts\activate   # Windows
pip install -U pip maturin pytest
maturin develop --release
pytest
```

Requires **Rust 1.94+** (see `rust-toolchain.toml`) and a CPython **3.10+**. Full setup (Rust on Windows, MSVC, Docker, **Dev Container**): **[docs/development.md](docs/development.md)**. Linux-in-Docker without local Rust: reopen this folder in a container (`.devcontainer/`).

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-dev.ps1
```

## Quick start

```python
import pypdfcraft as pdf

with pdf.open("document.pdf") as doc:
    assert not doc.needs_pass
    print(len(doc), doc[0].extract_text())
    png = doc[0].render(max_side=200, colorspace="gray")
    doc.save("out.pdf")

# Page-range surgery (1-based inclusive ranges)
out = pdf.extract_pages("big.pdf", ranges=[(1, 3), (10, 12)])

# Merge
merged = pdf.Document()
merged.append("a.pdf")
merged.append("b.pdf", pages=[0, 1])
merged.save("merged.pdf")

# Long tail (tool page numbers are 1-based)
with pdf.open("document.pdf") as doc:
    hits = doc.session.call("text_find", doc=doc.doc_id, query="invoice")
```

See [docs/JASON.md](docs/JASON.md) for jason seam mapping.

Use `doc.session.call("tool_name", **kwargs)` for any of pdfcraft’s automation tools; `pdf.tools()` lists schemas.

## Jason integration (drop-in targets)

| jason seam | pypdfcraft |
|---|---|
| `split_thumbs.render_page` | `page.render(max_side=…)` → `Bitmap.data` (PNG bytes) |
| `library._pdf_layer` / `TextLayer` | `doc.extract_text()` / `page.extract_text()` |
| `record_upload.cut_pages` | `extract_pages(...)` or `doc.extract(pages=…)` |
| `model_extractor.page_images` | `page.render(dpi=…).data` |
| `mailroom.page_count` | `len(doc)` |
| `pdf_preflight` media facts (partial) | `page.media()` / `doc.page_facts()` |

Thread safety: one `Document` / `Session` is not shared across threads without an external lock. The underlying pdfcraft session is single-threaded.

Page indices in the Python API are **0-based**; automation `call()` tools use **1-based** pages, matching pdfcraft-cli/MCP.

## License

MIT OR Apache-2.0, same as pdfcraft.
