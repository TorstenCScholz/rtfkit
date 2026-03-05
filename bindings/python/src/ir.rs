use pyo3::prelude::*;
use rtfkit_core::{
    Block, CellMerge, Document, Hyperlink, HyperlinkTarget, ImageBlock, Inline, ListBlock,
    ListItem, Paragraph, Run, Shading, TableBlock, TableCell, TableRow,
};

use crate::convert;

// =============================================================================
// Color
// =============================================================================

#[pyclass(name = "Color", frozen)]
#[derive(Clone)]
pub struct PyColor {
    pub(crate) inner: rtfkit_core::Color,
}

#[pymethods]
impl PyColor {
    #[getter]
    fn r(&self) -> u8 {
        self.inner.r
    }

    #[getter]
    fn g(&self) -> u8 {
        self.inner.g
    }

    #[getter]
    fn b(&self) -> u8 {
        self.inner.b
    }

    fn __repr__(&self) -> String {
        format!("Color(r={}, g={}, b={})", self.inner.r, self.inner.g, self.inner.b)
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
// Shading
// =============================================================================

#[pyclass(name = "Shading", frozen)]
#[derive(Clone)]
pub struct PyShading {
    pub(crate) inner: Shading,
}

#[pymethods]
impl PyShading {
    #[getter]
    fn fill_color(&self) -> Option<PyColor> {
        self.inner.fill_color.as_ref().map(|c| PyColor { inner: c.clone() })
    }

    #[getter]
    fn pattern_color(&self) -> Option<PyColor> {
        self.inner.pattern_color.as_ref().map(|c| PyColor { inner: c.clone() })
    }

    #[getter]
    fn pattern(&self) -> Option<String> {
        self.inner.pattern.as_ref().map(|p| {
            serde_json::to_value(p)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{:?}", p))
        })
    }

    fn __repr__(&self) -> String {
        format!("Shading(pattern={:?})", self.pattern())
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
// Run
// =============================================================================

#[pyclass(name = "Run", frozen)]
#[derive(Clone)]
pub struct PyRun {
    pub(crate) inner: Run,
}

#[pymethods]
impl PyRun {
    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    #[getter]
    fn bold(&self) -> bool {
        self.inner.bold
    }

    #[getter]
    fn italic(&self) -> bool {
        self.inner.italic
    }

    #[getter]
    fn underline(&self) -> bool {
        self.inner.underline
    }

    #[getter]
    fn font_family(&self) -> Option<&str> {
        self.inner.font_family.as_deref()
    }

    #[getter]
    fn font_size(&self) -> Option<f32> {
        self.inner.font_size
    }

    #[getter]
    fn color(&self) -> Option<PyColor> {
        self.inner.color.as_ref().map(|c| PyColor { inner: c.clone() })
    }

    #[getter]
    fn background_color(&self) -> Option<PyColor> {
        self.inner
            .background_color
            .as_ref()
            .map(|c| PyColor { inner: c.clone() })
    }

    #[getter]
    fn inline_type(&self) -> &str {
        "run"
    }

    fn __repr__(&self) -> String {
        format!("Run(text={:?}, bold={}, italic={})", self.inner.text, self.inner.bold, self.inner.italic)
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &Inline::Run(self.inner.clone()))
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&Inline::Run(self.inner.clone()), indent)
    }
}

// =============================================================================
// Hyperlink
// =============================================================================

#[pyclass(name = "Hyperlink", frozen)]
#[derive(Clone)]
pub struct PyHyperlink {
    pub(crate) inner: Hyperlink,
}

#[pymethods]
impl PyHyperlink {
    #[getter]
    fn target(&self) -> &str {
        match &self.inner.target {
            HyperlinkTarget::ExternalUrl(url) => url.as_str(),
            HyperlinkTarget::InternalBookmark(name) => name.as_str(),
        }
    }

    #[getter]
    fn target_type(&self) -> &str {
        match &self.inner.target {
            HyperlinkTarget::ExternalUrl(_) => "external_url",
            HyperlinkTarget::InternalBookmark(_) => "internal_bookmark",
        }
    }

    #[getter]
    fn runs(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .runs
            .iter()
            .map(|r| {
                Py::new(py, PyRun { inner: r.clone() }).map(|p| p.into_any().into())
            })
            .collect()
    }

    #[getter]
    fn inline_type(&self) -> &str {
        "hyperlink"
    }

    fn __repr__(&self) -> String {
        format!("Hyperlink(target={:?}, runs={})", self.target(), self.inner.runs.len())
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &Inline::Hyperlink(self.inner.clone()))
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&Inline::Hyperlink(self.inner.clone()), indent)
    }
}

// =============================================================================
// Paragraph
// =============================================================================

#[pyclass(name = "Paragraph", frozen)]
#[derive(Clone)]
pub struct PyParagraph {
    pub(crate) inner: Paragraph,
}

