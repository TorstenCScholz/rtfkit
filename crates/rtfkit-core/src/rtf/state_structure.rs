//! Structure State Module
//!
//! Tracks the current block-content sink (body, header, footer, or note),
//! accumulates blocks into per-channel buffers, and assembles the final
//! `DocumentStructure` at the end of parsing.

use crate::{Alignment, Block, DocumentStructure, HeaderFooterSet, Note, NoteKind, Paragraph};

// =============================================================================
// Sink Kind Enums
// =============================================================================

/// Which page-variant of header we are inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderKind {
    /// Default / odd-page header (`\header`, `\headerr`).
    Default,
    /// First-page-only header (`\headerf`).
    First,
    /// Even-page header (`\headerl`).
    Even,
}

/// Which page-variant of footer we are inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterKind {
    /// Default / odd-page footer (`\footer`, `\footerr`).
    Default,
    /// First-page-only footer (`\footerf`).
    First,
    /// Even-page footer (`\footerl`).
    Even,
}

/// Where blocks produced by `finalize_paragraph` / `finalize_current_table`
/// should be directed.
#[derive(Debug, Clone)]
pub enum StructureSink {
    /// Normal document body (default).
    Body,
    /// Inside a `\header` / `\headerr` / `\headerf` / `\headerl` group.
    Header(HeaderKind),
    /// Inside a `\footer` / `\footerr` / `\footerf` / `\footerl` group.
    Footer(FooterKind),
    /// Inside a `\footnote` or `\endnote` group.
    Note { id: u32, kind: NoteKind },
}

// =============================================================================
// StructureState
// =============================================================================

/// All state needed to redirect block content into structure channels.
pub struct StructureState {
    /// Current block-content destination.
    pub sink: StructureSink,
    /// RTF group depth at which the current sink was entered.
    pub sink_group_depth: usize,
    /// Stack of outer sinks for nested structure destinations.
    pub sink_stack: Vec<(StructureSink, usize)>,

    // Header channel buffers
    pub header_default_blocks: Vec<Block>,
    pub header_first_blocks: Vec<Block>,
    pub header_even_blocks: Vec<Block>,

    // Footer channel buffers
    pub footer_default_blocks: Vec<Block>,
    pub footer_first_blocks: Vec<Block>,
    pub footer_even_blocks: Vec<Block>,

    /// Accumulates blocks for the note currently being parsed.
    pub note_blocks: Vec<Block>,
    /// Saved note block buffers for nested note destinations.
    pub note_blocks_stack: Vec<Vec<Block>>,
    /// Completed notes (footnotes and endnotes), in document order.
    pub completed_notes: Vec<Note>,

    /// Monotonically increasing note ID counter (1-based).
    pub next_note_id: u32,

    /// Saved body paragraph state while parsing inline note content.
    ///
    /// Footnotes and endnotes appear inline in the body paragraph (e.g.,
    /// `Text{\footnote Note}more text`).  When the note group opens, we save
    /// the in-progress body paragraph here; when it closes, we restore it so
    /// "more text" continues to accrete into the same body paragraph.
    ///
    /// Each entry is `(saved_paragraph, saved_current_text, saved_paragraph_alignment)`.
    pub saved_paragraph_stack: Vec<(Paragraph, String, Alignment)>,
}

impl Default for StructureState {
    fn default() -> Self {
        Self {
            sink: StructureSink::Body,
            sink_group_depth: 0,
            sink_stack: Vec::new(),
            header_default_blocks: Vec::new(),
            header_first_blocks: Vec::new(),
            header_even_blocks: Vec::new(),
            footer_default_blocks: Vec::new(),
            footer_first_blocks: Vec::new(),
            footer_even_blocks: Vec::new(),
            note_blocks: Vec::new(),
            note_blocks_stack: Vec::new(),
            completed_notes: Vec::new(),
            next_note_id: 1,
            saved_paragraph_stack: Vec::new(),
        }
    }
}

