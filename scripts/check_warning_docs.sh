#!/usr/bin/env bash
# Verifies all Warning enum variants are documented in docs/warning-reference.md
# Run from repo root. Add new variant names here when adding them to report.rs.
set -euo pipefail

WARNING_REF="docs/warning-reference.md"
FAILURES=0

check_variant() {
    local name="$1"
    if ! grep -qF "### \`${name}\`" "$WARNING_REF"; then
        echo "FAIL: Warning variant '${name}' has no section in ${WARNING_REF}"
        FAILURES=$((FAILURES + 1))
    fi
}

check_reason() {
    local reason="$1"
    if ! grep -qF "\"${reason}\"" "$WARNING_REF"; then
        echo "FAIL: Stable reason string missing from ${WARNING_REF}: ${reason}"
        FAILURES=$((FAILURES + 1))
    fi
}

# --- Warning variant coverage (must match Warning enum in crates/rtfkit-core/src/report.rs) ---
check_variant "unsupported_control_word"
check_variant "unknown_destination"
check_variant "dropped_content"
check_variant "unsupported_list_control"
check_variant "unresolved_list_override"
check_variant "unsupported_nesting_level"
check_variant "unsupported_table_control"
check_variant "malformed_table_structure"
check_variant "unclosed_table_cell"
check_variant "unclosed_table_row"
check_variant "merge_conflict"
check_variant "table_geometry_conflict"
check_variant "pattern_degraded"
check_variant "unsupported_image_format"
check_variant "malformed_image_hex_payload"
check_variant "unsupported_field"
check_variant "unsupported_page_field"
check_variant "unsupported_toc_switch"
check_variant "unresolved_page_reference"
check_variant "section_numbering_fallback"

# --- Stable DroppedContent reason strings ---
check_reason "merge_semantics"
check_reason "Dropped unsupported RTF destination content"
check_reason "Dropped unsupported binary RTF data"
check_reason "Dropped legacy paragraph numbering content"
check_reason "Unresolved list override ls_id=N"
check_reason "Dropped unsupported field type"
check_reason "Malformed or unsupported hyperlink URL"
check_reason "Unsupported hyperlink URL scheme"
check_reason "Nested fields are not supported"

if [ "$FAILURES" -gt 0 ]; then
    echo ""
    echo "Documentation check failed: $FAILURES issue(s)."
    echo "  -> Add missing sections to ${WARNING_REF}"
    echo "  -> Add new variant names to this script when adding to report.rs"
    exit 1
fi
echo "Warning documentation check: OK ($(grep -c 'check_variant\|check_reason' "$0") checks passed)"
