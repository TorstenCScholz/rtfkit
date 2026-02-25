//! Run construction and run-flush helpers.

use super::super::state::RuntimeState;
use crate::{Inline, Run};

/// Create a run from the current text and run style.
pub fn create_run(state: &RuntimeState) -> Run {
    // Resolve font_family from font_index -> font_table
    let font_family = state.resolve_font_family();

    // Resolve font_size from half-points to points
    let font_size = state.resolve_font_size();

    // Resolve color from color_index -> color_table
    let color = state.resolve_color();

    // Resolve background_color with precedence: highlight > background
    let background_color = state.resolve_background_color();

    Run {
        text: state.current_text.clone(),
        bold: state.current_run_style.bold,
        italic: state.current_run_style.italic,
        underline: state.current_run_style.underline,
        font_family,
        font_size,
        color,
        background_color,
    }
}

/// Flush current text as a run into the current paragraph.
pub fn flush_current_text_as_run(state: &mut RuntimeState) {
    if !state.current_text.is_empty() {
        let run = create_run(state);
        state.current_paragraph.inlines.push(Inline::Run(run));
        state.current_text.clear();
    }
}

/// Flush current text as a field run into `field_result_inlines`.
pub fn flush_current_text_as_field_run(state: &mut RuntimeState) {
    if !state.current_text.is_empty() {
        let run = create_run(state);
        state.fields.field_result_inlines.push(Inline::Run(run));
        state.current_text.clear();
    }
}

/// Count runs in a list of inlines.
pub fn inline_run_count(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Run(_) => 1,
            Inline::Hyperlink(link) => link.runs.len(),
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_run_count() {
        let inlines = vec![
            Inline::Run(Run {
                text: "Hello".to_string(),
                bold: false,
                italic: false,
                underline: false,
                font_family: None,
                font_size: None,
                color: None,
                background_color: None,
            }),
            Inline::Run(Run {
                text: "World".to_string(),
                bold: false,
                italic: false,
                underline: false,
                font_family: None,
                font_size: None,
                color: None,
                background_color: None,
            }),
        ];
        assert_eq!(inline_run_count(&inlines), 2);
    }
}
