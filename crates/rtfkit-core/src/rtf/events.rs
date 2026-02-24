//! RTF Events Module
//!
//! This module defines the event types emitted during RTF parsing
//! and provides conversion from tokens to events.

use super::tokenizer::Token;

// =============================================================================
// Event Types
// =============================================================================

/// Events emitted by the tokenizer for the interpreter to process.
#[derive(Debug, Clone, PartialEq)]
pub enum RtfEvent {
    /// Start of a group `{`
    GroupStart,
    /// End of a group `}`
    GroupEnd,
    /// A control word with optional parameter
    ControlWord {
        word: String,
        parameter: Option<i32>,
    },
    /// A single-char control symbol
    ControlSymbol(char),
    /// Text content
    Text(String),
}

// =============================================================================
// Token to Event Conversion
// =============================================================================

/// Convert a token to an event.
pub fn token_to_event(token: Token) -> RtfEvent {
    match token {
        Token::GroupStart => RtfEvent::GroupStart,
        Token::GroupEnd => RtfEvent::GroupEnd,
        Token::ControlWord { word, parameter } => RtfEvent::ControlWord { word, parameter },
        Token::Text(text) => RtfEvent::Text(text),
        Token::ControlSymbol(symbol) => RtfEvent::ControlSymbol(symbol),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_to_event_group_start() {
        let token = Token::GroupStart;
        let event = token_to_event(token);
        assert_eq!(event, RtfEvent::GroupStart);
    }

    #[test]
    fn test_token_to_event_group_end() {
        let token = Token::GroupEnd;
        let event = token_to_event(token);
        assert_eq!(event, RtfEvent::GroupEnd);
    }

    #[test]
    fn test_token_to_event_control_word() {
        let token = Token::ControlWord {
            word: "b".to_string(),
            parameter: None,
        };
        let event = token_to_event(token);
        assert_eq!(
            event,
            RtfEvent::ControlWord {
                word: "b".to_string(),
                parameter: None
            }
        );
    }

    #[test]
    fn test_token_to_event_control_word_with_param() {
        let token = Token::ControlWord {
            word: "fs".to_string(),
            parameter: Some(24),
        };
        let event = token_to_event(token);
        assert_eq!(
            event,
            RtfEvent::ControlWord {
                word: "fs".to_string(),
                parameter: Some(24)
            }
        );
    }

    #[test]
    fn test_token_to_event_text() {
        let token = Token::Text("Hello".to_string());
        let event = token_to_event(token);
        assert_eq!(event, RtfEvent::Text("Hello".to_string()));
    }

    #[test]
    fn test_token_to_event_control_symbol() {
        let token = Token::ControlSymbol('*');
        let event = token_to_event(token);
        assert_eq!(event, RtfEvent::ControlSymbol('*'));
    }
}
