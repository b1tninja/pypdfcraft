use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pdfcraft_automation::Automation;
use pdfcraft_engine::DocId;
use pdfcraft_render::inspect;
use pdfcraft_render::OpenError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::json;

use crate::error::{map_edit_error, map_open_error, map_tool_error, PdfError};
use crate::page::Page;
use crate::session::Session;
use crate::util::{
    call_json, doc_id_from_value, open_bytes_in_automation, pages_one_based, range_string_1based,
};

#[pyclass(module = "pypdfcraft")]
pub struct PageMedia {
    #[pyo3(get)]
    pub width: f32,
    #[pyo3(get)]
    pub height: f32,
    #[pyo3(get)]
    pub rotation: u16,
    #[pyo3(get)]
    pub image_count: usize,
    #[pyo3(get)]
    pub codecs: Vec<String>,
    #[pyo3(get)]
    pub has_jbig2: bool,
    #[pyo3(get)]
    pub has_ccitt: bool,
}

#[pyclass(module = "pypdfcraft", unsendable)]
pub struct Document {
    session: Py<Session>,
    automation: Arc<Mutex<Automation>>,
    doc_id: DocId,
    needs_pass: bool,
    closed: Arc<AtomicBool>,
}

impl Document {
    pub fn open_bytes(
        py: Python<'_>,
        bytes: Arc<Vec<u8>>,
        password: Option<String>,
    ) -> PyResult<Self> {
        let session = Py::new(py, Session::new_automation())?;
        let automation = session.bind(py).borrow().automation.clone();
        let pass_ref = password.as_deref();
        match inspect(bytes.clone(), pass_ref) {
            Ok(_) => {}
            Err(OpenError::NeedsPassword) if password.is_none() => {
                return Ok(Self {
                    session: session.clone_ref(py),
                    automation,
                    doc_id: DocId(0),
                    needs_pass: true,
                    closed: Arc::new(AtomicBool::new(false)),
                });
            }
            Err(e) => return Err(map_open_error(e)),
        }
        let id = {
            let mut auto = automation
                .lock()
                .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
            open_bytes_in_automation(&mut auto, &bytes, pass_ref)?
        };
        Ok(Self {
            session: session.clone_ref(py),
            automation,
            doc_id: id,
            needs_pass: false,
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn new_blank(py: Python<'_>) -> PyResult<Self> {
        let session = Py::new(py, Session::new_automation())?;
        let automation = session.bind(py).borrow().automation.clone();
        let summary = {
            let mut auto = automation
                .lock()
                .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
            call_json(&mut auto, "doc_create", &json!({ "from": "blank", "pages": 1 }))
                .map_err(map_tool_error)?
        };
        let id = doc_id_from_value(&summary)?;
        Ok(Self {
            session: session.clone_ref(py),
            automation,
            doc_id: id,
            needs_pass: false,
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn ensure_open(&self) -> PyResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(PdfError::new_err("this document has been closed"));
        }
        if self.needs_pass {
            return Err(PdfError::new_err(
                "the document needs a password; open it again with password=…",
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
impl Document {
    #[new]
    fn py_new(py: Python<'_>) -> PyResult<Self> {
        Document::new_blank(py)
    }

    #[getter]
    fn doc_id(&self) -> u64 {
        self.doc_id.0
    }

    #[getter]
    fn needs_pass(&self) -> bool {
        self.needs_pass
    }

    #[getter]
    fn page_count(&self) -> PyResult<usize> {
        if self.needs_pass {
            return Ok(0);
        }
        self.with_doc(|doc| Ok(doc.info.pages.len()))
    }

    fn __len__(&self) -> PyResult<usize> {
        self.page_count()
    }

    fn __getitem__(&self, py: Python<'_>, index: isize) -> PyResult<Page> {
        self.ensure_open()?;
        let n = self.page_count()?;
        let idx = if index < 0 {
            (n as isize + index) as usize
        } else {
            index as usize
        };
        if idx >= n {
            return Err(PdfError::new_err(format!(
                "page index {index} is out of range for {n} pages"
            )));
        }
        Ok(Page::new(
            self.session.clone_ref(py),
            self.automation.clone(),
            self.doc_id,
            idx,
            self.closed.clone(),
        ))
    }

    fn extract_text(&self) -> PyResult<String> {
        self.ensure_open()?;
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let out = call_json(&mut auto, "text_extract", &json!({ "doc": self.doc_id.0 }))
            .map_err(map_tool_error)?;
        let mut text = String::new();
        if let Some(pages) = out.get("pages").and_then(|p| p.as_array()) {
            for p in pages {
                if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
                    text.push_str(t);
                    text.push('\n');
                }
            }
        }
        Ok(text.trim_end().to_string())
    }

    #[pyo3(signature = (*, path=None, full=false))]
    fn save(&self, path: Option<&str>, full: bool) -> PyResult<()> {
        self.ensure_open()?;
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let mut args = json!({ "doc": self.doc_id.0, "full": full });
        if let Some(p) = path {
            args["path"] = json!(p);
        }
        call_json(&mut auto, "doc_save", &args).map_err(map_tool_error)?;
        Ok(())
    }

    #[pyo3(signature = (full=false))]
    fn to_bytes(&self, full: bool) -> PyResult<Vec<u8>> {
        self.ensure_open()?;
        let auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let bytes = if full {
            auto.session().save_full_bytes(self.doc_id)
        } else {
            auto.session().save_bytes(self.doc_id)
        }
        .map_err(map_edit_error)?;
        Ok(bytes.to_vec())
    }

    fn close(&mut self) -> PyResult<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if !self.needs_pass && self.doc_id.0 != 0 {
            let mut auto = self
                .automation
                .lock()
                .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
            call_json(
                &mut auto,
                "doc_close",
                &json!({ "doc": self.doc_id.0, "discard_changes": true }),
            )
            .map_err(map_tool_error)?;
        }
        Ok(())
    }

    fn __enter__(slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc=None, _tb=None))]
    fn __exit__(
        &mut self,
        _exc_type: Option<Py<PyAny>>,
        _exc: Option<Py<PyAny>>,
        _tb: Option<Py<PyAny>>,
    ) -> PyResult<()> {
        self.close()
    }

    #[getter]
    fn session(&self, py: Python<'_>) -> PyResult<Py<Session>> {
        Ok(self.session.clone_ref(py))
    }

    #[pyo3(signature = (*, pages))]
    fn extract(&self, pages: Vec<usize>) -> PyResult<Vec<u8>> {
        self.ensure_open()?;
        let auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let bytes = auto
            .session()
            .extract(self.doc_id, &pages)
            .map_err(map_edit_error)?;
        Ok(bytes.to_vec())
    }

    #[pyo3(signature = (*, path, pages=None))]
    fn append(&mut self, path: &str, pages: Option<Vec<usize>>) -> PyResult<()> {
        self.ensure_open()?;
        let other = std::fs::read(path).map_err(|e| PdfError::new_err(format!("{path}: {e}")))?;
        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "append.pdf".into());
        let page_range = pages.map(|ps| range_string_1based(&ps));
        let current = self.to_bytes(false)?;
        let combined = {
            let auto = self
                .automation
                .lock()
                .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
            auto.session()
                .combine_ranges(&[
                    ("current.pdf".into(), Arc::new(current), None),
                    (name, Arc::new(other), page_range),
                ])
                .map_err(map_edit_error)?
        };
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        call_json(
            &mut auto,
            "doc_close",
            &json!({ "doc": self.doc_id.0, "discard_changes": true }),
        )
        .map_err(map_tool_error)?;
        self.doc_id = open_bytes_in_automation(&mut auto, &combined, None)?;
        Ok(())
    }

    #[pyo3(signature = (*, pages, degrees))]
    fn rotate_pages(&mut self, pages: Vec<usize>, degrees: i64) -> PyResult<()> {
        self.ensure_open()?;
        if degrees % 90 != 0 {
            return Err(PdfError::new_err("degrees must be a multiple of 90"));
        }
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        call_json(
            &mut auto,
            "page_rotate",
            &json!({
                "doc": self.doc_id.0,
                "pages": pages_one_based(&pages),
                "degrees": degrees,
            }),
        )
        .map_err(map_tool_error)?;
        Ok(())
    }

    fn form_fields(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.ensure_open()?;
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let out = call_json(&mut auto, "form_fields", &json!({ "doc": self.doc_id.0 }))
            .map_err(map_tool_error)?;
        crate::session::json_to_py(py, &out)
    }

    fn form_fill(&mut self, values: Bound<'_, PyDict>) -> PyResult<()> {
        self.ensure_open()?;
        let mut map = serde_json::Map::new();
        for (k, v) in values.iter() {
            let key = k.extract::<String>()?;
            map.insert(key, crate::session::py_to_json(v)?);
        }
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        call_json(
            &mut auto,
            "form_fill",
            &json!({ "doc": self.doc_id.0, "values": map }),
        )
        .map_err(map_tool_error)?;
        Ok(())
    }

    #[pyo3(signature = (*, comments=false, forms=true))]
    fn flatten(&mut self, comments: bool, forms: bool) -> PyResult<()> {
        self.ensure_open()?;
        let mut auto = self
            .automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        call_json(
            &mut auto,
            "doc_flatten",
            &json!({
                "doc": self.doc_id.0,
                "comments": comments,
                "forms": forms,
            }),
        )
        .map_err(map_tool_error)?;
        Ok(())
    }

    fn page_facts(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.ensure_open()?;
        let n = self.page_count()?;
        let list = PyList::empty(py);
        for i in 0..n {
            list.append(page_media_py(py, &self.automation, self.doc_id, i)?)?;
        }
        Ok(list.into())
    }
}

pub fn page_media_py(
    py: Python<'_>,
    automation: &Arc<Mutex<Automation>>,
    doc_id: DocId,
    page_index: usize,
) -> PyResult<Py<PyAny>> {
    let (width, height, rotation) = {
        let auto = automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
        let doc = auto
            .session()
            .get(doc_id)
            .ok_or_else(|| PdfError::new_err("the document is no longer open"))?;
        let p = doc.info.pages.get(page_index).ok_or_else(|| {
            PdfError::new_err(format!("page {} is out of range", page_index))
        })?;
        (p.width, p.height, p.rotation)
    };
    let mut auto = automation
        .lock()
        .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))?;
    let out = call_json(
        &mut auto,
        "page_images",
        &json!({ "doc": doc_id.0, "page": page_index + 1 }),
    )
    .map_err(map_tool_error)?;
    let images = out
        .get("images")
        .and_then(|i| i.as_array())
        .cloned()
        .unwrap_or_default();
    let mut codecs = Vec::new();
    let mut has_jbig2 = false;
    let mut has_ccitt = false;
    for img in &images {
        if img.get("kind").and_then(|k| k.as_str()) == Some("image") {
            if let Some(px) = img.get("pixels").and_then(|p| p.as_array()) {
                if px.len() >= 2 {
                    let w = px[0].as_u64().unwrap_or(0);
                    let h = px[1].as_u64().unwrap_or(0);
                    codecs.push(format!("{w}x{h}"));
                }
            }
            if let Some(name) = img.get("name").and_then(|n| n.as_str()) {
                let lower = name.to_lowercase();
                if lower.contains("jbig2") {
                    has_jbig2 = true;
                }
                if lower.contains("ccitt") || lower.contains("fax") {
                    has_ccitt = true;
                }
            }
        }
    }
    let media = Py::new(
        py,
        PageMedia {
            width,
            height,
            rotation,
            image_count: images.len(),
            codecs,
            has_jbig2,
            has_ccitt,
        },
    )?;
    Ok(media.into_any())
}
