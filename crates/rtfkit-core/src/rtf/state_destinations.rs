//! Destination State Module
//!
//! This module contains destination skip state tracking for handling
//! RTF destination groups like {\*\destination ...}.

/// Destination parsing state.
///
/// Tracks whether we're currently skipping a destination group
/// and the depth of nested groups within it.
#[derive(Debug, Clone, Default)]
pub struct DestinationState {
    /// Tracks whether we just read a destination marker control symbol (\*)
    pub destination_marker: bool,
    /// Number of nested groups currently being skipped as a destination
    pub skip_destination_depth: usize,
}

impl DestinationState {
    /// Creates a new default destination state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if we're currently skipping a destination.
    pub fn is_skipping(&self) -> bool {
        self.skip_destination_depth > 0
    }

    /// Enter a skipped destination (increment depth).
    pub fn enter_destination(&mut self) {
        self.skip_destination_depth += 1;
    }

    /// Exit a skipped destination (decrement depth).
    pub fn exit_destination(&mut self) {
        self.skip_destination_depth = self.skip_destination_depth.saturating_sub(1);
    }

    /// Reset destination state when exiting all destinations.
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.destination_marker = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destination_state_default() {
        let state = DestinationState::new();
        assert!(!state.destination_marker);
        assert_eq!(state.skip_destination_depth, 0);
        assert!(!state.is_skipping());
    }

    #[test]
    fn test_destination_state_skipping() {
        let mut state = DestinationState::new();

        // Enter destination
        state.enter_destination();
        assert!(state.is_skipping());
        assert_eq!(state.skip_destination_depth, 1);

        // Enter nested group
        state.enter_destination();
        assert_eq!(state.skip_destination_depth, 2);

        // Exit nested group
        state.exit_destination();
        assert_eq!(state.skip_destination_depth, 1);

        // Exit destination
        state.exit_destination();
        assert!(!state.is_skipping());
    }

    #[test]
    fn test_destination_state_reset() {
        let mut state = DestinationState::new();
        state.destination_marker = true;
        state.skip_destination_depth = 3;

        state.reset();
        assert!(!state.destination_marker);
        assert_eq!(state.skip_destination_depth, 3); // reset doesn't change depth
    }
}
