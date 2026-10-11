use std::sync::{Arc, Mutex};

use pdfcraft_automation::{Automation, Content, ToolDef};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

use crate::error::{map_tool_error, PdfError};

#[pyclass(module = "pypdfcraft")]
pub struct Session {
    pub(crate) automation: Arc<Mutex<Automation>>,
}

impl Session {
    pub fn new_automation() -> Self {
        Self {
            automation: Arc::new(Mutex::new(Automation::new())),
        }
    }

    pub fn from_arc(automation: Arc<Mutex<Automation>>) -> Self {
        Self { automation }
    }

    pub fn lock(&self) -> PyResult<std::sync::MutexGuard<'_, Automation>> {
        self.automation
            .lock()
            .map_err(|_| PdfError::new_err("the pdfcraft session lock was poisoned"))
    }
}

#[pymethods]
impl Session {
    /// Call a pdfcraft automation tool by name. Tool page numbers are **1-based**.
    #[pyo3(signature = (name, kwargs=None))]
    fn call(
        &self,
        py: Python<'_>,
        name: &str,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        let args = kwargs_to_json(kwargs)?;
        let mut auto = self.lock()?;
        let out = auto.call(name, &args).map_err(map_tool_error)?;
        content_to_py(py, out)
    }
}

pub fn tools_list(py: Python<'_>) -> PyResult<Py<PyAny>> {
    let defs = pdfcraft_automation::tools();
    let list = PyList::empty(py);
    for def in defs {
        list.append(schema_to_dict(py, &def)?)?;
    }
    Ok(list.into())
}

fn kwargs_to_json(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Value> {
    let Some(dict) = kwargs else {
        return Ok(Value::Object(serde_json::Map::new()));
    };
    let mut map = serde_json::Map::new();
    for (k, v) in dict.iter() {
        let key = k.extract::<String>()?;
        map.insert(key, py_to_json(v)?);
    }
    Ok(Value::Object(map))
}

pub fn py_to_json(v: Bound<'_, PyAny>) -> PyResult<Value> {
    if v.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = v.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = v.extract::<i64>() {
        return Ok(Value::Number(i.into()));
    }
    if let Ok(f) = v.extract::<f64>() {
        return Ok(
            serde_json::Number::from_f64(f)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }
    if let Ok(s) = v.extract::<String>() {
        return Ok(Value::String(s));
    }
    if let Ok(list) = v.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(py_to_json(item)?);
        }
        return Ok(Value::Array(arr));
    }
    if let Ok(dict) = v.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, val) in dict.iter() {
            map.insert(k.extract::<String>()?, py_to_json(val)?);
        }
        return Ok(Value::Object(map));
    }
    Err(PdfError::new_err(
        "only JSON-like tool arguments are supported (str, int, float, bool, list, dict)",
    ))
}

fn schema_to_dict(py: Python<'_>, def: &ToolDef) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("name", &def.name)?;
    d.set_item("title", &def.title)?;
    d.set_item("description", &def.description)?;
    d.set_item("input_schema", json_to_py(py, &def.input_schema)?)?;
    Ok(d.into())
}

fn content_to_py(py: Python<'_>, parts: Vec<Content>) -> PyResult<Py<PyAny>> {
    if parts.is_empty() {
        return Ok(py.None());
    }
    if parts.len() == 1 {
        return single_content(py, &parts[0]);
    }
    let list = PyList::empty(py);
    for part in &parts {
        list.append(single_content(py, part)?)?;
    }
    Ok(list.into())
}

fn single_content(py: Python<'_>, part: &Content) -> PyResult<Py<PyAny>> {
    match part {
        Content::Json(v) => json_to_py(py, v),
        Content::Png { data, width, height } => {
            let d = PyDict::new(py);
            d.set_item("png", data.clone())?;
            d.set_item("width", width)?;
            d.set_item("height", height)?;
            Ok(d.into())
        }
    }
}

pub fn json_to_py(py: Python<'_>, v: &Value) -> PyResult<Py<PyAny>> {
    match v {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_py(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Ok(py.None())
            }
        }
        Value::String(s) => Ok(s.into_py(py)),
        Value::Array(a) => {
            let list = PyList::empty(py);
            for item in a {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        Value::Object(o) => {
            let dict = PyDict::new(py);
            for (k, val) in o {
                dict.set_item(k, json_to_py(py, val)?)?;
            }
            Ok(dict.into())
        }
    }
}