#[pymethods]
impl PyParagraph {
    #[getter]
    fn alignment(&self) -> String {
        serde_json::to_value(&self.inner.alignment)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self.inner.alignment))
    }

    #[getter]
    fn inlines(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .inlines
            .iter()
            .map(|i| inline_to_py(py, i))
            .collect()
    }

    #[getter]
    fn shading(&self) -> Option<PyShading> {
        self.inner.shading.as_ref().map(|s| PyShading { inner: s.clone() })
    }

    #[getter]
    fn block_type(&self) -> &str {
        "paragraph"
    }

    fn __repr__(&self) -> String {
        format!("Paragraph(inlines={}, alignment={:?})", self.inner.inlines.len(), self.alignment())
    }

    fn __len__(&self) -> usize {
        self.inner.inlines.len()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &Block::Paragraph(self.inner.clone()))
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&Block::Paragraph(self.inner.clone()), indent)
    }
}

// =============================================================================
// ListItem
// =============================================================================

#[pyclass(name = "ListItem", frozen)]
#[derive(Clone)]
pub struct PyListItem {
    pub(crate) inner: ListItem,
}

#[pymethods]
impl PyListItem {
    #[getter]
    fn level(&self) -> u8 {
        self.inner.level
    }

    #[getter]
    fn blocks(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .blocks
            .iter()
            .map(|b| block_to_py(py, b))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("ListItem(level={}, blocks={})", self.inner.level, self.inner.blocks.len())
    }
}

// =============================================================================
// ListBlock
// =============================================================================

#[pyclass(name = "ListBlock", frozen)]
#[derive(Clone)]
pub struct PyListBlock {
    pub(crate) inner: ListBlock,
}

#[pymethods]
impl PyListBlock {
    #[getter]
    fn list_id(&self) -> u32 {
        self.inner.list_id
    }

    #[getter]
    fn kind(&self) -> String {
        serde_json::to_value(&self.inner.kind)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self.inner.kind))
    }

    #[getter]
    fn items(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .items
            .iter()
            .map(|i| {
                Py::new(py, PyListItem { inner: i.clone() }).map(|p| p.into_any().into())
            })
            .collect()
    }

    #[getter]
    fn block_type(&self) -> &str {
        "listblock"
    }

    fn __repr__(&self) -> String {
        format!(
            "ListBlock(list_id={}, kind={:?}, items={})",
            self.inner.list_id,
            self.kind(),
            self.inner.items.len()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.items.len()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &Block::ListBlock(self.inner.clone()))
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&Block::ListBlock(self.inner.clone()), indent)
    }
}

// =============================================================================
// TableCell
// =============================================================================

#[pyclass(name = "TableCell", frozen)]
#[derive(Clone)]
pub struct PyTableCell {
    pub(crate) inner: TableCell,
}

#[pymethods]
impl PyTableCell {
    #[getter]
    fn blocks(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .blocks
            .iter()
            .map(|b| block_to_py(py, b))
            .collect()
    }

    #[getter]
    fn width_twips(&self) -> Option<i32> {
        self.inner.width_twips
    }

    #[getter]
    fn merge(&self) -> Option<String> {
        self.inner.merge.as_ref().map(|m| match m {
            CellMerge::None => "none".to_string(),
            CellMerge::HorizontalStart { .. } => "horizontal_start".to_string(),
            CellMerge::HorizontalContinue => "horizontal_continue".to_string(),
            CellMerge::VerticalStart => "vertical_start".to_string(),
            CellMerge::VerticalContinue => "vertical_continue".to_string(),
        })
    }

    #[getter]
    fn merge_span(&self) -> Option<u16> {
        self.inner.merge.as_ref().and_then(|m| match m {
            CellMerge::HorizontalStart { span } => Some(*span),
            _ => None,
        })
    }

    #[getter]
    fn v_align(&self) -> Option<String> {
        self.inner.v_align.as_ref().map(|a| {
            serde_json::to_value(a)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{:?}", a))
        })
    }

    #[getter]
    fn shading(&self) -> Option<PyShading> {
        self.inner.shading.as_ref().map(|s| PyShading { inner: s.clone() })
    }

    fn __repr__(&self) -> String {
        format!("TableCell(blocks={}, width_twips={:?})", self.inner.blocks.len(), self.inner.width_twips)
    }
}

// =============================================================================
// TableRow
// =============================================================================

#[pyclass(name = "TableRow", frozen)]
#[derive(Clone)]
pub struct PyTableRow {
    pub(crate) inner: TableRow,
}

