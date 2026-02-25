//! Image Parsing State Module
//!
//! This module contains state and utilities for parsing embedded RTF images
//! from `\pict` groups.

use crate::ImageFormat;

// =============================================================================
// Image Parsing State
// =============================================================================

/// State for parsing embedded images (\pict groups).
///
/// This struct tracks all state needed during the parsing of an RTF picture
/// destination, including format detection, dimension controls, and hex data
/// accumulation.
#[derive(Debug, Clone)]
pub struct ImageParsingState {
    /// Whether we're currently parsing a pict group
    pub parsing_pict: bool,
    /// Group depth when pict started (to detect group end)
    pub pict_group_depth: usize,
    /// Detected image format (PNG or JPEG)
    pub format: Option<ImageFormat>,
    /// Hex data buffer (accumulated hex characters)
    pub hex_buffer: String,
    /// Image width in twips (from \picw)
    pub picw: Option<i32>,
    /// Image height in twips (from \pich)
    pub pich: Option<i32>,
    /// Goal width in twips (from \picwgoal)
    pub picwgoal: Option<i32>,
    /// Goal height in twips (from \pichgoal)
    pub pichgoal: Option<i32>,
    /// Horizontal scale percentage (from \picscalex, default 100)
    pub picscalex: i32,
    /// Vertical scale percentage (from \picscaley, default 100)
    pub picscaley: i32,
    /// Whether we're in a \shppict group (preferred over \nonshppict)
    pub in_shppict: bool,
    /// Whether we've seen a \nonshppict group
    pub seen_nonshppict: bool,
}

impl Default for ImageParsingState {
    fn default() -> Self {
        Self {
            parsing_pict: false,
            pict_group_depth: 0,
            format: None,
            hex_buffer: String::new(),
            picw: None,
            pich: None,
            picwgoal: None,
            pichgoal: None,
            picscalex: 100,
            picscaley: 100,
            in_shppict: false,
            seen_nonshppict: false,
        }
    }
}

impl ImageParsingState {
    /// Creates a new image parsing state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset state for a new image.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Start parsing a new pict group at the given depth.
    pub fn start_pict(&mut self, depth: usize) {
        self.parsing_pict = true;
        self.pict_group_depth = depth;
        self.format = None;
        self.hex_buffer.clear();
        self.picw = None;
        self.pich = None;
        self.picwgoal = None;
        self.pichgoal = None;
        self.picscalex = 100;
        self.picscaley = 100;
    }

    /// Check if the pict group has ended (depth returned to start depth).
    pub fn is_pict_ended(&self, current_depth: usize) -> bool {
        self.parsing_pict && current_depth < self.pict_group_depth
    }

    /// Append hex characters to the buffer.
    pub fn append_hex(&mut self, hex: &str) {
        self.hex_buffer.push_str(hex);
    }
}

// =============================================================================
// Image Byte Tracker
// =============================================================================

/// Cumulative tracking for image byte limits.
///
/// Used to enforce the maximum total bytes for all decoded images
/// across a document.
#[derive(Debug, Clone)]
pub struct ImageByteTracker {
    /// Total bytes decoded so far
    pub total_bytes: usize,
    /// Maximum allowed bytes
    pub max_bytes: usize,
}

impl ImageByteTracker {
    /// Creates a new tracker with the given maximum.
    pub fn new(max_bytes: usize) -> Self {
        Self {
            total_bytes: 0,
            max_bytes,
        }
    }

    /// Check if adding the given number of bytes would exceed the limit.
    pub fn would_exceed(&self, additional: usize) -> bool {
        self.total_bytes.saturating_add(additional) > self.max_bytes
    }

    /// Add bytes to the tracker.
    ///
    /// Returns `true` if the addition was successful, `false` if it would
    /// exceed the limit (in which case no bytes are added).
    pub fn add(&mut self, bytes: usize) -> bool {
        if self.would_exceed(bytes) {
            return false;
        }
        self.total_bytes = self.total_bytes.saturating_add(bytes);
        true
    }

    /// Check if the limit has been reached or exceeded.
    pub fn is_exceeded(&self) -> bool {
        self.total_bytes > self.max_bytes
    }

    /// Get remaining bytes allowed.
    pub fn remaining(&self) -> usize {
        self.max_bytes.saturating_sub(self.total_bytes)
    }
}

// =============================================================================
// Hex Decode Error
// =============================================================================

/// Error type for hex decoding failures.
#[derive(Debug, Clone, PartialEq)]
pub enum PictDecodeError {
    /// Odd-length hex string (incomplete byte)
    OddLength,
    /// Invalid hex character encountered
    InvalidChar(char),
}

