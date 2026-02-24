//! Field Handlers Module
//!
//! This module contains field/hyperlink control handling, field text capture,
//! and field group finalization.

use super::state::RuntimeState;

/// Handle field-related control words.
///
/// Returns `true` if `word` was recognized and handled.
pub fn handle_field_control_word(state: &mut RuntimeState, word: &str) -> bool {
    match word {
        // \field - Start of a field group
        "field" => {
            // Flush any pending text to the paragraph before starting the field.
            // This ensures text like "Click " before a hyperlink stays outside the link.
            super::handlers_text::flush_current_text_as_run(state);
            state
                .fields
                .start_field(state.current_depth, state.style.snapshot());
            true
        }
        // \fldinst - Field instruction (contains HYPERLINK "url" for hyperlinks)
        "fldinst" => {
            state.fields.start_fldinst(state.current_depth);
            true
        }
        // \fldrslt - Field result (visible content)
        "fldrslt" => {
            state.fields.start_fldrslt(state.current_depth);
            true
        }
        _ => false,
    }
}

/// Handle text while a field is being parsed.
pub fn handle_field_text(state: &mut RuntimeState, text: String) {
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

/// Process field state transitions on group end.
pub fn process_field_group_end(state: &mut RuntimeState) {
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

pub(crate) fn is_supported_hyperlink_url(url: &str) -> bool {
    let lowered = url.trim().to_ascii_lowercase();
    lowered.starts_with("http://") || lowered.starts_with("https://") || lowered.starts_with("mailto:")
}

pub(crate) fn extract_hyperlink_url(instruction: &str) -> Option<String> {
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
