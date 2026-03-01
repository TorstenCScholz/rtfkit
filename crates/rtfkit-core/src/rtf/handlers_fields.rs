//! Field Handlers Module
//!
//! This module contains field/hyperlink control handling, field text capture,
//! and field group finalization.

use super::state::RuntimeState;
use crate::{
    BookmarkAnchor, GeneratedBlockKind, HyperlinkTarget, Inline, PageFieldRef, PageNumberFormat,
    TocOptions,
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

#[derive(Debug, Clone, PartialEq)]
enum ParsedFieldInstruction {
    Hyperlink(HyperlinkTarget),
    PageField(PageFieldRef),
    Toc {
        options: TocOptions,
        unsupported_switches: Vec<String>,
    },
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

    match parse_field_instruction(instruction) {
        Some(ParsedFieldInstruction::Hyperlink(target)) => {
            let valid = match &target {
                HyperlinkTarget::ExternalUrl(url) => is_supported_hyperlink_url(url),
                HyperlinkTarget::InternalBookmark(_) => true,
            };

            if valid {
                let mut pending_runs: Vec<crate::Run> = Vec::new();
                let mut emitted_hyperlink_segment = false;
                let mut saw_non_run_inline = false;
                let result_inlines: Vec<Inline> = state.fields.field_result_inlines.drain(..).collect();

                for inline in result_inlines {
                    match inline {
                        Inline::Run(run) => pending_runs.push(run),
                        other_inline => {
                            if !pending_runs.is_empty() {
                                state.capture_paragraph_alignment_if_start();
                                let runs = std::mem::take(&mut pending_runs);
                                state
                                    .current_paragraph
                                    .inlines
                                    .push(Inline::Hyperlink(Hyperlink {
                                        target: target.clone(),
                                        runs,
                                    }));
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

    let upper = text.to_ascii_uppercase();
    if !upper.starts_with("HYPERLINK") {
        return None;
    }
    if text.len() > "HYPERLINK".len()
        && !text["HYPERLINK".len()..]
            .chars()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(false)
    {
        return None;
    }

    let rest = text["HYPERLINK".len()..].trim_start();
    let tokens = tokenize_hyperlink_instruction(rest);

    let mut idx = 0usize;
    while idx < tokens.len() {
        match &tokens[idx] {
            HyperlinkToken::Switch(name) => {
                if name.eq_ignore_ascii_case("l") {
                    if let Some(value) = tokens.get(idx + 1).and_then(HyperlinkToken::as_value) {
                        return Some(HyperlinkTarget::InternalBookmark(value.to_string()));
                    }
                    return None;
                }

                if switch_takes_value(name) {
                    idx += 1;
                    if idx < tokens.len() && tokens[idx].as_value().is_some() {
                        idx += 1;
                        continue;
                    }
                }
                idx += 1;
            }
            token => {
                if let Some(url) = token.as_value() {
                    return Some(HyperlinkTarget::ExternalUrl(url.to_string()));
                }
                idx += 1;
            }
        }
    }

    None
}

fn parse_field_instruction(instruction: &str) -> Option<ParsedFieldInstruction> {
    if instruction.is_empty() {
        return None;
    }

    if let Some(target) = parse_hyperlink_target(instruction) {
        return Some(ParsedFieldInstruction::Hyperlink(target));
    }

    parse_page_field(instruction)
        .map(ParsedFieldInstruction::PageField)
        .or_else(|| parse_toc_field(instruction))
}

fn parse_page_field(instruction: &str) -> Option<PageFieldRef> {
    let text = instruction.trim();
    let upper = text.to_ascii_uppercase();

    let format = parse_page_number_format(text);

    if upper == "PAGE" || upper.starts_with("PAGE ") {
        return Some(PageFieldRef::CurrentPage { format });
    }

    if upper == "NUMPAGES" || upper.starts_with("NUMPAGES ") {
        return Some(PageFieldRef::TotalPages { format });
    }

    if upper == "SECTIONPAGES" || upper.starts_with("SECTIONPAGES ") {
        return Some(PageFieldRef::SectionPages { format });
    }

    if upper == "PAGEREF" || upper.starts_with("PAGEREF ") {
        let rest = text["PAGEREF".len()..].trim_start();
        let tokens = tokenize_field_words(rest);
        let mut idx = 0usize;
        while idx < tokens.len() {
            let token = &tokens[idx];
            if token.starts_with('\\') {
                // Skip switch values where applicable.
                if matches!(token.as_str(), "\\p" | "\\h") {
                    idx += 1;
                    continue;
                }
                idx += 1;
                continue;
            }
            return Some(PageFieldRef::PageRef {
                target: token.to_string(),
                format,
                fallback_text: None,
            });
        }
    }

    None
}

fn parse_toc_field(instruction: &str) -> Option<ParsedFieldInstruction> {
    let text = instruction.trim();
    let upper = text.to_ascii_uppercase();
    if upper != "TOC" && !upper.starts_with("TOC ") {
        return None;
    }

    let rest = text["TOC".len()..].trim_start();
    let tokens = tokenize_field_words(rest);
    let mut options = TocOptions::default();
    let mut unsupported_switches = Vec::new();

    let mut idx = 0usize;
    while idx < tokens.len() {
        let token = &tokens[idx];
        if !token.starts_with('\\') {
            idx += 1;
            continue;
        }

        let switch = token.trim_start_matches('\\');
        match switch.to_ascii_lowercase().as_str() {
            "o" => {
                if let Some(value) = tokens.get(idx + 1) {
                    if let Some(levels) = parse_toc_levels(value) {
                        options.levels = Some(levels);
                    } else {
                        unsupported_switches.push(format!("o={value}"));
                    }
                    idx += 2;
                    continue;
                }
                unsupported_switches.push("o".to_string());
            }
            "h" => {
                options.hyperlinks = true;
            }
            "*" | "mergeformat" => {
                // Common formatting switch, ignored intentionally.
            }
            other => {
                unsupported_switches.push(other.to_string());
            }
        }
        idx += 1;
    }

    Some(ParsedFieldInstruction::Toc {
        options,
        unsupported_switches,
    })
}

fn parse_toc_levels(value: &str) -> Option<(u8, u8)> {
    let mut parts = value.split('-');
    let start = parts.next()?.parse::<u8>().ok()?;
    let end = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() || start == 0 || end == 0 || start > end {
        return None;
    }
    Some((start, end))
}

fn parse_page_number_format(instruction: &str) -> PageNumberFormat {
    // Handle the most common field format switch patterns.
    if instruction.contains(r"\* roman") {
        return PageNumberFormat::RomanLower;
    }
    if instruction.contains(r"\* ROMAN") {
        return PageNumberFormat::RomanUpper;
    }
    PageNumberFormat::Arabic
}

fn tokenize_field_words(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '"' {
            chars.next();
            let mut value = String::new();
            let mut escaped = false;
            while let Some(next) = chars.next() {
                if escaped {
                    value.push(next);
                    escaped = false;
                    continue;
                }
                if next == '\\' {
                    escaped = true;
                    continue;
                }
                if next == '"' {
                    break;
                }
                value.push(next);
            }
            tokens.push(value);
            continue;
        }

        let mut token = String::new();
        while let Some(next) = chars.peek().copied() {
            if next.is_whitespace() {
                break;
            }
            token.push(next);
            chars.next();
        }
        if !token.is_empty() {
            tokens.push(token);
        }
    }

    tokens
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
            | Inline::GeneratedBlockMarker(_) => {}
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HyperlinkToken {
    Switch(String),
    Value(String),
}

impl HyperlinkToken {
    fn as_value(&self) -> Option<&str> {
        match self {
            HyperlinkToken::Value(value) => Some(value.as_str()),
            HyperlinkToken::Switch(_) => None,
        }
    }
}

fn switch_takes_value(name: &str) -> bool {
    // Common HYPERLINK switches with following value payload.
    matches!(name.to_ascii_lowercase().as_str(), "o" | "m" | "n" | "t")
}

fn tokenize_hyperlink_instruction(input: &str) -> Vec<HyperlinkToken> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '\\' {
            chars.next();
            let mut switch_name = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_alphabetic() {
                    switch_name.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if !switch_name.is_empty() {
                tokens.push(HyperlinkToken::Switch(switch_name));
                continue;
            }
        }

        if ch == '"' {
            chars.next();
            let mut value = String::new();
            let mut escaped = false;
            while let Some(next) = chars.next() {
                if escaped {
                    value.push(next);
                    escaped = false;
                    continue;
                }
                if next == '\\' {
                    escaped = true;
                    continue;
                }
                if next == '"' {
                    break;
                }
                value.push(next);
            }
            tokens.push(HyperlinkToken::Value(value));
            continue;
        }

        let mut value = String::new();
        while let Some(next) = chars.peek().copied() {
            if next.is_whitespace() {
                break;
            }
            value.push(next);
            chars.next();
        }
        if !value.is_empty() {
            tokens.push(HyperlinkToken::Value(value));
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hyperlink_target_external_url() {
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK "https://example.com""#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://example.com".to_string()
            ))
        );
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK "https://test.com/path""#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://test.com/path".to_string()
            ))
        );
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK \o "tooltip" "https://example.com""#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://example.com".to_string()
            ))
        );
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK https://example.com"#),
            Some(HyperlinkTarget::ExternalUrl(
                "https://example.com".to_string()
            ))
        );
        assert_eq!(parse_hyperlink_target("HYPERLINK"), None);
        assert_eq!(
            parse_hyperlink_target("HYPERLINK noquote"),
            Some(HyperlinkTarget::ExternalUrl("noquote".to_string()))
        );
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
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK \l noquote"#),
            Some(HyperlinkTarget::InternalBookmark("noquote".to_string()))
        );
        assert_eq!(
            parse_hyperlink_target(r#"HYPERLINK \o "tip" \l "section1""#),
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
}