impl std::fmt::Display for PictDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PictDecodeError::OddLength => write!(f, "odd-length hex string"),
            PictDecodeError::InvalidChar(c) => write!(f, "invalid hex character: '{}'", c),
        }
    }
}

impl std::error::Error for PictDecodeError {}

// =============================================================================
// Hex Decode Utility
// =============================================================================

/// Decode hex string to bytes, ignoring whitespace.
///
/// This function decodes a hex string (as found in RTF `\pict` groups) into
/// a byte vector. ASCII whitespace (space, tab, newline, carriage return)
/// is ignored during decoding.
///
/// # Errors
///
/// - Returns `Err(PictDecodeError::OddLength)` if the hex string (after
///   removing whitespace) has an odd number of characters.
/// - Returns `Err(PictDecodeError::InvalidChar(c))` if an invalid hex
///   character is encountered.
///
/// # Examples
///
/// ```
/// use rtfkit_core::rtf::decode_pict_hex;
///
/// let bytes = decode_pict_hex("48656c6c6f").unwrap();
/// assert_eq!(bytes, b"Hello");
///
/// // Whitespace is ignored
/// let bytes = decode_pict_hex("48 65 6c 6c 6f").unwrap();
/// assert_eq!(bytes, b"Hello");
/// ```
pub fn decode_pict_hex(input: &str) -> Result<Vec<u8>, PictDecodeError> {
    // Filter out whitespace characters
    let hex_chars: String = input
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();

    // Check for odd length
    if hex_chars.len() % 2 != 0 {
        return Err(PictDecodeError::OddLength);
    }

    let mut bytes = Vec::with_capacity(hex_chars.len() / 2);
    let mut chars = hex_chars.chars();

    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        let high_val = hex_char_to_val(high)?;
        let low_val = hex_char_to_val(low)?;
        bytes.push((high_val << 4) | low_val);
    }

    Ok(bytes)
}

/// Convert a single hex character to its numeric value.
fn hex_char_to_val(c: char) -> Result<u8, PictDecodeError> {
    match c {
        '0'..='9' => Ok(c as u8 - b'0'),
        'a'..='f' => Ok(c as u8 - b'a' + 10),
        'A'..='F' => Ok(c as u8 - b'A' + 10),
        _ => Err(PictDecodeError::InvalidChar(c)),
    }
}

// =============================================================================
// Dimension Resolver
// =============================================================================

