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
    Alignment, Block, CellMerge, CellVerticalAlign, Color, ColorEntry, Document, Hyperlink, Inline,
    ListBlock, ListId, ListItem, ListKind, Paragraph, Report, RowAlignment, RowProps, Run,
    Shading, ShadingPattern, TableBlock, TableCell, TableProps, TableRow, ThemeColor,
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
    /// Current font index (from \fN)
    pub font_index: Option<i32>,
    /// Font size in half-points (from \fsN)
    pub font_size_half_points: Option<i32>,
    /// Current color index (from \cfN)
    pub color_index: Option<i32>,
    /// Current highlight color index (from \highlightN)
    pub highlight_color_index: Option<i32>,
    /// Current background color index (from \cbN)
    pub background_color_index: Option<i32>,
    // =============================================================================
    // Paragraph Shading State (reset by \pard, NOT reset by \plain)
    // =============================================================================
    /// Paragraph background color index (from \cbpatN)
    pub paragraph_cbpat: Option<i32>,
    /// Paragraph pattern color index (from \cfpatN) - for Slice B
    pub paragraph_cfpat: Option<i32>,
    /// Paragraph shading percentage (from \shadingN) - for Slice B
    pub paragraph_shading: Option<i32>,
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
    /// Field instruction destination - handled specially for hyperlink parsing
    FldInst,
    /// Field result destination - handled specially for hyperlink parsing
    FldRslt,
    /// Font table destination - parse font definitions
    FontTable,
    /// Color table destination - parse color definitions
    ColorTable,
}

#[derive(Debug, Clone, Default)]
struct NestedFieldState {
    field_group_depth: usize,
    parsing_fldinst: bool,
    fldinst_group_depth: usize,
    parsing_fldrslt: bool,
    fldrslt_group_depth: usize,
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
    // Document Style Resources (Font/Color)
    // =============================================================================
    /// Default font index (from \deffN)
    default_font_index: Option<i32>,
    /// Font table mapping font index to font family name
    font_table: HashMap<i32, String>,
    /// Color table (index 0 is auto/default, represented as None)
    color_table: Vec<ColorEntry>,
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
    // Font Table Parsing State
    // =============================================================================
    /// Whether we're currently parsing a font table destination
    parsing_font_table: bool,
    /// Current font index being parsed (from \fN)
    current_font_index: Option<i32>,
    /// Current font name being accumulated
    current_font_name: String,
    // =============================================================================
    // Color Table Parsing State
    // =============================================================================
    /// Whether we're currently parsing a color table destination
    parsing_color_table: bool,
    /// Current red component (from \redN)
    current_red: u8,
    /// Current green component (from \greenN)
    current_green: u8,
    /// Current blue component (from \blueN)
    current_blue: u8,
    /// Whether any color component has been set since last semicolon
    color_components_seen: bool,
    /// Current theme color index (from \themecolorN in color table)
    current_theme_color: Option<ThemeColor>,
    /// Current theme tint value (from \ctintN in color table)
    current_theme_tint: Option<u8>,
    /// Current theme shade value (from \cshadeN in color table)
    current_theme_shade: Option<u8>,
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
    /// Merge state per cell boundary (from \clmgf, \clmrg, etc. before each \cellx)
    pending_cell_merges: Vec<Option<CellMerge>>,
    /// Vertical alignment per cell boundary (from \clvertalt, etc. before each \cellx)
    pending_cell_v_aligns: Vec<Option<CellVerticalAlign>>,
    /// Whether the current paragraph saw \intbl.
    ///
    /// This flag is scoped to the current paragraph and reset at paragraph boundaries.
    /// Table membership itself is derived from active row/cell state.
    seen_intbl_in_paragraph: bool,
    // =============================================================================
    // Table Merge/Property State (Phase 5)
    // =============================================================================
    /// Pending cell merge state (reset per cell)
    pending_cell_merge: Option<CellMerge>,
    /// Pending cell vertical alignment (reset per cell)
    pending_cell_v_align: Option<CellVerticalAlign>,
    /// Pending row properties (reset per row)
    pending_row_props: RowProps,
    // =============================================================================
    // Table Cell Shading State (Phase 5 - Shading)
    // =============================================================================
    /// Pending cell background color index (from \clcbpatN, reset per cell)
    pending_cell_cbpat: Option<i32>,
    /// Pending row background color index (from \trcbpatN, reset per row)
    pending_row_cbpat: Option<i32>,
    /// Pending table background color index (from \trcbpatN at table level)
    pending_table_cbpat: Option<i32>,
    /// Pending table pattern color index (from first-row \trcfpatN)
    pending_table_cfpat: Option<i32>,
    /// Pending table shading percentage (from first-row \trshdngN)
    pending_table_shading: Option<i32>,
    /// Pending row pattern color index (from \trcfpatN) - for Slice B
    pending_row_cfpat: Option<i32>,
    /// Pending row shading percentage (from \trshdngN) - for Slice B
    pending_row_shading: Option<i32>,
    /// Pending cell pattern color index (from \clcfpatN) - for Slice B
    pending_cell_cfpat: Option<i32>,
    /// Pending cell shading percentage (from \clshdngN) - for Slice B
    pending_cell_shading: Option<i32>,
    /// Cell background color indexes per cell boundary (stored at each \cellx)
    pending_cell_cbpats: Vec<Option<i32>>,
    /// Cell pattern color indexes per cell boundary - for Slice B
    pending_cell_cfpats: Vec<Option<i32>>,
    /// Cell shading percentages per cell boundary - for Slice B
    pending_cell_shadings: Vec<Option<i32>>,
    /// Fatal parser failure encountered mid-parse.
    ///
    /// Used for hard-limit violations discovered in helper methods that
    /// don't return `Result` directly.
    hard_failure: Option<ParseError>,
    // =============================================================================
    // Field Parsing State (Hyperlinks)
    // =============================================================================
    /// Whether we're currently parsing a field group
    parsing_field: bool,
    /// Depth of the field group (for tracking nested groups within field)
    field_group_depth: usize,
    /// Whether we're currently in the fldinst (instruction) part of a field
    parsing_fldinst: bool,
    /// Depth of the fldinst group
    fldinst_group_depth: usize,
    /// Whether we're currently in the fldrslt (result) part of a field
    parsing_fldrslt: bool,
    /// Depth of the fldrslt group
    fldrslt_group_depth: usize,
    /// Accumulated instruction text from fldinst
    field_instruction_text: String,
    /// Accumulated runs from fldrslt (visible content)
    field_result_inlines: Vec<Inline>,
    /// Style state at field start (to restore after field)
    field_style_snapshot: Option<StyleState>,
    /// Nested field state stack (fields inside fldrslt are degraded to plain text)
    nested_fields: Vec<NestedFieldState>,
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
            // Document style resources
            default_font_index: None,
            font_table: HashMap::new(),
            color_table: Vec::new(),
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
            // Font table parsing state
            parsing_font_table: false,
            current_font_index: None,
            current_font_name: String::new(),
            // Color table parsing state
            parsing_color_table: false,
            current_red: 0,
            current_green: 0,
            current_blue: 0,
            color_components_seen: false,
            current_theme_color: None,
            current_theme_tint: None,
            current_theme_shade: None,
            // Table parsing state
            current_table: None,
            current_row: None,
            current_cell: None,
            pending_cellx: Vec::new(),
            pending_cell_merges: Vec::new(),
            pending_cell_v_aligns: Vec::new(),
            seen_intbl_in_paragraph: false,
            // Table merge/property state (Phase 5)
            pending_cell_merge: None,
            pending_cell_v_align: None,
            pending_row_props: RowProps::default(),
            // Table cell shading state (Phase 5 - Shading)
            pending_cell_cbpat: None,
            pending_cell_cfpat: None,
            pending_cell_shading: None,
            pending_cell_cbpats: Vec::new(),
            pending_cell_cfpats: Vec::new(),
            pending_cell_shadings: Vec::new(),
            // Row/table fallback shading state
            pending_row_cbpat: None,
            pending_table_cbpat: None,
            pending_table_cfpat: None,
            pending_table_shading: None,
            pending_row_cfpat: None,
            pending_row_shading: None,
            hard_failure: None,
            // Field parsing state (Hyperlinks)
            parsing_field: false,
            field_group_depth: 0,
            parsing_fldinst: false,
            fldinst_group_depth: 0,
            parsing_fldrslt: false,
            fldrslt_group_depth: 0,
            field_instruction_text: String::new(),
            field_result_inlines: Vec::new(),
            field_style_snapshot: None,
            nested_fields: Vec::new(),
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

        if let Some(err) = interpreter.hard_failure.take() {
            return Err(ConversionError::Parse(err));
        }

        // Build the final report
        let report = interpreter.report_builder.build();

