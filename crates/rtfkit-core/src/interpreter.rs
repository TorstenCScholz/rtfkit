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
//! use rtfkit_core::ParserLimits;
//!
//! let rtf = r#"{\rtf1\ansi Hello \b World\b0 !}"#;
//! let (document, report) = Interpreter::parse(rtf)?;
//!
//! // Or with custom limits:
//! let limits = ParserLimits::default().with_max_input_bytes(5 * 1024 * 1024);
//! let (document, report) = Interpreter::parse_with_limits(rtf, limits)?;
//! ```

use crate::error::{ConversionError, ParseError};
use crate::limits::ParserLimits;
use crate::report::ReportBuilder;
use crate::{
    Alignment, Block, Document, ListBlock, ListId, ListItem, ListKind, Paragraph, Report, Run,
    TableBlock, TableCell, TableRow,
};
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{take, take_while1},
    character::complete::{anychar, char, digit1},
    combinator::{map, opt, recognize, verify},
    sequence::{preceded, tuple},
};
use std::collections::HashMap;

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

// =============================================================================
// List Parsing Types
// =============================================================================

/// Parsed list level from \listtable.
///
/// Represents a single level within a list definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedListLevel {
    /// The level number (0-8)
    pub level: u8,
    /// The kind of numbering for this level
    pub kind: ListKind,
}

/// Parsed list definition from \listtable.
///
/// Represents a complete list definition with all its levels.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedListDefinition {
    /// The list ID from RTF (\listidN)
    pub list_id: i32,
    /// The levels defined for this list
    pub levels: Vec<ParsedListLevel>,
}

impl ParsedListDefinition {
    /// Creates a new empty list definition with the given ID.
    pub fn new(list_id: i32) -> Self {
        Self {
            list_id,
            levels: Vec::new(),
        }
    }

    /// Gets the kind for a specific level, defaulting to Bullet.
    pub fn kind_for_level(&self, level: u8) -> ListKind {
        self.levels
            .iter()
            .find(|l| l.level == level)
            .map(|l| l.kind)
            .unwrap_or_default()
    }
}

/// Parsed list override from \listoverridetable.
///
/// List overrides map an ls_id to a list definition and can optionally
/// override the starting number.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedListOverride {
    /// The ls_id used in \lsN control words
    pub ls_id: i32,
    /// The list_id this override references
    pub list_id: i32,
    /// Optional start number override
    pub start_override: Option<i32>,
}

impl ParsedListOverride {
    /// Creates a new list override.
    pub fn new(ls_id: i32, list_id: i32) -> Self {
        Self {
            ls_id,
            list_id,
            start_override: None,
        }
    }
}

/// Resolved list reference for the current paragraph.
///
/// This is populated when \lsN and \ilvlN are encountered,
/// and used during paragraph finalization to create list blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphListRef {
    /// The resolved list ID
    pub list_id: ListId,
    /// The nesting level (0-8)
    pub level: u8,
    /// The kind of list
    pub kind: ListKind,
}

