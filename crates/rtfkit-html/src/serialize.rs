//! Deterministic HTML string emission.
//!
//! This module provides utilities for building HTML output strings
//! in a deterministic manner.

use crate::escape::escape_html;

/// A buffer for building HTML output.
///
/// Provides methods for appending HTML elements and text in a
/// deterministic way.
#[derive(Debug, Default)]
pub struct HtmlBuffer {
    content: String,
}

impl HtmlBuffer {
    /// Creates a new empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends raw text without escaping.
    ///
    /// # Safety
    ///
    /// The caller must ensure the text is valid HTML.
    pub fn push_raw(&mut self, text: &str) {
        self.content.push_str(text);
    }

    /// Appends escaped text content.
    pub fn push_text(&mut self, text: &str) {
        self.content.push_str(&escape_html(text));
    }

    /// Appends an opening tag with optional attributes.
    pub fn push_open_tag(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.content.push('<');
        self.content.push_str(tag);
        for (name, value) in attrs {
            self.content.push(' ');
            self.content.push_str(name);
            self.content.push_str("=\"");
            self.content.push_str(&escape_html(value));
            self.content.push('"');
        }
        self.content.push('>');
    }

    /// Appends a closing tag.
    pub fn push_close_tag(&mut self, tag: &str) {
        self.content.push_str("</");
        self.content.push_str(tag);
        self.content.push('>');
    }

    /// Appends a self-closing tag with optional attributes.
    pub fn push_self_closing_tag(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.content.push('<');
        self.content.push_str(tag);
        for (name, value) in attrs {
            self.content.push(' ');
            self.content.push_str(name);
            self.content.push_str("=\"");
            self.content.push_str(&escape_html(value));
            self.content.push('"');
        }
        self.content.push_str(" />");
    }

    /// Returns the current content as a string.
    pub fn as_str(&self) -> &str {
        &self.content
    }

    /// Consumes the buffer and returns the content string.
    pub fn into_string(self) -> String {
        self.content
    }

    /// Returns the current length of the content.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer() {
        let buf = HtmlBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn push_raw() {
        let mut buf = HtmlBuffer::new();
        buf.push_raw("<div>");
        assert_eq!(buf.as_str(), "<div>");
    }

    #[test]
    fn push_text_escapes() {
        let mut buf = HtmlBuffer::new();
        buf.push_text("<script>");
        assert_eq!(buf.as_str(), "&lt;script&gt;");
    }

    #[test]
    fn push_open_tag_no_attrs() {
        let mut buf = HtmlBuffer::new();
        buf.push_open_tag("div", &[]);
        assert_eq!(buf.as_str(), "<div>");
    }

    #[test]
    fn push_open_tag_with_attrs() {
        let mut buf = HtmlBuffer::new();
        buf.push_open_tag("div", &[("class", "container"), ("id", "main")]);
        assert_eq!(buf.as_str(), r#"<div class="container" id="main">"#);
    }

    #[test]
    fn push_close_tag() {
        let mut buf = HtmlBuffer::new();
        buf.push_close_tag("div");
        assert_eq!(buf.as_str(), "</div>");
    }

    #[test]
    fn into_string() {
        let mut buf = HtmlBuffer::new();
        buf.push_raw("content");
        let s = buf.into_string();
        assert_eq!(s, "content");
    }
}