        Ok((interpreter.document, report))
    }

    /// Process a single RTF event.
    fn process_event(&mut self, event: RtfEvent) -> Result<(), ConversionError> {
        if let Some(err) = self.hard_failure.take() {
            return Err(ConversionError::Parse(err));
        }

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

                // Process field group end
                self.process_field_group_end();
            }
            RtfEvent::ControlWord { word, parameter } => {
                self.handle_control_word(&word, parameter);
            }
            RtfEvent::ControlSymbol(symbol) => {
                self.handle_control_symbol(symbol);
            }
            RtfEvent::Text(text) => {
                self.mark_current_group_non_destination();
                // Route text to field handler if in field parsing mode
                if self.parsing_field {
                    self.handle_field_text(text);
                } else {
                    self.handle_text(text);
                }
            }
            RtfEvent::BinaryData(data) => {
                self.report_builder
                    .dropped_content("Dropped unsupported binary RTF data", Some(data.len()));
            }
        }

        if let Some(err) = self.hard_failure.take() {
            return Err(ConversionError::Parse(err));
        }

        Ok(())
    }

    fn process_skipped_destination_event(
        &mut self,
        event: RtfEvent,
    ) -> Result<(), ConversionError> {
        if let Some(err) = self.hard_failure.take() {
            return Err(ConversionError::Parse(err));
        }

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
                // Handle nested groups in font table parsing (each font entry is in a group)
                if self.parsing_font_table && self.skip_destination_depth == 2 {
                    // Starting a new font entry group - reset font parsing state
                    self.current_font_index = None;
                    self.current_font_name.clear();
                }
            }
            RtfEvent::GroupEnd => {
                // Finalize list definition when closing a \list group
                if self.parsing_list_table
                    && self.skip_destination_depth == 2
                    && let Some(list_def) = self.current_list_def.take()
                {
                    self.list_table.insert(list_def.list_id, list_def);
                }

                // Finalize list override when closing a \listoverride group
                if self.parsing_list_override_table
                    && self.skip_destination_depth == 2
                    && let Some(list_override) = self.current_list_override.take()
                {
                    self.list_overrides
                        .insert(list_override.ls_id, list_override);
                }

                // Finalize list level when closing a \listlevel group
                if self.parsing_list_table
                    && self.skip_destination_depth == 3
                    && let Some(level) = self.current_list_level.take()
                    && let Some(ref mut list_def) = self.current_list_def
                {
                    list_def.levels.push(level);
                }

                // Finalize font entry when closing a font group
                if self.parsing_font_table
                    && self.skip_destination_depth == 2
                    && let Some(font_idx) = self.current_font_index.take()
                    && !self.current_font_name.is_empty()
                {
                    // Clean up font name: trim whitespace and remove trailing semicolon if present
                    let mut name = self.current_font_name.trim().to_string();
                    if name.ends_with(';') {
                        name.pop();
                    }
                    let name = name.trim().to_string();
                    if !name.is_empty() {
                        self.font_table.insert(font_idx, name);
                    }
                    self.current_font_name.clear();
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
                    self.parsing_font_table = false;
                    self.parsing_color_table = false;
                }
            }
            RtfEvent::ControlWord { word, parameter } => {
                self.handle_destination_control_word(&word, parameter);
            }
            RtfEvent::Text(text) => {
                // Handle text in font/color table destinations
                if self.parsing_font_table {
                    // Accumulate font name text
                    self.current_font_name.push_str(&text);
                } else if self.parsing_color_table {
                    // Handle semicolons in color table
                    // Semicolons separate color entries
                    for ch in text.chars() {
                        if ch == ';' {
                            // First semicolon marks auto/default color slot (index 0)
                            // Subsequent semicolons mark end of color definitions
                            if self.color_components_seen {
                                // We have RGB components, push the defined color
                                let color = Color {
                                    r: self.current_red,
                                    g: self.current_green,
                                    b: self.current_blue,
                                };
                                // Check if we also have theme color metadata
                                if let Some(theme_color) = self.current_theme_color.take() {
                                    // Create a ColorEntry with both RGB and theme info
                                    self.color_table.push(ColorEntry {
                                        rgb: Some(color),
                                        theme_color: Some(theme_color),
                                        theme_tint: self.current_theme_tint.take(),
                                        theme_shade: self.current_theme_shade.take(),
                                    });
                                } else {
                                    // Just RGB, no theme color
                                    self.color_table.push(ColorEntry::rgb(color));
                                }
                            } else if let Some(theme_color) = self.current_theme_color.take() {
                                // Theme color without explicit RGB
                                self.color_table.push(ColorEntry::theme(
                                    theme_color,
                                    self.current_theme_tint.take(),
                                    self.current_theme_shade.take(),
                                ));
                            } else {
                                // No RGB components seen - this is auto/default slot
                                self.color_table.push(ColorEntry::auto_color());
                            }
                            // Reset for next color
                            self.current_red = 0;
                            self.current_green = 0;
                            self.current_blue = 0;
                            self.color_components_seen = false;
                            self.current_theme_color = None;
                            self.current_theme_tint = None;
                            self.current_theme_shade = None;
                        }
                    }
                }
                // Other destinations ignore text
            }
            RtfEvent::ControlSymbol(_) | RtfEvent::BinaryData(_) => {}
        }

        if let Some(err) = self.hard_failure.take() {
            return Err(ConversionError::Parse(err));
        }

        Ok(())
    }

    /// Handle control words within destination parsing (list table, font table, color table).
    fn handle_destination_control_word(&mut self, word: &str, parameter: Option<i32>) {
        // Font table control words
        if self.parsing_font_table {
            match word {
                "f" => {
                    // Font index
                    self.current_font_index = parameter;
                }
                // Font property controls - we skip these but don't warn
                "fnil" | "froman" | "fswiss" | "fmodern" | "fscript" | "fdecor" | "ftech"
                | "fbidi" => {
                    // Font family - ignore
                }
                "fcharset" => {
                    // Character set - ignore
                }
                "fprq" => {
                    // Font pitch - ignore
                }
                "panose" | "ftnil" | "fttruetype" => {
                    // Font technology - ignore
                }
                _ => {
                    // For font table, we don't warn on unknown controls
                    // They're likely font-specific properties we don't need
                }
            }
            return;
        }

        // Color table control words
        if self.parsing_color_table {
            match word {
                "red" => {
                    if let Some(val) = parameter {
                        self.current_red = val.clamp(0, 255) as u8;
                        self.color_components_seen = true;
                    }
                }
                "green" => {
                    if let Some(val) = parameter {
                        self.current_green = val.clamp(0, 255) as u8;
                        self.color_components_seen = true;
                    }
                }
                "blue" => {
                    if let Some(val) = parameter {
                        self.current_blue = val.clamp(0, 255) as u8;
                        self.color_components_seen = true;
                    }
                }
                // Theme color controls
                "themecolor" => {
                    if let Some(val) = parameter {
                        self.current_theme_color = ThemeColor::from_index(val);
                    }
                }
                "ctint" => {
                    if let Some(val) = parameter {
                        self.current_theme_tint = Some(val.clamp(0, 255) as u8);
                    }
                }
                "cshade" => {
                    if let Some(val) = parameter {
                        self.current_theme_shade = Some(val.clamp(0, 255) as u8);
                    }
                }
                _ => {
                    // Unknown control in color table - ignore silently
                }
            }
            return;
        }

        // List table control words (existing logic)
        self.handle_list_table_control_word(word, parameter);
    }

    fn set_hard_failure(&mut self, err: ParseError) {
        if self.hard_failure.is_none() {
            self.hard_failure = Some(err);
        }
    }

    /// Reset character formatting to defaults (called by \plain).
    ///
    /// This resets ONLY character-style fields:
    /// - bold, italic, underline
    /// - font_index, font_size_half_points, color_index
    /// - highlight_color_index, background_color_index
    ///
    /// It does NOT reset:
    /// - paragraph alignment
    /// - list state
    /// - table state
    fn reset_character_formatting(&mut self) {
        self.current_style.bold = false;
        self.current_style.italic = false;
        self.current_style.underline = false;
        // Reset font to default font index if available
        self.current_style.font_index = self.default_font_index;
        self.current_style.font_size_half_points = None;
        self.current_style.color_index = None;
        self.current_style.highlight_color_index = None;
        self.current_style.background_color_index = None;
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
                if let Some(id) = parameter
                    && self.parsing_list_override_table
                    && let Some(ref mut list_override) = self.current_list_override
                {
                    list_override.ls_id = id;
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
            // Field destinations - handled specially for hyperlink parsing
            "fldinst" => Some(DestinationBehavior::FldInst),
            "fldrslt" => Some(DestinationBehavior::FldRslt),
            // Font table - parse font definitions
            "fonttbl" => Some(DestinationBehavior::FontTable),
            // Color table - parse color definitions
            "colortbl" => Some(DestinationBehavior::ColorTable),
            // Metadata destinations that are intentionally excluded from body content.
            "stylesheet" | "info" | "title" | "author" | "operator" | "keywords" | "comment"
            | "version" | "vern" | "creatim" | "revtim" | "printim" | "buptim" | "edmins"
            | "nofpages" | "nofwords" | "nofchars" | "nofcharsws" | "id" => {
                Some(DestinationBehavior::Metadata)
            }
            // Destinations that represent currently unsupported visible content.
            "pict" | "obj" | "objclass" | "objdata" | "shppict" | "nonshppict" | "picprop"
            | "datafield" | "header" | "headerl" | "headerr" | "footer" | "footerl" | "footerr"
            | "footnote" | "annotation" | "pn" | "pntext" | "pntxtb" | "pntxta" | "pnseclvl" => {
                Some(DestinationBehavior::Dropped(
                    "Dropped unsupported RTF destination content",
                ))
            }
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
                DestinationBehavior::FontTable => {
                    self.parsing_font_table = true;
                    // Use the latest font table definition if multiple are present.
                    self.font_table.clear();
                    self.current_font_index = None;
                    self.current_font_name.clear();
                    self.skip_destination_depth = 1;
                }
                DestinationBehavior::ColorTable => {
                    self.parsing_color_table = true;
                    // Use the latest color table definition if multiple are present.
                    self.color_table.clear();
                    self.current_red = 0;
                    self.current_green = 0;
                    self.current_blue = 0;
                    self.color_components_seen = false;
                    self.current_theme_color = None;
                    self.current_theme_tint = None;
                    self.current_theme_shade = None;
                    // Don't pre-initialize - first semicolon marks auto/default color
                    self.skip_destination_depth = 1;
                }
                DestinationBehavior::FldInst => {
                    // Field instruction - handle specially for hyperlink parsing
                    // Set state but don't skip - we need to capture the instruction text
                    if let Some(nested) = self.nested_fields.last_mut() {
                        nested.parsing_fldinst = true;
                        nested.fldinst_group_depth = self.current_depth;
                    } else if self.parsing_field {
                        self.parsing_fldinst = true;
                        self.fldinst_group_depth = self.current_depth;
                    }
                    self.destination_marker = false;
                    self.mark_current_group_non_destination();
                    return false; // Don't skip - continue processing
                }
                DestinationBehavior::FldRslt => {
                    // Field result - handle specially for hyperlink parsing
                    // Set state but don't skip - we need to capture the result content
                    if let Some(nested) = self.nested_fields.last_mut() {
                        nested.parsing_fldrslt = true;
                        nested.fldrslt_group_depth = self.current_depth;
                    } else if self.parsing_field {
                        self.parsing_fldrslt = true;
                        self.fldrslt_group_depth = self.current_depth;
                    }
                    self.destination_marker = false;
                    self.mark_current_group_non_destination();
                    return false; // Don't skip - continue processing
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
                    // Reset paragraph shading state (reset by \pard, NOT by \plain)
                    self.current_style.paragraph_cbpat = None;
                    self.current_style.paragraph_cfpat = None;
                    self.current_style.paragraph_shading = None;
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
                    // Store the current merge state and vertical alignment for this cell
                    self.pending_cell_merges
                        .push(self.pending_cell_merge.take());
                    self.pending_cell_v_aligns
                        .push(self.pending_cell_v_align.take());
                    // Store the current cell shading state for this cell
                    self.pending_cell_cbpats
                        .push(self.pending_cell_cbpat.take());
                    self.pending_cell_cfpats
                        .push(self.pending_cell_cfpat.take());
                    self.pending_cell_shadings
                        .push(self.pending_cell_shading.take());
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
            // =============================================================================
            // Row Property Controls (Phase 5)
            // =============================================================================
            // Row alignment
            "trql" => {
                self.pending_row_props.alignment = Some(RowAlignment::Left);
            }
            "trqc" => {
                self.pending_row_props.alignment = Some(RowAlignment::Center);
            }
            "trqr" => {
                self.pending_row_props.alignment = Some(RowAlignment::Right);
            }
            // Row left indent (in twips)
            "trleft" => {
                if let Some(value) = parameter {
                    self.pending_row_props.left_indent = Some(value);
                }
            }
            // Row formatting controls - recognized but not fully supported
            "trgaph" => {
                self.report_builder.unsupported_table_control(word);
            }
            // =============================================================================
            // Cell Vertical Alignment Controls (Phase 5)
            // =============================================================================
            "clvertalt" => {
                self.pending_cell_v_align = Some(CellVerticalAlign::Top);
            }
            "clvertalc" => {
                self.pending_cell_v_align = Some(CellVerticalAlign::Center);
            }
            "clvertalb" => {
                self.pending_cell_v_align = Some(CellVerticalAlign::Bottom);
            }
            // =============================================================================
            // Cell Merge Controls (Phase 5)
            // =============================================================================
            // Horizontal merge start - this cell starts a merge
            "clmgf" => {
                // Mark this cell as merge start
                // Span will be calculated when we see continuation markers
                self.pending_cell_merge = Some(CellMerge::HorizontalStart { span: 1 });
            }
            // Horizontal merge continuation - this cell is merged with previous
            "clmrg" => {
                self.pending_cell_merge = Some(CellMerge::HorizontalContinue);
            }
            // Vertical merge start - this cell starts a vertical merge
            "clvmgf" => {
                self.pending_cell_merge = Some(CellMerge::VerticalStart);
            }
            // Vertical merge continuation - this cell continues vertical merge
            "clvmrg" => {
                self.pending_cell_merge = Some(CellMerge::VerticalContinue);
            }
            // =============================================================================
            // Paragraph Shading Controls (Phase 5 - Shading)
            // =============================================================================
            // \cbpatN - Paragraph background color index
            "cbpat" => {
                self.current_style.paragraph_cbpat = parameter;
            }
            // \cfpatN - Paragraph pattern color index (for Slice B)
            "cfpat" => {
                self.current_style.paragraph_cfpat = parameter;
            }
            // \shadingN - Paragraph shading percentage (for Slice B)
            "shading" => {
                self.current_style.paragraph_shading = parameter;
            }
            // =============================================================================
            // Cell Shading Controls (Phase 5 - Shading)
            // =============================================================================
            // \clcbpatN - Cell background color index
            "clcbpat" => {
                self.pending_cell_cbpat = parameter;
            }
            // \clcfpatN - Cell pattern color index (for Slice B)
            "clcfpat" => {
                self.pending_cell_cfpat = parameter;
            }
            // \clshdngN - Cell shading percentage (for Slice B)
            "clshdng" => {
                self.pending_cell_shading = parameter;
            }
            // =============================================================================
            // Row/Table Fallback Shading Controls (Phase 5 - Shading)
            // =============================================================================
            // \trcbpatN - Row background color index
            "trcbpat" => {
                // Capture for row-level shading (applied when row is finalized)
                self.pending_row_cbpat = parameter;
                // Also capture for table-level shading if no table shading set yet
                if self.pending_table_cbpat.is_none() {
                    self.pending_table_cbpat = parameter;
                }
            }
            // \trcfpatN - Row pattern color index (for Slice B)
            "trcfpat" => {
                self.pending_row_cfpat = parameter;
                // Capture first-row value as table-level default
                if self.pending_table_cfpat.is_none() {
                    self.pending_table_cfpat = parameter;
                }
            }
            // \trshdngN - Row shading percentage (for Slice B)
            "trshdng" => {
                self.pending_row_shading = parameter;
                // Capture first-row value as table-level default
                if self.pending_table_shading.is_none() {
                    self.pending_table_shading = parameter;
                }
            }
            // =============================================================================
            // Field Control Words (Hyperlinks)
            // =============================================================================
            // \field - Start of a field group
            "field" => {
                self.start_field_parsing();
            }
            // \fldinst - Field instruction (contains HYPERLINK "url" for hyperlinks)
            "fldinst" => {
                if let Some(nested) = self.nested_fields.last_mut() {
                    nested.parsing_fldinst = true;
                    nested.fldinst_group_depth = self.current_depth;
                } else if self.parsing_field {
                    self.parsing_fldinst = true;
                    self.fldinst_group_depth = self.current_depth;
                }
            }
            // \fldrslt - Field result (visible content)
            "fldrslt" => {
                if let Some(nested) = self.nested_fields.last_mut() {
                    nested.parsing_fldrslt = true;
                    nested.fldrslt_group_depth = self.current_depth;
                } else if self.parsing_field {
                    self.parsing_fldrslt = true;
                    self.fldrslt_group_depth = self.current_depth;
                }
            }
            // RTF header control words - silently ignored (not user-facing)
            "rtf" | "ansi" | "ansicpg" | "deflang" | "deflangfe" | "adeflang" | "result"
            | "hwid" | "emdash" | "endash" | "emspace" | "enspace" | "qmspace" | "bullet"
            | "lquote" | "rquote" | "ldblquote" | "rdblquote" | "tab"
            | "strike" | "striked" | "sub" | "super" | "nosupersub" | "caps" | "scaps" | "outl"
            | "shad" | "expnd" | "expndtw" | "kerning" | "charscalex" | "lang" | "langfe"
            | "langnp" | "langfenp" => {
                // Silently ignore these structural/formatting control words
            }
            // \deffN - Default font index
            "deff" => {
                if let Some(index) = parameter {
                    self.default_font_index = Some(index);
                    // Also set as current font if no font is currently set
                    if self.current_style.font_index.is_none() {
                        self.current_style.font_index = Some(index);
                    }
                }
            }
            // \fN - Font index
            "f" => {
                self.current_style.font_index = parameter;
            }
            // \fsN - Font size in half-points
            "fs" => {
                self.current_style.font_size_half_points = parameter;
            }
            // \cfN - Foreground color index
            "cf" => {
                self.current_style.color_index = parameter;
            }
            // \highlightN - Highlight color index
            "highlight" => {
                self.current_style.highlight_color_index = parameter.and_then(|n| {
                    if n > 0 { Some(n) } else { None }
                });
            }
            // \cbN - Background color index
            "cb" => {
                self.current_style.background_color_index = parameter.and_then(|n| {
                    if n > 0 { Some(n) } else { None }
                });
            }
            // \plain - Reset character formatting only
            "plain" => {
                self.reset_character_formatting();
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
        if self.current_paragraph.inlines.is_empty()
            && self.current_text.is_empty()
            && self.current_table.is_some()
            && self.current_row.is_none()
            && !self.seen_intbl_in_paragraph
        {
            self.finalize_current_table();
        }

        // If this is the first text in the paragraph, capture the alignment
        if self.current_paragraph.inlines.is_empty() && self.current_text.is_empty() {
            self.paragraph_alignment = self.current_style.alignment;
        }

        // Check if style has changed
        if self.style_changed() {
            // Flush current text as a run if any
            if !self.current_text.is_empty() {
                let run = self.create_run();
                self.current_paragraph.inlines.push(Inline::Run(run));
                self.current_text.clear();
            }
            self.current_run_style = self.current_style.snapshot();
        }

        // Append text
        self.current_text.push_str(&text);
    }

    /// Check if the current style differs from the run style.
    fn style_changed(&self) -> bool {
        self.character_style_changed()
    }

    /// Check if character-level style differs from the active run style.
    fn character_style_changed(&self) -> bool {
        self.current_style.bold != self.current_run_style.bold
            || self.current_style.italic != self.current_run_style.italic
            || self.current_style.underline != self.current_run_style.underline
            || self.current_style.font_index != self.current_run_style.font_index
            || self.current_style.font_size_half_points
                != self.current_run_style.font_size_half_points
            || self.current_style.color_index != self.current_run_style.color_index
            || self.current_style.highlight_color_index
                != self.current_run_style.highlight_color_index
            || self.current_style.background_color_index
                != self.current_run_style.background_color_index
    }

    /// Create a run from the current text and run style.
    fn create_run(&self) -> Run {
        // Resolve font_family from font_index -> font_table
        let font_family = self.resolve_font_family();

        // Resolve font_size from half-points to points
        let font_size = self.resolve_font_size();

        // Resolve color from color_index -> color_table
        let color = self.resolve_color();

        // Resolve background_color with precedence: highlight > background
        let background_color = self.resolve_background_color();

        Run {
            text: self.current_text.clone(),
            bold: self.current_run_style.bold,
            italic: self.current_run_style.italic,
            underline: self.current_run_style.underline,
            font_family,
            font_size,
            color,
            background_color,
        }
    }

    /// Resolve font size from current run style.
    ///
    /// Invalid non-positive half-point values degrade to None.
    fn resolve_font_size(&self) -> Option<f32> {
        self.current_run_style
            .font_size_half_points
            .and_then(|hp| if hp > 0 { Some(hp as f32 / 2.0) } else { None })
    }

    /// Resolve font family name from current font index.
    ///
    /// Falls back to default font index if current index is not set or not found.
    fn resolve_font_family(&self) -> Option<String> {
        // Try current font index first
        if let Some(font_idx) = self.current_run_style.font_index {
            if let Some(family) = self.font_table.get(&font_idx) {
                return Some(family.clone());
            }
        }

        // Fall back to default font index
        if let Some(default_idx) = self.default_font_index {
            if let Some(family) = self.font_table.get(&default_idx) {
                return Some(family.clone());
            }
        }

        None
    }

    /// Resolve color from current color index.
    ///
    /// \cf0 means auto/default color (represented as None).
    /// Invalid indices degrade to None without warnings.
    /// Theme colors are resolved to concrete RGB values.
    fn resolve_color(&self) -> Option<Color> {
        let color_idx = self.current_run_style.color_index?;

        // cf0 is auto/default color, represented as None
        if color_idx == 0 {
            return None;
        }

        // Color table stores: [auto (None), color1, color2, ...]
        // cf1 maps to color_table[1], cf2 to color_table[2], etc.
        let table_index = color_idx as usize;
        self.color_table
            .get(table_index)
            .and_then(|entry| entry.resolve())
    }

    /// Resolve background color from highlight_color_index or background_color_index.
    ///
    /// Precedence: highlight_color_index if set and resolvable, otherwise
    /// background_color_index if set and resolvable, otherwise None.
    /// Invalid indices degrade to None without warnings.
    fn resolve_background_color(&self) -> Option<Color> {
        // Try highlight_color_index first (takes precedence)
        if let Some(highlight_idx) = self.current_run_style.highlight_color_index {
            if let Some(color) = self.resolve_color_from_index(highlight_idx) {
                return Some(color);
            }
        }

        // Fall back to background_color_index
        if let Some(bg_idx) = self.current_run_style.background_color_index {
            if let Some(color) = self.resolve_color_from_index(bg_idx) {
                return Some(color);
            }
        }

        None
    }

    /// Resolve a color from a color table index.
    ///
    /// Index 0 means auto/default color (represented as None).
    /// Invalid indices degrade to None without warnings.
    /// Theme colors are resolved to concrete RGB values.
    fn resolve_color_from_index(&self, color_idx: i32) -> Option<Color> {
        // Index 0 is auto/default color, represented as None
        if color_idx == 0 {
            return None;
        }

        // Color table stores: [auto (None), color1, color2, ...]
        let table_index = color_idx as usize;
        self.color_table
            .get(table_index)
            .and_then(|entry| entry.resolve())
    }

    /// Map RTF shading percentage (0-10000) to ShadingPattern.
    ///
    /// RTF `\shadingN` and `\clshdngN` use percentage values where:
    /// - 0 = Clear (transparent)
    /// - 10000 = Solid (100%)
    /// - Other values map to Percent patterns
    fn shading_percentage_to_pattern(percentage: i32) -> Option<ShadingPattern> {
        // Clamp to valid range
        let clamped = percentage.clamp(0, 10000);

        match clamped {
            0 => Some(ShadingPattern::Clear),
            10000 => Some(ShadingPattern::Solid),
            // Map percentage to closest Percent pattern
            // RTF uses 0-10000, we map to discrete percentages
            p if p <= 75 => Some(ShadingPattern::Percent5),
            p if p <= 150 => Some(ShadingPattern::Percent10),
            p if p <= 250 => Some(ShadingPattern::Percent20),
            p if p <= 375 => Some(ShadingPattern::Percent25),
            p if p <= 450 => Some(ShadingPattern::Percent30),
            p if p <= 550 => Some(ShadingPattern::Percent40),
            p if p <= 650 => Some(ShadingPattern::Percent50),
            p if p <= 750 => Some(ShadingPattern::Percent60),
            p if p <= 825 => Some(ShadingPattern::Percent70),
            p if p <= 875 => Some(ShadingPattern::Percent75),
            p if p <= 950 => Some(ShadingPattern::Percent80),
            p if p < 10000 => Some(ShadingPattern::Percent90),
            _ => Some(ShadingPattern::Solid),
        }
    }

    /// Build a Shading object from fill color index, pattern color index, and shading percentage.
    ///
    /// This combines the three shading-related RTF controls into a single Shading struct:
    /// - `cbpat`/`clcbpat`: fill/background color index
    /// - `cfpat`/`clcfpat`: pattern/foreground color index
    /// - `shading`/`clshdng`: shading percentage (0-10000)
    fn build_shading(
        &self,
        fill_color_idx: Option<i32>,
        pattern_color_idx: Option<i32>,
        shading_percentage: Option<i32>,
    ) -> Option<Shading> {
        // Resolve fill color (required for any shading)
        let fill_color = fill_color_idx.and_then(|idx| self.resolve_color_from_index(idx));

        // If no fill color, no shading
        fill_color.map(|fill| {
            // Resolve pattern color (optional)
            let pattern_color = pattern_color_idx.and_then(|idx| self.resolve_color_from_index(idx));

            // Map shading percentage to pattern
            let pattern = shading_percentage.and_then(|p| Self::shading_percentage_to_pattern(p));

            // Determine the final pattern:
            // - If we have an explicit shading percentage, use the mapped pattern
            // - If we have a pattern color but no shading percentage, default to Solid
            // - If we have neither, leave pattern as None (flat fill, no pattern overlay)
            let final_pattern = match (pattern, pattern_color.is_some()) {
                (Some(p), _) => Some(p),
                (None, true) => Some(ShadingPattern::Solid),
                (None, false) => None,
            };

            Shading {
                fill_color: Some(fill),
                pattern_color,
                pattern: final_pattern,
            }
        })
    }

    fn flush_current_text_as_run(&mut self) {
        if !self.current_text.is_empty() {
            let run = self.create_run();
            self.current_paragraph.inlines.push(Inline::Run(run));
            self.current_text.clear();
        }
    }

    fn has_pending_paragraph_content(&self) -> bool {
        !self.current_text.is_empty() || !self.current_paragraph.inlines.is_empty()
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
        if !self.current_paragraph.inlines.is_empty() {
            self.current_paragraph.alignment = self.paragraph_alignment;

            // Apply paragraph shading from current style state
            self.current_paragraph.shading = self.build_shading(
                self.current_style.paragraph_cbpat,
                self.current_style.paragraph_cfpat,
                self.current_style.paragraph_shading,
            );

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
                .add_runs(Self::inline_run_count(&self.current_paragraph.inlines));
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
        if let Some(Block::ListBlock(last_list)) = self.document.blocks.last_mut()
            && last_list.list_id == list_id
            && last_list.kind == kind
        {
            last_list.add_item(ListItem::from_paragraph(level, paragraph));
            return;
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
            if let Some(Block::ListBlock(last_list)) = cell.blocks.last_mut()
                && last_list.list_id == list_id
                && last_list.kind == kind
            {
                last_list.add_item(ListItem::from_paragraph(level, paragraph));
                return;
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
        self.pending_cell_merges.clear();
        self.pending_cell_v_aligns.clear();
        // Clear pending cell shading vectors for new row
        self.pending_cell_cbpats.clear();
        self.pending_cell_cfpats.clear();
        self.pending_cell_shadings.clear();
        self.current_row = Some(TableRow::new());

        // Reset pending row properties for new row
        self.pending_row_props = RowProps::default();
        // Reset row-level shading state for new row
        self.pending_row_cbpat = None;
        self.pending_row_cfpat = None;
        self.pending_row_shading = None;

        // Reset pending cell properties
        self.pending_cell_merge = None;
        self.pending_cell_v_align = None;
        // Reset pending cell shading state
        self.pending_cell_cbpat = None;
        self.pending_cell_cfpat = None;
        self.pending_cell_shading = None;

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

        // Clear pending cellx and merge states for next row
        self.pending_cellx.clear();
        self.pending_cell_merges.clear();
        self.pending_cell_v_aligns.clear();
        self.seen_intbl_in_paragraph = false;
    }

    /// Finalize the current cell and attach it to the current row.
    fn finalize_current_cell(&mut self) {
        // Create cell if needed (for empty cells or cells with content)
        if self.current_cell.is_none() {
            self.current_cell = Some(TableCell::new());
        }

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

            let mut cell_with_props = cell;

            // Apply width
            if let Some(w) = width {
                if w > 0 {
                    cell_with_props.width_twips = Some(w);
                } else {
                    self.report_builder.malformed_table_structure(&format!(
                        "Non-increasing \\cellx boundaries at cell {}",
                        cell_index
                    ));
                }
            }

            // Apply merge state from stored per-cellx state
            cell_with_props.merge = self.pending_cell_merges.get(cell_index).cloned().flatten();

            // Apply vertical alignment from stored per-cellx state
            cell_with_props.v_align = self
                .pending_cell_v_aligns
                .get(cell_index)
                .copied()
                .flatten();

            // Apply shading with fallback precedence: cell > row > table
            // 1. Check for explicit cell shading first
            let cell_cbpat = self.pending_cell_cbpats.get(cell_index).copied().flatten();
            let cell_cfpat = self.pending_cell_cfpats.get(cell_index).copied().flatten();
            let cell_shading = self.pending_cell_shadings.get(cell_index).copied().flatten();
            
            if let Some(shading) = self.build_shading(cell_cbpat, cell_cfpat, cell_shading) {
                cell_with_props.shading = Some(shading);
            }
            // 2. Fall back to row shading if cell has no explicit shading
            else if let Some(shading) = self.build_shading(
                self.pending_row_cbpat,
                self.pending_row_cfpat,
                self.pending_row_shading,
            ) {
                cell_with_props.shading = Some(shading);
            }
            // 3. Fall back to table shading if no cell or row shading
            else if let Some(shading) = self.build_shading(
                self.pending_table_cbpat,
                self.pending_table_cfpat,
                self.pending_table_shading,
            ) {
                cell_with_props.shading = Some(shading);
            }

            if let Some(ref mut row) = self.current_row {
                row.cells.push(cell_with_props);
            }
        }

        // Reset pending cell state for next cell
        self.pending_cell_merge = None;
        self.pending_cell_v_align = None;
        self.pending_cell_cbpat = None;
        self.pending_cell_cfpat = None;
        self.pending_cell_shading = None;
    }

    /// Finalize the current row and attach it to the current table.
    fn finalize_current_row(&mut self) {
        if let Some(mut row) = self.current_row.take() {
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

            // Apply row-level shading to RowProps if present
            if let Some(shading) = self.build_shading(
                self.pending_row_cbpat,
                self.pending_row_cfpat,
                self.pending_row_shading,
            ) {
                self.pending_row_props.shading = Some(shading);
            }

            // Apply pending row properties
            if self.pending_row_props != RowProps::default() {
                row.row_props = Some(self.pending_row_props.clone());
            }

            // Normalize merge semantics
            self.normalize_row_merges(&mut row);

            // Resolve conflicts
            self.resolve_merge_conflicts(&mut row);

            // Check table limits
            if let Err(e) = self.check_table_limits(&row) {
                self.set_hard_failure(e);
                return;
            }

            if let Some(existing_row_count) = self.current_table.as_ref().map(|t| t.rows.len())
                && existing_row_count >= self.limits.max_rows_per_table
            {
                self.set_hard_failure(ParseError::InvalidStructure(format!(
                    "Table has {} rows, maximum is {}",
                    existing_row_count + 1,
                    self.limits.max_rows_per_table
                )));
                return;
            }

            if let Some(ref mut table) = self.current_table {
                table.rows.push(row);
            }

            // Reset pending row state
            self.pending_row_props = RowProps::default();
            self.pending_row_cbpat = None;
            self.pending_row_cfpat = None;
            self.pending_row_shading = None;
        }
    }

    /// Normalize merge semantics in a row.
    ///
    /// This calculates the span for horizontal merge start cells based on
    /// the number of continuation cells that follow.
    fn normalize_row_merges(&mut self, row: &mut TableRow) {
        // First, collect merge information
        let merge_info: Vec<Option<CellMerge>> =
            row.cells.iter().map(|c| c.merge.clone()).collect();

        let mut span_count: u16 = 0;
        let mut merge_start_idx: Option<usize> = None;

        // Process merge chains and calculate spans
        for (idx, merge) in merge_info.iter().enumerate() {
            match merge {
                Some(CellMerge::HorizontalStart { .. }) => {
                    // Close out any previous merge chain
                    if let Some(start_idx) = merge_start_idx {
                        if span_count > 1 {
                            row.cells[start_idx].merge =
                                Some(CellMerge::HorizontalStart { span: span_count });
                        } else {
                            // Single cell "merge" - clear it
                            row.cells[start_idx].merge = None;
                        }
                    }
                    merge_start_idx = Some(idx);
                    span_count = 1;
                }
                Some(CellMerge::HorizontalContinue) => {
                    span_count += 1;
                }
                _ => {
                    // Not in a merge chain - close out previous if any
                    if let Some(start_idx) = merge_start_idx {
                        if span_count > 1 {
                            row.cells[start_idx].merge =
                                Some(CellMerge::HorizontalStart { span: span_count });
                        } else {
                            // Single cell "merge" - clear it
                            row.cells[start_idx].merge = None;
                        }
                    }
                    merge_start_idx = None;
                    span_count = 0;
                }
            }
        }

        // Close out any trailing merge
        if let Some(start_idx) = merge_start_idx {
            if span_count > 1 {
                row.cells[start_idx].merge = Some(CellMerge::HorizontalStart { span: span_count });
            } else {
                row.cells[start_idx].merge = None;
            }
        }
    }

    /// Resolve merge conflicts with deterministic degradation.
    ///
    /// This handles:
    /// - Orphan continuations (continuation without start)
    /// - Spans exceeding row bounds
    fn resolve_merge_conflicts(&mut self, row: &mut TableRow) {
        // Track expected continuation cells from the most recent horizontal start.
        let mut expected_continuations: usize = 0;

        // Detect orphan continuations regardless of column position.
        for idx in 0..row.cells.len() {
            let merge = row.cells[idx].merge.clone();
            match merge {
                Some(CellMerge::HorizontalStart { span }) => {
                    expected_continuations = span.saturating_sub(1) as usize;
                }
                Some(CellMerge::HorizontalContinue) => {
                    if expected_continuations == 0 {
                        row.cells[idx].merge = None;
                        self.report_builder.merge_conflict(
                            "Orphan merge continuation without start - treating as standalone cell",
                        );
                        self.report_builder.dropped_content("merge_semantics", None);
                    } else {
                        expected_continuations -= 1;
                    }
                }
                _ => {
                    expected_continuations = 0;
                }
            }
        }

        // Collect merge info after orphan cleanup to avoid borrow issues.
        let merge_info: Vec<Option<CellMerge>> =
            row.cells.iter().map(|c| c.merge.clone()).collect();

        // Check for span exceeding row bounds.
        for (idx, merge) in merge_info.iter().enumerate() {
            if let Some(CellMerge::HorizontalStart { span }) = merge {
                let span_val = *span;
                let available_cells = row.cells.len() - idx;
                if span_val as usize > available_cells {
                    // Clamp span to available cells
                    row.cells[idx].merge = Some(CellMerge::HorizontalStart {
                        span: available_cells as u16,
                    });
                    self.report_builder.table_geometry_conflict(&format!(
                        "Merge span {} exceeds available cells {} - clamped",
                        span_val, available_cells
                    ));
                    self.report_builder.dropped_content("merge_semantics", None);
                }
            }
        }
    }

    /// Check table limits for a row.
    fn check_table_limits(&self, row: &TableRow) -> Result<(), ParseError> {
        // Check cells per row limit
        if row.cells.len() > self.limits.max_cells_per_row {
            return Err(ParseError::InvalidStructure(format!(
                "Row has {} cells, maximum is {}",
                row.cells.len(),
                self.limits.max_cells_per_row
            )));
        }

        // Check merge spans
        for cell in &row.cells {
            if let Some(CellMerge::HorizontalStart { span }) = cell.merge
                && span > self.limits.max_merge_span
            {
                return Err(ParseError::InvalidStructure(format!(
                    "Merge span {} exceeds maximum {}",
                    span, self.limits.max_merge_span
                )));
            }
        }

        Ok(())
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
        if let Some(mut table) = self.current_table.take()
            && !table.is_empty()
        {
            // Apply table-level shading to TableProps if present
            if let Some(shading) = self.build_shading(
                self.pending_table_cbpat,
                self.pending_table_cfpat,
                self.pending_table_shading,
            ) {
                table.table_props = Some(TableProps {
                    shading: Some(shading),
                });
            }
            self.document.blocks.push(Block::TableBlock(table));
        }

        // Reset table state
        self.current_cell = None;
        self.pending_cellx.clear();
        self.pending_cell_merges.clear();
        self.pending_cell_v_aligns.clear();
        self.seen_intbl_in_paragraph = false;
        // Reset table-level shading state
        self.pending_table_cbpat = None;
        self.pending_table_cfpat = None;
        self.pending_table_shading = None;
    }

    /// Finalize paragraph for table context (routes to current cell instead of document).
    fn finalize_paragraph_for_table(&mut self) {
        self.flush_current_text_as_run();

        // Add paragraph to current cell if it has content
        if !self.current_paragraph.inlines.is_empty() {
            self.current_paragraph.alignment = self.paragraph_alignment;

            // Apply paragraph shading from current style state
            self.current_paragraph.shading = self.build_shading(
                self.current_style.paragraph_cbpat,
                self.current_style.paragraph_cfpat,
                self.current_style.paragraph_shading,
            );

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
                .add_runs(Self::inline_run_count(&self.current_paragraph.inlines));
        }

        self.reset_paragraph_state();
    }

    // =============================================================================
    // Field Parsing Methods (Hyperlinks)
    // =============================================================================

    /// Start parsing a field group.
    fn start_field_parsing(&mut self) {
        if self.parsing_field {
            // Nested fields are currently degraded: preserve visible fldrslt text but
            // do not attempt nested hyperlink semantics.
            self.flush_current_text_as_field_run();
            self.report_builder
                .dropped_content("Nested fields are not supported", None);
            self.nested_fields.push(NestedFieldState {
                field_group_depth: self.current_depth,
                ..Default::default()
            });
            return;
        }

        // Flush any pending text before starting the field
        // This ensures text before the field is preserved
        self.flush_current_text_as_run();

        self.parsing_field = true;
        self.field_group_depth = self.current_depth;
        self.field_instruction_text.clear();
        self.field_result_inlines.clear();
        self.field_style_snapshot = Some(self.current_style.snapshot());
        self.parsing_fldinst = false;
        self.parsing_fldrslt = false;
    }

    /// Handle text content within a field (instruction text or result text).
    fn handle_field_text(&mut self, text: String) {
        enum FieldTextTarget {
            NestedFldInst,
            NestedFldRslt,
            OuterFldInst,
            OuterFldRslt,
            Ignore,
        }

        let target = if let Some(nested) = self.nested_fields.last() {
            if nested.parsing_fldinst {
                FieldTextTarget::NestedFldInst
            } else if nested.parsing_fldrslt {
                FieldTextTarget::NestedFldRslt
            } else {
                FieldTextTarget::Ignore
            }
        } else if self.parsing_fldinst {
            FieldTextTarget::OuterFldInst
        } else if self.parsing_fldrslt {
            FieldTextTarget::OuterFldRslt
        } else {
            FieldTextTarget::Ignore
        };

        match target {
            FieldTextTarget::NestedFldInst => {
                // Ignore nested field instruction text.
            }
            FieldTextTarget::NestedFldRslt | FieldTextTarget::OuterFldRslt => {
                self.handle_field_result_text(text);
            }
            FieldTextTarget::OuterFldInst => {
                self.field_instruction_text.push_str(&text);
            }
            FieldTextTarget::Ignore => {}
        }
    }

    /// Handle text within fldrslt (visible content of the field).
    fn handle_field_result_text(&mut self, text: String) {
        // Skip fallback characters if needed (after \u escape)
        if self.skip_next_chars > 0 {
            let skip = self.skip_next_chars.min(text.chars().count());
            let chars_to_take = text.chars().skip(skip);
            let remaining: String = chars_to_take.collect();
            self.skip_next_chars -= skip;

            if remaining.is_empty() {
                return;
            }

            self.handle_field_result_text_internal(remaining);
        } else {
            self.handle_field_result_text_internal(text);
        }
    }

    /// Internal handler for field result text.
    fn handle_field_result_text_internal(&mut self, text: String) {
        // Check if style has changed
        if self.field_style_changed() {
            // Flush current text as a run if any
            self.flush_current_text_as_field_run();
            self.current_run_style = self.current_style.snapshot();
        }

        // Append text
        self.current_text.push_str(&text);
    }

    /// Check if the current style differs from the run style (for field result).
    fn field_style_changed(&self) -> bool {
        self.character_style_changed()
    }

    fn flush_current_text_as_field_run(&mut self) {
        if !self.current_text.is_empty() {
            let run = self.create_run();
            self.field_result_inlines.push(Inline::Run(run));
            self.current_text.clear();
        }
    }

    fn is_supported_hyperlink_url(url: &str) -> bool {
        let lowered = url.trim().to_ascii_lowercase();
        lowered.starts_with("http://")
            || lowered.starts_with("https://")
            || lowered.starts_with("mailto:")
    }

    /// Finalize the current field and emit the appropriate inline(s).
    fn finalize_field(&mut self) {
        // Flush any remaining text in field result
        if self.parsing_fldrslt && !self.current_text.is_empty() {
            self.flush_current_text_as_field_run();
        }

        let instruction = self.field_instruction_text.trim();
        let has_instruction = !instruction.is_empty();
        let is_hyperlink_instruction = instruction.to_ascii_uppercase().starts_with("HYPERLINK");
        let had_result_content = !self.field_result_inlines.is_empty();
        let parsed_url = if is_hyperlink_instruction {
            self.extract_hyperlink_url()
                .map(|url| url.trim().to_string())
        } else {
            None
        };

        if let Some(url) = parsed_url {
            if Self::is_supported_hyperlink_url(&url) {
                // Emit Inline::Hyperlink
                let runs: Vec<Run> = self
                    .field_result_inlines
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Run(run) => Some(run.clone()),
                        _ => None,
                    })
                    .collect();

                if !runs.is_empty() {
                    let hyperlink = Hyperlink { url, runs };
                    self.current_paragraph
                        .inlines
                        .push(Inline::Hyperlink(hyperlink));
                } else {
                    // Field with no result text - emit DroppedContent
                    self.report_builder
                        .dropped_content("Field with no result text", None);
                }
            } else {
                for inline in self.field_result_inlines.drain(..) {
                    self.current_paragraph.inlines.push(inline);
                }
                self.report_builder
                    .dropped_content("Unsupported hyperlink URL scheme", None);
            }
        } else {
            for inline in self.field_result_inlines.drain(..) {
                self.current_paragraph.inlines.push(inline);
            }

            if is_hyperlink_instruction {
                if had_result_content {
                    self.report_builder
                        .dropped_content("Malformed or unsupported hyperlink URL", None);
                } else {
                    self.report_builder
                        .dropped_content("Field with no result text", None);
                }
            } else if has_instruction {
                self.report_builder
                    .dropped_content("Dropped unsupported field type", None);
            } else if had_result_content {
                self.report_builder
                    .dropped_content("Field with no instruction text", None);
            } else {
                // Field with no instruction and no result
                self.report_builder
                    .dropped_content("Field with no instruction and no result", None);
            }
        }

        // Reset field state
        self.parsing_field = false;
        self.field_group_depth = 0;
        self.parsing_fldinst = false;
        self.fldinst_group_depth = 0;
        self.parsing_fldrslt = false;
        self.fldrslt_group_depth = 0;
        self.field_instruction_text.clear();
        self.field_result_inlines.clear();
        self.nested_fields.clear();

        // Restore style from snapshot
        if let Some(style) = self.field_style_snapshot.take() {
            self.current_run_style = style;
        }
    }

    /// Extract URL from HYPERLINK instruction text.
    /// Pattern: HYPERLINK "url"
    fn extract_hyperlink_url(&self) -> Option<String> {
        let text = self.field_instruction_text.trim();

        // Check if it starts with HYPERLINK
        if !text.to_uppercase().starts_with("HYPERLINK") {
            return None;
        }

        // Find the quoted URL
        // Simple pattern: HYPERLINK "url"
        let rest = &text["HYPERLINK".len()..];
        let rest = rest.trim_start();

        // Look for opening quote
        if !rest.starts_with('"') {
            return None;
        }

        // Find closing quote
        let rest = &rest[1..]; // Skip opening quote
        if let Some(end_quote_pos) = rest.find('"') {
            let url = &rest[..end_quote_pos];
            return Some(url.to_string());
        }

        None
    }

    /// Process field-related events during group end.
    fn process_field_group_end(&mut self) {
        if !self.parsing_field {
            return;
        }

        let (exit_nested_fldinst, exit_nested_fldrslt, exit_nested_field) =
            if let Some(nested) = self.nested_fields.last() {
                (
                    nested.parsing_fldinst && self.current_depth < nested.fldinst_group_depth,
                    nested.parsing_fldrslt && self.current_depth < nested.fldrslt_group_depth,
                    self.current_depth < nested.field_group_depth,
                )
            } else {
                (false, false, false)
            };

        if exit_nested_fldrslt {
            self.flush_current_text_as_field_run();
        }

        if let Some(nested) = self.nested_fields.last_mut() {
            if exit_nested_fldinst {
                nested.parsing_fldinst = false;
            }
            if exit_nested_fldrslt {
                nested.parsing_fldrslt = false;
            }
        }

        if exit_nested_field {
            self.nested_fields.pop();
        }

        // Check if we're exiting fldinst group
        if self.parsing_fldinst && self.current_depth < self.fldinst_group_depth {
            self.parsing_fldinst = false;
        }

        // Check if we're exiting fldrslt group
        if self.parsing_fldrslt && self.current_depth < self.fldrslt_group_depth {
            // Flush remaining text before exiting fldrslt
            self.flush_current_text_as_field_run();
            self.parsing_fldrslt = false;
        }

        // Check if we're exiting the field group itself
        if self.current_depth < self.field_group_depth {
            self.finalize_field();
        }
    }

    fn inline_run_count(inlines: &[Inline]) -> usize {
        inlines
            .iter()
            .map(|inline| match inline {
                Inline::Run(_) => 1,
                Inline::Hyperlink(link) => link.runs.len(),
            })
            .sum()
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
            assert!(
                para.inlines
                    .iter()
                    .any(|i| matches!(i, Inline::Run(r) if r.bold))
            );
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
        input.push('}');

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
            assert_eq!(para.inlines.len(), 1);
            match &para.inlines[0] {
                Inline::Run(r) => assert!(r.bold),
                _ => panic!("Expected Run"),
            }
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
                .inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Run(run) => Some(run.text.as_str()),
                    _ => None,
                })
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
            assert_eq!(para.inlines.len(), 1);
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, "{braced} and \\ slash"),
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_unknown_destination_is_skipped_and_reported() {
        let input = r#"{\rtf1\ansi {\*\foo hidden} shown}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();
        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, " shown"),
                _ => panic!("Expected Run"),
            }
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
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, " Hello"),
                _ => panic!("Expected Run"),
            }
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
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, "Item text"),
                _ => panic!("Expected Run"),
            }
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
            let text: String = para
                .inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Run(r) => Some(r.text.as_str()),
                    _ => None,
                })
                .collect();
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
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, "Item"),
                _ => panic!("Expected Run"),
            }
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
            let text: String = para
                .inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Run(r) => Some(r.text.as_str()),
                    _ => None,
                })
                .collect();
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
                Block::Paragraph(p) => p
                    .inlines
                    .iter()
                    .filter_map(|i| match i {
                        Inline::Run(r) => Some(r.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();
        let second: String = table.rows[0].cells[1]
            .blocks
            .iter()
            .flat_map(|b| match b {
                Block::Paragraph(p) => p
                    .inlines
                    .iter()
                    .filter_map(|i| match i {
                        Inline::Run(r) => Some(r.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
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

    #[test]
    fn test_orphan_merge_continuation_mid_row_is_degraded() {
        let input = r#"{\rtf1\ansi
\trowd\cellx2880\clmrg\cellx5760\cellx8640
\intbl A\cell B\cell C\cell\row
}"#;

        let (doc, report) = Interpreter::parse(input).unwrap();
        let table = match &doc.blocks[0] {
            Block::TableBlock(table) => table,
            _ => panic!("Expected table block"),
        };

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells.len(), 3);
        assert_eq!(table.rows[0].cells[1].merge, None);

        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::MergeConflict { .. }))
        );
        assert!(report.warnings.iter().any(|w| matches!(
            w,
            crate::report::Warning::DroppedContent { reason, .. } if reason == "merge_semantics"
        )));
    }

    #[test]
    fn test_max_cells_per_row_violation_is_hard_failure() {
        let input = r#"{\rtf1\ansi
\trowd\cellx1000\cellx2000
\intbl A\cell B\cell\row
}"#;

        let limits = ParserLimits::new().with_max_cells_per_row(1);
        let result = Interpreter::parse_with_limits(input, limits);

        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::InvalidStructure(_)))
        ));
    }

    #[test]
    fn test_max_rows_per_table_violation_is_hard_failure() {
        let input = r#"{\rtf1\ansi
\trowd\cellx1000\intbl A\cell\row
\trowd\cellx1000\intbl B\cell\row
}"#;

        let limits = ParserLimits::new().with_max_rows_per_table(1);
        let result = Interpreter::parse_with_limits(input, limits);

        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::InvalidStructure(_)))
        ));
    }

    #[test]
    fn test_max_merge_span_violation_is_hard_failure() {
        let input = r#"{\rtf1\ansi
\trowd\clmgf\cellx2880\clmrg\cellx5760
\intbl A\cell\cell\row
}"#;

        let limits = ParserLimits::new().with_max_merge_span(1);
        let result = Interpreter::parse_with_limits(input, limits);

        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::InvalidStructure(_)))
        ));
    }

    // ==========================================================================
    // Hyperlink Field Parsing Tests (Step 3)
    // ==========================================================================

    #[test]
    fn test_hyperlink_field_parses_to_inline_hyperlink() {
        let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt Example Link}}}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.inlines.len(), 1);
            match &para.inlines[0] {
                Inline::Hyperlink(hyperlink) => {
                    assert_eq!(hyperlink.url, "https://example.com");
                    assert_eq!(hyperlink.runs.len(), 1);
                    assert_eq!(hyperlink.runs[0].text, "Example Link");
                }
                _ => panic!("Expected Hyperlink inline"),
            }
        } else {
            panic!("Expected Paragraph block");
        }

        // HYPERLINK fields should NOT emit DroppedContent
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_hyperlink_field_with_formatted_text() {
        let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt \b Bold\b0  Link}}}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Hyperlink(hyperlink) => {
                    assert_eq!(hyperlink.url, "https://example.com");
                    // Should have two runs: "Bold " (bold) and " Link" (not bold)
                    assert_eq!(hyperlink.runs.len(), 2);
                    assert!(hyperlink.runs[0].bold);
                    assert!(!hyperlink.runs[1].bold);
                }
                _ => panic!("Expected Hyperlink inline"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_hyperlink_field_mixed_with_regular_text() {
        let input = r#"{\rtf1\ansi Before {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt Link}} After}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            // Should have 3 inlines: "Before ", Hyperlink, " After"
            assert_eq!(para.inlines.len(), 3);
            match (&para.inlines[0], &para.inlines[1], &para.inlines[2]) {
                (Inline::Run(r1), Inline::Hyperlink(_), Inline::Run(r2)) => {
                    assert_eq!(r1.text, "Before ");
                    assert_eq!(r2.text, " After");
                }
                _ => panic!("Expected Run, Hyperlink, Run pattern"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_non_hyperlink_field_emits_dropped_content() {
        let input = r#"{\rtf1\ansi {\field{\*\fldinst PAGE}{\fldrslt 1}}}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            // Non-HYPERLINK fields should emit result as plain runs
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, "1"),
                _ => panic!("Expected Run inline"),
            }
        } else {
            panic!("Expected Paragraph block");
        }

        // Should emit DroppedContent for unsupported field type
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_field_with_no_result_emits_dropped_content() {
        let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://example.com"}}}"#;
        let (_doc, report) = Interpreter::parse(input).unwrap();

        // Field with no result text should emit DroppedContent
        assert!(
            report
                .warnings
                .iter()
                .any(|w| matches!(w, crate::report::Warning::DroppedContent { .. }))
        );
    }

    #[test]
    fn test_extract_hyperlink_url_basic() {
        let input =
            r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://test.com/path"}{\fldrslt Text}}}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Hyperlink(h) => assert_eq!(h.url, "https://test.com/path"),
                _ => panic!("Expected Hyperlink"),
            }
        }
    }

    #[test]
    fn test_hyperlink_case_insensitive() {
        let input =
            r#"{\rtf1\ansi {\field{\*\fldinst hyperlink "https://example.com"}{\fldrslt Link}}}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Hyperlink(h) => assert_eq!(h.url, "https://example.com"),
                _ => panic!("Expected Hyperlink"),
            }
        }
    }

    #[test]
    fn test_hyperlink_with_unsupported_scheme_falls_back_to_plain_text() {
        let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "ftp://example.com/file"}{\fldrslt ftp link}}}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.inlines.len(), 1);
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, "ftp link"),
                _ => panic!("Expected fallback run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }

        assert!(report.warnings.iter().any(|w| {
            matches!(
                w,
                crate::report::Warning::DroppedContent { reason, .. }
                if reason == "Unsupported hyperlink URL scheme"
            )
        }));
    }

    #[test]
    fn test_hyperlink_with_leading_space_javascript_scheme_is_rejected() {
        let input =
            r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK " javascript:alert(1)"}{\fldrslt click}}}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.inlines.len(), 1);
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, "click"),
                _ => panic!("Expected fallback run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }

        assert!(report.warnings.iter().any(|w| {
            matches!(
                w,
                crate::report::Warning::DroppedContent { reason, .. }
                if reason == "Unsupported hyperlink URL scheme"
            )
        }));
    }

    #[test]
    fn test_field_with_result_but_no_instruction_emits_warning_and_preserves_text() {
        let input = r#"{\rtf1\ansi {\field{\fldrslt Visible}}}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.inlines.len(), 1);
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.text, "Visible"),
                _ => panic!("Expected fallback run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }

        assert!(report.warnings.iter().any(|w| {
            matches!(
                w,
                crate::report::Warning::DroppedContent { reason, .. }
                if reason == "Field with no instruction text"
            )
        }));
    }

    #[test]
    fn test_nested_fields_degrade_inner_semantics_without_losing_outer_hyperlink() {
        let input = r#"{\rtf1\ansi {\field{\*\fldinst HYPERLINK "https://outer.example"}{\fldrslt Outer {\field{\*\fldinst HYPERLINK "https://inner.example"}{\fldrslt Inner}} Tail}}}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.inlines.len(), 1);
            match &para.inlines[0] {
                Inline::Hyperlink(link) => {
                    assert_eq!(link.url, "https://outer.example");
                    let text: String = link.runs.iter().map(|r| r.text.as_str()).collect();
                    assert_eq!(text, "Outer Inner Tail");
                }
                _ => panic!("Expected outer hyperlink"),
            }
        } else {
            panic!("Expected Paragraph block");
        }

        assert!(report.warnings.iter().any(|w| {
            matches!(
                w,
                crate::report::Warning::DroppedContent { reason, .. }
                if reason == "Nested fields are not supported"
            )
        }));
    }

    // ==========================================================================
    // Font Table Parsing Tests (PR2)
    // ==========================================================================

    #[test]
    fn test_fonttbl_single_font() {
        // Need \deff0 to set default font, or \f0 to set current font
        let input = r#"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}Hello}"#;
        let (doc, report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Hello");
                    assert_eq!(r.font_family, Some("Arial".to_string()));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_fonttbl_multiple_fonts() {
        let input = r#"{\rtf1\ansi{\fonttbl{\f0 Arial;}{\f1 Times New Roman;}{\f2 Courier New;}}\f0 Arial \f1 Times \f2 Courier}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.inlines.len(), 3);
            match (&para.inlines[0], &para.inlines[1], &para.inlines[2]) {
                (Inline::Run(r1), Inline::Run(r2), Inline::Run(r3)) => {
                    assert_eq!(r1.font_family, Some("Arial".to_string()));
                    assert_eq!(r2.font_family, Some("Times New Roman".to_string()));
                    assert_eq!(r3.font_family, Some("Courier New".to_string()));
                }
                _ => panic!("Expected three Runs"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fonttbl_with_font_properties() {
        // Font entries often include font family and charset info
        let input = r#"{\rtf1\ansi{\fonttbl{\f0\fnil\fcharset0 Arial;}{\f1\fswiss\fcharset0 Helvetica;}}\f0 Test}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.font_family, Some("Arial".to_string()));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fonttbl_empty() {
        let input = r#"{\rtf1\ansi{\fonttbl}Hello}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Hello");
                    // No font family should be set
                    assert_eq!(r.font_family, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fonttbl_malformed_no_font_index() {
        // Font entry without \fN - should be ignored
        let input = r#"{\rtf1\ansi{\fonttbl{Arial;}}Hello}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Hello");
                    // Font should not be registered
                    assert_eq!(r.font_family, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fonttbl_malformed_no_font_name() {
        // Font entry with index but no name - should be ignored
        let input = r#"{\rtf1\ansi{\fonttbl{\f0;}}Hello}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Hello");
                    // Font should not be registered (empty name)
                    assert_eq!(r.font_family, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_deff_default_font() {
        // \deffN sets the default font index
        let input = r#"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}Hello}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Hello");
                    // Should use default font
                    assert_eq!(r.font_family, Some("Arial".to_string()));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fonttbl_redefinition_uses_latest_table() {
        let input = r#"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\fonttbl{\f0 Courier New;}}Hello}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.font_family, Some("Courier New".to_string()));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fs_zero_degrades_to_none() {
        let input = r#"{\rtf1\ansi\fs0 Zero}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.font_size, None),
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fs_negative_degrades_to_none() {
        let input = r#"{\rtf1\ansi\fs-8 Invalid}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => assert_eq!(r.font_size, None),
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    // ==========================================================================
    // Color Table Parsing Tests (PR2)
    // ==========================================================================

    #[test]
    fn test_colortbl_single_color() {
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cf1 Red Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Red Text");
                    assert_eq!(r.color, Some(crate::Color { r: 255, g: 0, b: 0 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_colortbl_multiple_colors() {
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;\red0\green255\blue0;\red0\green0\blue255;}\cf1 Red \cf2 Green \cf3 Blue}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.inlines.len(), 3);
            match (&para.inlines[0], &para.inlines[1], &para.inlines[2]) {
                (Inline::Run(r1), Inline::Run(r2), Inline::Run(r3)) => {
                    assert_eq!(r1.color, Some(crate::Color { r: 255, g: 0, b: 0 }));
                    assert_eq!(r2.color, Some(crate::Color { r: 0, g: 255, b: 0 }));
                    assert_eq!(r3.color, Some(crate::Color { r: 0, g: 0, b: 255 }));
                }
                _ => panic!("Expected three Runs"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_colortbl_cf0_is_auto() {
        // \cf0 means auto/default color (None)
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cf0 Auto Color}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Auto Color");
                    assert_eq!(r.color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_colortbl_empty() {
        let input = r#"{\rtf1\ansi{\colortbl}Hello}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Hello");
                    assert_eq!(r.color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_colortbl_rgb_clamping() {
        // RGB values should be clamped to 0-255
        let input = r#"{\rtf1\ansi{\colortbl;\red300\green-10\blue128;}\cf1 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(
                        r.color,
                        Some(crate::Color {
                            r: 255,
                            g: 0,
                            b: 128
                        })
                    );
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_colortbl_incomplete_color() {
        // Color with missing components - should use 0 for missing
        let input = r#"{\rtf1\ansi{\colortbl;\red255;}\cf1 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    // Missing green and blue should be 0
                    assert_eq!(r.color, Some(crate::Color { r: 255, g: 0, b: 0 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_colortbl_unresolved_index() {
        // Referencing a color index that doesn't exist
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cf99 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Text");
                    // Unresolved index should degrade to None
                    assert_eq!(r.color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_colortbl_redefinition_uses_latest_table() {
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}{\colortbl;\red0\green0\blue255;}\cf1 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.color, Some(crate::Color { r: 0, g: 0, b: 255 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_fonttbl_and_colortbl_combined() {
        let input = r#"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\colortbl;\red255\green0\blue0;}\cf1 Styled Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Styled Text");
                    assert_eq!(r.font_family, Some("Arial".to_string()));
                    assert_eq!(r.color, Some(crate::Color { r: 255, g: 0, b: 0 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    // ==========================================================================
    // Background/Highlight Color Tests (Slice 1)
    // ==========================================================================

    #[test]
    fn test_highlight_maps_to_background_color() {
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green255\blue0;}\highlight1 Highlighted Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Highlighted Text");
                    assert_eq!(r.background_color, Some(crate::Color { r: 255, g: 255, b: 0 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_cb_maps_to_background_color() {
        let input = r#"{\rtf1\ansi{\colortbl;\red0\green255\blue255;}\cb1 Background Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Background Text");
                    assert_eq!(r.background_color, Some(crate::Color { r: 0, g: 255, b: 255 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_highlight_takes_precedence_over_cb() {
        // When both highlight and cb are set, highlight should take precedence
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;\red0\green255\blue0;}\highlight1\cb2 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Text");
                    // Should use highlight color (red), not cb color (green)
                    assert_eq!(r.background_color, Some(crate::Color { r: 255, g: 0, b: 0 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_highlight_zero_results_in_none() {
        // \highlight0 means auto/default (None)
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\highlight0 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Text");
                    assert_eq!(r.background_color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_cb_zero_results_in_none() {
        // \cb0 means auto/default (None)
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cb0 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Text");
                    assert_eq!(r.background_color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_unresolved_highlight_index_degrades_to_none() {
        // Referencing a highlight index that doesn't exist should degrade gracefully
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\highlight99 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Text");
                    // Unresolved index should degrade to None
                    assert_eq!(r.background_color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_unresolved_cb_index_degrades_to_none() {
        // Referencing a cb index that doesn't exist should degrade gracefully
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cb99 Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Text");
                    // Unresolved index should degrade to None
                    assert_eq!(r.background_color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_plain_clears_background_highlight_but_keeps_alignment() {
        // \plain should reset character formatting including background/highlight
        // but keep paragraph alignment
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\qc\highlight1 Styled\plain  Plain}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            // Paragraph alignment should be preserved
            assert_eq!(para.alignment, Alignment::Center);
            
            // First run should have highlight
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Styled");
                    assert_eq!(r.background_color, Some(crate::Color { r: 255, g: 0, b: 0 }));
                }
                _ => panic!("Expected Run"),
            }
            
            // Second run should have no background (plain reset it)
            match &para.inlines[1] {
                Inline::Run(r) => {
                    assert_eq!(r.text, " Plain");
                    assert_eq!(r.background_color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_plain_clears_cb_but_keeps_alignment() {
        // \plain should reset character formatting including cb
        let input = r#"{\rtf1\ansi{\colortbl;\red0\green255\blue0;}\qr\cb1 Styled\plain  Plain}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            // Paragraph alignment should be preserved
            assert_eq!(para.alignment, Alignment::Right);
            
            // First run should have background
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Styled");
                    assert_eq!(r.background_color, Some(crate::Color { r: 0, g: 255, b: 0 }));
                }
                _ => panic!("Expected Run"),
            }
            
            // Second run should have no background (plain reset it)
            match &para.inlines[1] {
                Inline::Run(r) => {
                    assert_eq!(r.text, " Plain");
                    assert_eq!(r.background_color, None);
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_hyperlink_preserves_background_color() {
        // Hyperlink field runs should preserve background color
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green255\blue0;}{\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt \highlight1 Link}}}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Hyperlink(hyperlink) => {
                    assert_eq!(hyperlink.url, "https://example.com");
                    assert_eq!(hyperlink.runs.len(), 1);
                    assert_eq!(hyperlink.runs[0].text, "Link");
                    assert_eq!(hyperlink.runs[0].background_color, Some(crate::Color { r: 255, g: 255, b: 0 }));
                }
                _ => panic!("Expected Hyperlink"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_hyperlink_with_cb_preserves_background_color() {
        // Hyperlink field runs should preserve cb background color
        let input = r#"{\rtf1\ansi{\colortbl;\red0\green0\blue255;}{\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt \cb1 Link}}}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Hyperlink(hyperlink) => {
                    assert_eq!(hyperlink.url, "https://example.com");
                    assert_eq!(hyperlink.runs.len(), 1);
                    assert_eq!(hyperlink.runs[0].text, "Link");
                    assert_eq!(hyperlink.runs[0].background_color, Some(crate::Color { r: 0, g: 0, b: 255 }));
                }
                _ => panic!("Expected Hyperlink"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_background_color_with_foreground_color() {
        // Both foreground and background colors should work together
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;\red0\green0\blue255;}\cf1\highlight2 Colored Text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            match &para.inlines[0] {
                Inline::Run(r) => {
                    assert_eq!(r.text, "Colored Text");
                    assert_eq!(r.color, Some(crate::Color { r: 255, g: 0, b: 0 }));
                    assert_eq!(r.background_color, Some(crate::Color { r: 0, g: 0, b: 255 }));
                }
                _ => panic!("Expected Run"),
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_plain_keeps_list_state() {
        // \plain should not reset list state
        let input = r#"{\rtf1\ansi {\listtable{\list\listid1{\listlevel\levelnfc23}}}{\listoverridetable{\listoverride\listid1\ls1}}\ls1\highlight1 Item\plain text}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // Should still be a list block (not paragraph)
        if let Block::ListBlock(list) = &doc.blocks[0] {
            assert_eq!(list.items.len(), 1);
            // Check that the item has content
            if let Block::Paragraph(para) = &list.items[0].blocks[0] {
                // First run has highlight, second doesn't (plain reset it)
                assert_eq!(para.inlines.len(), 2);
            } else {
                panic!("Expected Paragraph in list item");
            }
        } else {
            panic!("Expected ListBlock");
        }
    }

    // ==========================================================================
    // Paragraph Shading Tests (Phase 5 - Shading)
    // ==========================================================================

    #[test]
    fn test_paragraph_fill_from_cbpat() {
        // \cbpatN sets paragraph background color
        // Note: \par is needed to finalize the paragraph inside the group where shading is active
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cbpat1 Shaded paragraph\par}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert!(para.shading.is_some());
            let shading = para.shading.as_ref().unwrap();
            assert_eq!(shading.fill_color, Some(crate::Color { r: 255, g: 0, b: 0 }));
            assert_eq!(shading.pattern_color, None); // For Slice B
            assert_eq!(shading.pattern, None); // For Slice B
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_paragraph_shading_unresolved_index_degrades_to_none() {
        // \cbpatN with invalid index should degrade to None
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cbpat99 Shaded paragraph\par}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            // Invalid color index should result in no shading
            assert!(para.shading.is_none());
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_pard_resets_paragraph_shading() {
        // \pard should reset paragraph shading
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cbpat1 First\par\pard Second\par}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        // First paragraph should have shading
        if let Block::Paragraph(para) = &doc.blocks[0] {
            assert!(para.shading.is_some());
        } else {
            panic!("Expected first block to be Paragraph");
        }

        // Second paragraph (after \pard) should NOT have shading
        if let Block::Paragraph(para) = &doc.blocks[1] {
            assert!(para.shading.is_none());
        } else {
            panic!("Expected second block to be Paragraph");
        }
    }

    #[test]
    fn test_plain_does_not_reset_paragraph_shading() {
        // \plain should NOT reset paragraph shading (character-only reset)
        // Need to finalize paragraph inside the group where shading is active
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}\cbpat1\b Bold\plain  normal\par}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::Paragraph(para) = &doc.blocks[0] {
            // Paragraph should still have shading after \plain
            assert!(para.shading.is_some());
            let shading = para.shading.as_ref().unwrap();
            assert_eq!(shading.fill_color, Some(crate::Color { r: 255, g: 0, b: 0 }));
        } else {
            panic!("Expected Paragraph block");
        }
    }

    // ==========================================================================
    // Cell Shading Tests (Phase 5 - Shading)
    // ==========================================================================

    #[test]
    fn test_cell_fill_from_clcbpat() {
        // \clcbpatN sets cell background color
        let input = r#"{\rtf1\ansi{\colortbl;\red0\green255\blue0;}
\trowd\clcbpat1\cellx2880\cellx5760
\intbl Cell 1\cell Cell 2\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            assert_eq!(table.rows.len(), 1);
            assert_eq!(table.rows[0].cells.len(), 2);

            // First cell should have shading
            let cell1 = &table.rows[0].cells[0];
            assert!(cell1.shading.is_some());
            let shading1 = cell1.shading.as_ref().unwrap();
            assert_eq!(shading1.fill_color, Some(crate::Color { r: 0, g: 255, b: 0 }));

            // Second cell should NOT have shading (no \clcbpat before its \cellx)
            let cell2 = &table.rows[0].cells[1];
            assert!(cell2.shading.is_none());
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_cell_shading_unresolved_index_degrades_to_none() {
        // \clcbpatN with invalid index should degrade to None
        let input = r#"{\rtf1\ansi{\colortbl;\red0\green255\blue0;}
\trowd\clcbpat99\cellx2880
\intbl Cell\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            let cell = &table.rows[0].cells[0];
            // Invalid color index should result in no shading
            assert!(cell.shading.is_none());
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_cell_shading_per_cell_independent() {
        // Each cell can have its own shading
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;\red0\green0\blue255;}
\trowd\clcbpat1\cellx1440\clcbpat2\cellx2880\cellx4320
\intbl Red\cell Blue\cell None\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // First cell: red shading
            let cell1 = &table.rows[0].cells[0];
            assert!(cell1.shading.is_some());
            assert_eq!(cell1.shading.as_ref().unwrap().fill_color, Some(crate::Color { r: 255, g: 0, b: 0 }));

            // Second cell: blue shading
            let cell2 = &table.rows[0].cells[1];
            assert!(cell2.shading.is_some());
            assert_eq!(cell2.shading.as_ref().unwrap().fill_color, Some(crate::Color { r: 0, g: 0, b: 255 }));

            // Third cell: no shading
            let cell3 = &table.rows[0].cells[2];
            assert!(cell3.shading.is_none());
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_cell_shading_resets_on_new_row() {
        // Cell shading should reset at new row definition (\trowd)
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}
\trowd\clcbpat1\cellx2880
\intbl Shaded\cell\row
\trowd\cellx2880
\intbl Not shaded\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // First row: cell has shading
            let cell1 = &table.rows[0].cells[0];
            assert!(cell1.shading.is_some());

            // Second row: cell has NO shading (reset by \trowd)
            let cell2 = &table.rows[1].cells[0];
            assert!(cell2.shading.is_none());
        } else {
            panic!("Expected TableBlock");
        }
    }

    // ==========================================================================
    // Row/Table Shading Fallback Tests (Phase 5 - Step 4)
    // ==========================================================================

    #[test]
    fn test_cell_shading_ignores_row_table_fallback() {
        // Cell with explicit shading ignores row/table fallback
        // Precedence: cell > row > table
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;\red0\green255\blue0;\red0\green0\blue255;}
\trowd\trcbpat2\clcbpat1\cellx2880\cellx5760
\intbl Cell\cell NoShading\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // First cell: explicit cell shading (red) should take precedence
            let cell1 = &table.rows[0].cells[0];
            assert!(cell1.shading.is_some());
            assert_eq!(
                cell1.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 255, g: 0, b: 0 })
            );

            // Second cell: no explicit shading, should fall back to row shading (green)
            let cell2 = &table.rows[0].cells[1];
            assert!(cell2.shading.is_some());
            assert_eq!(
                cell2.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 0, g: 255, b: 0 })
            );
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_cell_without_shading_uses_row_shading() {
        // Cell without shading uses row shading when available
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}
\trowd\trcbpat1\cellx2880\cellx5760
\intbl Cell 1\cell Cell 2\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // Both cells should inherit row shading
            for cell in &table.rows[0].cells {
                assert!(cell.shading.is_some());
                assert_eq!(
                    cell.shading.as_ref().unwrap().fill_color,
                    Some(crate::Color { r: 255, g: 0, b: 0 })
                );
            }
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_cell_without_shading_uses_table_shading() {
        // Cell without shading uses table shading when row shading unavailable
        // \trcbpat at first row sets table-level shading
        let input = r#"{\rtf1\ansi{\colortbl;\red0\green0\blue255;}
\trowd\trcbpat1\cellx2880
\intbl Cell 1\cell\row
\trowd\cellx2880
\intbl Cell 2\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // First row: row shading is set (blue)
            let cell1 = &table.rows[0].cells[0];
            assert!(cell1.shading.is_some());
            assert_eq!(
                cell1.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 0, g: 0, b: 255 })
            );

            // Second row: no row shading, should fall back to table shading (blue)
            let cell2 = &table.rows[1].cells[0];
            assert!(cell2.shading.is_some());
            assert_eq!(
                cell2.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 0, g: 0, b: 255 })
            );
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_later_row_shading_overrides_table_shading() {
        // First row establishes table shading (blue). Later row shading (red)
        // must win for cells in that row.
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;\red0\green0\blue255;}
\trowd\trcbpat2\cellx2880\cellx5760
\intbl A1\cell A2\cell\row
\trowd\trcbpat1\cellx2880\cellx5760
\intbl B1\cell B2\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // First row uses blue (also the table fallback color).
            for cell in &table.rows[0].cells {
                assert_eq!(
                    cell.shading.as_ref().unwrap().fill_color,
                    Some(crate::Color { r: 0, g: 0, b: 255 })
                );
            }

            // Second row must use explicit row shading (red), not table fallback (blue).
            for cell in &table.rows[1].cells {
                assert_eq!(
                    cell.shading.as_ref().unwrap().fill_color,
                    Some(crate::Color { r: 255, g: 0, b: 0 })
                );
            }
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_cell_shading_precedence_cell_row_table() {
        // Full precedence test: cell > row > table
        // \clcbpat must come BEFORE the \cellx that defines the cell boundary
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;\red0\green255\blue0;\red0\green0\blue255;}
\trowd\trcbpat3\clcbpat1\cellx1440\cellx2880\cellx4320
\intbl Explicit\cell RowOnly\cell TableOnly\cell\row
\trowd\cellx1440\cellx2880\cellx4320
\intbl A\cell B\cell C\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // Row 1
            // Cell 1: explicit cell shading (red) - should override row/table
            let cell1 = &table.rows[0].cells[0];
            assert_eq!(
                cell1.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 255, g: 0, b: 0 })
            );

            // Cell 2: no explicit cell shading, uses row shading (blue from trcbpat3)
            let cell2 = &table.rows[0].cells[1];
            assert_eq!(
                cell2.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 0, g: 0, b: 255 })
            );

            // Cell 3: no explicit cell shading, uses row shading (blue)
            let cell3 = &table.rows[0].cells[2];
            assert_eq!(
                cell3.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 0, g: 0, b: 255 })
            );

            // Row 2: no row shading, should fall back to table shading (blue)
            for cell in &table.rows[1].cells {
                assert_eq!(
                    cell.shading.as_ref().unwrap().fill_color,
                    Some(crate::Color { r: 0, g: 0, b: 255 })
                );
            }
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_row_shading_applied_to_row_props() {
        // Row shading should be stored in RowProps for writers
        let input = r#"{\rtf1\ansi{\colortbl;\red255\green0\blue0;}
\trowd\trcbpat1\cellx2880
\intbl Cell\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // Row should have row_props with shading
            let row = &table.rows[0];
            assert!(row.row_props.is_some());
            let row_props = row.row_props.as_ref().unwrap();
            assert!(row_props.shading.is_some());
            assert_eq!(
                row_props.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 255, g: 0, b: 0 })
            );
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_table_shading_applied_to_table_props() {
        // Table shading should be stored in TableProps for writers
        let input = r#"{\rtf1\ansi{\colortbl;\red0\green0\blue255;}
\trowd\trcbpat1\cellx2880
\intbl Cell\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            // Table should have table_props with shading
            assert!(table.table_props.is_some());
            let table_props = table.table_props.as_ref().unwrap();
            assert!(table_props.shading.is_some());
            assert_eq!(
                table_props.shading.as_ref().unwrap().fill_color,
                Some(crate::Color { r: 0, g: 0, b: 255 })
            );
        } else {
            panic!("Expected TableBlock");
        }
    }

    #[test]
    fn test_no_shading_when_no_fallback_available() {
        // Cell should have no shading when no cell/row/table shading is defined
        let input = r#"{\rtf1\ansi
\trowd\cellx2880
\intbl Cell\cell\row
}"#;
        let (doc, _report) = Interpreter::parse(input).unwrap();

        if let Block::TableBlock(table) = &doc.blocks[0] {
            let cell = &table.rows[0].cells[0];
            assert!(cell.shading.is_none());
        } else {
            panic!("Expected TableBlock");
        }
    }
}
