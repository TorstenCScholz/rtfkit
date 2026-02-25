use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rtfkit_docx::{write_docx, write_docx_to_bytes};
use std::path::Path;

use crate::errors;
use crate::ir::PyDocument;

#[pyfunction]
#[pyo3(name = "to_docx_bytes")]
pub fn py_to_docx_bytes<'py>(py: Python<'py>, document: &PyDocument) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = write_docx_to_bytes(&document.inner)
        .map_err(|e| errors::DocxWriterError::new_err(e.to_string()))?;
    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
#[pyo3(name = "to_docx_file")]
pub fn py_to_docx_file(document: &PyDocument, path: &str) -> PyResult<()> {
    write_docx(&document.inner, Path::new(path))
        .map_err(|e| errors::DocxWriterError::new_err(e.to_string()))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_to_docx_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(py_to_docx_file, m)?)?;
    Ok(())
}
