//! Configuration options for HTML writer.
//!
//! This module provides [`HtmlWriterOptions`] and [`CssMode`] for controlling HTML output behavior.

/// CSS output mode for HTML writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssMode {
    /// Embed the built-in polished stylesheet (default).
    #[default]
    Default,
    /// Omit built-in CSS, emit semantic HTML only.
    None,
}

/// Configuration options for HTML output.
///
/// Controls how the HTML writer generates output from a `Document`.
///
/// # Example
///
/// ```rust
/// use rtfkit_html::{HtmlWriterOptions, CssMode};
///
/// // Default options: full HTML document with embedded CSS
/// let options = HtmlWriterOptions::default();
///
/// // Fragment output: just the content, no wrapper
/// let fragment = HtmlWriterOptions {
///     emit_document_wrapper: false,
///     css_mode: CssMode::None,
///     custom_css: None,
/// };
///
/// // Custom CSS only (no built-in styles)
/// let custom = HtmlWriterOptions {
///     css_mode: CssMode::None,
///     custom_css: Some("body { background: #fff; }".to_string()),
///     ..HtmlWriterOptions::default()
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

    /// CSS output mode.
    ///
    /// Controls whether the built-in stylesheet is embedded.
    /// See [`CssMode`] for available options.
    pub css_mode: CssMode,

    /// Optional custom CSS content to append after built-in CSS.
    ///
    /// When set, the custom CSS is wrapped in its own `<style>` block
    /// and appended after any built-in CSS (or as the only CSS if
    /// `css_mode` is `CssMode::None`).
    pub custom_css: Option<String>,
}

impl Default for HtmlWriterOptions {
    fn default() -> Self {
        Self {
            emit_document_wrapper: true,
            css_mode: CssMode::Default,
            custom_css: None,
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
        assert_eq!(options.css_mode, CssMode::Default);
        assert!(options.custom_css.is_none());
    }

    #[test]
    fn fragment_options() {
        let options = HtmlWriterOptions {
            emit_document_wrapper: false,
            css_mode: CssMode::None,
            custom_css: None,
        };
        assert!(!options.emit_document_wrapper);
        assert_eq!(options.css_mode, CssMode::None);
    }

    #[test]
    fn css_mode_default() {
        assert_eq!(CssMode::default(), CssMode::Default);
    }

    #[test]
    fn custom_css_options() {
        let options = HtmlWriterOptions {
            css_mode: CssMode::None,
            custom_css: Some("body { color: red; }".to_string()),
            ..HtmlWriterOptions::default()
        };
        assert_eq!(options.css_mode, CssMode::None);
        assert_eq!(options.custom_css, Some("body { color: red; }".to_string()));
    }

    /// Test CSS mode switching between Default and None.
    #[test]
    fn css_mode_switching() {
        // Test Default mode
        let default_mode = CssMode::Default;
        assert_eq!(default_mode, CssMode::Default);
        assert_ne!(default_mode, CssMode::None);

        // Test None mode
        let none_mode = CssMode::None;
        assert_eq!(none_mode, CssMode::None);
        assert_ne!(none_mode, CssMode::Default);

        // Test that Default is the default
        assert_eq!(CssMode::default(), CssMode::Default);
    }

    /// Test that CssMode variants can be copied and compared.
    #[test]
    fn css_mode_copy_equality() {
        let mode1 = CssMode::Default;
        let mode2 = mode1; // Copy
        assert_eq!(mode1, mode2);

        let mode3 = CssMode::None;
        let mode4 = mode3; // Copy
        assert_eq!(mode3, mode4);

        assert_ne!(mode1, mode3);
    }

    /// Test custom CSS option with various content.
    #[test]
    fn custom_css_various_content() {
        // Empty custom CSS
        let options = HtmlWriterOptions {
            custom_css: Some("".to_string()),
            ..HtmlWriterOptions::default()
        };
        assert_eq!(options.custom_css, Some("".to_string()));

        // Complex custom CSS
        let complex_css = r#"
            .custom-class { color: red; }
            @media print { .custom-class { color: black; } }
        "#
        .to_string();
        let options = HtmlWriterOptions {
            custom_css: Some(complex_css.clone()),
            ..HtmlWriterOptions::default()
        };
        assert_eq!(options.custom_css, Some(complex_css));
    }

    /// Test combining CssMode::None with custom CSS.
    #[test]
    fn css_mode_none_with_custom_css() {
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::None,
            custom_css: Some(".my-style { color: blue; }".to_string()),
        };

        assert!(options.emit_document_wrapper);
        assert_eq!(options.css_mode, CssMode::None);
        assert!(options.custom_css.is_some());
    }

    /// Test combining CssMode::Default with custom CSS.
    #[test]
    fn css_mode_default_with_custom_css() {
        let options = HtmlWriterOptions {
            emit_document_wrapper: true,
            css_mode: CssMode::Default,
            custom_css: Some(".override { margin: 0; }".to_string()),
        };

        assert!(options.emit_document_wrapper);
        assert_eq!(options.css_mode, CssMode::Default);
        assert!(options.custom_css.is_some());
    }
}
