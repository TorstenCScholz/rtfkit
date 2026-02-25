use pyo3::prelude::*;
use rtfkit_core::limits::ParserLimits;

#[pyclass(name = "ParserLimits")]
#[derive(Clone)]
pub struct PyParserLimits {
    pub(crate) inner: ParserLimits,
}

#[pymethods]
impl PyParserLimits {
    #[new]
    #[pyo3(signature = (
        *,
        max_input_bytes = None,
        max_group_depth = None,
        max_warning_count = None,
        max_rows_per_table = None,
        max_cells_per_row = None,
        max_merge_span = None,
        max_image_bytes_total = None,
    ))]
    fn new(
        max_input_bytes: Option<usize>,
        max_group_depth: Option<usize>,
        max_warning_count: Option<usize>,
        max_rows_per_table: Option<usize>,
        max_cells_per_row: Option<usize>,
        max_merge_span: Option<u16>,
        max_image_bytes_total: Option<usize>,
    ) -> Self {
        let mut limits = ParserLimits::default();
        if let Some(v) = max_input_bytes {
            limits.max_input_bytes = v;
        }
        if let Some(v) = max_group_depth {
            limits.max_group_depth = v;
        }
        if let Some(v) = max_warning_count {
            limits.max_warning_count = v;
        }
        if let Some(v) = max_rows_per_table {
            limits.max_rows_per_table = v;
        }
        if let Some(v) = max_cells_per_row {
            limits.max_cells_per_row = v;
        }
        if let Some(v) = max_merge_span {
            limits.max_merge_span = v;
        }
        if let Some(v) = max_image_bytes_total {
            limits.max_image_bytes_total = v;
        }
        Self { inner: limits }
    }

    #[staticmethod]
    fn unlimited() -> Self {
        Self {
            inner: ParserLimits::none(),
        }
    }

    #[getter]
    fn max_input_bytes(&self) -> usize {
        self.inner.max_input_bytes
    }

    #[getter]
    fn max_group_depth(&self) -> usize {
        self.inner.max_group_depth
    }

    #[getter]
    fn max_warning_count(&self) -> usize {
        self.inner.max_warning_count
    }

    #[getter]
    fn max_rows_per_table(&self) -> usize {
        self.inner.max_rows_per_table
    }

    #[getter]
    fn max_cells_per_row(&self) -> usize {
        self.inner.max_cells_per_row
    }

    #[getter]
    fn max_merge_span(&self) -> u16 {
        self.inner.max_merge_span
    }

    #[getter]
    fn max_image_bytes_total(&self) -> usize {
        self.inner.max_image_bytes_total
    }

    fn __repr__(&self) -> String {
        format!(
            "ParserLimits(max_input_bytes={}, max_group_depth={}, max_warning_count={})",
            self.inner.max_input_bytes, self.inner.max_group_depth, self.inner.max_warning_count,
        )
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParserLimits>()?;
    Ok(())
}
