//! Block-level HTML emission modules.
//!
//! This module contains submodules for converting each block type
//! to HTML.

pub mod image;
pub mod list;
pub mod paragraph;
pub mod structure;
pub mod table;

// Re-export public functions for convenience
pub use image::image_to_html;
