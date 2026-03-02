//! Writer options for DOCX output.

use rtfkit_style_tokens::StyleProfileName;

/// Options for DOCX writer output.
///
/// Use `DocxWriterOptions::default()` to get the default options, which
/// preserve byte-identical output compared to the previous `write_docx` API.
#[derive(Debug, Clone, Default)]
pub struct DocxWriterOptions {
    /// Style profile to apply. `None` = current behavior (no profile defaults applied).
    ///
    /// When `Some`, the specified profile's typography, spacing, and component
    /// tokens are applied as document-level defaults.
    pub style_profile: Option<StyleProfileName>,
}
