use pyo3::prelude::*;

#[pyclass(module = "pypdfcraft")]
pub struct Bitmap {
    #[pyo3(get)]
    pub data: Vec<u8>,
    #[pyo3(get)]
    pub width: u32,
    #[pyo3(get)]
    pub height: u32,
    #[pyo3(get)]
    pub mime: String,
}

impl Bitmap {
    pub fn png(data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
            mime: "image/png".into(),
        }
    }
}
