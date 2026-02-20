//! RTF Interpreter Module
//!
//! This module implements a stateful interpreter that converts RTF parser events
//! into the intermediate representation (IR) types defined in the crate.
//!
//! # Architecture
//!
//! The interpreter follows the "stateful interpreter" pattern described in the ADR:
//! 1. Tokenizer parses RTF input and emits events
//! 2. Interpreter maintains style state and group stack
//! 3. Events are processed to build the IR Document
//!
//! # Example
//!
//! ```ignore
//! use rtfkit_core::interpreter::Interpreter;
//!
//! let rtf = r#"{\rtf1\ansi Hello \b World\b0 !}"#;
//! let (document, report) = Interpreter::parse(rtf)?;
//! ```

use crate::report::ReportBuilder;
use crate::{Alignment, Block, Document, Paragraph, Report, Run};
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{take, take_while1},
    character::complete::{anychar, char, digit1},
    combinator::{map, opt, recognize, verify},
    sequence::{preceded, tuple},
};

// =============================================================================
// Token Types
// =============================================================================

/// A token representing a parsed RTF element.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Start of a group `{`
    GroupStart,
    /// End of a group `}`
    GroupEnd,
    /// A control word with optional parameter
    ControlWord {
        word: String,
        parameter: Option<i32>,
    },
    /// Text content
    Text(String),
    /// A control symbol (like `\*`, `\'`, etc.)
    ControlSymbol(char),
}

// =============================================================================
// Events
// =============================================================================

/// Events emitted by the tokenizer for the interpreter to process.
#[derive(Debug, Clone, PartialEq)]
pub enum RtfEvent {
    /// Start of a group `{`
    GroupStart,
    /// End of a group `}`
    GroupEnd,
    /// A control word with optional parameter
    ControlWord {
        word: String,
        parameter: Option<i32>,
    },
    /// A single-char control symbol
    ControlSymbol(char),
    /// Text content
    Text(String),
    /// Binary data (rarely used in basic RTF)
    BinaryData(Vec<u8>),
}

// =============================================================================
// Style State
// =============================================================================

/// Tracks the current formatting state within the RTF document.
///
/// The style state is pushed onto the group stack when entering a new group
/// and popped when exiting, allowing RTF's scoping rules to be properly handled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleState {
    /// Whether text is bold
    pub bold: bool,
    /// Whether text is italic
    pub italic: bool,
    /// Whether text is underlined
    pub underline: bool,
    /// Paragraph alignment
    pub alignment: Alignment,
}

