#![allow(unsafe_code)]

mod bitmap;
mod document;
mod error;
mod page;
mod render_util;
mod session;
mod util;

use std::fs;
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use crate::document::Document;
use crate::error::{map_edit_error, map_tool_error, PdfError, PasswordError, PageUnreadable, RendererUnavailable};
use crate::session::{tools_list, Session};
use crate::util::{expand_ranges_1based, open_bytes_in_automation};

const VERSION: &str = "0.1.0";

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("__version__", VERSION)?;
    m.add_class::<Document>()?;
    m.add_class::<page::Page>()?;
    m.add_class::<page::Word>()?;
    m.add_class::<bitmap::Bitmap>()?;
    m.add_class::<document::PageMedia>()?;
    m.add_class::<Session>()?;
    m.add("PdfError", py.get_type::<PdfError>())?;
    m.add("PasswordError", py.get_type::<PasswordError>())?;
    m.add("PageUnreadable", py.get_type::<PageUnreadable>())?;
    m.add("RendererUnavailable", py.get_type::<RendererUnavailable>())?;
    m.add_function(wrap_pyfunction!(open, m)?)?;
    m.add_function(wrap_pyfunction!(extract_pages, m)?)?;
    m.add_function(wrap_pyfunction!(tools, m)?)?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (*, path=None, bytes=None, password=None))]
fn open(
    py: Python<'_>,
    path: Option<&str>,
    bytes: Option<&[u8]>,
    password: Option<String>,
) -> PyResult<Document> {
    let data = read_bytes(path, bytes)?;
    Document::open_bytes(py, Arc::new(data), password)
}

#[pyfunction]
#[pyo3(signature = (*, path=None, bytes=None, password=None, ranges))]
fn extract_pages(
    path: Option<&str>,
    bytes: Option<&[u8]>,
    password: Option<String>,
    ranges: Vec<(u32, u32)>,
) -> PyResult<Vec<u8>> {
    let data = read_bytes(path, bytes)?;
    let pages = expand_ranges_1based(&ranges);
    if pages.is_empty() {
        return Err(PdfError::new_err("ranges must include at least one page"));
    }
    let mut auto = pdfcraft_automation::Automation::new();
    let pass = password.as_deref();
    let id = open_bytes_in_automation(&mut auto, &data, pass)?;
    let out = auto
        .session()
        .extract(id, &pages)
        .map_err(map_edit_error)?;
    let _ = util::call_json(
        &mut auto,
        "doc_close",
        &serde_json::json!({ "doc": id.0, "discard_changes": true }),
    )
    .map_err(map_tool_error)?;
    Ok(out.to_vec())
}

#[pyfunction]
fn tools(py: Python<'_>) -> PyResult<Py<PyAny>> {
    tools_list(py)
}

fn read_bytes(path: Option<&str>, bytes: Option<&[u8]>) -> PyResult<Vec<u8>> {
    match (path, bytes) {
        (Some(p), None) => fs::read(p).map_err(|e| PdfError::new_err(format!("{p}: {e}"))),
        (None, Some(b)) => Ok(b.to_vec()),
        (Some(_), Some(_)) => Err(PdfError::new_err("pass path or bytes, not both")),
        (None, None) => Err(PdfError::new_err("pass path or bytes to open a PDF")),
    }
}

