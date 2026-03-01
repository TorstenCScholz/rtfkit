//! RTF Parser Module
//!
//! This module provides a modular RTF parsing implementation with:
//! - Tokenization of RTF input
//! - Event-based processing
//! - Document building
//!
//! # Architecture
//!
//! The module is organized into the following components:
//! - `tokenizer`: Token model and nom-based tokenization
//! - `events`: Event model and token-to-event conversion
//! - `state`: Runtime state container
//! - `state_style`: Style state and run management
//! - `state_destinations`: Destination skip state
//! - `state_lists`: List parsing and resolution state
//! - `state_tables`: Table, merge, and shading state
//! - `state_fields`: Field and hyperlink state
//! - `state_resources`: Font and color table state
//! - `handlers_control_words`: Control word dispatch
//! - `handlers_destinations`: Destination handling and skip-state processing
//! - `handlers_text`: Text/run handling
//! - `handlers_lists`: List controls and list-table destination parsing
//! - `handlers_tables`: Table controls and row/cell event handling
//! - `handlers_fields`: Field control/text/group handling
//! - `handlers_resources`: Font/color controls and resource destinations
//! - `finalize/runs`: Shared run construction/flush helpers
//! - `finalize/shading`: Shading conversion and builder helpers
//! - `finalize/lists`: List reference resolution and list-block insertion
//! - `finalize/paragraphs`: Paragraph finalization for document/table-cell contexts
//! - `finalize/tables`: Cell/row/table finalization and merge normalization
//! - `finalize/document`: End-of-document finalization
//! - `pipeline`: Parsing orchestration
//! - `api`: Public entrypoints

// Public API
pub use api::{RtfParser, parse, parse_with_limits};

// Internal modules - Phase 1
mod events;
mod tokenizer;

// Internal modules - Phase 2 (state)
mod state;
mod state_destinations;
mod state_fields;
mod state_images;
mod state_lists;
mod state_resources;
mod state_structure;
mod state_style;
mod state_tables;

// Public re-exports from state modules
pub use state_images::{
    ImageByteTracker, ImageParsingState, PictDecodeError, decode_pict_hex, resolve_image_dimensions,
};

// Internal modules - Phase 3 (handlers)
mod handlers_control_words;
mod handlers_destinations;
mod handlers_fields;
mod handlers_lists;
mod handlers_resources;
mod handlers_structure;
mod handlers_tables;
mod handlers_text;

// Internal modules - Phase 4 (finalization, pipeline, api)
mod api;
mod finalize;
mod pipeline;

// Test module - Phase 6
#[cfg(test)]
mod tests;