impl StyleState {
    /// Creates a new default style state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a snapshot of the current style state.
    /// This is used when pushing onto the group stack.
    pub fn snapshot(&self) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestinationBehavior {
    Metadata,
    Dropped(&'static str),
}

// =============================================================================
// Interpreter
// =============================================================================

/// The RTF interpreter that converts events into IR.
///
/// The interpreter maintains:
/// - A group stack for handling RTF's nested group structure
/// - The current style state for formatting
/// - Text aggregation for building runs
/// - The document being built
/// - A report builder for collecting warnings and stats
pub struct Interpreter {
    /// Stack of style states for group handling
    group_stack: Vec<StyleState>,
    /// Current style state
    current_style: StyleState,
    /// The document being built
    document: Document,
    /// Current paragraph being built
    current_paragraph: Paragraph,
    /// Current text being aggregated
    current_text: String,
    /// Current run style (for detecting style changes)
    current_run_style: StyleState,
    /// Paragraph alignment (captured when paragraph starts)
    paragraph_alignment: Alignment,
    /// Report builder for collecting warnings and stats
    report_builder: ReportBuilder,
    /// Number of fallback characters to skip after a \u escape (from \ucN)
    unicode_skip_count: usize,
    /// How many characters to skip next (set after processing \u)
    skip_next_chars: usize,
    /// Tracks whether we just read a destination marker control symbol (\*)
    destination_marker: bool,
    /// Number of nested groups currently being skipped as a destination
    skip_destination_depth: usize,
    /// Whether each open group still allows destination detection at its start
    group_can_start_destination: Vec<bool>,
}

impl Interpreter {
    /// Creates a new interpreter with default state.
    pub fn new() -> Self {
        Self {
            group_stack: Vec::new(),
            current_style: StyleState::new(),
            document: Document::new(),
            current_paragraph: Paragraph::new(),
            current_text: String::new(),
            current_run_style: StyleState::new(),
            paragraph_alignment: Alignment::default(),
            report_builder: ReportBuilder::new(),
            unicode_skip_count: 1, // Default: 1 fallback character
            skip_next_chars: 0,
            destination_marker: false,
            skip_destination_depth: 0,
            group_can_start_destination: Vec::new(),
        }
    }

    /// Parses RTF text and returns a Document with a Report.
    ///
    /// # Arguments
    ///
    /// * `input` - The RTF text to parse
    ///
    /// # Returns
    ///
    /// A tuple of `(Document, Report)` containing the parsed content
    /// and conversion report, or an error message.
    pub fn parse(input: &str) -> Result<(Document, Report), String> {
        let mut interpreter = Self::new();
        let tokens = tokenize(input).map_err(|e| format!("Tokenization error: {:?}", e))?;
        validate_tokens(&tokens)?;

        // Track bytes processed
        interpreter.report_builder.set_bytes_processed(input.len());

        for token in tokens {
            let event = token_to_event(token);
            interpreter.process_event(event)?;
        }

        // Finalize any remaining content
        interpreter.finalize_paragraph();

        // Build the final report
        let report = interpreter.report_builder.build();

        Ok((interpreter.document, report))
    }

    /// Process a single RTF event.
    fn process_event(&mut self, event: RtfEvent) -> Result<(), String> {
        if self.skip_destination_depth > 0 {
            self.process_skipped_destination_event(event);
            return Ok(());
        }

        match event {
            RtfEvent::GroupStart => {
                // Push current style onto stack
                self.group_stack.push(self.current_style.snapshot());
                self.group_can_start_destination.push(true);
            }
            RtfEvent::GroupEnd => {
                // Pop style from stack
                if let Some(previous_style) = self.group_stack.pop() {
                    self.current_style = previous_style;
                }
                self.group_can_start_destination.pop();
                self.destination_marker = false;
            }
            RtfEvent::ControlWord { word, parameter } => {
                self.handle_control_word(&word, parameter);
            }
            RtfEvent::ControlSymbol(symbol) => {
                self.handle_control_symbol(symbol);
            }
            RtfEvent::Text(text) => {
                self.mark_current_group_non_destination();
                self.handle_text(text);
            }
            RtfEvent::BinaryData(data) => {
                self.report_builder
                    .dropped_content("Dropped unsupported binary RTF data", Some(data.len()));
            }
        }
        Ok(())
    }

    fn process_skipped_destination_event(&mut self, event: RtfEvent) {
        match event {
            RtfEvent::GroupStart => {
                self.group_stack.push(self.current_style.snapshot());
                self.group_can_start_destination.push(false);
                self.skip_destination_depth += 1;
            }
            RtfEvent::GroupEnd => {
                if let Some(previous_style) = self.group_stack.pop() {
                    self.current_style = previous_style;
                }
                self.group_can_start_destination.pop();
                self.skip_destination_depth = self.skip_destination_depth.saturating_sub(1);
                if self.skip_destination_depth == 0 {
                    self.destination_marker = false;
                }
            }
            RtfEvent::ControlWord { .. }
            | RtfEvent::ControlSymbol(_)
            | RtfEvent::Text(_)
            | RtfEvent::BinaryData(_) => {}
        }
    }

    fn handle_control_symbol(&mut self, symbol: char) {
        match symbol {
            // Destination marker at the start of a group: {\*\destination ...}
            '*' if self.can_start_destination() => {
                self.destination_marker = true;
            }
            '\\' => {
                self.mark_current_group_non_destination();
                self.handle_text("\\".to_string());
            }
            '{' => {
                self.mark_current_group_non_destination();
                self.handle_text("{".to_string());
            }
            '}' => {
                self.mark_current_group_non_destination();
                self.handle_text("}".to_string());
            }
            '~' => {
                self.mark_current_group_non_destination();
                self.handle_text("\u{00A0}".to_string());
            }
            '_' => {
                self.mark_current_group_non_destination();
                self.handle_text("\u{2011}".to_string());
            }
            '-' | '\n' | '\r' => {
                // Optional hyphen / source formatting characters are ignored.
                self.mark_current_group_non_destination();
            }
            _ => {
                self.mark_current_group_non_destination();
            }
        }
    }

    fn can_start_destination(&self) -> bool {
        self.group_can_start_destination
            .last()
            .copied()
            .unwrap_or(false)
    }

    fn mark_current_group_non_destination(&mut self) {
        if let Some(can_start) = self.group_can_start_destination.last_mut() {
            *can_start = false;
        }
    }

    fn destination_behavior(word: &str) -> Option<DestinationBehavior> {
        match word {
            // Metadata destinations that are intentionally excluded from body content.
            "fonttbl" | "colortbl" | "stylesheet" | "info" | "title" | "author" | "operator"
            | "keywords" | "comment" | "version" | "vern" | "creatim" | "revtim" | "printim"
            | "buptim" | "edmins" | "nofpages" | "nofwords" | "nofchars" | "nofcharsws" | "id"
            | "listtable" | "listoverridetable" => Some(DestinationBehavior::Metadata),
            // Destinations that represent currently unsupported visible content.
            "pict" | "obj" | "objclass" | "objdata" | "shppict" | "nonshppict" | "picprop"
            | "field" | "fldinst" | "fldrslt" | "datafield" | "header" | "headerl" | "headerr"
            | "footer" | "footerl" | "footerr" | "footnote" | "annotation" | "pn" | "pntext"
            | "pntxtb" | "pntxta" | "pnseclvl" => Some(DestinationBehavior::Dropped(
                "Dropped unsupported RTF destination content",
            )),
            _ => None,
        }
    }

    fn maybe_start_destination(&mut self, word: &str) -> bool {
        if !self.can_start_destination() {
            self.destination_marker = false;
            return false;
        }

        if let Some(behavior) = Self::destination_behavior(word) {
            if let DestinationBehavior::Dropped(reason) = behavior {
                self.report_builder.dropped_content(reason, None);
            }
            self.skip_destination_depth = 1;
            self.destination_marker = false;
            self.mark_current_group_non_destination();
            return true;
        }

        if self.destination_marker {
            self.report_builder.unknown_destination(word);
            let reason = format!("Dropped unknown destination group \\{word}");
            self.report_builder.dropped_content(&reason, None);
            self.skip_destination_depth = 1;
            self.destination_marker = false;
            self.mark_current_group_non_destination();
            return true;
        }

        false
    }

    /// Handle a control word.
    fn handle_control_word(&mut self, word: &str, parameter: Option<i32>) {
        if self.maybe_start_destination(word) {
            return;
        }
        self.mark_current_group_non_destination();

        match word {
            // Bold: \b or \bN (N=0 turns off, N>0 turns on)
            "b" => {
                self.current_style.bold = parameter.map(|p| p != 0).unwrap_or(true);
            }
            // Italic: \i or \iN
            "i" => {
                self.current_style.italic = parameter.map(|p| p != 0).unwrap_or(true);
            }
            // Underline: \ul or \ulN
            "ul" => {
                self.current_style.underline = parameter.map(|p| p != 0).unwrap_or(true);
            }
            // Underline none: \ulnone
            "ulnone" => {
                self.current_style.underline = false;
            }
            // Paragraph break
            "par" | "pard" => {
                // \pard resets paragraph properties
                if word == "pard" {
                    self.current_style.alignment = Alignment::default();
                } else {
                    self.finalize_paragraph();
                }
            }
            // Line break (treated as paragraph for MVP)
            "line" => {
                self.finalize_paragraph();
            }
            // Alignment
            "ql" => {
                self.current_style.alignment = Alignment::Left;
            }
            "qc" => {
                self.current_style.alignment = Alignment::Center;
            }
            "qr" => {
                self.current_style.alignment = Alignment::Right;
            }
            "qj" => {
                self.current_style.alignment = Alignment::Justify;
            }
            // Unicode escape: \uN (N is a signed 16-bit integer)
            "u" => {
                if let Some(code) = parameter {
                    // RTF uses signed 16-bit integers for Unicode codepoints
                    // Negative values represent codepoints in the range U+FFFF8000 to U+FFFFFFFF
                    // which should be interpreted as unsigned for the actual Unicode codepoint
                    let codepoint = if code < 0 {
                        (code as u16) as u32
                    } else {
                        code as u32
                    };
                    if let Some(c) = char::from_u32(codepoint) {
                        self.handle_text(c.to_string());
                    }
                    // Mark that we need to skip the fallback characters
                    self.skip_next_chars = self.unicode_skip_count;
                }
            }
            // Unicode skip count: \ucN (number of fallback characters after \u)
            "uc" => {
                self.unicode_skip_count =
                    parameter.and_then(|p| usize::try_from(p).ok()).unwrap_or(1);
            }
            // RTF header control words - silently ignored (not user-facing)
            "rtf" | "ansi" | "ansicpg" | "deff" | "deflang" | "deflangfe" | "adeflang"
            | "result" | "hwid" | "emdash" | "endash" | "emspace" | "enspace" | "qmspace"
            | "bullet" | "lquote" | "rquote" | "ldblquote" | "rdblquote" | "tab" | "plain"
            | "f" | "fs" | "cf" | "cb" | "highlight" | "strike" | "striked" | "sub" | "super"
            | "nosupersub" | "caps" | "scaps" | "outl" | "shad" | "expnd" | "expndtw"
            | "kerning" | "charscalex" | "lang" | "langfe" | "langnp" | "langfenp" => {
                // Silently ignore these structural/formatting control words
            }
            // Unknown control words - report as unsupported
            _ => {
                self.report_builder
                    .unsupported_control_word(word, parameter);
            }
        }
    }

    /// Handle text content.
    fn handle_text(&mut self, text: String) {
        // Skip fallback characters if needed (after \u escape)
        if self.skip_next_chars > 0 {
            let skip = self.skip_next_chars.min(text.chars().count());
            let chars_to_take = text.chars().skip(skip);
            let remaining: String = chars_to_take.collect();
            self.skip_next_chars = 0;

            if remaining.is_empty() {
                return;
            }

            // Process remaining text
            self.handle_text_internal(remaining);
        } else {
            self.handle_text_internal(text);
        }
    }

    /// Internal text handling (after skip logic)
    fn handle_text_internal(&mut self, text: String) {
        // If this is the first text in the paragraph, capture the alignment
        if self.current_paragraph.runs.is_empty() && self.current_text.is_empty() {
            self.paragraph_alignment = self.current_style.alignment;
        }

        // Check if style has changed
        if self.style_changed() {
            // Flush current text as a run if any
            if !self.current_text.is_empty() {
                let run = self.create_run();
                self.current_paragraph.runs.push(run);
                self.current_text.clear();
            }
            self.current_run_style = self.current_style.snapshot();
        }

        // Append text
        self.current_text.push_str(&text);
    }

    /// Check if the current style differs from the run style.
    fn style_changed(&self) -> bool {
        self.current_style.bold != self.current_run_style.bold
            || self.current_style.italic != self.current_run_style.italic
            || self.current_style.underline != self.current_run_style.underline
    }

    /// Create a run from the current text and run style.
    fn create_run(&self) -> Run {
        Run {
            text: self.current_text.clone(),
            bold: self.current_run_style.bold,
            italic: self.current_run_style.italic,
            underline: self.current_run_style.underline,
            font_size: None,
            color: None,
        }
    }

    /// Finalize the current paragraph and add it to the document.
    fn finalize_paragraph(&mut self) {
        // Flush any remaining text as a run
        if !self.current_text.is_empty() {
            let run = Run {
                text: self.current_text.clone(),
                bold: self.current_run_style.bold,
                italic: self.current_run_style.italic,
                underline: self.current_run_style.underline,
                font_size: None,
                color: None,
            };
            self.current_paragraph.runs.push(run);
            self.current_text.clear();
        }

        // Add paragraph to document if it has content
        if !self.current_paragraph.runs.is_empty() {
            self.current_paragraph.alignment = self.paragraph_alignment;
            self.document
                .blocks
                .push(Block::Paragraph(self.current_paragraph.clone()));

            // Track stats
            self.report_builder.increment_paragraph_count();
            self.report_builder
                .add_runs(self.current_paragraph.runs.len());
        }

        // Reset current paragraph
        self.current_paragraph = Paragraph::new();
        self.current_run_style = self.current_style.snapshot();
        // Reset paragraph alignment for the next paragraph
        self.paragraph_alignment = self.current_style.alignment;
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tokenizer (using nom)
// =============================================================================

/// Tokenizes RTF input into a vector of tokens.
pub fn tokenize(input: &str) -> Result<Vec<Token>, nom::Err<nom::error::Error<&str>>> {
    let mut tokens = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        // Skip source formatting whitespace. Spaces are meaningful and preserved.
        remaining = skip_ignorable_whitespace(remaining);

        // If only whitespace remained, we're done
        if remaining.is_empty() {
            break;
        }

        match parse_token(remaining) {
            Ok((new_remaining, token)) => {
                tokens.push(token);
                remaining = new_remaining;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(tokens)
}

fn validate_tokens(tokens: &[Token]) -> Result<(), String> {
    if tokens.is_empty() {
        return Err("Invalid RTF: empty input".to_string());
    }

    let mut depth = 0usize;
    for token in tokens {
        match token {
            Token::GroupStart => depth += 1,
            Token::GroupEnd => {
                if depth == 0 {
                    return Err("Invalid RTF: unmatched group end ('}')".to_string());
                }
                depth -= 1;
            }
            Token::ControlWord { .. } | Token::Text(_) | Token::ControlSymbol(_) => {}
        }
    }
    if depth != 0 {
        return Err("Invalid RTF: unbalanced groups".to_string());
    }

    // Basic format guard: RTF should begin with {\rtf...}
    let mut iter = tokens
        .iter()
        .filter(|t| !matches!(t, Token::Text(text) if text.trim().is_empty()));
    match (iter.next(), iter.next()) {
        (Some(Token::GroupStart), Some(Token::ControlWord { word, .. })) if word == "rtf" => Ok(()),
        _ => Err("Invalid RTF: missing \\rtf header".to_string()),
    }
}

/// Decode a Windows-1252 codepoint to a Unicode character.
/// Windows-1252 is the default encoding for RTF documents with \ansi.
fn decode_windows1252(codepoint: u8) -> char {
    // Windows-1252 has some characters in the 0x80-0x9F range that differ from ISO-8859-1
    // See: https://en.wikipedia.org/wiki/Windows-1252
    match codepoint {
        0x80 => '\u{20AC}', // Euro sign
        0x82 => '\u{201A}', // Single low-9 quotation mark
        0x83 => '\u{0192}', // Latin small letter f with hook
        0x84 => '\u{201E}', // Double low-9 quotation mark
        0x85 => '\u{2026}', // Horizontal ellipsis
        0x86 => '\u{2020}', // Dagger
        0x87 => '\u{2021}', // Double dagger
        0x88 => '\u{02C6}', // Modifier letter circumflex accent
        0x89 => '\u{2030}', // Per mille sign
        0x8A => '\u{0160}', // Latin capital letter S with caron
        0x8B => '\u{2039}', // Single left-pointing angle quotation mark
        0x8C => '\u{0152}', // Latin capital ligature OE
        0x8E => '\u{017D}', // Latin capital letter Z with caron
        0x91 => '\u{2018}', // Left single quotation mark
        0x92 => '\u{2019}', // Right single quotation mark
        0x93 => '\u{201C}', // Left double quotation mark
        0x94 => '\u{201D}', // Right double quotation mark
        0x95 => '\u{2022}', // Bullet
        0x96 => '\u{2013}', // En dash
        0x97 => '\u{2014}', // Em dash
        0x98 => '\u{02DC}', // Small tilde
        0x99 => '\u{2122}', // Trade mark sign
        0x9A => '\u{0161}', // Latin small letter s with caron
        0x9B => '\u{203A}', // Single right-pointing angle quotation mark
        0x9C => '\u{0153}', // Latin small ligature oe
        0x9E => '\u{017E}', // Latin small letter z with caron
        0x9F => '\u{0178}', // Latin capital letter Y with diaeresis
        // For all other values (0x00-0x7F and 0xA0-0xFF), they match ISO-8859-1/Unicode
        byte => byte as char,
    }
}

/// Parse a single token from the input.
fn parse_token(input: &str) -> IResult<&str, Token> {
    alt((
        // Group start
        map(char('{'), |_| Token::GroupStart),
        // Group end
        map(char('}'), |_| Token::GroupEnd),
        // Control word or symbol
        preceded(
            char('\\'),
            alt((
                // Hex escape: \'hh (exactly two hex digits)
                map(
                    preceded(
                        char('\''),
                        verify(take(2usize), |hex: &&str| {
                            hex.chars().all(|c| c.is_ascii_hexdigit())
                        }),
                    ),
                    |hex: &str| {
                        if let Ok(byte) = u8::from_str_radix(hex, 16) {
                            Token::Text(decode_windows1252(byte).to_string())
                        } else {
                            Token::Text(String::new())
                        }
                    },
                ),
                // Control symbol (single non-letter character)
                map(
                    verify(anychar, |c| !c.is_ascii_alphabetic()),
                    Token::ControlSymbol,
                ),
                // Control word with optional parameter
                map(
                    tuple((
                        // Word: letters only
                        take_while1(|c: char| c.is_ascii_alphabetic()),
                        // Optional parameter: digits, possibly negative
                        opt(recognize(tuple((opt(char('-')), digit1)))),
                        // An optional single space delimiter is consumed by the RTF grammar.
                        opt(char(' ')),
                    )),
                    |(word, param, _): (&str, Option<&str>, Option<char>)| {
                        let parameter = param.and_then(|p| {
                            if p.is_empty() || p == "-" {
                                None
                            } else {
                                p.parse::<i32>().ok()
                            }
                        });
                        Token::ControlWord {
                            word: word.to_string(),
                            parameter,
                        }
                    },
                ),
            )),
        ),
        // Text content (until special character)
        map(parse_text, Token::Text),
    ))(input)
}

/// Parse text content until a special character.
fn parse_text(input: &str) -> IResult<&str, String> {
    let (remaining, text) =
        take_while1(|c: char| c != '\\' && c != '{' && c != '}' && !c.is_control())(input)?;

    // Decode RTF special characters in the text
    let decoded = decode_text(text);

    Ok((remaining, decoded))
}

/// Decode RTF special characters in text.
fn decode_text(text: &str) -> String {
    let mut result = String::new();

    for c in text.chars() {
        result.push(c);
    }

    result
}

/// Skip ignorable source formatting whitespace.
fn skip_ignorable_whitespace(input: &str) -> &str {
    let mut remaining = input;
    while let Some(c) = remaining.chars().next() {
        if c == '\n' || c == '\r' || c == '\t' {
            remaining = &remaining[c.len_utf8()..];
        } else {
            break;
        }
    }
    remaining
}

/// Convert a token to an event.
fn token_to_event(token: Token) -> RtfEvent {
    match token {
        Token::GroupStart => RtfEvent::GroupStart,
        Token::GroupEnd => RtfEvent::GroupEnd,
        Token::ControlWord { word, parameter } => RtfEvent::ControlWord { word, parameter },
        Token::Text(text) => RtfEvent::Text(text),
        Token::ControlSymbol(symbol) => RtfEvent::ControlSymbol(symbol),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let input = r#"{\rtf1 Hello}"#;
        let tokens = tokenize(input).unwrap();
        assert!(tokens.contains(&Token::GroupStart));
        assert!(tokens.contains(&Token::GroupEnd));
    }

    #[test]
    fn test_tokenize_control_word() {
        let input = r#"\b"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(
            tokens,
            vec![Token::ControlWord {
                word: "b".to_string(),
                parameter: None
            }]
        );
    }

    #[test]
    fn test_tokenize_control_word_with_param() {
        let input = r#"\b1"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(
            tokens,
            vec![Token::ControlWord {
                word: "b".to_string(),
                parameter: Some(1)
            }]
        );
    }

    #[test]
    fn test_style_state_default() {
        let state = StyleState::new();
        assert!(!state.bold);
        assert!(!state.italic);
        assert!(!state.underline);
        assert_eq!(state.alignment, Alignment::Left);
    }

    #[test]
    fn test_interpreter_simple_text() {
        let input = r#"{\rtf1\ansi Hello World}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(report.stats.paragraph_count, 1);
    }

    #[test]
    fn test_interpreter_bold() {
        let input = r#"{\rtf1\ansi \b Bold\b0 text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // Should have runs with different bold states
        let Block::Paragraph(para) = &doc.blocks[0];
        assert!(para.runs.iter().any(|r| r.bold));
    }

    #[test]
    fn test_interpreter_paragraph() {
        let input = r#"{\rtf1\ansi First\par Second}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(report.stats.paragraph_count, 2);
    }

    #[test]
    fn test_interpreter_alignment() {
        let input = r#"{\rtf1\ansi \qc Centered}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        let Block::Paragraph(para) = &doc.blocks[0];
        assert_eq!(para.alignment, Alignment::Center);
    }

    #[test]
    fn test_group_stack() {
        let input = r#"{\rtf1\ansi \b{Nested}\b0 After}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // Document should parse without error
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_report_stats() {
        let input = r#"{\rtf1\ansi First\par Second\par Third}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        assert_eq!(doc.blocks.len(), 3);
        assert_eq!(report.stats.paragraph_count, 3);
        assert!(report.stats.bytes_processed > 0);
        // duration_ms is a u64, so it's always >= 0
    }

    #[test]
    fn test_report_warnings() {
        let input = r#"{\rtf1\ansi \unknownword content}"#;
        let (_doc, report) = Interpreter::parse(input).unwrap();

        assert_eq!(report.warnings.len(), 1);
        assert!(matches!(
            &report.warnings[0],
            crate::report::Warning::UnsupportedControlWord { word, .. } if word == "unknownword"
        ));
    }

    #[test]
    fn test_report_no_warnings_for_known_words() {
        let input = r#"{\rtf1\ansi{\fonttbl\f0 Arial;}Hello}"#;
        let (_doc, report) = Interpreter::parse(input).unwrap();

        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_parse_rejects_non_rtf_input() {
        let input = "not rtf at all";
        let err = Interpreter::parse(input).unwrap_err();
        assert!(err.contains("missing \\rtf header"));
    }

    #[test]
    fn test_parse_rejects_unbalanced_groups() {
        let input = r#"{\rtf1\ansi missing_end"#;
        let err = Interpreter::parse(input).unwrap_err();
        assert!(err.contains("unbalanced groups"));
    }

    #[test]
    fn test_paragraph_finalization_uses_run_style() {
        let input = r#"{\rtf1\ansi \b Bold\b0\par}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();
        let Block::Paragraph(para) = &doc.blocks[0];
        assert_eq!(para.runs.len(), 1);
        assert!(para.runs[0].bold);
    }

    #[test]
    fn test_space_after_group_is_preserved() {
        let input = r#"{\rtf1\ansi {\b Bold} text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();
        let Block::Paragraph(para) = &doc.blocks[0];
        let rendered = para
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>();
        assert_eq!(rendered, "Bold text");
    }

    #[test]
    fn test_escaped_symbols_are_preserved() {
        let input = r#"{\rtf1\ansi \{braced\} and \\ slash}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();
        let Block::Paragraph(para) = &doc.blocks[0];
        assert_eq!(para.runs.len(), 1);
        assert_eq!(para.runs[0].text, "{braced} and \\ slash");
    }

    #[test]
    fn test_unknown_destination_is_skipped_and_reported() {
        let input = r#"{\rtf1\ansi {\*\foo hidden} shown}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();
        let Block::Paragraph(para) = &doc.blocks[0];
        assert_eq!(para.runs[0].text, " shown");
        assert!(
            report.warnings.iter().any(|warning| matches!(
                warning,
                crate::report::Warning::UnknownDestination { .. }
            ))
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| matches!(warning, crate::report::Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_metadata_destination_is_skipped_without_warning() {
        let input = r#"{\rtf1\ansi {\fonttbl\f0 Arial;} Hello}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();
        let Block::Paragraph(para) = &doc.blocks[0];
        assert_eq!(para.runs[0].text, " Hello");
        assert!(report.warnings.is_empty());
    }
}