/// Resolve image dimensions from pict controls.
///
/// Returns `(width_twips, height_twips)` tuple based on the priority:
/// 1. `picwgoal/pichgoal` when present
/// 2. Fallback to `picw/pich`
/// 3. Apply scale (`picscalex`, `picscaley`, default 100)
/// 4. Non-positive dimensions become `None`
///
/// # Examples
///
/// ```
/// use rtfkit_core::rtf::{ImageParsingState, resolve_image_dimensions};
///
/// let mut state = ImageParsingState::default();
/// state.picwgoal = Some(1440); // 1 inch
/// state.pichgoal = Some(720);  // 0.5 inch
///
/// let (w, h) = resolve_image_dimensions(&state);
/// assert_eq!(w, Some(1440));
/// assert_eq!(h, Some(720));
/// ```
pub fn resolve_image_dimensions(state: &ImageParsingState) -> (Option<i32>, Option<i32>) {
    // Priority 1: Use goal dimensions if present
    // Priority 2: Fall back to picw/pich
    let base_width = state.picwgoal.or(state.picw);
    let base_height = state.pichgoal.or(state.pich);

    // Priority 3: Apply scale (default is 100)
    let scale_x = if state.picscalex > 0 { state.picscalex } else { 100 };
    let scale_y = if state.picscaley > 0 { state.picscaley } else { 100 };

    // Calculate scaled dimensions
    let width = base_width.and_then(|w| {
        if w <= 0 {
            return None;
        }
        // Apply scale: scaled = base * scale / 100
        let scaled = (w as i64 * scale_x as i64 / 100) as i32;
        if scaled > 0 { Some(scaled) } else { None }
    });

    let height = base_height.and_then(|h| {
        if h <= 0 {
            return None;
        }
        // Apply scale: scaled = base * scale / 100
        let scaled = (h as i64 * scale_y as i64 / 100) as i32;
        if scaled > 0 { Some(scaled) } else { None }
    });

    (width, height)
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // ImageParsingState Tests
    // ==========================================================================

    #[test]
    fn test_image_parsing_state_default() {
        let state = ImageParsingState::default();
        assert!(!state.parsing_pict);
        assert_eq!(state.pict_group_depth, 0);
        assert!(state.format.is_none());
        assert!(state.hex_buffer.is_empty());
        assert!(state.picw.is_none());
        assert!(state.pich.is_none());
        assert!(state.picwgoal.is_none());
        assert!(state.pichgoal.is_none());
        assert_eq!(state.picscalex, 100);
        assert_eq!(state.picscaley, 100);
        assert!(!state.in_shppict);
        assert!(!state.seen_nonshppict);
    }

    #[test]
    fn test_image_parsing_state_start_pict() {
        let mut state = ImageParsingState::default();
        state.start_pict(5);

        assert!(state.parsing_pict);
        assert_eq!(state.pict_group_depth, 5);
        assert!(state.format.is_none());
        assert!(state.hex_buffer.is_empty());
    }

    #[test]
    fn test_image_parsing_state_is_pict_ended() {
        let mut state = ImageParsingState::default();
        state.start_pict(5);

        // At depth 5, not ended
        assert!(!state.is_pict_ended(5));
        // At depth 4, ended
        assert!(state.is_pict_ended(4));
        // At depth 3, ended
        assert!(state.is_pict_ended(3));
    }

    #[test]
    fn test_image_parsing_state_reset() {
        let mut state = ImageParsingState::default();
        state.start_pict(5);
        state.format = Some(ImageFormat::Png);
        state.hex_buffer.push_str("abcd");
        state.picw = Some(100);
        state.picscalex = 50;

        state.reset();

        assert!(!state.parsing_pict);
        assert_eq!(state.pict_group_depth, 0);
        assert!(state.format.is_none());
        assert!(state.hex_buffer.is_empty());
        assert!(state.picw.is_none());
        assert_eq!(state.picscalex, 100);
    }

    #[test]
    fn test_image_parsing_state_append_hex() {
        let mut state = ImageParsingState::default();
        state.append_hex("abc");
        state.append_hex("def");

        assert_eq!(state.hex_buffer, "abcdef");
    }

    // ==========================================================================
    // ImageByteTracker Tests
    // ==========================================================================

    #[test]
    fn test_image_byte_tracker_new() {
        let tracker = ImageByteTracker::new(1000);
        assert_eq!(tracker.total_bytes, 0);
        assert_eq!(tracker.max_bytes, 1000);
    }

    #[test]
    fn test_image_byte_tracker_add() {
        let mut tracker = ImageByteTracker::new(1000);

        assert!(tracker.add(500));
        assert_eq!(tracker.total_bytes, 500);

        assert!(tracker.add(400));
        assert_eq!(tracker.total_bytes, 900);
    }

    #[test]
    fn test_image_byte_tracker_would_exceed() {
        let mut tracker = ImageByteTracker::new(1000);
        tracker.add(800);

        assert!(!tracker.would_exceed(100));  // 800 + 100 = 900 <= 1000
        assert!(!tracker.would_exceed(200));  // 800 + 200 = 1000 <= 1000
        assert!(tracker.would_exceed(201));   // 800 + 201 = 1001 > 1000
    }

    #[test]
    fn test_image_byte_tracker_add_exceeds_limit() {
        let mut tracker = ImageByteTracker::new(1000);
        tracker.add(800);

        // This should fail and not add
        assert!(!tracker.add(300));
        assert_eq!(tracker.total_bytes, 800); // Unchanged
    }

    #[test]
    fn test_image_byte_tracker_remaining() {
        let mut tracker = ImageByteTracker::new(1000);
        assert_eq!(tracker.remaining(), 1000);

        tracker.add(300);
        assert_eq!(tracker.remaining(), 700);
    }

    #[test]
    fn test_image_byte_tracker_is_exceeded() {
        let mut tracker = ImageByteTracker::new(1000);
        assert!(!tracker.is_exceeded());

        tracker.add(1000);
        assert!(!tracker.is_exceeded()); // At limit, not over

        // Manually set over limit (shouldn't happen via add, but test the check)
        tracker.total_bytes = 1001;
        assert!(tracker.is_exceeded());
    }

    // ==========================================================================
    // Hex Decode Tests
    // ==========================================================================

    #[test]
    fn test_decode_pict_hex_valid() {
        // "Hello" in hex
        let result = decode_pict_hex("48656c6c6f").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_pict_hex_uppercase() {
        // "Hello" in uppercase hex
        let result = decode_pict_hex("48656C6C6F").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_pict_hex_mixed_case() {
        // Mixed case hex
        let result = decode_pict_hex("48656C6c6F").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_pict_hex_whitespace_tolerant() {
        // Whitespace should be ignored
        let result = decode_pict_hex("48 65 6c 6c 6f").unwrap();
        assert_eq!(result, b"Hello");

        let result = decode_pict_hex("48\t65\n6c\r6c6f").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_pict_hex_empty() {
        let result = decode_pict_hex("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_decode_pict_hex_whitespace_only() {
        let result = decode_pict_hex("  \t\n\r  ").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_decode_pict_hex_odd_length() {
        let result = decode_pict_hex("486");
        assert_eq!(result, Err(PictDecodeError::OddLength));

        // With whitespace (odd after filtering)
        let result = decode_pict_hex("48 6");
        assert_eq!(result, Err(PictDecodeError::OddLength));
    }

    #[test]
    fn test_decode_pict_hex_invalid_char() {
        let result = decode_pict_hex("486x6c6c6f");
        assert_eq!(result, Err(PictDecodeError::InvalidChar('x')));

        let result = decode_pict_hex("486Z6c6c6f");
        assert_eq!(result, Err(PictDecodeError::InvalidChar('Z')));
    }

    #[test]
    fn test_decode_pict_hex_binary_data() {
        // Test with binary data (all byte values possible)
        let hex = "00ff7f80fe";
        let result = decode_pict_hex(hex).unwrap();
        assert_eq!(result, vec![0x00, 0xff, 0x7f, 0x80, 0xfe]);
    }

    // ==========================================================================
    // Dimension Resolver Tests
    // ==========================================================================

    #[test]
    fn test_resolve_image_dimensions_goal_precedence() {
        // picwgoal/pichgoal should take precedence over picw/pich
        let mut state = ImageParsingState::default();
        state.picw = Some(1000);
        state.pich = Some(500);
        state.picwgoal = Some(1440);
        state.pichgoal = Some(720);

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, Some(1440));
        assert_eq!(h, Some(720));
    }

    #[test]
    fn test_resolve_image_dimensions_fallback_to_picw() {
        // When no goal, fall back to picw/pich
        let mut state = ImageParsingState::default();
        state.picw = Some(1000);
        state.pich = Some(500);

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, Some(1000));
        assert_eq!(h, Some(500));
    }

    #[test]
    fn test_resolve_image_dimensions_no_dimensions() {
        // No dimensions set
        let state = ImageParsingState::default();

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, None);
        assert_eq!(h, None);
    }

    #[test]
    fn test_resolve_image_dimensions_scaling() {
        // Apply 50% scale
        let mut state = ImageParsingState::default();
        state.picwgoal = Some(1000);
        state.pichgoal = Some(500);
        state.picscalex = 50;
        state.picscaley = 50;

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, Some(500));  // 1000 * 50 / 100
        assert_eq!(h, Some(250));  // 500 * 50 / 100
    }

    #[test]
    fn test_resolve_image_dimensions_scaling_200_percent() {
        // Apply 200% scale
        let mut state = ImageParsingState::default();
        state.picwgoal = Some(1000);
        state.pichgoal = Some(500);
        state.picscalex = 200;
        state.picscaley = 200;

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, Some(2000));  // 1000 * 200 / 100
        assert_eq!(h, Some(1000));  // 500 * 200 / 100
    }

    #[test]
    fn test_resolve_image_dimensions_non_positive_base() {
        // Non-positive base dimensions become None
        let mut state = ImageParsingState::default();
        state.picwgoal = Some(0);
        state.pichgoal = Some(-100);

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, None);
        assert_eq!(h, None);
    }

    #[test]
    fn test_resolve_image_dimensions_non_positive_scale() {
        // Non-positive scale should use default 100
        let mut state = ImageParsingState::default();
        state.picwgoal = Some(1000);
        state.pichgoal = Some(500);
        state.picscalex = 0;
        state.picscaley = -50;

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, Some(1000));  // Uses default scale 100
        assert_eq!(h, Some(500));   // Uses default scale 100
    }

    #[test]
    fn test_resolve_image_dimensions_partial_goal() {
        // Only one goal dimension set
        let mut state = ImageParsingState::default();
        state.picw = Some(800);
        state.pich = Some(400);
        state.picwgoal = Some(1440);
        // pichgoal is None

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, Some(1440));  // Uses goal
        assert_eq!(h, Some(400));   // Falls back to pich
    }

    #[test]
    fn test_resolve_image_dimensions_scaling_truncation() {
        // Test that scaling doesn't cause overflow with large values
        let mut state = ImageParsingState::default();
        state.picwgoal = Some(100000);
        state.pichgoal = Some(100000);
        state.picscalex = 100;
        state.picscaley = 100;

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, Some(100000));
        assert_eq!(h, Some(100000));
    }

    #[test]
    fn test_resolve_image_dimensions_scale_result_zero() {
        // If scaling results in 0, should return None
        let mut state = ImageParsingState::default();
        state.picwgoal = Some(1);
        state.pichgoal = Some(1);
        state.picscalex = 1;  // 1 * 1 / 100 = 0
        state.picscaley = 1;

        let (w, h) = resolve_image_dimensions(&state);
        assert_eq!(w, None);
        assert_eq!(h, None);
    }
}
