use pyo3::prelude::*;

mod convert;
mod docx;
mod errors;
mod html;
mod ir;
mod limits;
mod parse;
mod pdf;
mod report;

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    errors::register(m)?;
    ir::register(m)?;
    report::register(m)?;
    limits::register(m)?;
    parse::register(m)?;
    html::register(m)?;
    docx::register(m)?;
    pdf::register(m)?;
    Ok(())
}
