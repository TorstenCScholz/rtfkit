//! Field instruction parsing subsystem.
//!
//! This module provides isolated, pure parsing of RTF `\fldinst` strings
//! into typed `ParsedFieldInstruction` values.  It has no dependency on
//! runtime state, reports, or paragraph mutation — those concerns live in
//! `handlers_fields.rs`.
//!
//! # Structure
//!
//! - `types.rs` — output types (`ParsedFieldInstruction`, `SwitchKind`).
//! - `spec.rs`  — per-field switch-kind lookup tables.
//! - `tokenize.rs` — raw tokenizer for field instruction text.
//! - `parse.rs` — per-field-type parsing functions.

mod parse;
mod spec;
mod tokenize;
mod types;

pub use parse::parse_field_instruction;
pub use types::ParsedFieldInstruction;

// Re-export internal helpers used by handlers_fields.rs tests.
#[cfg(test)]
pub use parse::{parse_hyperlink, parse_semantic_field};
