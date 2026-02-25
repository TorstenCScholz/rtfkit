//! Finalization module facade.
//!
//! This module re-exports finalization helpers split by cohesive domain.

mod document;
mod images;
mod lists;
mod paragraphs;
pub(crate) mod runs;
mod shading;
mod tables;

pub use document::finalize_document;
pub use images::{
    finalize_image, ImageFinalizationResult,
    DROPPED_MALFORMED_IMAGE_HEX, DROPPED_UNSUPPORTED_IMAGE_FORMAT,
};
pub use paragraphs::{finalize_paragraph, finalize_paragraph_for_table};
pub use runs::{flush_current_text_as_field_run, flush_current_text_as_run};
pub use tables::{auto_close_table_cell_if_needed, finalize_current_cell, finalize_current_row};
