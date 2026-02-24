//! Public API Module (Phase 4)
//!
//! This module provides the public entrypoints for RTF parsing.

use crate::error::ConversionError;
use crate::limits::ParserLimits;
use crate::{Document, Report};

// =============================================================================
// Public Functions
// =============================================================================

/// Parse RTF input and return a Document with a Report.
///
/// Uses default parser limits. For custom limits, use
/// [`parse_with_limits`].
///
/// # Arguments
///
/// * `input` - The RTF text to parse
///
/// # Returns
///
/// A tuple of `(Document, Report)` containing the parsed content
/// and conversion report, or an error.
///
/// # Example
///
/// ```ignore
/// use rtfkit_core::rtf::parse;
///
/// let rtf = r#"{\rtf1\ansi Hello \b World\b0 !}"#;
/// let (document, report) = parse(rtf)?;
/// ```
pub fn parse(input: &str) -> Result<(Document, Report), ConversionError> {
    parse_with_limits(input, ParserLimits::default())
}

/// Parse RTF input with custom limits and return a Document with a Report.
///
/// # Arguments
///
/// * `input` - The RTF text to parse
/// * `limits` - Parser limits for resource protection
///
/// # Returns
///
/// A tuple of `(Document, Report)` containing the parsed content
/// and conversion report, or an error.
///
/// # Errors
///
/// Returns [`ConversionError::Parse`] with [`ParseError::InputTooLarge`]
/// if the input exceeds `max_input_bytes`.
///
/// Returns [`ConversionError::Parse`] with [`ParseError::GroupDepthExceeded`]
/// if group nesting exceeds `max_group_depth`.
pub fn parse_with_limits(
    input: &str,
    limits: ParserLimits,
) -> Result<(Document, Report), ConversionError> {
    super::pipeline::parse_pipeline(input, limits)
}

// =============================================================================
// RtfParser Struct
// =============================================================================

/// A configurable RTF parser.
///
/// This struct provides a builder-style interface for parsing RTF documents
/// with custom limits.
///
/// # Example
///
/// ```ignore
/// use rtfkit_core::rtf::RtfParser;
/// use rtfkit_core::ParserLimits;
///
/// let limits = ParserLimits::default()
///     .with_max_input_bytes(5 * 1024 * 1024)
///     .with_max_group_depth(100);
///
/// let mut parser = RtfParser::new(limits);
/// let (document, report) = parser.parse(rtf)?;
/// ```
pub struct RtfParser {
    limits: ParserLimits,
}

impl RtfParser {
    /// Create a new RTF parser with the given limits.
    pub fn new(limits: ParserLimits) -> Self {
        Self { limits }
    }

    /// Create a new RTF parser with default limits.
    pub fn default_parser() -> Self {
        Self::new(ParserLimits::default())
    }

    /// Parse RTF input and return a Document with a Report.
    ///
    /// # Arguments
    ///
    /// * `input` - The RTF text to parse
    ///
    /// # Returns
    ///
    /// A tuple of `(Document, Report)` containing the parsed content
    /// and conversion report, or an error.
    pub fn parse(&mut self, input: &str) -> Result<(Document, Report), ConversionError> {
        parse_with_limits(input, self.limits.clone())
    }

    /// Get a reference to the parser limits.
    pub fn limits(&self) -> &ParserLimits {
        &self.limits
    }

    /// Set the parser limits.
    pub fn set_limits(&mut self, limits: ParserLimits) {
        self.limits = limits;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParseError;

    #[test]
    fn test_parse_simple() {
        let input = r#"{\rtf1\ansi Hello World}"#;
        let result = parse(input);
        assert!(result.is_ok());

        let (doc, report) = result.unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(report.stats.paragraph_count, 1);
    }

    #[test]
    fn test_parse_with_limits_simple() {
        let input = r#"{\rtf1\ansi Hello World}"#;
        let limits = ParserLimits::default();
        let result = parse_with_limits(input, limits);
        assert!(result.is_ok());

        let (doc, report) = result.unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(report.stats.paragraph_count, 1);
    }

    #[test]
    fn test_parse_with_limits_too_large() {
        let input = r#"{\rtf1\ansi Hello World}"#;
        let limits = ParserLimits::new().with_max_input_bytes(5);
        let result = parse_with_limits(input, limits);

        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::InputTooLarge { .. }))
        ));
    }

    #[test]
    fn test_rtf_parser_new() {
        let limits = ParserLimits::default()
            .with_max_input_bytes(1024)
            .with_max_group_depth(50);

        let parser = RtfParser::new(limits.clone());
        assert_eq!(parser.limits().max_input_bytes, 1024);
        assert_eq!(parser.limits().max_group_depth, 50);
    }

    #[test]
    fn test_rtf_parser_default_parser() {
        let parser = RtfParser::default_parser();
        assert_eq!(parser.limits(), &ParserLimits::default());
    }

    #[test]
    fn test_rtf_parser_parse() {
        let mut parser = RtfParser::default_parser();
        let input = r#"{\rtf1\ansi Test}"#;
        let result = parser.parse(input);

        assert!(result.is_ok());
        let (doc, _report) = result.unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_rtf_parser_set_limits() {
        let mut parser = RtfParser::default_parser();

        let new_limits = ParserLimits::new().with_max_input_bytes(2048);
        parser.set_limits(new_limits.clone());

        assert_eq!(parser.limits().max_input_bytes, 2048);
    }

    #[test]
    fn test_parse_rejects_non_rtf() {
        let input = "not rtf at all";
        let result = parse(input);

        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::MissingRtfHeader))
        ));
    }

    #[test]
    fn test_parse_rejects_unbalanced() {
        let input = r#"{\rtf1\ansi missing_end"#;
        let result = parse(input);

        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::UnbalancedGroups))
        ));
    }

    #[test]
    fn test_parse_bold_text() {
        let input = r#"{\rtf1\ansi \b Bold\b0  text}"#;
        let result = parse(input);
        assert!(result.is_ok());

        let (doc, _report) = result.unwrap();
        if let crate::Block::Paragraph(para) = &doc.blocks[0] {
            assert!(
                para.inlines
                    .iter()
                    .any(|i| { matches!(i, crate::Inline::Run(r) if r.bold) })
            );
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_parse_multiple_paragraphs() {
        let input = r#"{\rtf1\ansi First\par Second\par Third}"#;
        let result = parse(input);
        assert!(result.is_ok());

        let (doc, report) = result.unwrap();
        assert_eq!(doc.blocks.len(), 3);
        assert_eq!(report.stats.paragraph_count, 3);
    }

    #[test]
    fn test_parse_alignment() {
        let input = r#"{\rtf1\ansi \qc Centered}"#;
        let result = parse(input);
        assert!(result.is_ok());

        let (doc, _report) = result.unwrap();
        if let crate::Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.alignment, crate::Alignment::Center);
        } else {
            panic!("Expected Paragraph block");
        }
    }
}
