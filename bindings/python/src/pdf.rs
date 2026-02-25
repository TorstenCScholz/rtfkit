use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rtfkit_render_typst::{
    DeterminismOptions, Margins, PageSize, RenderOptions, document_to_pdf_with_warnings,
};
use rtfkit_style_tokens::StyleProfileName;

use crate::errors;
use crate::ir::PyDocument;

// =============================================================================
// PdfOutput
// =============================================================================

#[pyclass(name = "PdfOutput", frozen)]
pub struct PyPdfOutput {
    pdf_data: Vec<u8>,
    warning_messages: Vec<String>,
}

#[pymethods]
impl PyPdfOutput {
    #[getter]
    fn pdf_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.pdf_data)
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.warning_messages.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "PdfOutput(pdf_len={}, warnings={})",
            self.pdf_data.len(),
            self.warning_messages.len(),
        )
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn build_render_options(
    page_size: Option<&str>,
    margin_top: Option<f32>,
    margin_bottom: Option<f32>,
    margin_left: Option<f32>,
    margin_right: Option<f32>,
    style_profile: Option<&str>,
    fixed_timestamp: Option<String>,
) -> PyResult<RenderOptions> {
    let mut opts = RenderOptions::default();

    if let Some(ps) = page_size {
        opts.page_size = match ps {
            "a4" | "A4" => PageSize::A4,
            "letter" | "Letter" => PageSize::Letter,
            "legal" | "Legal" => PageSize::Legal,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid page_size: {:?}. Must be 'a4', 'letter', or 'legal'.",
                    other
                )));
            }
        };
    }

    if margin_top.is_some() || margin_bottom.is_some() || margin_left.is_some() || margin_right.is_some() {
        let defaults = Margins::default();
        opts.margins = Margins {
            top: margin_top.unwrap_or(defaults.top),
            bottom: margin_bottom.unwrap_or(defaults.bottom),
            left: margin_left.unwrap_or(defaults.left),
            right: margin_right.unwrap_or(defaults.right),
        };
    }

    if let Some(profile) = style_profile {
        opts.style_profile = match profile {
            "classic" => StyleProfileName::Classic,
            "report" => StyleProfileName::Report,
            "compact" => StyleProfileName::Compact,
            other => StyleProfileName::Custom(other.to_string()),
        };
    }

    if let Some(ts) = fixed_timestamp {
        opts.determinism = DeterminismOptions {
            fixed_timestamp: Some(ts),
            normalize_metadata: false,
        };
    }

    Ok(opts)
}

// =============================================================================
// Public functions
// =============================================================================

#[pyfunction]
#[pyo3(name = "to_pdf", signature = (
    document,
    *,
    page_size = None,
    margin_top = None,
    margin_bottom = None,
    margin_left = None,
    margin_right = None,
    style_profile = None,
    fixed_timestamp = None,
))]
pub fn py_to_pdf<'py>(
    py: Python<'py>,
    document: &PyDocument,
    page_size: Option<&str>,
    margin_top: Option<f32>,
    margin_bottom: Option<f32>,
    margin_left: Option<f32>,
    margin_right: Option<f32>,
    style_profile: Option<&str>,
    fixed_timestamp: Option<String>,
) -> PyResult<Bound<'py, PyBytes>> {
    let opts = build_render_options(
        page_size,
        margin_top,
        margin_bottom,
        margin_left,
        margin_right,
        style_profile,
        fixed_timestamp,
    )?;

    let output = document_to_pdf_with_warnings(&document.inner, &opts)
        .map_err(|e| errors::PdfRenderError::new_err(e.to_string()))?;

    Ok(PyBytes::new(py, &output.pdf_bytes))
}

#[pyfunction]
#[pyo3(name = "to_pdf_with_warnings", signature = (
    document,
    *,
    page_size = None,
    margin_top = None,
    margin_bottom = None,
    margin_left = None,
    margin_right = None,
    style_profile = None,
    fixed_timestamp = None,
))]
pub fn py_to_pdf_with_warnings(
    document: &PyDocument,
    page_size: Option<&str>,
    margin_top: Option<f32>,
    margin_bottom: Option<f32>,
    margin_left: Option<f32>,
    margin_right: Option<f32>,
    style_profile: Option<&str>,
    fixed_timestamp: Option<String>,
) -> PyResult<PyPdfOutput> {
    let opts = build_render_options(
        page_size,
        margin_top,
        margin_bottom,
        margin_left,
        margin_right,
        style_profile,
        fixed_timestamp,
    )?;

    let output = document_to_pdf_with_warnings(&document.inner, &opts)
        .map_err(|e| errors::PdfRenderError::new_err(e.to_string()))?;

    Ok(PyPdfOutput {
        pdf_data: output.pdf_bytes,
        warning_messages: output.warnings.iter().map(|w| w.message.clone()).collect(),
    })
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPdfOutput>()?;
    m.add_function(wrap_pyfunction!(py_to_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(py_to_pdf_with_warnings, m)?)?;
    Ok(())
}
