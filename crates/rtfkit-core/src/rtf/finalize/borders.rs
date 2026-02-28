//! Border finalization helpers.
//!
//! Converts parser-side `CellBorderCapture` / `RowBorderCapture` (which hold
//! color-table indices) into fully-resolved `BorderSet` IR values.

use super::super::state::RuntimeState;
use super::super::state_tables::{CellBorderCapture, PendingBorderAttrs, RowBorderCapture};
use crate::{Border, BorderSet, BorderStyle};

/// Resolve a single `PendingBorderAttrs` into an IR `Border`.
///
/// Returns `None` only when the attrs describe a `None`-style border with no
/// color or width (i.e. the border is effectively absent).  A `BorderStyle::None`
/// border **is** returned so that the IR can represent "explicitly suppressed".
fn build_border(attrs: &PendingBorderAttrs, state: &RuntimeState) -> Option<Border> {
    let color = attrs
        .color_idx
        .and_then(|idx| state.resolve_color_from_index(idx));
    Some(Border {
        style: attrs.style,
        width_half_pts: attrs.width_half_pts,
        color,
    })
}

/// Build a `BorderSet` from a per-cellx `CellBorderCapture`.
///
/// Returns `None` when all four sides are absent.
pub fn build_border_set_from_cell(
    capture: &CellBorderCapture,
    state: &RuntimeState,
) -> Option<BorderSet> {
    let top = capture.top.as_ref().and_then(|a| build_border(a, state));
    let left = capture.left.as_ref().and_then(|a| build_border(a, state));
    let bottom = capture
        .bottom
        .as_ref()
        .and_then(|a| build_border(a, state));
    let right = capture.right.as_ref().and_then(|a| build_border(a, state));

    if top.is_none() && left.is_none() && bottom.is_none() && right.is_none() {
        return None;
    }
    Some(BorderSet {
        top,
        left,
        bottom,
        right,
        inside_h: None,
        inside_v: None,
    })
}

/// Build a `BorderSet` from a `RowBorderCapture`.
///
/// Returns `None` when all six sides are absent.
pub fn build_border_set_from_row(
    capture: &RowBorderCapture,
    state: &RuntimeState,
) -> Option<BorderSet> {
    let top = capture.top.as_ref().and_then(|a| build_border(a, state));
    let left = capture.left.as_ref().and_then(|a| build_border(a, state));
    let bottom = capture
        .bottom
        .as_ref()
        .and_then(|a| build_border(a, state));
    let right = capture.right.as_ref().and_then(|a| build_border(a, state));
    let inside_h = capture
        .inside_h
        .as_ref()
        .and_then(|a| build_border(a, state));
    let inside_v = capture
        .inside_v
        .as_ref()
        .and_then(|a| build_border(a, state));

    if top.is_none()
        && left.is_none()
        && bottom.is_none()
        && right.is_none()
        && inside_h.is_none()
        && inside_v.is_none()
    {
        return None;
    }
    Some(BorderSet {
        top,
        left,
        bottom,
        right,
        inside_h,
        inside_v,
    })
}

// Suppress unused-import lint for BorderStyle used transitively via PendingBorderAttrs
const _: fn() = || {
    let _ = BorderStyle::Single;
};
