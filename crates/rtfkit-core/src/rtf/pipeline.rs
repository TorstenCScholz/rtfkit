//! Pipeline Module
//!
//! This module contains the main parsing pipeline that orchestrates:
//! - Tokenization
//! - Token validation
//! - Event processing
//! - Document finalization

use crate::error::{ConversionError, ParseError};
use crate::limits::ParserLimits;
use crate::{Document, Report};

use super::events::{RtfEvent, token_to_event};
use super::state::RuntimeState;
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

    process_field_group_end(state);
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
        "cell" => {
            handle_cell_event(state)?;
            return Ok(());
        }
        "row" => {
            handle_row_event(state)?;
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
        handle_field_text(state, text);
    } else {
        super::handlers_text::handle_text(state, text);
    }
}

fn handle_cell_event(state: &mut RuntimeState) -> Result<(), ConversionError> {
    if !state.tables.in_table() || !state.tables.in_row() {
        state
            .report_builder
            .malformed_table_structure("\\cell encountered outside table context");
        state
            .report_builder
            .dropped_content("Table cell control outside table context", None);
        super::finalize::finalize_paragraph(state);
        return Ok(());
    }

    super::finalize::finalize_paragraph_for_table(state);

    if state.tables.current_cell.is_none() {
        state.tables.current_cell = Some(crate::TableCell::new());
    }

    super::finalize::finalize_current_cell(state);
    Ok(())
}

fn handle_row_event(state: &mut RuntimeState) -> Result<(), ConversionError> {
    if !state.tables.in_table() || !state.tables.in_row() {
        state
            .report_builder
            .malformed_table_structure("\\row encountered outside table context");
        state
            .report_builder
            .dropped_content("Table row control outside table context", None);
        return Ok(());
    }

    super::finalize::auto_close_table_cell_if_needed(state, "Unclosed table cell at row end");
    super::finalize::finalize_current_row(state);

    state.tables.pending_cellx.clear();
    state.tables.pending_cell_merges.clear();
    state.tables.pending_cell_v_aligns.clear();
    state.tables.seen_intbl_in_paragraph = false;

    Ok(())
}

fn handle_field_text(state: &mut RuntimeState, text: String) {
    if let Some(nested) = state.fields.nested_fields.last() {
        if nested.parsing_fldinst {
            return;
        }
        if nested.parsing_fldrslt {
            super::handlers_text::handle_field_result_text(state, text);
        }
        return;
    }

    if state.fields.parsing_fldinst {
        state.fields.field_instruction_text.push_str(&text);
    } else if state.fields.parsing_fldrslt {
        super::handlers_text::handle_field_result_text(state, text);
    }
}

fn process_field_group_end(state: &mut RuntimeState) {
    if !state.fields.parsing_field {
        return;
    }

    let (exit_nested_fldinst, exit_nested_fldrslt, exit_nested_field) =
        if let Some(nested) = state.fields.nested_fields.last() {
            (
                nested.parsing_fldinst && state.current_depth < nested.fldinst_group_depth,
                nested.parsing_fldrslt && state.current_depth < nested.fldrslt_group_depth,
                state.current_depth < nested.field_group_depth,
            )
        } else {
            (false, false, false)
        };

    if exit_nested_fldrslt {
        super::handlers_text::flush_current_text_as_field_run(state);
    }

    if let Some(nested) = state.fields.nested_fields.last_mut() {
        if exit_nested_fldinst {
            nested.parsing_fldinst = false;
        }
        if exit_nested_fldrslt {
            nested.parsing_fldrslt = false;
        }
    }

    if exit_nested_field {
        state.fields.nested_fields.pop();
    }

    if state.fields.parsing_fldinst && state.current_depth < state.fields.fldinst_group_depth {
        state.fields.parsing_fldinst = false;
    }

    if state.fields.parsing_fldrslt && state.current_depth < state.fields.fldrslt_group_depth {
        super::handlers_text::flush_current_text_as_field_run(state);
        state.fields.parsing_fldrslt = false;
    }

    if state.current_depth < state.fields.field_group_depth {
        finalize_field(state);
    }
}

