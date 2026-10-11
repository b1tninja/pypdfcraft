use std::io::Write;

use pdfcraft_automation::{Automation, Content, ToolError};
use pdfcraft_engine::DocId;
use pyo3::PyResult;
use serde_json::Value;
use tempfile::NamedTempFile;

use crate::error::{map_tool_error, PdfError};

pub fn call_json(auto: &mut Automation, name: &str, args: &Value) -> Result<Value, ToolError> {
    let parts = auto.call(name, args)?;
    match parts.first() {
        Some(Content::Json(v)) => Ok(v.clone()),
        Some(Content::Png { .. }) => Err(ToolError::Failed(format!(
            "tool {name} returned an image, not JSON"
        ))),
        None => Ok(Value::Null),
    }
}

pub fn doc_id_from_value(v: &Value) -> PyResult<DocId> {
    let id = v
        .get("doc")
        .and_then(|d| d.as_u64())
        .ok_or_else(|| PdfError::new_err("pdfcraft returned no document id"))?;
    Ok(DocId(id))
}

pub fn write_temp_pdf(bytes: &[u8]) -> PyResult<NamedTempFile> {
    let mut tmp = NamedTempFile::new().map_err(|e| PdfError::new_err(e.to_string()))?;
    tmp.write_all(bytes).map_err(|e| PdfError::new_err(e.to_string()))?;
    Ok(tmp)
}

pub fn open_bytes_in_automation(
    auto: &mut Automation,
    bytes: &[u8],
    password: Option<&str>,
) -> PyResult<DocId> {
    let tmp = write_temp_pdf(bytes)?;
    let path = tmp.path().to_string_lossy().to_string();
    let mut args = serde_json::json!({ "path": path });
    if let Some(pw) = password {
        args["password"] = Value::String(pw.to_string());
    }
    let summary = call_json(auto, "doc_open", &args).map_err(map_tool_error)?;
    doc_id_from_value(&summary)
}

pub fn pages_one_based(pages: &[usize]) -> Vec<i64> {
    pages.iter().map(|p| (*p as i64) + 1).collect()
}

pub fn range_string_1based(pages: &[usize]) -> String {
    pages
        .iter()
        .map(|p| (p + 1).to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn expand_ranges_1based(ranges: &[(u32, u32)]) -> Vec<usize> {
    let mut out = Vec::new();
    for &(start, end) in ranges {
        for p in start..=end {
            if p > 0 {
                out.push((p - 1) as usize);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_ranges() {
        assert_eq!(expand_ranges_1based(&[(1, 2), (4, 4)]), vec![0, 1, 3]);
    }
}