#[pymethods]
impl PyTableRow {
    #[getter]
    fn cells(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .cells
            .iter()
            .map(|c| {
                Py::new(py, PyTableCell { inner: c.clone() }).map(|p| p.into_any().into())
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("TableRow(cells={})", self.inner.cells.len())
    }

    fn __len__(&self) -> usize {
        self.inner.cells.len()
    }
}

// =============================================================================
// TableBlock
// =============================================================================

#[pyclass(name = "TableBlock", frozen)]
#[derive(Clone)]
pub struct PyTableBlock {
    pub(crate) inner: TableBlock,
}

#[pymethods]
impl PyTableBlock {
    #[getter]
    fn rows(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .rows
            .iter()
            .map(|r| {
                Py::new(py, PyTableRow { inner: r.clone() }).map(|p| p.into_any().into())
            })
            .collect()
    }

    #[getter]
    fn block_type(&self) -> &str {
        "tableblock"
    }

    fn __repr__(&self) -> String {
        format!("TableBlock(rows={})", self.inner.rows.len())
    }

    fn __len__(&self) -> usize {
        self.inner.rows.len()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &Block::TableBlock(self.inner.clone()))
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&Block::TableBlock(self.inner.clone()), indent)
    }
}

// =============================================================================
// ImageBlock
// =============================================================================

#[pyclass(name = "ImageBlock", frozen)]
#[derive(Clone)]
pub struct PyImageBlock {
    pub(crate) inner: ImageBlock,
}

#[pymethods]
impl PyImageBlock {
    #[getter]
    fn format(&self) -> String {
        serde_json::to_value(&self.inner.format)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self.inner.format))
    }

    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new(py, &self.inner.data)
    }

    #[getter]
    fn width_twips(&self) -> Option<i32> {
        self.inner.width_twips
    }

    #[getter]
    fn height_twips(&self) -> Option<i32> {
        self.inner.height_twips
    }

    #[getter]
    fn block_type(&self) -> &str {
        "imageblock"
    }

    fn __repr__(&self) -> String {
        format!(
            "ImageBlock(format={:?}, data_len={}, width_twips={:?}, height_twips={:?})",
            self.format(),
            self.inner.data.len(),
            self.inner.width_twips,
            self.inner.height_twips,
        )
    }

    fn __len__(&self) -> usize {
        self.inner.data.len()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        convert::rust_to_py_dict(py, &Block::ImageBlock(self.inner.clone()))
    }

    #[pyo3(signature = (indent=None))]
    fn to_json(&self, indent: Option<usize>) -> PyResult<String> {
        convert::rust_to_json_string(&Block::ImageBlock(self.inner.clone()), indent)
    }
}

// =============================================================================
// Document
// =============================================================================

#[pyclass(name = "Document", frozen)]
#[derive(Clone)]
pub struct PyDocument {
    pub(crate) inner: Document,
}

#[pymethods]
impl PyDocument {
    #[getter]
    fn blocks(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .blocks
            .iter()
            .map(|b| block_to_py(py, b))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("Document(blocks={})", self.inner.blocks.len())
    }

    fn __len__(&self) -> usize {
        self.inner.blocks.len()
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
// Dispatch helpers
// =============================================================================

pub fn block_to_py(py: Python<'_>, block: &Block) -> PyResult<PyObject> {
    match block {
        Block::Paragraph(p) => Ok(Py::new(py, PyParagraph { inner: p.clone() })?.into_any().into()),
        Block::ListBlock(l) => Ok(Py::new(py, PyListBlock { inner: l.clone() })?.into_any().into()),
        Block::TableBlock(t) => Ok(Py::new(py, PyTableBlock { inner: t.clone() })?.into_any().into()),
        Block::ImageBlock(i) => Ok(Py::new(py, PyImageBlock { inner: i.clone() })?.into_any().into()),
    }
}

pub fn inline_to_py(py: Python<'_>, inline: &Inline) -> PyResult<PyObject> {
    match inline {
        Inline::Run(r) => Ok(Py::new(py, PyRun { inner: r.clone() })?.into_any().into()),
        Inline::Hyperlink(h) => Ok(Py::new(py, PyHyperlink { inner: h.clone() })?.into_any().into()),
        Inline::SemanticField(sf) if !sf.runs.is_empty() => {
            // Expose the first visible run as a plain Run; remaining runs are intentionally dropped
            // at the Python boundary until dedicated SemanticField bindings are added.
            Ok(Py::new(py, PyRun { inner: sf.runs[0].clone() })?.into_any().into())
        }
        _ => {
            // BookmarkAnchor, NoteRef, PageField, SemanticField (no runs), GeneratedBlockMarker
            // have no direct Python representation yet; surface as an empty run.
            Ok(Py::new(py, PyRun { inner: Run::new("") })?.into_any().into())
        }
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyColor>()?;
    m.add_class::<PyShading>()?;
    m.add_class::<PyRun>()?;
    m.add_class::<PyHyperlink>()?;
    m.add_class::<PyParagraph>()?;
    m.add_class::<PyListItem>()?;
    m.add_class::<PyListBlock>()?;
    m.add_class::<PyTableCell>()?;
    m.add_class::<PyTableRow>()?;
    m.add_class::<PyTableBlock>()?;
    m.add_class::<PyImageBlock>()?;
    m.add_class::<PyDocument>()?;
    Ok(())
}
