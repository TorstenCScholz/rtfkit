//! Field instruction parsers.
//!
//! Each function parses one recognized field type from a raw instruction string.
//! These functions are pure — no warnings, no state mutation.

use super::spec;
use super::tokenize::{HyperlinkToken, tokenize_field_words, tokenize_hyperlink_instruction};
use super::types::{ParsedFieldInstruction, SwitchKind};
use crate::{HyperlinkTarget, PageFieldRef, PageNumberFormat, SemanticFieldRef, TocOptions};

// =============================================================================
// Public entry point
// =============================================================================

/// Parse a raw `\fldinst` string into a typed instruction.
///
/// Returns `None` when the instruction is empty or not a recognized field type.
/// This function is pure — it does not mutate any state or emit warnings.
pub fn parse_field_instruction(instruction: &str) -> Option<ParsedFieldInstruction> {
    let text = instruction.trim();
    if text.is_empty() {
        return None;
    }

    if let Some(target) = parse_hyperlink(text) {
        return Some(ParsedFieldInstruction::Hyperlink(target));
    }

    if let Some(field_ref) = parse_semantic_field(text) {
        return Some(ParsedFieldInstruction::SemanticField(field_ref));
    }

    parse_page_field(text)
        .map(ParsedFieldInstruction::PageField)
        .or_else(|| parse_toc_field(text))
}

// =============================================================================
// HYPERLINK
// =============================================================================

