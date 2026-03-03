//! Pipeline Module
//!
//! This module contains the main parsing pipeline that orchestrates:
//! - Tokenization
//! - Token validation
//! - Event processing
//! - Document finalization

use crate::error::{ConversionError, ParseError};
use crate::limits::ParserLimits;
use crate::{Block, Document, Report, TableCell};

use super::events::{RtfEvent, token_to_event};
use super::state::RuntimeState;
use super::state_images::ImageByteTracker;
use super::tokenizer::{tokenize, validate_tokens};

/// Parse RTF input and return a Document with a Report.
pub fn parse_pipeline(
    input: &str,
    limits: ParserLimits,
) -> Result<(Document, Report), ConversionError> {
    if input.len() > limits.max_input_bytes {
        return Err(ConversionError::Parse(ParseError::InputTooLarge {
            size: input.len(),
            limit: limits.max_input_bytes,
        }));
    }

    let mut state = RuntimeState::new(limits.clone());
    state.report_builder.set_bytes_processed(input.len());
    state.report_builder.set_limits(limits);

    let tokens = tokenize(input)
        .map_err(|e| ConversionError::Parse(ParseError::TokenizationError(format!("{:?}", e))))?;
    validate_tokens(&tokens)?;

    for token in tokens {
        process_event(&mut state, token_to_event(token))?;
    }

    super::finalize::finalize_document(&mut state);

    if let Some(err) = state.hard_failure.take() {
        return Err(ConversionError::Parse(err));
    }

    let report = state.report_builder.build();
    Ok((state.document, report))
}

fn process_event(state: &mut RuntimeState, event: RtfEvent) -> Result<(), ConversionError> {
    if let Some(err) = state.hard_failure.take() {
        return Err(ConversionError::Parse(err));
    }

    if state.destinations.skip_destination_depth > 0 {
        return super::handlers_destinations::process_skipped_destination_event(state, event);
    }

    match event {
        RtfEvent::GroupStart => handle_group_start(state)?,
        RtfEvent::GroupEnd => handle_group_end(state),
        RtfEvent::ControlWord { word, parameter } => {
            handle_control_word_event(state, &word, parameter)?;
        }
        RtfEvent::ControlSymbol(symbol) => handle_control_symbol_event(state, symbol),
        RtfEvent::Text(text) => handle_text_event(state, text),
    }

    if let Some(err) = state.hard_failure.take() {
        return Err(ConversionError::Parse(err));
    }

    Ok(())
}

fn handle_group_start(state: &mut RuntimeState) -> Result<(), ConversionError> {
    state.current_depth += 1;
    if state.current_depth > state.limits.max_group_depth {
        return Err(ConversionError::Parse(ParseError::GroupDepthExceeded {
            depth: state.current_depth,
            limit: state.limits.max_group_depth,
        }));
    }

    state.push_group();
    Ok(())
}

fn handle_group_end(state: &mut RuntimeState) {
    if let Some(previous_style) = state.pop_group() {
        state.style = previous_style;
    }
    state.current_depth = state.current_depth.saturating_sub(1);
    state.destinations.destination_marker = false;

    super::handlers_fields::process_bookmark_group_end(state);
    super::handlers_fields::process_field_group_end(state);
    super::handlers_structure::process_structure_group_end(state);

    // Check if we're ending a pict group
    if state.image.is_pict_ended(state.current_depth) {
        // Finalize the image
        finalize_pict_group(state);
    }

    state.image.clear_closed_group_contexts(state.current_depth);
}

/// Finalize a pict group by creating an image block or recording dropped content.
fn finalize_pict_group(state: &mut RuntimeState) {
    // Create a tracker from the current state
    let mut tracker = ImageByteTracker::new(state.limits.max_image_bytes_total);
    tracker.total_bytes = state.image_bytes_used;

    // Call the finalization logic
    let result = super::finalize::finalize_image(&state.image, &mut tracker);

    match result {
        super::finalize::ImageFinalizationResult::Success(block) => {
            // Update the tracker state
            state.image_bytes_used = tracker.total_bytes;
            insert_block_in_current_context(state, block);
        }
        super::finalize::ImageFinalizationResult::Dropped(reason) => {
            // Add warning to report
            let size_hint = Some(state.image.hex_buffer.len());
            state.report_builder.dropped_content(reason, size_hint);
        }
        super::finalize::ImageFinalizationResult::ByteLimitExceeded { attempted_total } => {
            // Hard failure - set the error
            state.set_hard_failure(ParseError::ImageBytesExceeded {
                total: attempted_total,
                limit: state.limits.max_image_bytes_total,
            });
        }
    }

    // Reset image state
    state.image.reset_pict_state();
}

