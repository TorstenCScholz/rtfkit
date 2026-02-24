//! HTML escaping utilities.
//!
//! This module provides functions for escaping special HTML characters
//! to prevent XSS and ensure valid HTML output.

/// Escapes special HTML characters in a string.
///
/// Converts the following characters to their HTML entity equivalents:
/// - & -> &amp;
/// - < -> &lt;
/// - > -> &gt;
/// - " -> &quot;
/// - ' -> &#39;
pub fn escape_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

/// Escapes text for use inside an HTML attribute value.
pub fn escape_attribute(input: &str) -> String {
    escape_html(input)
}

/// Sanitizes a font family name for safe use in CSS context.
///
/// Font family names come from untrusted RTF input and must be sanitized
/// before being embedded in CSS to prevent injection attacks.
///
/// # Security
///
/// This function prevents CSS injection attacks by:
/// 1. Only allowing safe characters: alphanumeric, spaces, hyphens, underscores
/// 2. Stripping any characters that could break out of the CSS string context
///
/// Characters that are stripped (not allowed):
/// - Quote characters: `"`, `'`
/// - CSS delimiters: `;`, `{`, `}`
/// - HTML/XML characters: `<`, `>`
/// - Control characters: `\n`, `\r`, `\t`, `\0`
/// - Backslash: `\`
///
/// # Example
///
/// ```
/// use rtfkit_html::escape::sanitize_font_family;
///
/// // Safe font names pass through unchanged
/// assert_eq!(sanitize_font_family("Arial"), "Arial");
/// assert_eq!(sanitize_font_family("Times New Roman"), "Times New Roman");
/// assert_eq!(sanitize_font_family("Open-Sans"), "Open-Sans");
///
/// // Dangerous characters are stripped
/// assert_eq!(sanitize_font_family("Arial\"; } body { display: none"), "Arial  body  display none");
/// ```
pub fn sanitize_font_family(font_name: &str) -> String {
    // Only allow safe characters: alphanumeric, spaces, hyphens, underscores
    font_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect()
}

