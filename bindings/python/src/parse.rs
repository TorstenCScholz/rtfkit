use pyo3::prelude::*;
use rtfkit_core::error::ConversionError;

use crate::errors;
use crate::ir::PyDocument;
use crate::limits::PyParserLimits;
use crate::report::PyReport;

// =============================================================================
// ParseResult
// =============================================================================

#[pyclass(name = "ParseResult", frozen)]
pub struct PyParseResult {
    #[pyo3(get)]
    document: Py<PyDocument>,
    #[pyo3(get)]
    report: Py<PyReport>,
}

#[pymethods]
impl PyParseResult {
    fn __repr__(&self, py: Python<'_>) -> String {
        let doc = self.document.borrow(py);
        let report = self.report.borrow(py);
        format!(
            "ParseResult(document={}, warnings={})",
            doc.inner.blocks.len(),
            report.inner.warnings.len(),
        )
    }
}

// =============================================================================
// Error mapping
// =============================================================================

fn conversion_error_to_py(e: ConversionError) -> PyErr {
    match e {
        ConversionError::Parse(pe) => errors::ParseError::new_err(pe.to_string()),
        ConversionError::Report(re) => errors::ReportError::new_err(re.to_string()),
        ConversionError::Other(msg) => errors::RtfkitError::new_err(msg),
    }
}

// =============================================================================
// Public functions
// =============================================================================

#[pyfunction]
#[pyo3(name = "parse")]
pub fn py_parse(py: Python<'_>, rtf: &str) -> PyResult<PyParseResult> {
    let (doc, report) =
        rtfkit_core::parse(rtf).map_err(conversion_error_to_py)?;

    let py_doc = Py::new(py, PyDocument { inner: doc })?;
    let py_report = Py::new(py, PyReport { inner: report })?;

    Ok(PyParseResult {
        document: py_doc,
        report: py_report,
    })
}

#[pyfunction]
#[pyo3(name = "parse_with_limits")]
pub fn py_parse_with_limits(
    py: Python<'_>,
    rtf: &str,
    limits: &PyParserLimits,
) -> PyResult<PyParseResult> {
    let (doc, report) =
        rtfkit_core::parse_with_limits(rtf, limits.inner.clone())
            .map_err(conversion_error_to_py)?;

    let py_doc = Py::new(py, PyDocument { inner: doc })?;
    let py_report = Py::new(py, PyReport { inner: report })?;

    Ok(PyParseResult {
        document: py_doc,
        report: py_report,
    })
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParseResult>()?;
    m.add_function(wrap_pyfunction!(py_parse, m)?)?;
    m.add_function(wrap_pyfunction!(py_parse_with_limits, m)?)?;
    Ok(())
}