fn insert_block_in_current_context(state: &mut RuntimeState, block: Block) {
    // Flush pending paragraph content first to keep block order deterministic.
    if state.has_pending_paragraph_content() {
        super::finalize::finalize_paragraph(state);
    }

    if state.tables.in_row() {
        if state.tables.current_cell.is_none() {
            state.tables.current_cell = Some(TableCell::new());
        }
        if let Some(cell) = state.tables.current_cell.as_mut() {
            cell.blocks.push(block);
        }
        return;
    }

    if state.tables.in_table() {
        super::finalize::finalize_current_table(state);
    }

    state.push_block_to_current_sink(block);
}

fn handle_control_word_event(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) -> Result<(), ConversionError> {
    match word {
        "par" | "line" => {
            super::finalize::finalize_paragraph(state);
            return Ok(());
        }
        "cell" | "nestcell" => {
            super::handlers_tables::handle_cell_event(state)?;
            return Ok(());
        }
        "row" | "nestrow" => {
            super::handlers_tables::handle_row_event(state)?;
            return Ok(());
        }
        _ => {}
    }

    super::handlers_control_words::handle_control_word(state, word, parameter);
    Ok(())
}

fn handle_control_symbol_event(state: &mut RuntimeState, symbol: char) {
    super::handlers_control_words::handle_control_symbol(state, symbol);
}

fn handle_text_event(state: &mut RuntimeState, text: String) {
    state.mark_current_group_non_destination();

    if state.fields.parsing_field {
        super::handlers_fields::handle_field_text(state, text);
    } else {
        super::handlers_text::handle_text(state, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pipeline_simple() {
        let input = r#"{\rtf1\ansi Hello World}"#;
        let result = parse_pipeline(input, ParserLimits::default());
        assert!(result.is_ok());

        let (doc, report) = result.expect("expected parse success");
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(report.stats.paragraph_count, 1);
    }

    #[test]
    fn test_parse_pipeline_bold() {
        let input = r#"{\rtf1\ansi \b Bold\b0  text}"#;
        let result = parse_pipeline(input, ParserLimits::default());
        assert!(result.is_ok());

        let (doc, _report) = result.expect("expected parse success");
        if let crate::Block::Paragraph(para) = &doc.blocks[0] {
            assert!(
                para.inlines
                    .iter()
                    .any(|i| matches!(i, crate::Inline::Run(r) if r.bold))
            );
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_parse_pipeline_paragraph() {
        let input = r#"{\rtf1\ansi First\par Second}"#;
        let result = parse_pipeline(input, ParserLimits::default());
        assert!(result.is_ok());

        let (doc, report) = result.expect("expected parse success");
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(report.stats.paragraph_count, 2);
    }

    #[test]
    fn test_parse_pipeline_alignment() {
        let input = r#"{\rtf1\ansi \qc Centered}"#;
        let result = parse_pipeline(input, ParserLimits::default());
        assert!(result.is_ok());

        let (doc, _report) = result.expect("expected parse success");
        if let crate::Block::Paragraph(para) = &doc.blocks[0] {
            assert_eq!(para.alignment, crate::Alignment::Center);
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_parse_pipeline_rejects_non_rtf() {
        let input = "not rtf at all";
        let result = parse_pipeline(input, ParserLimits::default());
        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::MissingRtfHeader))
        ));
    }

    #[test]
    fn test_parse_pipeline_rejects_unbalanced() {
        let input = r#"{\rtf1\ansi missing_end"#;
        let result = parse_pipeline(input, ParserLimits::default());
        assert!(matches!(
            result,
            Err(ConversionError::Parse(ParseError::UnbalancedGroups))
        ));
    }
}
