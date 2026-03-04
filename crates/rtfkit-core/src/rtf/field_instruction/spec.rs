//! Switch specification tables for RTF field types.
//!
//! Each entry maps a switch name (without the leading backslash, case-insensitive)
//! to its `SwitchKind`.  Any switch not listed is treated as `Flag` by default —
//! this is the conservative choice for positional-argument extraction: when we
//! don't know a switch, we do NOT consume the next token as its value.

use super::types::SwitchKind;

/// Lookup the kind of a switch for the `REF` field.
///
/// RTF spec switches for REF:
///   `\d` (separator, takes value), `\f` (footnote, takes value),
///   `\h` (hyperlink, flag), `\n` (no number, flag),
///   `\p` (relative position, flag), `\r` (relative, flag),
///   `\t` (suppress non-delimiter, flag), `\w` (word, flag).
pub fn ref_switch_kind(name: &str) -> SwitchKind {
    match name.to_ascii_lowercase().as_str() {
        "d" | "f" => SwitchKind::Value,
        _ => SwitchKind::Flag,
    }
}

/// Lookup the kind of a switch for the `NOTEREF` field.
///
/// Switches: `\f` (footnote, flag), `\h` (hyperlink, flag), `\p` (relative, flag).
pub fn noteref_switch_kind(_name: &str) -> SwitchKind {
    // All common NOTEREF switches are flags.
    SwitchKind::Flag
}

/// Lookup the kind of a switch for the `SEQ` field.
///
/// Switches: `\c` (current, flag), `\h` (hidden, flag), `\n` (next, flag),
///           `\r` (reset, takes value), `\s` (sequence-level, takes value).
pub fn seq_switch_kind(name: &str) -> SwitchKind {
    match name.to_ascii_lowercase().as_str() {
        "r" | "s" => SwitchKind::Value,
        _ => SwitchKind::Flag,
    }
}

/// Lookup the kind of a switch for the `PAGEREF` field.
///
/// Switches: `\h` (hyperlink, flag), `\p` (relative position, flag).
pub fn pageref_switch_kind(_name: &str) -> SwitchKind {
    SwitchKind::Flag
}

/// Lookup the kind of a switch for the `TOC` field.
///
/// Switches: `\a` (value), `\b` (value), `\c` (value), `\d` (value),
///           `\f` (value), `\h` (flag), `\l` (value), `\n` (value),
///           `\o` (value), `\p` (value), `\s` (value), `\t` (value),
///           `\u` (flag), `\w` (flag), `\x` (flag), `\z` (flag).
#[allow(dead_code)]
pub fn toc_switch_kind(name: &str) -> SwitchKind {
    match name.to_ascii_lowercase().as_str() {
        "a" | "b" | "c" | "d" | "f" | "l" | "n" | "o" | "p" | "s" | "t" => SwitchKind::Value,
        _ => SwitchKind::Flag,
    }
}

/// Lookup the kind of a switch for the `HYPERLINK` field.
///
/// Switches: `\l` (bookmark, takes value), `\m` (image map, takes value),
///           `\n` (new window, flag), `\o` (tooltip, takes value),
///           `\t` (target frame, takes value).
pub fn hyperlink_switch_kind(name: &str) -> SwitchKind {
    match name.to_ascii_lowercase().as_str() {
        "l" | "m" | "o" | "t" => SwitchKind::Value,
        _ => SwitchKind::Flag,
    }
}

/// Default switch kind used when no field-specific spec is available.
///
/// Defaults to `Flag` (do not consume next token) — the conservative choice.
pub fn default_switch_kind(_name: &str) -> SwitchKind {
    SwitchKind::Flag
}
