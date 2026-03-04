//! Finalization module facade.
//!
//! This module re-exports finalization helpers split by cohesive domain.

mod borders;
mod document;
mod images;
mod lists;
mod page_management;
mod paragraphs;
pub(crate) mod runs;
mod semantic_fields;
mod shading;
mod tables;

pub use document::finalize_document;
pub use semantic_fields::resolve_semantic_cross_references;
pub use images::{ImageFinalizationResult, finalize_image};
pub use paragraphs::{finalize_paragraph, finalize_paragraph_for_table};
pub use runs::{flush_current_text_as_field_run, flush_current_text_as_run};
pub use tables::{
    auto_close_table_cell_if_needed, finalize_current_cell, finalize_current_row,
    finalize_current_table,
};
