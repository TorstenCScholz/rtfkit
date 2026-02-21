//! HTML escaping utilities.
///
/// This module provides functions for escaping special HTML characters
/// to prevent XSS and ensure valid HTML output.

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
}
