//! Image Finalization Module
//!
//! This module contains logic for finalizing embedded images from RTF `\pict`
//! groups into `Block::ImageBlock` IR nodes.

use super::super::{
    ImageByteTracker, ImageParsingState, decode_pict_hex, resolve_image_dimensions,
};
use crate::{Block, ImageBlock, ImageFormat};

// =============================================================================
// Dropped Content Reason Strings
// =============================================================================

/// Reason string for unsupported image format (not PNG/JPEG).
pub const DROPPED_UNSUPPORTED_IMAGE_FORMAT: &str = "Dropped unsupported image format";

/// Reason string for malformed hex payload in image data.
pub const DROPPED_MALFORMED_IMAGE_HEX: &str = "Dropped malformed image hex payload";

// =============================================================================
// Image Finalization Result
// =============================================================================

/// Result of finalizing an image.
#[derive(Debug)]
pub enum ImageFinalizationResult {
    /// Successfully created image block.
    Success(Block),
    /// Image was dropped due to error (contains reason string).
    Dropped(&'static str),
    /// Hard failure - byte limit exceeded.
    ByteLimitExceeded {
        /// Total decoded bytes after hypothetically adding this image.
        attempted_total: usize,
    },
}

// =============================================================================
// Image Finalization
// =============================================================================

/// Finalize a pict group into an image block.
///
/// This function takes the accumulated image parsing state and produces
/// either a valid `Block::ImageBlock` or a dropped content result.
///
/// # Arguments
///
/// * `state` - The image parsing state containing format, hex data, and dimensions
/// * `tracker` - The byte tracker for enforcing cumulative image byte limits
///
/// # Returns
///
/// * `ImageFinalizationResult::Success(Block)` - Successfully created image
/// * `ImageFinalizationResult::Dropped(reason)` - Image dropped with reason
/// * `ImageFinalizationResult::ByteLimitExceeded` - Hard failure on byte limit
///
/// # Determinism
///
/// This function is deterministic: the same input state will always produce
/// the same output.
pub fn finalize_image(
    state: &ImageParsingState,
    tracker: &mut ImageByteTracker,
) -> ImageFinalizationResult {
    // Step 1: Check if format is set (pngblip or jpegblip)
    let format = match state.format {
        Some(ImageFormat::Png) => ImageFormat::Png,
        Some(ImageFormat::Jpeg) => ImageFormat::Jpeg,
        None => {
            // No format specified or unsupported format
            return ImageFinalizationResult::Dropped(DROPPED_UNSUPPORTED_IMAGE_FORMAT);
        }
    };

    // Step 2: Decode hex data
    let data = match decode_pict_hex(&state.hex_buffer) {
        Ok(bytes) => bytes,
        Err(_) => {
            return ImageFinalizationResult::Dropped(DROPPED_MALFORMED_IMAGE_HEX);
        }
    };

    // Step 3: Check byte limit (hard failure)
    let byte_count = data.len();
    if tracker.would_exceed(byte_count) {
        return ImageFinalizationResult::ByteLimitExceeded {
            attempted_total: tracker.total_bytes.saturating_add(byte_count),
        };
    }
    let _ = tracker.add(byte_count);

    // Step 4: Resolve dimensions
    let (width_twips, height_twips) = resolve_image_dimensions(state);

    // Step 5: Create ImageBlock
    let image_block = ImageBlock {
        format,
        data,
        width_twips,
        height_twips,
    };

    // Step 6: Wrap in Block::ImageBlock and return
    ImageFinalizationResult::Success(Block::ImageBlock(image_block))
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state(format: Option<ImageFormat>, hex: &str) -> ImageParsingState {
        let mut state = ImageParsingState::default();
        state.format = format;
        state.hex_buffer = hex.to_string();
        state
    }

    #[test]
    fn test_finalize_image_png_success() {
        // "Hello" in hex = 48656c6c6f
        let state = create_test_state(Some(ImageFormat::Png), "48656c6c6f");
        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Success(Block::ImageBlock(img)) => {
                assert_eq!(img.format, ImageFormat::Png);
                assert_eq!(img.data, b"Hello");
            }
            _ => panic!("Expected Success with ImageBlock"),
        }

        // Tracker should have recorded 5 bytes
        assert_eq!(tracker.total_bytes, 5);
    }

    #[test]
    fn test_finalize_image_jpeg_success() {
        // "Test" in hex = 54657374
        let state = create_test_state(Some(ImageFormat::Jpeg), "54657374");
        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Success(Block::ImageBlock(img)) => {
                assert_eq!(img.format, ImageFormat::Jpeg);
                assert_eq!(img.data, b"Test");
            }
            _ => panic!("Expected Success with ImageBlock"),
        }

