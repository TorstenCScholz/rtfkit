use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

// Exception hierarchy:
//   RtfkitError (base)
//     +-- ParseError
//     +-- ReportError
//     +-- HtmlWriterError
//     +-- DocxWriterError
//     +-- PdfRenderError

create_exception!(rtfkit._native, RtfkitError, PyException, "Base exception for all rtfkit errors.");
create_exception!(rtfkit._native, ParseError, RtfkitError, "RTF parsing failed.");
create_exception!(rtfkit._native, ReportError, RtfkitError, "Report generation failed.");
create_exception!(rtfkit._native, HtmlWriterError, RtfkitError, "HTML generation failed.");
create_exception!(rtfkit._native, DocxWriterError, RtfkitError, "DOCX generation failed.");
create_exception!(rtfkit._native, PdfRenderError, RtfkitError, "PDF rendering failed.");

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("RtfkitError", m.py().get_type::<RtfkitError>())?;
    m.add("ParseError", m.py().get_type::<ParseError>())?;
    m.add("ReportError", m.py().get_type::<ReportError>())?;
    m.add("HtmlWriterError", m.py().get_type::<HtmlWriterError>())?;
    m.add("DocxWriterError", m.py().get_type::<DocxWriterError>())?;
    m.add("PdfRenderError", m.py().get_type::<PdfRenderError>())?;
    Ok(())
}
