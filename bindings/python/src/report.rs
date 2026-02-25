use pyo3::prelude::*;
use rtfkit_core::report::{Report, Stats, Warning};

use crate::convert;

// =============================================================================
// Stats
// =============================================================================

#[pyclass(name = "Stats", frozen)]
#[derive(Clone)]
pub struct PyStats {
    pub(crate) inner: Stats,
}

#[pymethods]
impl PyStats {
    #[getter]
    fn paragraph_count(&self) -> usize {
        self.inner.paragraph_count
    }

    #[getter]
    fn run_count(&self) -> usize {
        self.inner.run_count
    }

    #[getter]
    fn bytes_processed(&self) -> usize {
        self.inner.bytes_processed
    }

    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    fn __repr__(&self) -> String {
        format!(
            "Stats(paragraphs={}, runs={}, bytes={}, duration_ms={})",
            self.inner.paragraph_count,
            self.inner.run_count,
            self.inner.bytes_processed,
            self.inner.duration_ms,
        )
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &self.inner)
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&self.inner, indent)
    }
}

// =============================================================================
// Warning
// =============================================================================

#[pyclass(name = "Warning", frozen)]
#[derive(Clone)]
pub struct PyWarning {
    pub(crate) inner: Warning,
}

#[pymethods]
impl PyWarning {
    #[getter]
    fn warning_type(&self) -> String {
        serde_json::to_value(&self.inner)
            .ok()
            .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(String::from))
            .unwrap_or_else(|| "unknown".to_string())
    }

    #[getter]
    fn severity(&self) -> String {
        serde_json::to_value(self.inner.severity())
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self.inner.severity()))
    }

    fn __repr__(&self) -> String {
        format!("Warning(type={:?}, severity={:?})", self.warning_type(), self.severity())
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &self.inner)
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&self.inner, indent)
    }
}

// =============================================================================
// Report
// =============================================================================

#[pyclass(name = "Report", frozen)]
#[derive(Clone)]
pub struct PyReport {
    pub(crate) inner: Report,
}

#[pymethods]
impl PyReport {
    #[getter]
    fn warnings(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .warnings
            .iter()
            .map(|w| {
                Py::new(py, PyWarning { inner: w.clone() }).map(|p| p.into_any().into())
            })
            .collect()
    }

    #[getter]
    fn stats(&self) -> PyStats {
        PyStats {
            inner: self.inner.stats.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Report(warnings={}, stats={})",
            self.inner.warnings.len(),
            format!(
                "Stats(paragraphs={}, runs={})",
                self.inner.stats.paragraph_count, self.inner.stats.run_count
            )
        )
    }

    fn __len__(&self) -> usize {
        self.inner.warnings.len()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &self.inner)
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&self.inner, indent)
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyStats>()?;
    m.add_class::<PyWarning>()?;
    m.add_class::<PyReport>()?;
    Ok(())
}