/// Parse a `HYPERLINK` instruction.
///
/// Handles both `HYPERLINK "url"` and `HYPERLINK \l "bookmark"`.
pub fn parse_hyperlink(instruction: &str) -> Option<HyperlinkTarget> {
    let upper = instruction.to_ascii_uppercase();
    if !upper.starts_with("HYPERLINK") {
        return None;
    }
    // Must be followed by whitespace or end-of-string after the keyword.
    if instruction.len() > "HYPERLINK".len()
        && !instruction["HYPERLINK".len()..]
            .chars()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(false)
    {
        return None;
    }

    let rest = instruction["HYPERLINK".len()..].trim_start();
    let tokens = tokenize_hyperlink_instruction(rest);

    let mut idx = 0usize;
    while idx < tokens.len() {
        match &tokens[idx] {
            HyperlinkToken::Switch(name) => {
                if name.eq_ignore_ascii_case("l") {
                    // \l takes a bookmark value
                    if let Some(value) = tokens.get(idx + 1).and_then(HyperlinkToken::as_value) {
                        return Some(HyperlinkTarget::InternalBookmark(value.to_string()));
                    }
                    return None;
                }
                // Skip known value-taking switches with their argument.
                if matches!(spec::hyperlink_switch_kind(name), SwitchKind::Value) {
                    idx += 1; // skip switch
                    if idx < tokens.len() && tokens[idx].as_value().is_some() {
                        idx += 1; // skip value
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

// =============================================================================
// Semantic fields (REF, NOTEREF, SEQ, DOCPROPERTY, MERGEFIELD, built-ins)
// =============================================================================

/// Parse a semantic field instruction.
pub fn parse_semantic_field(instruction: &str) -> Option<SemanticFieldRef> {
    let (keyword, rest) = split_keyword(instruction)?;
    let upper = keyword.to_ascii_uppercase();

    match upper.as_str() {
        "AUTHOR" | "TITLE" | "SUBJECT" | "KEYWORDS" => Some(SemanticFieldRef::DocProperty {
            name: upper,
            fallback_text: None,
        }),
        "REF" => {
            let tokens = tokenize_field_words(rest);
            let args = positional_args_with_spec(&tokens, spec::ref_switch_kind);
            Some(SemanticFieldRef::Ref {
                target: args.first()?.to_string(),
                fallback_text: None,
            })
        }
        "NOTEREF" => {
            let tokens = tokenize_field_words(rest);
            let args = positional_args_with_spec(&tokens, spec::noteref_switch_kind);
            Some(SemanticFieldRef::NoteRef {
                target: args.first()?.to_string(),
                fallback_text: None,
            })
        }
        "SEQ" => {
            let tokens = tokenize_field_words(rest);
            let args = positional_args_with_spec(&tokens, spec::seq_switch_kind);
            Some(SemanticFieldRef::Sequence {
                identifier: args.first()?.to_string(),
                fallback_text: None,
            })
        }
        "DOCPROPERTY" => {
            let tokens = tokenize_field_words(rest);
            let args = positional_args_with_spec(&tokens, spec::default_switch_kind);
            Some(SemanticFieldRef::DocProperty {
                name: args.first()?.to_string(),
                fallback_text: None,
            })
        }
        "MERGEFIELD" => {
            let tokens = tokenize_field_words(rest);
            let args = positional_args_with_spec(&tokens, spec::default_switch_kind);
            Some(SemanticFieldRef::MergeField {
                name: args.first()?.to_string(),
                fallback_text: None,
            })
        }
        _ => None,
    }
}

// =============================================================================
// PAGE fields
// =============================================================================

/// Parse a page-management field instruction.
pub fn parse_page_field(instruction: &str) -> Option<PageFieldRef> {
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
        let args = positional_args_with_spec(&tokens, spec::pageref_switch_kind);
        if let Some(target) = args.first() {
            return Some(PageFieldRef::PageRef {
                target: target.to_string(),
                format,
                fallback_text: None,
            });
        }
    }

    None
}

// =============================================================================
// TOC
// =============================================================================

/// Parse a TOC field instruction.
pub fn parse_toc_field(instruction: &str) -> Option<ParsedFieldInstruction> {
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

// =============================================================================
// Helpers
// =============================================================================

/// Split a field instruction into its keyword and the remainder.
fn split_keyword(instruction: &str) -> Option<(&str, &str)> {
    let text = instruction.trim();
    if text.is_empty() {
        return None;
    }
    let mut split = text.splitn(2, char::is_whitespace);
    let keyword = split.next()?;
    let rest = split.next().unwrap_or("");
    Some((keyword, rest))
}

/// Extract positional (non-switch) arguments from a token list using a
/// field-specific switch-kind lookup function.
///
/// This correctly handles flag switches that appear before positional arguments:
///   `REF \h target` → `["target"]`   (was broken before; `\h` is a flag)
///   `REF target \h` → `["target"]`   (always worked)
pub fn positional_args_with_spec(
    tokens: &[String],
    switch_kind: fn(&str) -> SwitchKind,
) -> Vec<&str> {
    let mut args = Vec::new();
    let mut idx = 0usize;
    while idx < tokens.len() {
        let token = tokens[idx].as_str();
        if token.starts_with('\\') {
            let switch_name = &token[1..];
            idx += 1;
            if matches!(switch_kind(switch_name), SwitchKind::Value) {
                // Value switch: skip the following value token if it is not itself a switch.
                if idx < tokens.len() && !tokens[idx].starts_with('\\') {
                    idx += 1;
                }
            }
            // Flag switch: do NOT advance past the next token.
            continue;
        }
        args.push(token);
        idx += 1;
    }
    args
}

fn parse_page_number_format(instruction: &str) -> PageNumberFormat {
    if instruction.contains(r"\* roman") {
        return PageNumberFormat::RomanLower;
    }
    if instruction.contains(r"\* ROMAN") {
        return PageNumberFormat::RomanUpper;
    }
    PageNumberFormat::Arabic
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_switch_before_arg() {
        // Bug fix: \h is a flag switch; target must not be skipped.
        let result = parse_semantic_field(r"REF \h myBookmark");
        assert_eq!(
            result,
            Some(SemanticFieldRef::Ref {
                target: "myBookmark".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_ref_switch_after_arg() {
        let result = parse_semantic_field(r"REF myBookmark \h");
        assert_eq!(
            result,
            Some(SemanticFieldRef::Ref {
                target: "myBookmark".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_ref_multiple_switches_before_arg() {
        let result = parse_semantic_field(r"REF \h \n myTarget");
        assert_eq!(
            result,
            Some(SemanticFieldRef::Ref {
                target: "myTarget".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_noteref_switch_before_arg() {
        let result = parse_semantic_field(r"NOTEREF \h fn1");
        assert_eq!(
            result,
            Some(SemanticFieldRef::NoteRef {
                target: "fn1".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_seq_flag_before_arg() {
        let result = parse_semantic_field(r"SEQ \h Figure");
        assert_eq!(
            result,
            Some(SemanticFieldRef::Sequence {
                identifier: "Figure".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_seq_value_switch_consumes_next() {
        // \r takes a value; the next token is its value, not the identifier.
        // So identifier comes AFTER \r <value>.
        let result = parse_semantic_field(r"SEQ Figure \r 1");
        assert_eq!(
            result,
            Some(SemanticFieldRef::Sequence {
                identifier: "Figure".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_mergefield_quoted() {
        let result = parse_semantic_field(r#"MERGEFIELD "CustomerName" \* MERGEFORMAT"#);
        assert_eq!(
            result,
            Some(SemanticFieldRef::MergeField {
                name: "CustomerName".to_string(),
                fallback_text: None
            })
        );
    }

    #[test]
    fn test_hyperlink_external() {
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK "https://example.com""#),
            Some(HyperlinkTarget::ExternalUrl("https://example.com".to_string()))
        );
    }

    #[test]
    fn test_hyperlink_internal() {
        assert_eq!(
            parse_hyperlink(r#"HYPERLINK \l "section1""#),
            Some(HyperlinkTarget::InternalBookmark("section1".to_string()))
        );
    }

    #[test]
    fn test_page_field_current() {
        assert_eq!(
            parse_page_field("PAGE"),
            Some(PageFieldRef::CurrentPage {
                format: PageNumberFormat::Arabic
            })
        );
    }

    #[test]
    fn test_positional_args_flag_before() {
        use crate::rtf::field_instruction::spec::ref_switch_kind;
        let tokens: Vec<String> = vec!["\\h".to_string(), "target".to_string()];
        let args = positional_args_with_spec(&tokens, ref_switch_kind);
        assert_eq!(args, vec!["target"]);
    }

    #[test]
    fn test_positional_args_value_switch_consumes_next() {
        use crate::rtf::field_instruction::spec::ref_switch_kind;
        // \d is a value switch; "sep" is its value, not a positional arg.
        let tokens: Vec<String> = vec!["\\d".to_string(), "sep".to_string(), "target".to_string()];
        let args = positional_args_with_spec(&tokens, ref_switch_kind);
        assert_eq!(args, vec!["target"]);
    }
}