fn finalize_field(state: &mut RuntimeState) {
    use crate::{Hyperlink, Inline};

    if state.fields.parsing_fldrslt && !state.current_text.is_empty() {
        super::handlers_text::flush_current_text_as_field_run(state);
    }

    let instruction = state.fields.field_instruction_text.trim();
    let has_instruction = !instruction.is_empty();
    let is_hyperlink_instruction = instruction.to_ascii_uppercase().starts_with("HYPERLINK");
    let had_result_content = !state.fields.field_result_inlines.is_empty();
    let parsed_url = if is_hyperlink_instruction {
        extract_hyperlink_url(instruction).map(|url| url.trim().to_string())
    } else {
        None
    };

    if let Some(url) = parsed_url {
        if is_supported_hyperlink_url(&url) {
            let runs: Vec<crate::Run> = state
                .fields
                .field_result_inlines
                .iter()
                .filter_map(|inline| match inline {
                    Inline::Run(run) => Some(run.clone()),
                    _ => None,
                })
                .collect();

            if !runs.is_empty() {
                let hyperlink = Hyperlink { url, runs };
                state
                    .current_paragraph
                    .inlines
                    .push(Inline::Hyperlink(hyperlink));
            } else {
                state
                    .report_builder
                    .dropped_content("Field with no result text", None);
            }
        } else {
            for inline in state.fields.field_result_inlines.drain(..) {
                state.current_paragraph.inlines.push(inline);
            }
            state
                .report_builder
                .dropped_content("Unsupported hyperlink URL scheme", None);
        }
    } else {
        for inline in state.fields.field_result_inlines.drain(..) {
            state.current_paragraph.inlines.push(inline);
        }

        if is_hyperlink_instruction {
            if had_result_content {
                state
                    .report_builder
                    .dropped_content("Malformed or unsupported hyperlink URL", None);
            } else {
                state
                    .report_builder
                    .dropped_content("Field with no result text", None);
            }
        } else if has_instruction {
            state
                .report_builder
                .dropped_content("Dropped unsupported field type", None);
        } else if had_result_content {
            state
                .report_builder
                .dropped_content("Field with no instruction text", None);
        } else {
            state
                .report_builder
                .dropped_content("Field with no instruction and no result", None);
        }
    }

    state.fields.parsing_field = false;
    state.fields.field_group_depth = 0;
    state.fields.parsing_fldinst = false;
    state.fields.fldinst_group_depth = 0;
    state.fields.parsing_fldrslt = false;
    state.fields.fldrslt_group_depth = 0;
    state.fields.field_instruction_text.clear();
    state.fields.field_result_inlines.clear();
    state.fields.nested_fields.clear();

    if let Some(style) = state.fields.field_style_snapshot.take() {
        state.current_run_style = style;
    }
}

fn is_supported_hyperlink_url(url: &str) -> bool {
    let lowered = url.trim().to_ascii_lowercase();
    lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("mailto:")
}

fn extract_hyperlink_url(instruction: &str) -> Option<String> {
    let text = instruction.trim();

    if !text.to_uppercase().starts_with("HYPERLINK") {
        return None;
    }

    let rest = &text["HYPERLINK".len()..];
    let rest = rest.trim_start();

    if !rest.starts_with('"') {
        return None;
    }

    let rest = &rest[1..];
    if let Some(end_quote_pos) = rest.find('"') {
        return Some(rest[..end_quote_pos].to_string());
    }

    None
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

    #[test]
    fn test_extract_hyperlink_url() {
        assert_eq!(
            extract_hyperlink_url(r#"HYPERLINK "https://example.com""#),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            extract_hyperlink_url(r#"HYPERLINK "https://test.com/path""#),
            Some("https://test.com/path".to_string())
        );
        assert_eq!(extract_hyperlink_url("HYPERLINK"), None);
        assert_eq!(extract_hyperlink_url("HYPERLINK noquote"), None);
    }

    #[test]
    fn test_is_supported_hyperlink_url() {
        assert!(is_supported_hyperlink_url("https://example.com"));
        assert!(is_supported_hyperlink_url("http://example.com"));
        assert!(is_supported_hyperlink_url("mailto:test@example.com"));
        assert!(!is_supported_hyperlink_url("ftp://example.com"));
        assert!(!is_supported_hyperlink_url("javascript:alert(1)"));
    }
}
