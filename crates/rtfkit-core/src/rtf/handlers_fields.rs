//! Field Handlers Module
//!
//! This module contains field/hyperlink control handling, field text capture,
//! and field group finalization.

use super::state::RuntimeState;
use crate::{BookmarkAnchor, HyperlinkTarget, Inline};

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
        // \l - Field switch for internal bookmark reference (HYPERLINK \l "name").
        // When inside a fldinst group, the RTF parser consumes \l as a control word;
        // we re-inject it as literal text so parse_hyperlink_target can see it.
        "l" if state.fields.parsing_fldinst => {
            state.fields.field_instruction_text.push_str("\\l");
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

/// Process bookmark state on group end, emitting a BookmarkAnchor if we've exited the group.
pub fn process_bookmark_group_end(state: &mut RuntimeState) {
    if state.fields.parsing_bkmkstart
        && state.current_depth < state.fields.bkmkstart_group_depth
    {
        let name = state.fields.bkmkstart_name.trim().to_string();
        if !name.is_empty() {
            // Capture alignment if this is the first content in the paragraph (no text yet).
            state.capture_paragraph_alignment_if_start();
            // Flush any pending text so preceding text runs appear before the anchor.
            super::handlers_text::flush_current_text_as_run(state);
            state
                .current_paragraph
                .inlines
                .push(Inline::BookmarkAnchor(BookmarkAnchor { name }));
        }
        state.fields.reset_bkmkstart();
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
    use crate::Hyperlink;

    if state.fields.parsing_fldrslt && !state.current_text.is_empty() {
        super::handlers_text::flush_current_text_as_field_run(state);
    }

    let instruction = state.fields.field_instruction_text.trim();
    let has_instruction = !instruction.is_empty();
    let is_hyperlink_instruction = instruction.to_ascii_uppercase().starts_with("HYPERLINK");
    let had_result_content = !state.fields.field_result_inlines.is_empty();

    let parsed_target = if is_hyperlink_instruction {
        parse_hyperlink_target(instruction)
    } else {
        None
    };

    if let Some(target) = parsed_target {
        let valid = match &target {
            HyperlinkTarget::ExternalUrl(url) => is_supported_hyperlink_url(url),
            HyperlinkTarget::InternalBookmark(_) => true,
        };

        if valid {
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
                state.capture_paragraph_alignment_if_start();
                let hyperlink = Hyperlink { target, runs };
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
            // Unsupported external URL scheme — preserve result text
            if had_result_content {
                state.capture_paragraph_alignment_if_start();
            }
            for inline in state.fields.field_result_inlines.drain(..) {
                state.current_paragraph.inlines.push(inline);
            }
            state
                .report_builder
                .dropped_content("Unsupported hyperlink URL scheme", None);
        }
    } else {
        // No recognized target — preserve result text
        if had_result_content {
            state.capture_paragraph_alignment_if_start();
        }
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
            if had_result_content {
                // Result text preserved — non-strict degradation
                state
                    .report_builder
                    .unsupported_field("Unrecognized field type; result text preserved");
            } else {
                // No result text — content is truly lost
                state
                    .report_builder
                    .dropped_content("Dropped unsupported field type with no result", None);
            }
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
    lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("mailto:")
}

/// Parse a HYPERLINK field instruction into a typed target.
///
/// Handles:
/// - `HYPERLINK "https://example.com"` → `ExternalUrl`
/// - `HYPERLINK \l "bookmark_name"` → `InternalBookmark`
///
/// Returns `None` if the instruction is not a HYPERLINK or cannot be parsed.
pub(crate) fn parse_hyperlink_target(instruction: &str) -> Option<HyperlinkTarget> {
    let text = instruction.trim();

    if !text.to_ascii_uppercase().starts_with("HYPERLINK") {
        return None;
    }

    let rest = text["HYPERLINK".len()..].trim_start();

    // Check for \l switch (internal bookmark link)
    if let Some(after_l) = rest.strip_prefix("\\l") {
        let after_l = after_l.trim_start();
        if let Some(name) = extract_quoted_string(after_l) {
            return Some(HyperlinkTarget::InternalBookmark(name));
        }
        return None;
    }

    // Otherwise: quoted external URL
    if let Some(url) = extract_quoted_string(rest) {
        return Some(HyperlinkTarget::ExternalUrl(url));
    }

    None
}

fn extract_quoted_string(s: &str) -> Option<String> {
    let s = s.trim_start();
    let s = s.strip_prefix('"')?;
    let end = s.find('"')?;
    Some(s[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hyperlink_target_external_url() {
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK "https://example.com""#),
            Some(HyperlinkTarget::ExternalUrl("https://example.com".to_string()))
        );
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK "https://test.com/path""#),
            Some(HyperlinkTarget::ExternalUrl("https://test.com/path".to_string()))
        );
        assert_eq!(parse_hyperlink_target("HYPERLINK"), None);
        assert_eq!(parse_hyperlink_target("HYPERLINK noquote"), None);
        assert_eq!(parse_hyperlink_target("DATE"), None);
    }

    #[test]
    fn test_parse_hyperlink_target_internal_bookmark() {
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK \l "section1""#),
            Some(HyperlinkTarget::InternalBookmark("section1".to_string()))
        );
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK \l "my bookmark""#),
            Some(HyperlinkTarget::InternalBookmark("my bookmark".to_string()))
        );
        // \l with no quoted string → None
        assert_eq!(parse_hyperlink_target(r#"HYPERLINK \l noquote"#), None);
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