        assert_eq!(tracker.total_bytes, 4);
    }

    #[test]
    fn test_finalize_image_no_format() {
        let state = create_test_state(None, "48656c6c6f");
        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Dropped(reason) => {
                assert_eq!(reason, DROPPED_UNSUPPORTED_IMAGE_FORMAT);
            }
            _ => panic!("Expected Dropped for missing format"),
        }

        // Tracker should not have recorded any bytes
        assert_eq!(tracker.total_bytes, 0);
    }

    #[test]
    fn test_finalize_image_malformed_hex_odd_length() {
        let state = create_test_state(Some(ImageFormat::Png), "48656c6c6"); // Odd length
        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Dropped(reason) => {
                assert_eq!(reason, DROPPED_MALFORMED_IMAGE_HEX);
            }
            _ => panic!("Expected Dropped for malformed hex"),
        }

        assert_eq!(tracker.total_bytes, 0);
    }

    #[test]
    fn test_finalize_image_malformed_hex_invalid_char() {
        let state = create_test_state(Some(ImageFormat::Png), "486x6c6c6f"); // 'x' is invalid
        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Dropped(reason) => {
                assert_eq!(reason, DROPPED_MALFORMED_IMAGE_HEX);
            }
            _ => panic!("Expected Dropped for malformed hex"),
        }

        assert_eq!(tracker.total_bytes, 0);
    }

    #[test]
    fn test_finalize_image_byte_limit_exceeded() {
        // 5 bytes of data
        let state = create_test_state(Some(ImageFormat::Png), "48656c6c6f");
        // Tracker with only 3 bytes remaining
        let mut tracker = ImageByteTracker::new(3);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::ByteLimitExceeded { attempted_total } => {
                // Expected
                assert_eq!(attempted_total, 5);
            }
            _ => panic!("Expected ByteLimitExceeded"),
        }

        // Tracker should not have recorded any bytes (add failed)
        assert_eq!(tracker.total_bytes, 0);
    }

    #[test]
    fn test_finalize_image_with_dimensions() {
        let mut state = ImageParsingState::default();
        state.format = Some(ImageFormat::Png);
        state.hex_buffer = "48656c6c6f".to_string();
        state.picwgoal = Some(1440); // 1 inch
        state.pichgoal = Some(720); // 0.5 inch

        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Success(Block::ImageBlock(img)) => {
                assert_eq!(img.width_twips, Some(1440));
                assert_eq!(img.height_twips, Some(720));
            }
            _ => panic!("Expected Success with ImageBlock"),
        }
    }

    #[test]
    fn test_finalize_image_with_scaling() {
        let mut state = ImageParsingState::default();
        state.format = Some(ImageFormat::Png);
        state.hex_buffer = "48656c6c6f".to_string();
        state.picwgoal = Some(1000);
        state.pichgoal = Some(500);
        state.picscalex = 50; // 50% scale
        state.picscaley = 50;

        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Success(Block::ImageBlock(img)) => {
                assert_eq!(img.width_twips, Some(500)); // 1000 * 50 / 100
                assert_eq!(img.height_twips, Some(250)); // 500 * 50 / 100
            }
            _ => panic!("Expected Success with ImageBlock"),
        }
    }

    #[test]
    fn test_finalize_image_empty_hex() {
        // Empty hex is valid (produces empty data)
        let state = create_test_state(Some(ImageFormat::Png), "");
        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Success(Block::ImageBlock(img)) => {
                assert!(img.data.is_empty());
            }
            _ => panic!("Expected Success with empty ImageBlock"),
        }

        assert_eq!(tracker.total_bytes, 0);
    }

    #[test]
    fn test_finalize_image_whitespace_hex() {
        // Whitespace-only hex is valid (produces empty data)
        let state = create_test_state(Some(ImageFormat::Png), "  \t\n  ");
        let mut tracker = ImageByteTracker::new(1000);

        let result = finalize_image(&state, &mut tracker);

        match result {
            ImageFinalizationResult::Success(Block::ImageBlock(img)) => {
                assert!(img.data.is_empty());
            }
            _ => panic!("Expected Success with empty ImageBlock"),
        }
    }

    #[test]
    fn test_finalize_image_cumulative_tracking() {
        let mut tracker = ImageByteTracker::new(100);

        // First image: 5 bytes
        let state1 = create_test_state(Some(ImageFormat::Png), "48656c6c6f");
        let result1 = finalize_image(&state1, &mut tracker);
        assert!(matches!(result1, ImageFinalizationResult::Success(_)));
        assert_eq!(tracker.total_bytes, 5);

        // Second image: 4 bytes
        let state2 = create_test_state(Some(ImageFormat::Jpeg), "54657374");
        let result2 = finalize_image(&state2, &mut tracker);
        assert!(matches!(result2, ImageFinalizationResult::Success(_)));
        assert_eq!(tracker.total_bytes, 9);

        // Third image: would exceed limit (tracker max is 100, but we set it low for test)
        // Let's create a new tracker with low limit
        let mut small_tracker = ImageByteTracker::new(8);
        let state3 = create_test_state(Some(ImageFormat::Png), "48656c6c6f"); // 5 bytes
        let result3 = finalize_image(&state3, &mut small_tracker);
        assert!(matches!(result3, ImageFinalizationResult::Success(_)));

        // Now try to add more - should fail
        let state4 = create_test_state(Some(ImageFormat::Jpeg), "54657374"); // 4 bytes
        let result4 = finalize_image(&state4, &mut small_tracker);
        assert!(matches!(
            result4,
            ImageFinalizationResult::ByteLimitExceeded { attempted_total: 9 }
        ));
    }

    #[test]
    fn test_determinism_same_input_same_output() {
        // Verify that the same input always produces the same output
        let state = create_test_state(Some(ImageFormat::Png), "48656c6c6f");

        let mut tracker1 = ImageByteTracker::new(1000);
        let result1 = finalize_image(&state, &mut tracker1);

        let mut tracker2 = ImageByteTracker::new(1000);
        let result2 = finalize_image(&state, &mut tracker2);

        // Both should succeed with identical results
        match (result1, result2) {
            (
                ImageFinalizationResult::Success(Block::ImageBlock(img1)),
                ImageFinalizationResult::Success(Block::ImageBlock(img2)),
            ) => {
                assert_eq!(img1.format, img2.format);
                assert_eq!(img1.data, img2.data);
                assert_eq!(img1.width_twips, img2.width_twips);
                assert_eq!(img1.height_twips, img2.height_twips);
            }
            _ => panic!("Both results should be Success"),
        }
    }
}
