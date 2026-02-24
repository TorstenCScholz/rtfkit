//! Resource State Module
//!
//! This module contains font and color table state for handling
//! RTF resource table parsing.

use crate::{ColorEntry, ThemeColor};
use std::collections::HashMap;

/// Resource table state.
///
/// Tracks font and color table parsing state and storage.
#[derive(Debug, Clone, Default)]
pub struct ResourceState {
    // =============================================================================
    // Font Table
    // =============================================================================
    /// Default font index (from \deffN)
    pub default_font_index: Option<i32>,
    /// Font table mapping font index to font family name
    pub font_table: HashMap<i32, String>,
    /// Whether we're currently parsing a font table destination
    pub parsing_font_table: bool,
    /// Current font index being parsed (from \fN)
    pub current_font_index: Option<i32>,
    /// Current font name being accumulated
    pub current_font_name: String,

    // =============================================================================
    // Color Table
    // =============================================================================
    /// Color table (index 0 is auto/default, represented as None)
    pub color_table: Vec<ColorEntry>,
    /// Whether we're currently parsing a color table destination
    pub parsing_color_table: bool,
    /// Current red component (from \redN)
    pub current_red: u8,
    /// Current green component (from \greenN)
    pub current_green: u8,
    /// Current blue component (from \blueN)
    pub current_blue: u8,
    /// Whether any color component has been set since last semicolon
    pub color_components_seen: bool,
    /// Current theme color index (from \themecolorN in color table)
    pub current_theme_color: Option<ThemeColor>,
    /// Current theme tint value (from \ctintN in color table)
    pub current_theme_tint: Option<u8>,
    /// Current theme shade value (from \cshadeN in color table)
    pub current_theme_shade: Option<u8>,
}

impl ResourceState {
    /// Creates a new default resource state.
    pub fn new() -> Self {
        Self::default()
    }

    // =============================================================================
    // Font Table Methods
    // =============================================================================

    /// Start parsing a font table.
    pub fn start_font_table(&mut self) {
        self.parsing_font_table = true;
        // Use the latest font table definition if multiple are present
        self.font_table.clear();
        self.current_font_index = None;
        self.current_font_name.clear();
    }

    /// Set the current font index (from \fN).
    pub fn set_current_font_index(&mut self, index: i32) {
        self.current_font_index = Some(index);
    }

    /// Append text to the current font name.
    pub fn append_font_name(&mut self, text: &str) {
        self.current_font_name.push_str(text);
    }

    /// Finalize the current font entry (when closing a font group).
    pub fn finalize_font_entry(&mut self) {
        if let Some(font_idx) = self.current_font_index.take() {
            // Clean up font name: trim whitespace and remove trailing semicolon if present
            let mut name = self.current_font_name.trim().to_string();
            if name.ends_with(';') {
                name.pop();
            }
            let name = name.trim().to_string();
            if !name.is_empty() {
                self.font_table.insert(font_idx, name);
            }
        }
        self.current_font_name.clear();
    }

    /// End font table parsing.
    #[cfg(test)]
    pub fn end_font_table(&mut self) {
        self.parsing_font_table = false;
    }

    /// Look up a font family by index.
    pub fn get_font_family(&self, index: i32) -> Option<&str> {
        self.font_table.get(&index).map(|s| s.as_str())
    }

    // =============================================================================
    // Color Table Methods
    // =============================================================================

    /// Start parsing a color table.
    pub fn start_color_table(&mut self) {
        self.parsing_color_table = true;
        // Use the latest color table definition if multiple are present
        self.color_table.clear();
        self.current_red = 0;
        self.current_green = 0;
        self.current_blue = 0;
        self.color_components_seen = false;
        self.current_theme_color = None;
        self.current_theme_tint = None;
        self.current_theme_shade = None;
    }

    /// Set the red component (from \redN).
    pub fn set_red(&mut self, value: i32) {
        self.current_red = value.clamp(0, 255) as u8;
        self.color_components_seen = true;
    }

    /// Set the green component (from \greenN).
    pub fn set_green(&mut self, value: i32) {
        self.current_green = value.clamp(0, 255) as u8;
        self.color_components_seen = true;
    }

    /// Set the blue component (from \blueN).
    pub fn set_blue(&mut self, value: i32) {
        self.current_blue = value.clamp(0, 255) as u8;
        self.color_components_seen = true;
    }

    /// Set the theme color (from \themecolorN).
    pub fn set_theme_color(&mut self, value: i32) {
        self.current_theme_color = ThemeColor::from_index(value);
    }

    /// Set the theme tint (from \ctintN).
    pub fn set_theme_tint(&mut self, value: i32) {
        self.current_theme_tint = Some(value.clamp(0, 255) as u8);
    }

    /// Set the theme shade (from \cshadeN).
    pub fn set_theme_shade(&mut self, value: i32) {
        self.current_theme_shade = Some(value.clamp(0, 255) as u8);
    }

