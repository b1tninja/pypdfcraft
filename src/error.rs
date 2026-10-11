use pdfcraft_render::OpenError;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(pypdfcraft, PdfError, PyException);
create_exception!(pypdfcraft, PasswordError, PdfError);
create_exception!(pypdfcraft, PageUnreadable, PdfError);
create_exception!(pypdfcraft, RendererUnavailable, PdfError);

pub fn map_open_error(err: OpenError) -> PyErr {
    match err {
        OpenError::NeedsPassword => PasswordError::new_err(
            "the document is protected by a password; open it with password=…",
        ),
        OpenError::WrongPassword => PasswordError::new_err("the password is incorrect"),
        OpenError::Invalid(m) => PdfError::new_err(format!("the file is not a readable PDF: {m}")),
        OpenError::Unsupported(m) => PdfError::new_err(m),
    }
}

pub fn map_tool_error(err: pdfcraft_automation::ToolError) -> PyErr {
    match err {
        pdfcraft_automation::ToolError::UnknownTool(t) => {
            PdfError::new_err(format!("unknown tool {t:?}"))
        }
        pdfcraft_automation::ToolError::InvalidArgs(m) | pdfcraft_automation::ToolError::Failed(m) => {
            PdfError::new_err(m)
        }
    }
}

pub fn map_edit_error(err: pdfcraft_engine::EditError) -> PyErr {
    PdfError::new_err(err.to_string())
}
