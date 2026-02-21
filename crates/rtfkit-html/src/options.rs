//! Configuration options for HTML writer.
//!
//! This module provides [`HtmlWriterOptions`] for controlling HTML output behavior.

/// Configuration options for HTML output.
///
/// Controls how the HTML writer generates output from a `Document`.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::HtmlWriterOptions;
///
/// // Default options: full HTML document with embedded CSS
/// let options = HtmlWriterOptions::default();
///
/// // Fragment output: just the content, no wrapper
/// let fragment = HtmlWriterOptions {
///     emit_document_wrapper: false,
///     include_default_css: false,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlWriterOptions {
    /// Whether to emit the full HTML document wrapper.
    ///
    /// When `true` (default), output includes:
    /// ```html
    /// <!doctype html>
    /// <html>
    /// <head>...</head>
    /// <body>...</body>
    /// </html>
    /// ```
    ///
    /// When `false`, only the content elements are emitted
    /// (useful for embedding in other documents).
    pub emit_document_wrapper: bool,

    /// Whether to include the default minimal stylesheet.
    ///
    /// When `true` (default), a stable minimal CSS is embedded in the output.
    /// When `false`, no CSS is included (rely on external styling).
    pub include_default_css: bool,
}

impl Default for HtmlWriterOptions {
    fn default() -> Self {
        Self {
            emit_document_wrapper: true,
            include_default_css: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options() {
        let options = HtmlWriterOptions::default();
        assert!(options.emit_document_wrapper);
        assert!(options.include_default_css);
    }

    #[test]
    fn fragment_options() {
        let options = HtmlWriterOptions {
            emit_document_wrapper: false,
            include_default_css: false,
        };
        assert!(!options.emit_document_wrapper);
        assert!(!options.include_default_css);
    }
}
