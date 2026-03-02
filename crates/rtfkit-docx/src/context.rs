//! Shared conversion context that replaces per-function parameter threading.
//!
//! `ConvertCtx` bundles the five state values that are passed to every
//! conversion function, reducing boilerplate at each call site.

use crate::allocators::{ImageAllocator, NoteLookup, NumberingAllocator};
use rtfkit_style_tokens::StyleProfile;

/// Shared state threaded through all IR → DOCX conversion functions.
pub(crate) struct ConvertCtx<'a> {
    /// Pre-built numbering allocator (read-only during conversion pass).
    pub numbering: &'a NumberingAllocator,
    /// Image ID allocator (mutated each time an image is embedded).
    pub images: &'a mut ImageAllocator,
    /// Monotonically increasing bookmark ID counter for this scope.
    pub bookmark_id: &'a mut usize,
    /// Optional map from note ID → Note, used to inline footnote bodies.
    pub note_lookup: Option<&'a NoteLookup>,
    /// Optional active style profile, used for profile-driven defaults.
    pub profile: Option<&'a StyleProfile>,
}