impl ParagraphListRef {
    /// Creates a new paragraph list reference.
    pub fn new(list_id: ListId, level: u8, kind: ListKind) -> Self {
        Self {
            list_id,
            level: level.min(8), // Clamp to DOCX max
            kind,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestinationBehavior {
    Metadata,
    Dropped(&'static str),
    ListTable,
    ListOverrideTable,
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
/// - Parser limits for resource protection
/// - List parsing state for Phase 3
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
    /// Parser limits for resource protection
    limits: ParserLimits,
    /// Current group depth (for limit enforcement)
    current_depth: usize,
    // =============================================================================
    // List Parsing State (Phase 3)
    // =============================================================================
    /// Parsed list definitions from \listtable
    list_table: HashMap<i32, ParsedListDefinition>,
    /// Parsed list overrides from \listoverridetable
    list_overrides: HashMap<i32, ParsedListOverride>,
    /// Current paragraph list reference (from \lsN and \ilvlN)
    current_list_ref: Option<ParagraphListRef>,
    /// Current ls_id (from \lsN, used to resolve list reference)
    pending_ls_id: Option<i32>,
    /// Current level (from \ilvlN)
    pending_level: u8,
    // =============================================================================
    // List Table Parsing State
    // =============================================================================
    /// Whether we're currently parsing a list table destination
    parsing_list_table: bool,
    /// Whether we're currently parsing a list override table destination
    parsing_list_override_table: bool,
    /// Current list definition being parsed (for listtable)
    current_list_def: Option<ParsedListDefinition>,
    /// Current list level being parsed (for listtable)
    current_list_level: Option<ParsedListLevel>,
    /// Current list override being parsed (for listoverridetable)
    current_list_override: Option<ParsedListOverride>,
    // =============================================================================
    // Table Parsing State (Phase 4)
    // =============================================================================
    /// Current table being built
    current_table: Option<TableBlock>,
    /// Current row being built
    current_row: Option<TableRow>,
    /// Current cell being built
    current_cell: Option<TableCell>,
    /// Cell boundaries encountered in current row (from \cellxN)
    pending_cellx: Vec<i32>,
    /// Whether the current paragraph saw \intbl.
    ///
    /// This flag is scoped to the current paragraph and reset at paragraph boundaries.
    /// Table membership itself is derived from active row/cell state.
    seen_intbl_in_paragraph: bool,
}

impl Interpreter {
    /// Creates a new interpreter with default state and limits.
    pub fn new() -> Self {
        Self::with_limits(ParserLimits::default())
    }

    /// Creates a new interpreter with custom limits.
    pub fn with_limits(limits: ParserLimits) -> Self {
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
            limits,
            current_depth: 0,
            // List parsing state
            list_table: HashMap::new(),
            list_overrides: HashMap::new(),
            current_list_ref: None,
            pending_ls_id: None,
            pending_level: 0,
            // List table parsing state
            parsing_list_table: false,
            parsing_list_override_table: false,
            current_list_def: None,
            current_list_level: None,
            current_list_override: None,
            // Table parsing state
            current_table: None,
            current_row: None,
            current_cell: None,
            pending_cellx: Vec::new(),
            seen_intbl_in_paragraph: false,
        }
    }

    /// Parses RTF text and returns a Document with a Report.
    ///
    /// Uses default parser limits. For custom limits, use
    /// [`parse_with_limits`](Self::parse_with_limits).
    ///
    /// # Arguments
    ///
    /// * `input` - The RTF text to parse
    ///
    /// # Returns
    ///
    /// A tuple of `(Document, Report)` containing the parsed content
    /// and conversion report, or an error.
    pub fn parse(input: &str) -> Result<(Document, Report), ConversionError> {
        Self::parse_with_limits(input, ParserLimits::default())
    }

    /// Parses RTF text with custom limits and returns a Document with a Report.
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
        // Check input size limit
        if input.len() > limits.max_input_bytes {
            return Err(ConversionError::Parse(ParseError::InputTooLarge {
                size: input.len(),
                limit: limits.max_input_bytes,
            }));
        }

        let mut interpreter = Self::with_limits(limits.clone());
        let tokens = tokenize(input).map_err(|e| {
            ConversionError::Parse(ParseError::TokenizationError(format!("{:?}", e)))
        })?;
        validate_tokens(&tokens)?;

        // Track bytes processed
        interpreter.report_builder.set_bytes_processed(input.len());
        interpreter.report_builder.set_limits(limits);

        for token in tokens {
            let event = token_to_event(token);
            interpreter.process_event(event)?;
        }

        // Finalize any remaining content
        interpreter.finalize_paragraph();

        // Finalize any remaining table context at document end
        if interpreter.current_table.is_some() {
            interpreter.finalize_current_table();
        }

        // Build the final report
        let report = interpreter.report_builder.build();

        Ok((interpreter.document, report))
    }

    /// Process a single RTF event.
    fn process_event(&mut self, event: RtfEvent) -> Result<(), ConversionError> {
        if self.skip_destination_depth > 0 {
            return self.process_skipped_destination_event(event);
        }

        match event {
            RtfEvent::GroupStart => {
                // Check depth limit
                self.current_depth += 1;
                if self.current_depth > self.limits.max_group_depth {
                    return Err(ConversionError::Parse(ParseError::GroupDepthExceeded {
                        depth: self.current_depth,
                        limit: self.limits.max_group_depth,
                    }));
                }
                // Push current style onto stack
                self.group_stack.push(self.current_style.snapshot());
                self.group_can_start_destination.push(true);
            }
            RtfEvent::GroupEnd => {
                // Pop style from stack
                if let Some(previous_style) = self.group_stack.pop() {
                    self.current_style = previous_style;
                }
                self.current_depth = self.current_depth.saturating_sub(1);
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

    fn process_skipped_destination_event(
        &mut self,
        event: RtfEvent,
    ) -> Result<(), ConversionError> {
        match event {
            RtfEvent::GroupStart => {
                self.current_depth += 1;
                if self.current_depth > self.limits.max_group_depth {
                    return Err(ConversionError::Parse(ParseError::GroupDepthExceeded {
                        depth: self.current_depth,
                        limit: self.limits.max_group_depth,
                    }));
                }
                self.group_stack.push(self.current_style.snapshot());
                self.group_can_start_destination.push(false);
                self.skip_destination_depth += 1;

                // Handle nested groups in list table parsing
                if self.parsing_list_table && self.skip_destination_depth == 2 {
                    // Starting a new \list or \listlevel group
                    // The depth check helps us know we're inside the listtable destination
                }
                if self.parsing_list_override_table && self.skip_destination_depth == 2 {
                    // Starting a new \listoverride group
                }
            }
            RtfEvent::GroupEnd => {
                // Finalize list definition when closing a \list group
                if self.parsing_list_table && self.skip_destination_depth == 2 {
                    if let Some(list_def) = self.current_list_def.take() {
                        self.list_table.insert(list_def.list_id, list_def);
                    }
                }

                // Finalize list override when closing a \listoverride group
                if self.parsing_list_override_table && self.skip_destination_depth == 2 {
                    if let Some(list_override) = self.current_list_override.take() {
                        self.list_overrides
                            .insert(list_override.ls_id, list_override);
                    }
                }

                // Finalize list level when closing a \listlevel group
                if self.parsing_list_table && self.skip_destination_depth == 3 {
                    if let Some(level) = self.current_list_level.take() {
                        if let Some(ref mut list_def) = self.current_list_def {
                            list_def.levels.push(level);
                        }
                    }
                }

                if let Some(previous_style) = self.group_stack.pop() {
                    self.current_style = previous_style;
                }
                self.current_depth = self.current_depth.saturating_sub(1);
                self.group_can_start_destination.pop();
                self.skip_destination_depth = self.skip_destination_depth.saturating_sub(1);

                if self.skip_destination_depth == 0 {
                    self.destination_marker = false;
                    self.parsing_list_table = false;
                    self.parsing_list_override_table = false;
                }
            }
            RtfEvent::ControlWord { word, parameter } => {
                self.handle_list_table_control_word(&word, parameter);
            }
            RtfEvent::ControlSymbol(_) | RtfEvent::Text(_) | RtfEvent::BinaryData(_) => {}
        }
        Ok(())
    }

    /// Handle control words within list table/override table destinations.
    fn handle_list_table_control_word(&mut self, word: &str, parameter: Option<i32>) {
        // Only process if we're in list table parsing mode
        if !self.parsing_list_table && !self.parsing_list_override_table {
            return;
        }

        match word {
            // List table control words
            "list" => {
                // Start of a new list definition
                // We'll set the list_id when we encounter \listidN
                self.current_list_def = Some(ParsedListDefinition::new(0));
            }
            "listlevel" => {
                // Start of a new level definition
                // \listlevel has no numeric parameter; infer level by declaration order.
                let level = self
                    .current_list_def
                    .as_ref()
                    .map(|def| def.levels.len() as u8)
                    .unwrap_or(0)
                    .min(8);
                self.current_list_level = Some(ParsedListLevel {
                    level,
                    kind: ListKind::Bullet, // Default, will be updated by \levelnfcN
                });
            }
            "levelnfc" => {
                // Number format code determines the list kind
                // 0 = decimal, 23 = bullet, etc.
                if let Some(nfc) = parameter {
                    let kind = match nfc {
                        0 => ListKind::OrderedDecimal, // Decimal (1, 2, 3)
                        1 => ListKind::OrderedDecimal, // Upper Roman (I, II, III)
                        2 => ListKind::OrderedDecimal, // Lower Roman (i, ii, iii)
                        3 => ListKind::OrderedDecimal, // Upper Alpha (A, B, C)
                        4 => ListKind::OrderedDecimal, // Lower Alpha (a, b, c)
                        23 => ListKind::Bullet,        // Bullet
                        _ => ListKind::Bullet,         // Default to bullet for unknown
                    };
                    if let Some(ref mut level) = self.current_list_level {
                        level.kind = kind;
                    }
                }
            }

            // List override table control words
            "listoverride" => {
                // Start of a new list override
                // We'll set the values when we encounter \listidN and \lsN
                self.current_list_override = Some(ParsedListOverride::new(0, 0));
            }
            "ls" => {
                // In override table, this sets the ls_id
                if let Some(id) = parameter {
                    if self.parsing_list_override_table {
                        if let Some(ref mut list_override) = self.current_list_override {
                            list_override.ls_id = id;
                        }
                    }
                }
            }
            "listoverridepad" => {
                // Padding/hack for Word compatibility - ignore
            }

            // listid is used in both listtable and listoverridetable
            "listid" => {
                if let Some(id) = parameter {
                    if self.parsing_list_table {
                        // In listtable, this sets the list definition's ID
                        if let Some(ref mut list_def) = self.current_list_def {
                            list_def.list_id = id;
                        }
                    } else if self.parsing_list_override_table {
                        // In listoverridetable, this sets the referenced list_id
                        if let Some(ref mut list_override) = self.current_list_override {
                            list_override.list_id = id;
                        }
                    }
                }
            }

            // Other list-related control words we recognize but don't fully process
            "listtemplateid" | "listsimple" | "listhybrid" | "listname" | "liststyleid"
            | "levels" | "levelstartat" | "levelindent" | "levelspace" | "levelfollow"
            | "levellegal" | "levelnorestart" | "leveljcn" | "levelnfcn" | "levelnfcN"
            | "listoverridestart" => {
                // These are recognized but not fully processed - no warning needed
            }

            _ => {
                // Unknown control word in list table context
                // Report as unsupported list control
                self.report_builder.unsupported_list_control(word);
            }
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
            // List table destinations - need special parsing
            "listtable" => Some(DestinationBehavior::ListTable),
            "listoverridetable" => Some(DestinationBehavior::ListOverrideTable),
            // Metadata destinations that are intentionally excluded from body content.
            "fonttbl" | "colortbl" | "stylesheet" | "info" | "title" | "author" | "operator"
            | "keywords" | "comment" | "version" | "vern" | "creatim" | "revtim" | "printim"
            | "buptim" | "edmins" | "nofpages" | "nofwords" | "nofchars" | "nofcharsws" | "id" => {
                Some(DestinationBehavior::Metadata)
            }
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
            match behavior {
                DestinationBehavior::Dropped(reason) => {
                    self.report_builder.dropped_content(reason, None);
                    self.skip_destination_depth = 1;
                }
                DestinationBehavior::Metadata => {
                    self.skip_destination_depth = 1;
                }
                DestinationBehavior::ListTable => {
                    self.parsing_list_table = true;
                    self.skip_destination_depth = 1;
                }
                DestinationBehavior::ListOverrideTable => {
                    self.parsing_list_override_table = true;
                    self.skip_destination_depth = 1;
                }
            }
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
                    // Also reset list state on \pard
                    self.current_list_ref = None;
                    self.pending_ls_id = None;
                    self.pending_level = 0;
                    // Reset paragraph-local table marker.
                    self.seen_intbl_in_paragraph = false;
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
            // =============================================================================
            // List Control Words (Phase 3)
            // =============================================================================
            // \lsN - List identifier reference (from listoverridetable)
            "ls" => {
                self.pending_ls_id = parameter;
            }
            // \ilvlN - List level (0-indexed)
            "ilvl" => {
                let level = parameter.and_then(|p| u8::try_from(p).ok()).unwrap_or(0);

                // Emit warning if level exceeds max
                if level > 8 {
                    self.report_builder.unsupported_nesting_level(level, 8);
                }

                self.pending_level = level.min(8); // Clamp to DOCX max
            }
            // Legacy paragraph numbering controls are intentionally unsupported.
            "pnlvl" | "pnlvlblt" | "pnlvlbody" | "pnlvlcont" | "pnstart" | "pnindent"
            | "pntxta" | "pntxtb" => {
                self.report_builder.unsupported_list_control(word);
                self.report_builder
                    .dropped_content("Dropped legacy paragraph numbering content", None);
            }
            // =============================================================================
            // Table Control Words (Phase 4)
            // =============================================================================
            // \trowd - Start of a new table row definition
            "trowd" => {
                self.handle_trowd();
            }
            // \cellxN - Cell boundary position in twips
            "cellx" => {
                if let Some(boundary) = parameter {
                    self.pending_cellx.push(boundary);
                }
            }
            // \intbl - Paragraph is inside a table
            "intbl" => {
                self.seen_intbl_in_paragraph = true;
            }
            // \cell - End of a table cell
            "cell" => {
                self.handle_cell();
            }
            // \row - End of a table row
            "row" => {
                self.handle_row();
            }
            // Row formatting controls - recognized but not fully supported
            "trgaph" | "trleft" | "trql" | "trqr" | "trqc" => {
                self.report_builder.unsupported_table_control(word);
            }
            // Cell vertical alignment controls - recognized but not fully supported
            "clvertalt" | "clvertalc" | "clvertalb" => {
                self.report_builder.unsupported_table_control(word);
            }
            // Cell merge markers - recognized but not fully supported (degraded path)
            "clmgf" | "clmrg" => {
                self.report_builder.unsupported_table_control(word);
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
            self.skip_next_chars -= skip;

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
        // If we already completed table rows and prose starts, close table first.
        if self.current_paragraph.runs.is_empty()
            && self.current_text.is_empty()
            && self.current_table.is_some()
            && self.current_row.is_none()
            && !self.seen_intbl_in_paragraph
        {
            self.finalize_current_table();
        }

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

    fn flush_current_text_as_run(&mut self) {
        if !self.current_text.is_empty() {
            let run = self.create_run();
            self.current_paragraph.runs.push(run);
            self.current_text.clear();
        }
    }

    fn has_pending_paragraph_content(&self) -> bool {
        !self.current_text.is_empty() || !self.current_paragraph.runs.is_empty()
    }

    fn reset_paragraph_state(&mut self) {
        self.current_paragraph = Paragraph::new();
        self.current_run_style = self.current_style.snapshot();
        self.paragraph_alignment = self.current_style.alignment;
        self.current_list_ref = None;
        self.pending_ls_id = None;
        self.pending_level = 0;
        self.seen_intbl_in_paragraph = false;
    }

    /// Finalize the current paragraph and add it to the document.
    fn finalize_paragraph(&mut self) {
        // Active row context always wins over \intbl marker state.
        if self.current_row.is_some() {
            self.finalize_paragraph_for_table();
            return;
        }

        // \intbl without an active row is malformed. Keep text as normal paragraph.
        if self.seen_intbl_in_paragraph && self.has_pending_paragraph_content() {
            self.report_builder
                .malformed_table_structure("\\intbl content without table row context");
            self.report_builder
                .dropped_content("Table paragraph without row context", None);
            self.seen_intbl_in_paragraph = false;
        }

        // If a completed table is pending, close it before adding prose blocks.
        if self.current_table.is_some() {
            self.finalize_current_table();
        }

        self.flush_current_text_as_run();

        // Add paragraph to document if it has content
        if !self.current_paragraph.runs.is_empty() {
            self.current_paragraph.alignment = self.paragraph_alignment;

            // Resolve list reference if we have pending list state
            self.resolve_list_reference();

            // Check if this paragraph belongs to a list
            if let Some(list_ref) = &self.current_list_ref {
                // Add as list item to existing or new list block
                self.add_list_item(
                    list_ref.list_id,
                    list_ref.level,
                    list_ref.kind,
                    self.current_paragraph.clone(),
                );
            } else {
                // Regular paragraph
                self.document
                    .blocks
                    .push(Block::Paragraph(self.current_paragraph.clone()));
            }

            // Track stats
            self.report_builder.increment_paragraph_count();
            self.report_builder
                .add_runs(self.current_paragraph.runs.len());
        }

        self.reset_paragraph_state();
    }

    /// Resolve list reference from pending state.
    fn resolve_list_reference(&mut self) {
        // First, try to resolve from \lsN (modern list mechanism)
        if let Some(ls_id) = self.pending_ls_id {
            if let Some(override_def) = self.list_overrides.get(&ls_id) {
                let list_id = override_def.list_id as ListId;
                let level = self.pending_level;

                // Emit warning if level exceeds max
                if level > 8 {
                    self.report_builder.unsupported_nesting_level(level, 8);
                }

                let kind = self
                    .list_table
                    .get(&override_def.list_id)
                    .map(|def| def.kind_for_level(level))
                    .unwrap_or_default();
                self.current_list_ref = Some(ParagraphListRef::new(list_id, level, kind));
                return;
            }

            // If we have ls_id but no override, emit warnings and keep paragraph output.
            self.report_builder.unresolved_list_override(ls_id);

            // Emit DroppedContent for strict mode compatibility
            self.report_builder
                .dropped_content(&format!("Unresolved list override ls_id={}", ls_id), None);
        }
    }

    /// Add a list item to the document, creating or appending to a list block.
    fn add_list_item(&mut self, list_id: ListId, level: u8, kind: ListKind, paragraph: Paragraph) {
        // Check if we can append to the last block if it's a list with the same ID
        if let Some(Block::ListBlock(last_list)) = self.document.blocks.last_mut() {
            if last_list.list_id == list_id && last_list.kind == kind {
                last_list.add_item(ListItem::from_paragraph(level, paragraph));
                return;
            }
        }

        // Create a new list block
        let mut list_block = ListBlock::new(list_id, kind);
        list_block.add_item(ListItem::from_paragraph(level, paragraph));
        self.document.blocks.push(Block::ListBlock(list_block));
    }

    fn add_list_item_to_current_cell(
        &mut self,
        list_id: ListId,
        level: u8,
        kind: ListKind,
        paragraph: Paragraph,
    ) {
        if self.current_cell.is_none() {
            self.current_cell = Some(TableCell::new());
        }

        if let Some(ref mut cell) = self.current_cell {
            if let Some(Block::ListBlock(last_list)) = cell.blocks.last_mut() {
                if last_list.list_id == list_id && last_list.kind == kind {
                    last_list.add_item(ListItem::from_paragraph(level, paragraph));
                    return;
                }
            }

            let mut list_block = ListBlock::new(list_id, kind);
            list_block.add_item(ListItem::from_paragraph(level, paragraph));
            cell.blocks.push(Block::ListBlock(list_block));
        }
    }

    fn has_open_or_pending_table_cell_content(&self) -> bool {
        self.current_cell.is_some() || self.has_pending_paragraph_content()
    }

    fn auto_close_table_cell_if_needed(&mut self, dropped_reason: &str) {
        if self.current_row.is_none() || !self.has_open_or_pending_table_cell_content() {
            return;
        }

        self.report_builder.unclosed_table_cell();
        self.report_builder.dropped_content(dropped_reason, None);
        self.finalize_paragraph_for_table();
        self.finalize_current_cell();
    }

    // =============================================================================
    // Table Parsing Methods (Phase 4)
    // =============================================================================

    /// Handle \trowd - start of a new table row definition.
    fn handle_trowd(&mut self) {
        // Finalize any dangling row/cell with warning.
        if self.current_row.is_some() {
            self.auto_close_table_cell_if_needed("Unclosed table cell at row boundary");
            self.report_builder.unclosed_table_row();
            self.report_builder
                .dropped_content("Unclosed table row at new row definition", None);
            self.finalize_current_row();
        }

        // Start fresh row context
        self.pending_cellx.clear();
        self.current_row = Some(TableRow::new());

        // Ensure we have a table context
        if self.current_table.is_none() {
            self.current_table = Some(TableBlock::new());
        }

        self.seen_intbl_in_paragraph = false;
    }

    /// Handle \cell - end of a table cell.
    fn handle_cell(&mut self) {
        // Check if we're in a table context
        if self.current_table.is_none() || self.current_row.is_none() {
            // Orphan \cell outside table context
            self.report_builder
                .malformed_table_structure("\\cell encountered outside table context");
            self.report_builder
                .dropped_content("Table cell control outside table context", None);
            // Preserve any text content as regular paragraph
            self.finalize_paragraph();
            return;
        }

        // Finalize any current paragraph into the cell first
        self.finalize_paragraph_for_table();

        // Preserve explicit empty cells.
        if self.current_cell.is_none() {
            self.current_cell = Some(TableCell::new());
        }

        // Finalize the current cell
        self.finalize_current_cell();
    }

    /// Handle \row - end of a table row.
    fn handle_row(&mut self) {
        // Check if we're in a table context
        if self.current_table.is_none() || self.current_row.is_none() {
            // Orphan \row outside table context
            self.report_builder
                .malformed_table_structure("\\row encountered outside table context");
            self.report_builder
                .dropped_content("Table row control outside table context", None);
            return;
        }

        // Finalize current cell if not closed.
        self.auto_close_table_cell_if_needed("Unclosed table cell at row end");

        // Finalize the current row
        self.finalize_current_row();

        // Clear pending cellx for next row
        self.pending_cellx.clear();
        self.seen_intbl_in_paragraph = false;
    }

    /// Finalize the current cell and attach it to the current row.
    fn finalize_current_cell(&mut self) {
        if let Some(cell) = self.current_cell.take() {
            // Convert \cellx right-boundary positions to actual cell widths.
            let cell_index = self
                .current_row
                .as_ref()
                .map(|r| r.cells.len())
                .unwrap_or(0);
            let width = self
                .pending_cellx
                .get(cell_index)
                .copied()
                .and_then(|right_boundary| {
                    if cell_index == 0 {
                        Some(right_boundary)
                    } else {
                        self.pending_cellx
                            .get(cell_index - 1)
                            .map(|left_boundary| right_boundary - *left_boundary)
                    }
                });

            let mut cell_with_width = cell;
            if let Some(w) = width {
                if w > 0 {
                    cell_with_width.width_twips = Some(w);
                } else {
                    self.report_builder.malformed_table_structure(&format!(
                        "Non-increasing \\cellx boundaries at cell {}",
                        cell_index
                    ));
                }
            }

            if let Some(ref mut row) = self.current_row {
                row.cells.push(cell_with_width);
            }
        }
    }

    /// Finalize the current row and attach it to the current table.
    fn finalize_current_row(&mut self) {
        if let Some(row) = self.current_row.take() {
            // Check for cellx count mismatch
            if !self.pending_cellx.is_empty() && row.cells.len() != self.pending_cellx.len() {
                let reason = format!(
                    "Cell count ({}) does not match \\cellx count ({})",
                    row.cells.len(),
                    self.pending_cellx.len()
                );
                self.report_builder.malformed_table_structure(&reason);
                self.report_builder
                    .dropped_content("Table cell count mismatch", None);
            }

            if let Some(ref mut table) = self.current_table {
                table.rows.push(row);
            }
        }
    }

    /// Finalize the current table and add it to the document.
    fn finalize_current_table(&mut self) {
        // Finalize any dangling row/cell.
        if self.current_row.is_some() {
            self.auto_close_table_cell_if_needed("Unclosed table cell at document end");
            self.report_builder.unclosed_table_row();
            self.report_builder
                .dropped_content("Unclosed table row at document end", None);
            self.finalize_current_row();
        }

        // Add the table to the document if it has content
        if let Some(table) = self.current_table.take() {
            if !table.is_empty() {
                self.document.blocks.push(Block::TableBlock(table));
            }
        }

        // Reset table state
        self.current_cell = None;
        self.pending_cellx.clear();
        self.seen_intbl_in_paragraph = false;
    }

    /// Finalize paragraph for table context (routes to current cell instead of document).
    fn finalize_paragraph_for_table(&mut self) {
        self.flush_current_text_as_run();

        // Add paragraph to current cell if it has content
        if !self.current_paragraph.runs.is_empty() {
            self.current_paragraph.alignment = self.paragraph_alignment;
            self.resolve_list_reference();

            let paragraph = self.current_paragraph.clone();

            if let Some(list_ref) = self.current_list_ref.clone() {
                self.add_list_item_to_current_cell(
                    list_ref.list_id,
                    list_ref.level,
                    list_ref.kind,
                    paragraph,
                );
            } else {
                // Create cell if needed.
                if self.current_cell.is_none() {
                    self.current_cell = Some(TableCell::new());
                }

                if let Some(ref mut cell) = self.current_cell {
                    cell.blocks.push(Block::Paragraph(paragraph));
                }
            }

            // Track stats
            self.report_builder.increment_paragraph_count();
            self.report_builder
                .add_runs(self.current_paragraph.runs.len());
        }

        self.reset_paragraph_state();
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

fn validate_tokens(tokens: &[Token]) -> Result<(), ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut depth = 0usize;
    for token in tokens {
        match token {
            Token::GroupStart => depth += 1,
            Token::GroupEnd => {
                if depth == 0 {
                    return Err(ParseError::UnmatchedGroupEnd);
                }
                depth -= 1;
            }
            Token::ControlWord { .. } | Token::Text(_) | Token::ControlSymbol(_) => {}
        }
    }
    if depth != 0 {
        return Err(ParseError::UnbalancedGroups);
    }

    // Basic format guard: RTF should begin with {\rtf...}
    let mut iter = tokens
        .iter()
        .filter(|t| !matches!(t, Token::Text(text) if text.trim().is_empty()));
    match (iter.next(), iter.next()) {
        (Some(Token::GroupStart), Some(Token::ControlWord { word, .. })) if word == "rtf" => Ok(()),
        _ => Err(ParseError::MissingRtfHeader),
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
        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert!(para.runs.iter().any(|r| r.bold));
        } else {
            panic!("Expected Paragraph block");
        }
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

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.alignment, Alignment::Center);
        } else {
            panic!("Expected Paragraph block");
        }
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
        assert!(matches!(
            err,
            ConversionError::Parse(ParseError::MissingRtfHeader)
        ));
    }

    #[test]
    fn test_parse_rejects_unbalanced_groups() {
        let input = r#"{\rtf1\ansi missing_end"#;
        let err = Interpreter::parse(input).unwrap_err();
        assert!(matches!(
            err,
            ConversionError::Parse(ParseError::UnbalancedGroups)
        ));
    }

    // ==========================================================================
    // Limit Enforcement Tests
    // ==========================================================================

    #[test]
    fn test_input_size_limit() {
        // Create a small RTF input
        let input = r#"{\rtf1\ansi Hello}"#;

        // Set a limit smaller than the input
        let limits = ParserLimits::new().with_max_input_bytes(5);

        let result = Interpreter::parse_with_limits(input, limits);
        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::InputTooLarge { .. }))
        ));

        // Verify error message contains sizes
        if let Err(ConversionError::Parse(ParseError::InputTooLarge { size, limit })) = result {
            assert_eq!(size, input.len());
            assert_eq!(limit, 5);
        }
    }

    #[test]
    fn test_input_size_limit_allows_normal_input() {
        let input = r#"{\rtf1\ansi Hello World}"#;

        // Set a limit larger than the input
        let limits = ParserLimits::new().with_max_input_bytes(1000);

        let result = Interpreter::parse_with_limits(input, limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_group_depth_limit() {
        // Create deeply nested RTF (10 levels of nesting inside the root group)
        // Format: {\rtf1\ansi {{{{{{{{{{Hello}}}}}}}}}}
        let inner_braces = "{".repeat(10);
        let outer_braces = "}".repeat(10);
        let input = format!("{{\\rtf1\\ansi {}Hello{}}}", inner_braces, outer_braces);

        // Set a depth limit of 5
        let limits = ParserLimits::new().with_max_group_depth(5);

        let result = Interpreter::parse_with_limits(&input, limits);
        assert!(matches!(
            result,
            Err(ConversionError::Parse(
                ParseError::GroupDepthExceeded { .. }
            ))
        ));

        // Verify error message contains depths
        if let Err(ConversionError::Parse(ParseError::GroupDepthExceeded { depth, limit })) = result
        {
            assert_eq!(depth, 6); // Fails on the 6th opening brace
            assert_eq!(limit, 5);
        }
    }

    #[test]
    fn test_group_depth_limit_allows_normal_nesting() {
        // Create RTF with 5 levels of nesting inside the root group
        let inner_braces = "{".repeat(3);
        let outer_braces = "}".repeat(3);
        let input = format!("{{\\rtf1\\ansi {}Hello{}}}", inner_braces, outer_braces);

        // Set a depth limit of 10
        let limits = ParserLimits::new().with_max_group_depth(10);

        let result = Interpreter::parse_with_limits(&input, limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_group_depth_limit_applies_inside_skipped_destination() {
        // Unknown destination group with deep nested content.
        let inner_braces = "{".repeat(10);
        let outer_braces = "}".repeat(10);
        let input = format!(
            "{{\\rtf1\\ansi {{\\*\\foo {}X{}}} Hello}}",
            inner_braces, outer_braces
        );

        // Root group + destination group + 10 nested groups exceeds limit 5.
        let limits = ParserLimits::new().with_max_group_depth(5);

        let result = Interpreter::parse_with_limits(&input, limits);
        assert!(matches!(
            result,
            Err(ConversionError::Parse(
                ParseError::GroupDepthExceeded { .. }
            ))
        ));
    }

    #[test]
    fn test_warning_count_limit() {
        // Create RTF with many unknown control words
        let mut input = String::from("{\\rtf1\\ansi ");
        for i in 0..20 {
            input.push_str(&format!("\\unknown{} ", i));
        }
        input.push_str("}");

        // Set a warning limit of 5
        let limits = ParserLimits::new().with_max_warning_count(5);

        let result = Interpreter::parse_with_limits(&input, limits);
        assert!(result.is_ok());

        // Verify warning count is capped
        let (_, report) = result.unwrap();
        assert_eq!(report.warnings.len(), 5);
    }

    #[test]
    fn test_warning_count_limit_allows_normal_warnings() {
        // Create RTF with a few unknown control words
        let input = r#"{\rtf1\ansi \unknown1 \unknown2 \unknown3}"#;

        // Set a warning limit of 10
        let limits = ParserLimits::new().with_max_warning_count(10);

        let result = Interpreter::parse_with_limits(input, limits);
        assert!(result.is_ok());

        // Verify all warnings are captured
        let (_, report) = result.unwrap();
        assert_eq!(report.warnings.len(), 3);
    }

    #[test]
    fn test_default_limits_allow_normal_documents() {
        // Test that default limits work with a typical RTF document
        let input = r#"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}
{\colortbl;\red0\green0\blue0;}
\viewkind4\uc1\pard\f0\fs20 Hello World\par
This is a \b bold\b0  test.\par
}"#;

        let result = Interpreter::parse(input);
        assert!(result.is_ok());

        let (doc, _report) = result.unwrap();
        assert!(!doc.blocks.is_empty());
        // Note: Some control words like \viewkind may generate warnings,
        // but the document should still parse successfully
    }

    #[test]
    fn test_paragraph_finalization_uses_run_style() {
        let input = r#"{\rtf1\ansi \b Bold\b0\par}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();
        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.runs.len(), 1);
            assert!(para.runs[0].bold);
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_space_after_group_is_preserved() {
        let input = r#"{\rtf1\ansi {\b Bold} text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();
        if let Block::Paragraph(para) = &doc.blocks[0] {
            let rendered = para
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>();
            assert_eq!(rendered, "Bold text");
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_escaped_symbols_are_preserved() {
        let input = r#"{\rtf1\ansi \{braced\} and \\ slash}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();
        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.runs.len(), 1);
            assert_eq!(para.runs[0].text, "{braced} and \\ slash");
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_unknown_destination_is_skipped_and_reported() {
        let input = r#"{\rtf1\ansi {\*\foo hidden} shown}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();
        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.runs[0].text, " shown");
            assert!(report.warnings.iter().any(|warning| matches!(
                warning,
                crate::report::Warning::UnknownDestination { .. }
            )));
            assert!(
                report.warnings.iter().any(|warning| matches!(
                    warning,
                    crate::report::Warning::DroppedContent { .. }
                ))
            );
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_metadata_destination_is_skipped_without_warning() {
        let input = r#"{\rtf1\ansi {\fonttbl\f0 Arial;} Hello}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();
        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.runs[0].text, " Hello");
            assert!(report.warnings.is_empty());
        } else {
            panic!("Expected Paragraph block");
        }
    }

    // ==========================================================================
    // List Parsing Tests (Phase 3)
    // ==========================================================================

    #[test]
    fn test_parsed_list_definition_new() {
        let def = ParsedListDefinition::new(42);
        assert_eq!(def.list_id, 42);
        assert!(def.levels.is_empty());
    }

    #[test]
    fn test_parsed_list_definition_kind_for_level() {
        let mut def = ParsedListDefinition::new(1);
        def.levels.push(ParsedListLevel {
            level: 0,
            kind: ListKind::Bullet,
        });
        def.levels.push(ParsedListLevel {
            level: 1,
            kind: ListKind::OrderedDecimal,
        });

        assert_eq!(def.kind_for_level(0), ListKind::Bullet);
        assert_eq!(def.kind_for_level(1), ListKind::OrderedDecimal);
        // Default for undefined level
        assert_eq!(def.kind_for_level(2), ListKind::Bullet);
    }

    #[test]
    fn test_parsed_list_override_new() {
        let override_def = ParsedListOverride::new(1, 100);
        assert_eq!(override_def.ls_id, 1);
        assert_eq!(override_def.list_id, 100);
        assert!(override_def.start_override.is_none());
    }

    #[test]
    fn test_paragraph_list_ref_new() {
        let list_ref = ParagraphListRef::new(42, 3, ListKind::OrderedDecimal);
        assert_eq!(list_ref.list_id, 42);
        assert_eq!(list_ref.level, 3);
        assert_eq!(list_ref.kind, ListKind::OrderedDecimal);
    }

    #[test]
    fn test_paragraph_list_ref_level_clamped() {
        // Level should be clamped to 8
        let list_ref = ParagraphListRef::new(1, 15, ListKind::Bullet);
        assert_eq!(list_ref.level, 8);
    }

    #[test]
    fn test_unresolved_ls_keeps_paragraph_output() {
        let input = r#"{\rtf1\ansi \ls1 Item text}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        // Without a matching override we keep paragraph output and emit warnings.
        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.runs[0].text, "Item text");
        } else {
            panic!("Expected Paragraph, got {:?}", doc.blocks[0]);
        }
        assert!(report.warnings.iter().any(|w| matches!(
            w,
            crate::report::Warning::UnresolvedListOverride { ls_id: 1, .. }
        )));
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_ilvl_control_word_sets_level() {
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc0}\listid1}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1\ilvl2 Item text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::ListBlock(list) = &doc.blocks[0] {
            assert_eq!(list.items[0].level, 2);
        } else {
            panic!("Expected ListBlock");
        }
    }

    #[test]
    fn test_legacy_pn_controls_are_dropped_with_warnings() {
        let input = r#"{\rtf1\ansi \pnlvlbody Legacy item}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        assert!(matches!(&doc.blocks[0], Block::Paragraph(_)));
        assert!(report.warnings.iter().any(|w| matches!(
            w,
            crate::report::Warning::UnsupportedListControl { control_word, .. } if control_word == "pnlvlbody"
        )));
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_multiple_list_items_same_list() {
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc0}\listid1}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1 First\par \ls1 Second\par \ls1 Third}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // Should create a single ListBlock with 3 items
        assert_eq!(doc.blocks.len(), 1);
        if let Block::ListBlock(list) = &doc.blocks[0] {
            assert_eq!(list.items.len(), 3);
            assert_eq!(list.list_id, 1);
        } else {
            panic!("Expected ListBlock");
        }
    }