impl StructureState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enter a new sink, preserving the current sink on the stack.
    fn enter_sink(&mut self, sink: StructureSink, depth: usize) {
        self.sink_stack
            .push((self.sink.clone(), self.sink_group_depth));
        self.sink = sink;
        self.sink_group_depth = depth;
    }

    // =========================================================================
    // Sink Queries
    // =========================================================================

    /// Returns `true` when the current sink is the document body.
    pub fn in_body(&self) -> bool {
        matches!(self.sink, StructureSink::Body)
    }

    // =========================================================================
    // Sink Transitions
    // =========================================================================

    /// Switch the sink to a header channel and record the group depth.
    pub fn enter_header(&mut self, kind: HeaderKind, depth: usize) {
        self.enter_sink(StructureSink::Header(kind), depth);
    }

    /// Switch the sink to a footer channel and record the group depth.
    pub fn enter_footer(&mut self, kind: FooterKind, depth: usize) {
        self.enter_sink(StructureSink::Footer(kind), depth);
    }

    /// Switch the sink to a note channel, allocate an ID, and record the depth.
    ///
    /// Returns the newly allocated note ID.
    pub fn enter_note(&mut self, kind: NoteKind, depth: usize) -> u32 {
        let id = self.next_note_id;
        self.next_note_id += 1;

        if matches!(self.sink, StructureSink::Note { .. }) {
            self.note_blocks_stack
                .push(std::mem::take(&mut self.note_blocks));
        } else {
            self.note_blocks.clear();
        }

        self.enter_sink(StructureSink::Note { id, kind }, depth);
        id
    }

    /// If `current_depth < sink_group_depth`, the sink group has been exited.
    ///
    /// In that case, moves any accumulated note body into `completed_notes`,
    /// resets the sink to `Body`, and returns `true`.  Returns `false` when
    /// the depth condition is not met (still inside the sink group).
    pub fn exit_sink_if_at_depth(&mut self, current_depth: usize) -> bool {
        if current_depth < self.sink_group_depth {
            // Extract the old sink and immediately replace with Body to
            // avoid any borrow-checker conflicts with the field mutations below.
            let old_sink = std::mem::replace(&mut self.sink, StructureSink::Body);
            if let StructureSink::Note { id, kind } = old_sink {
                let blocks = std::mem::take(&mut self.note_blocks);
                if !blocks.is_empty() {
                    self.completed_notes.push(Note { id, kind, blocks });
                }
            }

            if let Some((previous_sink, previous_depth)) = self.sink_stack.pop() {
                self.sink = previous_sink;
                self.sink_group_depth = previous_depth;
            } else {
                self.sink = StructureSink::Body;
                self.sink_group_depth = 0;
            }

            if matches!(self.sink, StructureSink::Note { .. }) {
                self.note_blocks = self.note_blocks_stack.pop().unwrap_or_default();
            } else {
                self.note_blocks.clear();
            }
            true
        } else {
            false
        }
    }

    // =========================================================================
    // Block Routing
    // =========================================================================

    /// Returns a compact discriminant for the current sink (avoids borrow issues
    /// when we need to both read the sink kind and mutate a block buffer).
    fn sink_discriminant(&self) -> u8 {
        match self.sink {
            StructureSink::Body => 0,
            StructureSink::Header(HeaderKind::Default) => 1,
            StructureSink::Header(HeaderKind::First) => 2,
            StructureSink::Header(HeaderKind::Even) => 3,
            StructureSink::Footer(FooterKind::Default) => 4,
            StructureSink::Footer(FooterKind::First) => 5,
            StructureSink::Footer(FooterKind::Even) => 6,
            StructureSink::Note { .. } => 7,
        }
    }

    /// Push a block into the current structure sink buffer.
    ///
    /// Panics (debug) / is a no-op (release) when called while the sink is
    /// `Body` — callers should check `in_body()` first.
    pub fn push_block_to_sink(&mut self, block: Block) {
        match self.sink_discriminant() {
            1 => self.header_default_blocks.push(block),
            2 => self.header_first_blocks.push(block),
            3 => self.header_even_blocks.push(block),
            4 => self.footer_default_blocks.push(block),
            5 => self.footer_first_blocks.push(block),
            6 => self.footer_even_blocks.push(block),
            7 => self.note_blocks.push(block),
            _ => {
                debug_assert!(false, "push_block_to_sink called while in Body sink");
            }
        }
    }

    // =========================================================================
    // Finalization
    // =========================================================================

    /// Pop the most recently saved body paragraph state.
    ///
    /// Returns `None` if the stack is empty (unexpected in well-formed RTF).
    pub fn pop_saved_paragraph(&mut self) -> Option<(Paragraph, String, Alignment)> {
        self.saved_paragraph_stack.pop()
    }

    // =========================================================================
    // Finalization
    // =========================================================================

    /// Assemble a `DocumentStructure` from accumulated buffers.
    ///
    /// Returns `None` if all buffers are empty (no structure content was found).
    pub fn take_structure(&mut self) -> Option<DocumentStructure> {
        let has_headers = !self.header_default_blocks.is_empty()
            || !self.header_first_blocks.is_empty()
            || !self.header_even_blocks.is_empty();
        let has_footers = !self.footer_default_blocks.is_empty()
            || !self.footer_first_blocks.is_empty()
            || !self.footer_even_blocks.is_empty();
        let has_notes = !self.completed_notes.is_empty();

        if !has_headers && !has_footers && !has_notes {
            return None;
        }

        Some(DocumentStructure {
            headers: HeaderFooterSet {
                default: std::mem::take(&mut self.header_default_blocks),
                first: std::mem::take(&mut self.header_first_blocks),
                even: std::mem::take(&mut self.header_even_blocks),
            },
            footers: HeaderFooterSet {
                default: std::mem::take(&mut self.footer_default_blocks),
                first: std::mem::take(&mut self.footer_first_blocks),
                even: std::mem::take(&mut self.footer_even_blocks),
            },
            notes: std::mem::take(&mut self.completed_notes),
        })
    }
}
