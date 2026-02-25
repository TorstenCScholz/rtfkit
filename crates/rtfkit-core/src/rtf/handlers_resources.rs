//! Resource Handlers Module
//!
//! This module contains font/color control handling for both normal parsing
//! flow and skipped destination parsing.

use super::state::RuntimeState;

/// Handle regular (non-destination) resource-related control words.
///
/// Returns `true` if `word` was recognized and handled.
pub fn handle_resource_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) -> bool {
    match word {
        // \\deffN - Default font index
        "deff" => {
            if let Some(index) = parameter {
                state.resources.default_font_index = Some(index);
                // Also set as current font if no font is currently set
                if state.style.font_index.is_none() {
                    state.style.font_index = Some(index);
                }
            }
            true
        }
        // \\fN - Font index
        "f" => {
            state.style.font_index = parameter;
            true
        }
        // \\fsN - Font size in half-points
        "fs" => {
            state.style.font_size_half_points = parameter;
            true
        }
        // \\cfN - Foreground color index
        "cf" => {
            state.style.color_index = parameter;
            true
        }
        // \\highlightN - Highlight color index
        "highlight" => {
            state.style.highlight_color_index =
                parameter.and_then(|n| if n > 0 { Some(n) } else { None });
            true
        }
        // \\cbN - Background color index
        "cb" => {
            state.style.background_color_index =
                parameter.and_then(|n| if n > 0 { Some(n) } else { None });
            true
        }
        _ => false,
    }
}

/// Handle control words while parsing font/color destinations.
///
/// Returns `true` if the current state was in a font/color destination and the
/// control word was consumed (even if ignored by design).
pub fn handle_destination_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) -> bool {
    // Font table control words
    if state.resources.parsing_font_table {
        match word {
            "f" => {
                // Font index
                state
                    .resources
                    .set_current_font_index(parameter.unwrap_or(0));
            }
            // Font property controls - we skip these but don't warn
            "fnil" | "froman" | "fswiss" | "fmodern" | "fscript" | "fdecor" | "ftech" | "fbidi" => {
                // Font family - ignore
            }
            "fcharset" => {
                // Character set - ignore
            }
            "fprq" => {
                // Font pitch - ignore
            }
            "panose" | "ftnil" | "fttruetype" => {
                // Font technology - ignore
            }
            _ => {
                // For font table, we don't warn on unknown controls
                // They're likely font-specific properties we don't need
            }
        }
        return true;
    }

    // Color table control words
    if state.resources.parsing_color_table {
        match word {
            "red" => {
                if let Some(val) = parameter {
                    state.resources.set_red(val);
                }
            }
            "green" => {
                if let Some(val) = parameter {
                    state.resources.set_green(val);
                }
            }
            "blue" => {
                if let Some(val) = parameter {
                    state.resources.set_blue(val);
                }
            }
            // Theme color controls
            "themecolor" => {
                if let Some(val) = parameter {
                    state.resources.set_theme_color(val);
                }
            }
            "ctint" => {
                if let Some(val) = parameter {
                    state.resources.set_theme_tint(val);
                }
            }
            "cshade" => {
                if let Some(val) = parameter {
                    state.resources.set_theme_shade(val);
                }
            }
            _ => {
                // Unknown control in color table - ignore silently
            }
        }
        return true;
    }

    false
}

/// Handle text while parsing font/color destinations.
pub fn handle_destination_text(state: &mut RuntimeState, text: &str) {
    if state.resources.parsing_font_table {
        // Accumulate font name text
        state.resources.append_font_name(text);
    } else if state.resources.parsing_color_table {
        // Semicolons separate color entries.
        for ch in text.chars() {
            if ch == ';' {
                state.resources.finalize_color();
            }
        }
    }
}

/// Handle group start effects for resource destinations.
pub fn handle_destination_group_start(state: &mut RuntimeState, skip_destination_depth: usize) {
    // Each font entry is in a nested group one level below \fonttbl.
    if state.resources.parsing_font_table && skip_destination_depth == 2 {
        state.resources.current_font_index = None;
        state.resources.current_font_name.clear();
    }
}

/// Handle group end effects for resource destinations.
pub fn handle_destination_group_end(state: &mut RuntimeState, skip_destination_depth: usize) {
    // Finalize font entry when closing a font group.
    if state.resources.parsing_font_table && skip_destination_depth == 2 {
        state.resources.finalize_font_entry();
    }
}
