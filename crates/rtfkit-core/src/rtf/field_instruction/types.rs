//! Typed output of field instruction parsing.

use crate::{HyperlinkTarget, PageFieldRef, SemanticFieldRef, TocOptions};

/// Parsed field instruction — the typed output of parsing a `\fldinst` string.
///
/// This type is produced by the field instruction parser and is independent
/// of any runtime state or report mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedFieldInstruction {
    /// Hyperlink to an external URL or internal bookmark.
    Hyperlink(HyperlinkTarget),
    /// Page-management field (PAGE, NUMPAGES, PAGEREF, etc.).
    PageField(PageFieldRef),
    /// Semantic cross-reference or document property field.
    SemanticField(SemanticFieldRef),
    /// Table-of-contents generation request.
    Toc {
        /// TOC options.
        options: TocOptions,
        /// Switch tokens that were not recognized.
        unsupported_switches: Vec<String>,
    },
}

/// Token kind for a field switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchKind {
    /// Flag switch — no value token follows it.
    Flag,
    /// Value switch — one value token follows it.
    Value,
}
