use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pdfcraft_automation::Automation;
use pdfcraft_engine::DocId;
use pdfcraft_render::{PageRenderer, RenderConfig, RenderRequest, RequestKind};
use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::bitmap::Bitmap;
use crate::error::{map_tool_error, PageUnreadable, PdfError};
use crate::render_util::{dpi_for_max_side, render_page_png};
use crate::session::Session;

#[pyclass(module = "pypdfcraft")]
pub struct Word {
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub x0: f32,
    #[pyo3(get)]
    pub y0: f32,
    #[pyo3(get)]
    pub x1: f32,
    #[pyo3(get)]
    pub y1: f32,
}

#[pyclass(module = "pypdfcraft", unsendable)]
pub struct Page {
    session: Py<Session>,
    automation: Arc<Mutex<Automation>>,
    doc_id: DocId,
    index: usize,
    closed: Arc<AtomicBool>,
}

impl Page {
    pub fn new(
        session: Py<Session>,
        automation: Arc<Mutex<Automation>>,
        doc_id: DocId,
        index: usize,
        closed: Arc<AtomicBool>,
    ) -> Self {
        Self {
            session,
            automation,
            doc_id,
            index,
            closed,
        }
    }

    fn ensure_open(&self) -> PyResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(PdfError::new_err(
                "this page belongs to a document that has been closed",
            ));
        }
        Ok(())
    }

    fn with_doc<F, T>(&self, f: F) -> PyResult<T>
    where
        F: FnOnce(&pdfcraft_engine::Document) -> PyResult<T>,
    {
        self.ensure_open()?;
        let auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let doc = auto
            .session()
            .get(self.doc_id)
            .ok_or_else(|| PdfError::new_err("the document is no longer open"))?;
        f(doc)
    }
}

#[pymethods]
impl Page {
    #[getter]
    fn number(&self) -> PyResult<usize> {
        self.ensure_open()?;
        Ok(self.index)
    }

    #[getter]
    fn rotation(&self) -> PyResult<u16> {
        self.with_doc(|doc| {
            let p = doc.info.pages.get(self.index).ok_or_else(|| {
                PdfError::new_err(format!("page {} is out of range", self.index))
            })?;
            Ok(p.rotation)
        })
    }

    #[getter]
    fn width(&self) -> PyResult<f32> {
        self.with_doc(|doc| {
            let p = doc.info.pages.get(self.index).ok_or_else(|| {
                PdfError::new_err(format!("page {} is out of range", self.index))
            })?;
            Ok(p.width)
        })
    }

    #[getter]
    fn height(&self) -> PyResult<f32> {
        self.with_doc(|doc| {
            let p = doc.info.pages.get(self.index).ok_or_else(|| {
                PdfError::new_err(format!("page {} is out of range", self.index))
            })?;
            Ok(p.height)
        })
    }

    fn extract_text(&self) -> PyResult<String> {
        self.ensure_open()?;
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let args = serde_json::json!({
            "doc": self.doc_id.0,
            "pages": [self.index + 1]
        });
        let out = crate::util::call_json(&mut auto, "text_extract", &args)
            .map_err(map_tool_error)?;
        let pages = out
            .get("pages")
            .and_then(|p| p.as_array())
            .ok_or_else(|| PdfError::new_err("text_extract returned an unexpected shape"))?;
        let text = pages
            .first()
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        Ok(text)
    }

    fn get_words(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.ensure_open()?;
        let (display, password) = {
            let auto = self
                .automation
                .lock()
                .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
            let doc = auto
                .session()
                .get(self.doc_id)
                .ok_or_else(|| PdfError::new_err("the document is no longer open"))?;
            (doc.display.clone(), doc.password.clone())
        };
        let config = RenderConfig {
            password: password.as_deref().map(Arc::from),
            ..Default::default()
        };
        let mut renderer = PageRenderer::new(display, config);
        let out = renderer.render(RenderRequest {
            page: self.index,
            kind: RequestKind::Text,
            tile: None,
            scale: 1.0,
            tag: 0,
        });
        if let Some(e) = out.error {
            return Err(PageUnreadable::new_err(format!(
                "page {} could not be read: {e}",
                self.index + 1
            )));
        }
        let text = out
            .text
            .ok_or_else(|| PageUnreadable::new_err("no text layer on this page"))?;
        let list = PyList::empty(py);
        for g in &text.glyphs {
            let t = g.text.trim();
            if t.is_empty() {
                continue;
            }
            let w = Py::new(
                py,
                Word {
                    text: g.text.clone(),
                    x0: g.rect[0],
                    y0: g.rect[1],
                    x1: g.rect[2],
                    y1: g.rect[3],
                },
            )?;
            list.append(w)?;
        }
        Ok(list.into())
    }

    #[pyo3(signature = (*, dpi=None, max_side=None, colorspace="rgb", rotate=0))]
    fn render(
        &self,
        dpi: Option<f64>,
        max_side: Option<u32>,
        colorspace: &str,
        rotate: i32,
    ) -> PyResult<Bitmap> {
        self.ensure_open()?;
        if dpi.is_some() && max_side.is_some() {
            return Err(PdfError::new_err("pass dpi or max_side, not both"));
        }
        let (display, password, width_pts, height_pts) = {
            let auto = self
                .automation
                .lock()
                .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
            let doc = auto
                .session()
                .get(self.doc_id)
                .ok_or_else(|| PdfError::new_err("the document is no longer open"))?;
            let p = doc.info.pages.get(self.index).ok_or_else(|| {
                PdfError::new_err(format!("page {} is out of range", self.index))
            })?;
            (
                doc.display.clone(),
                doc.password.clone(),
                p.width,
                p.height,
            )
        };
        let dpi = match (dpi, max_side) {
            (Some(d), None) => d,
            (None, Some(ms)) => dpi_for_max_side(width_pts, height_pts, ms),
            (None, None) => 96.0,
            _ => unreachable!(),
        };
        if !(1.0..=600.0).contains(&dpi) {
            return Err(PdfError::new_err("dpi must be between 1 and 600"));
        }
        let pass = password.as_deref();
        let (data, width, height) =
            render_page_png(&display, pass, self.index, dpi, colorspace).map_err(map_tool_error)?;
        let _ = rotate; // visual rotation applied in a later revision; page /Rotate is in the PDF
        Ok(Bitmap::png(data, width, height))
    }

    fn media(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use crate::document::page_media_py;
        self.ensure_open()?;
        page_media_py(py, &self.automation, self.doc_id, self.index)
    }
}
