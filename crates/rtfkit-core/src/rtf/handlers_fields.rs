//! Field Handlers Module
//!
//! This module contains field/hyperlink control handling, field text capture,
//! and field group finalization.

use super::state::RuntimeState;
use crate::{
    BookmarkAnchor, GeneratedBlockKind, HyperlinkTarget, Inline, PageFieldRef, Run,
    SemanticField, SemanticFieldRef,
};

/// Capture a control word as literal fldinst instruction content.
///
/// Returns `true` when the control word was consumed as instruction text.
pub fn capture_fldinst_control_word(
    state: &mut RuntimeState,
    word: &str,
    parameter: Option<i32>,
) -> bool {
    if !state.fields.parsing_fldinst {
        return false;
    }

    // Structural field words are handled by normal field state transitions.
    if matches!(word, "field" | "fldinst" | "fldrslt") {
        return false;
    }

    state.fields.field_instruction_text.push('\\');
    state.fields.field_instruction_text.push_str(word);
    if let Some(value) = parameter {
        state
            .fields
            .field_instruction_text
            .push_str(&value.to_string());
    }
    true
}

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
    // Destination payload for \bkmkstart should never become visible field text.
    if state.fields.parsing_bkmkstart {
        state.fields.bkmkstart_name.push_str(&text);
        return;
    }

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
    if state.fields.parsing_bkmkstart && state.current_depth < state.fields.bkmkstart_group_depth {
        let name = state.fields.bkmkstart_name.trim().to_string();
        if !name.is_empty() {
            // Capture alignment if this is the first content in the paragraph (no text yet).
            state.capture_paragraph_alignment_if_start();
            // If this bookmark lives inside fldrslt, keep it in field result flow
            // so relative ordering with link text is preserved.
            if state.fields.parsing_fldrslt {
                super::handlers_text::flush_current_text_as_field_run(state);
                state
                    .fields
                    .field_result_inlines
                    .push(Inline::BookmarkAnchor(BookmarkAnchor { name }));
            } else {
                // Flush any pending text so preceding text runs appear before the anchor.
                super::handlers_text::flush_current_text_as_run(state);
                state
                    .current_paragraph
                    .inlines
                    .push(Inline::BookmarkAnchor(BookmarkAnchor { name }));
            }
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
    use super::field_instruction::{ParsedFieldInstruction, parse_field_instruction};

    if state.fields.parsing_fldrslt && !state.current_text.is_empty() {
        super::handlers_text::flush_current_text_as_field_run(state);
    }

    let instruction = state.fields.field_instruction_text.trim();
    let has_instruction = !instruction.is_empty();
    let is_hyperlink_instruction = instruction.to_ascii_uppercase().starts_with("HYPERLINK");
    let had_result_content = !state.fields.field_result_inlines.is_empty();

    match parse_field_instruction(instruction) {
        Some(ParsedFieldInstruction::Hyperlink(target)) => {
            let valid = match &target {
                HyperlinkTarget::ExternalUrl(url) => is_supported_hyperlink_url(url),
                HyperlinkTarget::InternalBookmark(_) => true,
            };

            if valid {
                let mut pending_runs: Vec<Run> = Vec::new();
                let mut emitted_hyperlink_segment = false;
                let mut saw_non_run_inline = false;
                let result_inlines: Vec<Inline> =
                    state.fields.field_result_inlines.drain(..).collect();

                for inline in result_inlines {
                    match inline {
                        Inline::Run(run) => pending_runs.push(run),
                        other_inline => {
                            if !pending_runs.is_empty() {
                                state.capture_paragraph_alignment_if_start();
                                let runs = std::mem::take(&mut pending_runs);
                                state.current_paragraph.inlines.push(Inline::Hyperlink(
                                    Hyperlink {
                                        target: target.clone(),
                                        runs,
                                    },
                                ));
                                emitted_hyperlink_segment = true;
                            }
                            state.capture_paragraph_alignment_if_start();
                            state.current_paragraph.inlines.push(other_inline);
                            saw_non_run_inline = true;
                        }
                    }
                }

                if !pending_runs.is_empty() {
                    state.capture_paragraph_alignment_if_start();
                    state
                        .current_paragraph
                        .inlines
                        .push(Inline::Hyperlink(Hyperlink {
                            target,
                            runs: pending_runs,
                        }));
                    emitted_hyperlink_segment = true;
                }

                if !emitted_hyperlink_segment {
                    if !saw_non_run_inline {
                        state
                            .report_builder
                            .dropped_content("Field with no result text", None);
                    } else {
                        state.report_builder.dropped_content(
                            "Hyperlink field had no text runs for clickable content",
                            None,
                        );
                    }
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
        }
        Some(ParsedFieldInstruction::PageField(mut page_field)) => {
            if let PageFieldRef::PageRef { fallback_text, .. } = &mut page_field
                && fallback_text.is_none()
                && had_result_content
            {
                let text = extract_visible_text_from_inlines(&state.fields.field_result_inlines);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    *fallback_text = Some(trimmed.to_string());
                }
            }

            state.capture_paragraph_alignment_if_start();
            state
                .current_paragraph
                .inlines
                .push(Inline::PageField(page_field));
            state.fields.field_result_inlines.clear();
        }
        Some(ParsedFieldInstruction::SemanticField(mut semantic_ref)) => {
            // Build SemanticField struct projecting runs from fldrslt.
            let sf = build_semantic_field(&mut semantic_ref, &mut state.fields.field_result_inlines);

            let is_merge = matches!(sf.reference, SemanticFieldRef::MergeField { .. });

            state.capture_paragraph_alignment_if_start();
            state
                .current_paragraph
                .inlines
                .push(Inline::SemanticField(sf));

            if is_merge {
                state
                    .report_builder
                    .unsupported_field("Unsupported field type: MERGEFIELD (result preserved)");
            }
            state.fields.field_result_inlines.clear();
        }
        Some(ParsedFieldInstruction::Toc {
            options,
            unsupported_switches,
        }) => {
            for switch in unsupported_switches {
                state.report_builder.unsupported_toc_switch(&switch);
            }
            state.capture_paragraph_alignment_if_start();
            state
                .current_paragraph
                .inlines
                .push(Inline::GeneratedBlockMarker(
                    GeneratedBlockKind::TableOfContents { options },
                ));
            state.fields.field_result_inlines.clear();
        }
        None => {
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

/// Project `field_result_inlines` into a `SemanticField` struct, preserving run formatting.
///
/// Non-run inlines (BookmarkAnchor, nested fields, etc.) are counted and flagged
/// via `has_non_run_content`; their plain text is recorded as `fallback_text` for
/// legacy compatibility.
fn build_semantic_field(
    reference: &mut SemanticFieldRef,
    result_inlines: &mut Vec<Inline>,
) -> SemanticField {
    let mut runs: Vec<Run> = Vec::new();
    let mut has_non_run_content = false;

    for inline in result_inlines.iter() {
        match inline {
            Inline::Run(run) => runs.push(run.clone()),
            _ => {
                has_non_run_content = true;
            }
        }
    }

    // Keep fallback_text populated for consumers that have not yet adopted runs.
    if !runs.is_empty() && get_fallback_text(reference).is_none() {
        let text: String = runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join("");
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            set_fallback_text(reference, trimmed.to_string());
        }
    } else if get_fallback_text(reference).is_none() {
        let text = extract_visible_text_from_inlines(result_inlines);
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            set_fallback_text(reference, trimmed.to_string());
        }
    }

    SemanticField {
        reference: reference.clone(),
        runs,
        has_non_run_content,
        resolved: true, // may be set false by core cross-reference resolver later
    }
}

fn get_fallback_text(field_ref: &SemanticFieldRef) -> Option<&str> {
    match field_ref {
        SemanticFieldRef::Ref { fallback_text, .. }
        | SemanticFieldRef::NoteRef { fallback_text, .. }
        | SemanticFieldRef::Sequence { fallback_text, .. }
        | SemanticFieldRef::DocProperty { fallback_text, .. }
        | SemanticFieldRef::MergeField { fallback_text, .. } => fallback_text.as_deref(),
    }
}

fn set_fallback_text(field_ref: &mut SemanticFieldRef, text: String) {
    match field_ref {
        SemanticFieldRef::Ref { fallback_text, .. }
        | SemanticFieldRef::NoteRef { fallback_text, .. }
        | SemanticFieldRef::Sequence { fallback_text, .. }
        | SemanticFieldRef::DocProperty { fallback_text, .. }
        | SemanticFieldRef::MergeField { fallback_text, .. } => {
            if fallback_text.is_none() {
                *fallback_text = Some(text);
            }
        }
    }
}

pub(crate) fn is_supported_hyperlink_url(url: &str) -> bool {
    let lowered = url.trim().to_ascii_lowercase();
    lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("mailto:")
}

fn extract_visible_text_from_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Run(run) => out.push_str(&run.text),
            Inline::Hyperlink(link) => {
                for run in &link.runs {
                    out.push_str(&run.text);
                }
            }
            Inline::BookmarkAnchor(_)
            | Inline::NoteRef(_)
            | Inline::PageField(_)
            | Inline::SemanticField(_)
            | Inline::GeneratedBlockMarker(_) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HyperlinkTarget;
    use super::super::field_instruction::{parse_hyperlink, parse_semantic_field};

    #[test]
    fn test_parse_hyperlink_target_external_url() {
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK "https://example.com""#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://example.com".to_string()
            ))
        );
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK "https://test.com/path""#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://test.com/path".to_string()
            ))
        );
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK \o "tooltip" "https://example.com""#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://example.com".to_string()
            ))
        );
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK https://example.com"#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://example.com".to_string()
            ))
        );
        assert_eq!(parse_hyperlink("HYPERLINK"), None);
        assert_eq!(
            parse_hyperlink("HYPERLINK noquote"),
            Some(HyperlinkTarget::ExternalUrl("noquote".to_string()))
        );
        assert_eq!(parse_hyperlink("DATE"), None);
    }

    #[test]
    fn test_parse_hyperlink_target_internal_bookmark() {
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK \l "section1""#),
            Some(HyperlinkTarget::InternalBookmark("section1".to_string()))
        );
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK \l "my bookmark""#),
            Some(HyperlinkTarget::InternalBookmark("my bookmark".to_string()))
        );
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK \l noquote"#),
            Some(HyperlinkTarget::InternalBookmark("noquote".to_string()))
        );
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK \o "tip" \l "section1""#),
            Some(HyperlinkTarget::InternalBookmark("section1".to_string()))
        );
    }

    #[test]
    fn test_is_supported_hyperlink_url() {
        assert!(is_supported_hyperlink_url("https://example.com"));
        assert!(is_supported_hyperlink_url("http://example.com"));
        assert!(is_supported_hyperlink_url("mailto:test@example.com"));
        assert!(!is_supported_hyperlink_url("ftp://example.com"));
        assert!(!is_supported_hyperlink_url("javascript:alert(1)"));
    }

    #[test]
    fn test_parse_semantic_ref_field() {
        assert_eq!(
            parse_semantic_field(r#"REF myBookmark \h"#),
            Some(SemanticFieldRef::Ref {
                target: "myBookmark".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_parse_semantic_ref_switch_before_arg() {
        // New test: flag switch before positional arg must not skip the arg.
        assert_eq!(
            parse_semantic_field(r#"REF \h myBookmark"#),
            Some(SemanticFieldRef::Ref {
                target: "myBookmark".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_parse_semantic_mergefield() {
        assert_eq!(
            parse_semantic_field(r#"MERGEFIELD "CustomerName" \* MERGEFORMAT"#),
            Some(SemanticFieldRef::MergeField {
                name: "CustomerName".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_parse_semantic_docproperty_builtins() {
        assert_eq!(
            parse_semantic_field("AUTHOR"),
            Some(SemanticFieldRef::DocProperty {
                name: "AUTHOR".to_string(),
                fallback_text: None
            })
        );
        assert_eq!(
            parse_semantic_field("DOCPROPERTY Company"),
            Some(SemanticFieldRef::DocProperty {
                name: "Company".to_string(),
                fallback_text: None
            })
        );
    }
}
