use pyo3::prelude::*;
use rtfkit_html::{
    CssMode, HtmlWriterOptions, document_to_html, document_to_html_with_warnings,
};
use rtfkit_style_tokens::StyleProfileName;

use crate::errors;
use crate::ir::PyDocument;

// =============================================================================
// HtmlOutput
// =============================================================================

#[pyclass(name = "HtmlOutput", frozen)]
pub struct PyHtmlOutput {
    #[pyo3(get)]
    html: String,
    #[pyo3(get)]
    dropped_content_reasons: Vec<String>,
}

#[pymethods]
impl PyHtmlOutput {
    fn __repr__(&self) -> String {
        format!(
            "HtmlOutput(html_len={}, dropped_reasons={})",
            self.html.len(),
            self.dropped_content_reasons.len(),
        )
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn build_options(
    emit_wrapper: Option<bool>,
    css_mode: Option<&str>,
    style_profile: Option<&str>,
    custom_css: Option<String>,
) -> PyResult<HtmlWriterOptions> {
    let mut opts = HtmlWriterOptions::default();

    if let Some(ew) = emit_wrapper {
        opts.emit_document_wrapper = ew;
    }

    if let Some(mode) = css_mode {
        opts.css_mode = match mode {
            "default" => CssMode::Default,
            "none" => CssMode::None,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid css_mode: {:?}. Must be 'default' or 'none'.",
                    other
                )));
            }
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

    opts.custom_css = custom_css;

    Ok(opts)
}

// =============================================================================
// Public functions
// =============================================================================

#[pyfunction]
#[pyo3(name = "to_html", signature = (document, *, emit_wrapper=None, css_mode=None, style_profile=None, custom_css=None))]
pub fn py_to_html(
    document: &PyDocument,
    emit_wrapper: Option<bool>,
    css_mode: Option<&str>,
    style_profile: Option<&str>,
    custom_css: Option<String>,
) -> PyResult<String> {
    let opts = build_options(emit_wrapper, css_mode, style_profile, custom_css)?;
    document_to_html(&document.inner, &opts)
        .map_err(|e| errors::HtmlWriterError::new_err(e.to_string()))
}

#[pyfunction]
#[pyo3(name = "to_html_with_warnings", signature = (document, *, emit_wrapper=None, css_mode=None, style_profile=None, custom_css=None))]
pub fn py_to_html_with_warnings(
    document: &PyDocument,
    emit_wrapper: Option<bool>,
    css_mode: Option<&str>,
    style_profile: Option<&str>,
    custom_css: Option<String>,
) -> PyResult<PyHtmlOutput> {
    let opts = build_options(emit_wrapper, css_mode, style_profile, custom_css)?;
    let output = document_to_html_with_warnings(&document.inner, &opts)
        .map_err(|e| errors::HtmlWriterError::new_err(e.to_string()))?;

    Ok(PyHtmlOutput {
        html: output.html,
        dropped_content_reasons: output.dropped_content_reasons,
    })
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHtmlOutput>()?;
    m.add_function(wrap_pyfunction!(py_to_html, m)?)?;
    m.add_function(wrap_pyfunction!(py_to_html_with_warnings, m)?)?;
    Ok(())
}