    /// Finalize the current color (when encountering a semicolon).
    pub fn finalize_color(&mut self) {
        if self.color_components_seen {
            // We have RGB components, push the defined color
            let color = crate::Color {
                r: self.current_red,
                g: self.current_green,
                b: self.current_blue,
            };
            // Check if we also have theme color metadata
            if let Some(theme_color) = self.current_theme_color.take() {
                // Create a ColorEntry with both RGB and theme info
                self.color_table.push(ColorEntry {
                    rgb: Some(color),
                    theme_color: Some(theme_color),
                    theme_tint: self.current_theme_tint.take(),
                    theme_shade: self.current_theme_shade.take(),
                });
            } else {
                // Just RGB, no theme color
                self.color_table.push(ColorEntry::rgb(color));
            }
        } else if let Some(theme_color) = self.current_theme_color.take() {
            // Theme color without explicit RGB
            self.color_table.push(ColorEntry::theme(
                theme_color,
                self.current_theme_tint.take(),
                self.current_theme_shade.take(),
            ));
        } else {
            // No RGB components seen - this is auto/default slot
            self.color_table.push(ColorEntry::auto_color());
        }
        // Reset for next color
        self.current_red = 0;
        self.current_green = 0;
        self.current_blue = 0;
        self.color_components_seen = false;
        self.current_theme_color = None;
        self.current_theme_tint = None;
        self.current_theme_shade = None;
    }

    /// End color table parsing.
    #[cfg(test)]
    pub fn end_color_table(&mut self) {
        self.parsing_color_table = false;
    }

    /// Resolve a color from a color index.
    ///
    /// Index 0 means auto/default color (represented as None).
    /// Invalid indices degrade to None without warnings.
    /// Theme colors are resolved to concrete RGB values.
    pub fn resolve_color(&self, color_idx: i32) -> Option<crate::Color> {
        // Index 0 is auto/default color, represented as None
        if color_idx == 0 {
            return None;
        }

        // Color table stores: [auto (None), color1, color2, ...]
        let table_index = color_idx as usize;
        self.color_table
            .get(table_index)
            .and_then(|entry| entry.resolve())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_state_default() {
        let state = ResourceState::new();
        assert!(state.default_font_index.is_none());
        assert!(state.font_table.is_empty());
        assert!(state.color_table.is_empty());
        assert!(!state.parsing_font_table);
        assert!(!state.parsing_color_table);
    }

    #[test]
    fn test_font_table_parsing() {
        let mut state = ResourceState::new();
        state.start_font_table();

        state.set_current_font_index(0);
        state.append_font_name("Arial;");
        state.finalize_font_entry();

        state.end_font_table();

        assert_eq!(state.get_font_family(0), Some("Arial"));
    }

    #[test]
    fn test_font_table_cleanup() {
        let mut state = ResourceState::new();
        state.start_font_table();

        state.set_current_font_index(0);
        state.append_font_name("  Arial ; "); // Extra whitespace
        state.finalize_font_entry();

        assert_eq!(state.get_font_family(0), Some("Arial"));
    }

    #[test]
    fn test_color_table_rgb() {
        let mut state = ResourceState::new();
        state.start_color_table();

        // First semicolon creates auto/default slot
        state.finalize_color();

        // Add a red color
        state.set_red(255);
        state.set_green(0);
        state.set_blue(0);
        state.finalize_color();

        state.end_color_table();

        assert_eq!(state.color_table.len(), 2);
        assert!(state.color_table[0].rgb.is_none()); // Auto color
        assert_eq!(
            state.color_table[1].rgb,
            Some(crate::Color { r: 255, g: 0, b: 0 })
        );
    }

    #[test]
    fn test_color_table_clamping() {
        let mut state = ResourceState::new();
        state.start_color_table();

        state.finalize_color(); // Auto slot

        state.set_red(300); // Should clamp to 255
        state.set_green(-10); // Should clamp to 0
        state.set_blue(128);
        state.finalize_color();

        state.end_color_table();

        assert_eq!(
            state.color_table[1].rgb,
            Some(crate::Color {
                r: 255,
                g: 0,
                b: 128
            })
        );
    }

    #[test]
    fn test_resolve_color() {
        let mut state = ResourceState::new();
        state.start_color_table();
        state.finalize_color(); // Auto slot (index 0)
        state.set_red(255);
        state.finalize_color(); // Red (index 1)
        state.end_color_table();

        // Index 0 is auto (None)
        assert!(state.resolve_color(0).is_none());

        // Index 1 is red
        let color = state.resolve_color(1);
        assert_eq!(color, Some(crate::Color { r: 255, g: 0, b: 0 }));

        // Invalid index
        assert!(state.resolve_color(99).is_none());
    }
}