/// Escapes a string for safe use inside a CSS double-quoted string context.
///
/// This function escapes characters that have special meaning in CSS strings:
/// - Backslash is escaped as `\\`
/// - Double quote is escaped as `\"`
/// - Single quote is escaped as `\'`
/// - Newlines are escaped as `\a ` (with trailing space per CSS spec)
/// - Carriage returns are escaped as `\d `
///
/// # Example
///
/// ```
/// use rtfkit_html::escape::escape_css_string;
///
/// assert_eq!(escape_css_string("Arial"), "Arial");
/// assert_eq!(escape_css_string("Font\"Name"), "Font\\\"Name");
/// ```
pub fn escape_css_string(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\'' => result.push_str("\\'"),
            '\n' => result.push_str("\\a "),
            '\r' => result.push_str("\\d "),
            '\0' => result.push_str("\\0 "),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_ampersand() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn escape_angle_brackets() {
        assert_eq!(escape_html("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn escape_quotes() {
        assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(escape_html("'single'"), "&#39;single&#39;");
    }

    #[test]
    fn escape_no_special_chars() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn escape_empty_string() {
        assert_eq!(escape_html(""), "");
    }

    #[test]
    fn escape_complex_string() {
        let input = "<div class=\"test\">Tom & Jerry's</div>";
        let expected = "&lt;div class=&quot;test&quot;&gt;Tom &amp; Jerry&#39;s&lt;/div&gt;";
        assert_eq!(escape_html(input), expected);
    }

    // =========================================================================
    // Font family sanitization tests
    // =========================================================================

    #[test]
    fn sanitize_safe_font_names() {
        assert_eq!(sanitize_font_family("Arial"), "Arial");
        assert_eq!(sanitize_font_family("Times New Roman"), "Times New Roman");
        assert_eq!(sanitize_font_family("Open-Sans"), "Open-Sans");
        assert_eq!(sanitize_font_family("Font_Name"), "Font_Name");
        assert_eq!(sanitize_font_family("Consolas"), "Consolas");
    }

    #[test]
    fn sanitize_font_with_numbers() {
        assert_eq!(sanitize_font_family("Font123"), "Font123");
        assert_eq!(sanitize_font_family("Arial2"), "Arial2");
    }

    #[test]
    fn sanitize_font_strips_quotes() {
        assert_eq!(sanitize_font_family("Arial\""), "Arial");
        assert_eq!(sanitize_font_family("'Arial'"), "Arial");
        assert_eq!(sanitize_font_family("\"Arial\""), "Arial");
    }

    #[test]
    fn sanitize_font_strips_css_delimiters() {
        assert_eq!(sanitize_font_family("Arial;"), "Arial");
        assert_eq!(sanitize_font_family("Arial{}"), "Arial");
        assert_eq!(sanitize_font_family("Font{test}"), "Fonttest");
    }

    #[test]
    fn sanitize_font_strips_html_chars() {
        assert_eq!(sanitize_font_family("<script>"), "script");
        assert_eq!(sanitize_font_family("Arial>"), "Arial");
    }

    #[test]
    fn sanitize_font_strips_control_chars() {
        assert_eq!(sanitize_font_family("Arial\n"), "Arial");
        assert_eq!(sanitize_font_family("Arial\r"), "Arial");
        assert_eq!(sanitize_font_family("Arial\t"), "Arial");
        assert_eq!(sanitize_font_family("Arial\0"), "Arial");
    }

    #[test]
    fn sanitize_font_strips_backslash() {
        assert_eq!(sanitize_font_family("Arial\\"), "Arial");
        assert_eq!(sanitize_font_family("\\Arial"), "Arial");
    }

    #[test]
    fn sanitize_font_injection_attack() {
        // Real-world attack vector: try to break out of CSS context
        let malicious = "Arial\"; } body { display: none; /*";
        let sanitized = sanitize_font_family(malicious);
        assert!(!sanitized.contains('"'));
        assert!(!sanitized.contains(';'));
        assert!(!sanitized.contains('}'));
        assert!(!sanitized.contains('{'));
    }

    #[test]
    fn sanitize_font_xss_attempt() {
        // Try to inject script tag via font name
        let malicious = "</style><script>alert('xss')</script>";
        let sanitized = sanitize_font_family(malicious);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(!sanitized.contains('"'));
        assert!(!sanitized.contains('\''));
    }

    #[test]
    fn sanitize_empty_string() {
        assert_eq!(sanitize_font_family(""), "");
    }

    #[test]
    fn sanitize_only_dangerous_chars() {
        assert_eq!(sanitize_font_family("\"';{}<>\\"), "");
    }

    // =========================================================================
    // CSS string escaping tests
    // =========================================================================

    #[test]
    fn escape_css_safe_string() {
        assert_eq!(escape_css_string("Arial"), "Arial");
        assert_eq!(escape_css_string("Times New Roman"), "Times New Roman");
    }

    #[test]
    fn escape_css_double_quote() {
        assert_eq!(escape_css_string("Font\"Name"), "Font\\\"Name");
    }

    #[test]
    fn escape_css_single_quote() {
        assert_eq!(escape_css_string("Font'Name"), "Font\\'Name");
    }

    #[test]
    fn escape_css_backslash() {
        assert_eq!(escape_css_string("Font\\Name"), "Font\\\\Name");
    }

    #[test]
    fn escape_css_newline() {
        assert_eq!(escape_css_string("Font\nName"), "Font\\a Name");
    }

    #[test]
    fn escape_css_carriage_return() {
        assert_eq!(escape_css_string("Font\rName"), "Font\\d Name");
    }

    #[test]
    fn escape_css_null() {
        assert_eq!(escape_css_string("Font\0Name"), "Font\\0 Name");
    }

    #[test]
    fn escape_css_multiple_special_chars() {
        assert_eq!(
            escape_css_string("Font\"Name\\Test"),
            "Font\\\"Name\\\\Test"
        );
    }
}