    #[test]
    fn test_list_and_paragraph_mixed() {
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc0}\listid1}}{\listoverridetable{\listoverride\listid1\ls1}}Regular paragraph\par \ls1 List item\par Another paragraph}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        assert_eq!(doc.blocks.len(), 3);
        // First block should be a paragraph
        assert!(matches!(&doc.blocks[0], Block::Paragraph(_)));
        // Second block should be a list
        assert!(matches!(&doc.blocks[1], Block::ListBlock(_)));
        // Third block should be a paragraph
        assert!(matches!(&doc.blocks[2], Block::Paragraph(_)));
    }

    #[test]
    fn test_listtable_destination_parsed() {
        // listtable should be parsed to extract list definitions
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc0}\listid1}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1 Item}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        // Should not have warnings about listtable destination itself
        // (may have warnings for unknown control words inside, which is expected)
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::UnknownDestination { .. }))
        );
        // Should still create a list block
        assert!(matches!(&doc.blocks[0], Block::ListBlock(_)));
    }

    #[test]
    fn test_listoverridetable_destination_parsed() {
        // listoverridetable should be parsed to extract list overrides
        let input = r#"{\rtf1\ansi {\listoverridetable{\listoverride\listid1\ls1}}\ls1 Item}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        // Should not have warnings about listoverridetable destination itself
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::UnknownDestination { .. }))
        );
        // Should still create a list block
        assert!(matches!(&doc.blocks[0], Block::ListBlock(_)));
    }

    #[test]
    fn test_pntext_destination_dropped() {
        // pntext content should be dropped
        let input = r#"{\rtf1\ansi {\pntext Marker}Item text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // Should have content without the pntext
        if let Block::Paragraph(para) = &doc.blocks[0] {
            let text: String = para.runs.iter().map(|r| r.text.as_str()).collect();
            assert!(text.contains("Item text"));
            // pntext content should not appear
            assert!(!text.contains("bullet"));
        }
    }

    #[test]
    fn test_pard_resets_list_state() {
        // \pard resets paragraph properties including list state
        // Use resolved list metadata so list output does not rely on unresolved fallback behavior.
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc23}}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1 List item\par\pard Regular text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // First should be a list item
        assert!(matches!(&doc.blocks[0], Block::ListBlock(_)));
        // After pard, should be a regular paragraph
        assert!(matches!(&doc.blocks[1], Block::Paragraph(_)));
    }

    // ==========================================================================
    // List Table Parsing Tests (Phase 3 - PR 3)
    // ==========================================================================

    #[test]
    fn test_listtable_parses_list_id() {
        // Test that listtable parsing extracts list IDs correctly
        let input = r#"{\rtf1\ansi {\listtable{\list\listid123{\listlevel}\listid123}}{\listoverridetable{\listoverride\listid123\ls1}}\ls1 Item}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // Should create a list block
        assert!(matches!(&doc.blocks[0], Block::ListBlock(_)));
    }

    #[test]
    fn test_listtable_parses_levelnfc_decimal() {
        // Test that levelnfc0 (decimal) creates OrderedDecimal kind
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc0}}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1 Item}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::ListBlock(list) = &doc.blocks[0] {
            assert_eq!(list.kind, ListKind::OrderedDecimal);
        } else {
            panic!("Expected ListBlock");
        }
    }

    #[test]
    fn test_listtable_parses_levelnfc_bullet() {
        // Test that levelnfc23 (bullet) creates Bullet kind
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc23}}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1 Item}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::ListBlock(list) = &doc.blocks[0] {
            assert_eq!(list.kind, ListKind::Bullet);
        } else {
            panic!("Expected ListBlock");
        }
    }

    #[test]
    fn test_listoverridetable_maps_ls_to_list() {
        // Test that listoverridetable correctly maps ls_id to list_id
        let input = r#"{\rtf1\ansi {\listtable{\list\listid100{\listlevel\levelnfc0}}}{\listoverridetable{\listoverride\listid100\ls5}}\ls5 Item}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // Should create a list block with the resolved list
        assert!(matches!(&doc.blocks[0], Block::ListBlock(_)));
    }

    #[test]
    fn test_unresolved_list_override_emits_warning() {
        // Test that referencing a non-existent ls_id emits UnresolvedListOverride warning
        let input = r#"{\rtf1\ansi \ls999 Item}"#;
        let (_doc, report) = Interpreter::parse(input).unwrap();

        // Should have UnresolvedListOverride warning
        assert!(report.warnings.iter().any(|w| matches!(
            w,
            crate::report::Warning::UnresolvedListOverride { ls_id: 999, .. }
        )));
    }

    #[test]
    fn test_unresolved_list_override_emits_dropped_content() {
        // Test that unresolved override also emits DroppedContent for strict mode
        let input = r#"{\rtf1\ansi \ls999 Item}"#;
        let (_doc, report) = Interpreter::parse(input).unwrap();

        // Should have DroppedContent warning for strict mode compatibility
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_unsupported_nesting_level_emits_warning() {
        // Test that level > 8 emits UnsupportedNestingLevel warning
        let input = r#"{\rtf1\ansi \ls1\ilvl15 Item}"#;
        let (_doc, report) = Interpreter::parse(input).unwrap();

        // Should have UnsupportedNestingLevel warning
        assert!(report.warnings.iter().any(|w| matches!(
            w,
            crate::report::Warning::UnsupportedNestingLevel {
                level: 15,
                max: 8,
                ..
            }
        )));
    }

    #[test]
    fn test_unsupported_list_control_emits_warning() {
        // Test that unknown control words in list table emit UnsupportedListControl warning
        let input =
            r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\unknownlistword}}}\ls1 Item}"#;
        let (_doc, report) = Interpreter::parse(input).unwrap();

        // Should have UnsupportedListControl warning
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::UnsupportedListControl { .. }))
        );
    }

    #[test]
    fn test_list_kind_fallback_when_no_listtable() {
        // When no listtable/override is present, unresolved \ls should keep paragraph output.
        let input = r#"{\rtf1\ansi \ls1 Item}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.runs[0].text, "Item");
        } else {
            panic!("Expected Paragraph");
        }
    }

    #[test]
    fn test_level_clamped_to_eight() {
        // Test that level is clamped to 8 even when warning is emitted
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc0}\listid1}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1\ilvl15 Item}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::ListBlock(list) = &doc.blocks[0] {
            assert_eq!(list.items[0].level, 8); // Clamped from 15
        } else {
            panic!("Expected ListBlock");
        }
    }

    #[test]
    fn test_resolved_list_uses_correct_kind() {
        // Test that resolved list uses kind from listtable
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc0}}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1 Item}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::ListBlock(list) = &doc.blocks[0] {
            // levelnfc0 = decimal, so should be OrderedDecimal
            assert_eq!(list.kind, ListKind::OrderedDecimal);
        } else {
            panic!("Expected ListBlock");
        }
    }

    #[test]
    fn test_table_followed_by_prose_without_pard_is_preserved() {
        let input = r#"{\rtf1\ansi
\trowd\cellx2880\cellx5760
\intbl Cell 1\cell Cell 2\cell\row
After table text.\par
}"#;

        let (doc, report) = Interpreter::parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);

        assert!(matches!(doc.blocks[0], Block::TableBlock(_)));
        if let Block::Paragraph(para) = &doc.blocks[1] {
            let text: String = para.runs.iter().map(|r| r.text.as_str()).collect();
            assert!(text.contains("After table text."));
        } else {
            panic!("Expected paragraph after table");
        }

        assert!(
            !report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::UnclosedTableCell { .. }))
        );
    }

    #[test]
    fn test_missing_cell_terminator_before_row_auto_closes_cell() {
        let input = r#"{\rtf1\ansi
\trowd\cellx2880\cellx5760
\intbl Cell 1\cell Cell 2\row
}"#;

        let (doc, report) = Interpreter::parse(input).unwrap();

        let table = match &doc.blocks[0] {
            Block::TableBlock(table) => table,
            _ => panic!("Expected table block"),
        };

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells.len(), 2);

        let first: String = table.rows[0].cells[0]
            .blocks
            .iter()
            .flat_map(|b| match b {
                Block::Paragraph(p) => p.runs.iter().map(|r| r.text.clone()).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();
        let second: String = table.rows[0].cells[1]
            .blocks
            .iter()
            .flat_map(|b| match b {
                Block::Paragraph(p) => p.runs.iter().map(|r| r.text.clone()).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();

        assert_eq!(first, "Cell 1");
        assert_eq!(second, "Cell 2");

        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::UnclosedTableCell { .. }))
        );
    }

    #[test]
    fn test_table_cell_width_uses_cellx_deltas() {
        let input = r#"{\rtf1\ansi
\trowd\cellx2880\cellx5760
\intbl A\cell B\cell\row
}"#;

        let (doc, _report) = Interpreter::parse(input).unwrap();
        let table = match &doc.blocks[0] {
            Block::TableBlock(table) => table,
            _ => panic!("Expected table block"),
        };

        assert_eq!(table.rows[0].cells[0].width_twips, Some(2880));
        assert_eq!(table.rows[0].cells[1].width_twips, Some(2880));
    }

    #[test]
    fn test_table_cell_list_is_preserved_as_list_block() {
        let input = r#"{\rtf1\ansi
{\listtable{\list\listid1{\listlevel\levelnfc23}}}
{\listoverridetable{\listoverride\listid1\ls1}}
\trowd\cellx2880\cellx5760
\intbl Regular\cell {\ls1\ilvl0 Item A\par}{\ls1\ilvl0 Item B\par}\cell\row
}"#;

        let (doc, _report) = Interpreter::parse(input).unwrap();
        let table = match &doc.blocks[0] {
            Block::TableBlock(table) => table,
            _ => panic!("Expected table block"),
        };

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells.len(), 2);

        let list_block = table.rows[0].cells[1]
            .blocks
            .iter()
            .find_map(|b| match b {
                Block::ListBlock(list) => Some(list),
                _ => None,
            })
            .expect("Expected list block in table cell");

        assert_eq!(list_block.items.len(), 2);
    }
}
