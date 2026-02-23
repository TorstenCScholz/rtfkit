//! # rtfkit-style-tokens
//!
//! Cross-format style tokens for rtfkit HTML and PDF outputs.
//!
//! This crate provides a single source of truth for visual styling that can be
//! applied across different output formats (HTML via CSS, PDF via Typst).
//!
//! ## Design Principles
//!
//! - **Strong typing over stringly config**: All token values are strongly typed
//! - **Single ownership**: All token definitions live in this crate only
//! - **Deterministic serialization**: Same input always produces same output
//! - **No renderer dependencies**: This crate has no dependencies on HTML writer
//!   or Typst renderer internals
//!
//! ## Quick Start
//!
//! ```rust
//! use rtfkit_style_tokens::{StyleProfile, builtins, validate, serialize};
//!
//! // Get the default profile (Report)
//! let profile = builtins::default_profile();
//!
//! // Validate a profile
//! validate::validate_profile(&profile).expect("Profile should be valid");
//!
//! // Generate CSS variables
//! let css = serialize::to_css_variables(&profile);
//! println!("{}", css);
//!
//! // Generate Typst preamble
//! let typst = serialize::to_typst_preamble(&profile);
//! println!("{}", typst);
//! ```
//!
//! ## Built-in Profiles
//!
//! Three built-in profiles are provided:
//!
//! - **Classic**: Conservative style, close to current rtfkit behavior
//! - **Report**: Professional style with strong hierarchy (DEFAULT)
//! - **Compact**: Dense style for enterprise output
//!
//! ## Token Families
//!
//! The [`profile::StyleProfile`] contains five token families:
//!
//! - [`profile::ColorTokens`]: Text, surface, border, and link colors
//! - [`profile::TypographyTokens`]: Fonts, sizes, line heights, and weights
//! - [`profile::SpacingTokens`]: Consistent whitespace values
//! - [`profile::LayoutTokens`]: Page dimensions and margins
//! - [`profile::ComponentTokens`]: Table, list, and heading specific tokens

//! ## Feature Flags
//!
//! - `serde`: Enables serialization/deserialization support via serde

pub mod builtins;
pub mod profile;
pub mod serialize;
pub mod validate;

// Re-export commonly used types at crate root for convenience
pub use profile::{
    ColorHex, ColorHexError, ColorTokens, ComponentTokens, HeadingComponentTokens, LayoutTokens,
    ListComponentTokens, SpacingTokens, StyleProfile, StyleProfileName, TableComponentTokens,
    TableStripeMode, TypographyTokens,
};

pub use validate::StyleValidationError;
