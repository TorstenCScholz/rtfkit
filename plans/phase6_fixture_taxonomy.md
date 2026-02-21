# Phase 6 Workstream 5.1: Fixture Taxonomy Plan

## Executive Summary

This document provides a comprehensive analysis of the current fixture corpus and a detailed implementation plan for the fixture taxonomy cleanup as part of Phase 6, Workstream 5.1.

## 1. Current Fixture Inventory

### 1.1 Total Counts
- **RTF Fixtures**: 31 files
- **Golden JSON Files**: 31 files (1:1 correspondence)

### 1.2 Current Categorization

#### Text-related Fixtures (8 files)
| Current Name | Purpose | Taxonomy Match |
|--------------|---------|----------------|
| `alignment.rtf` | Tests left/center/right paragraph alignment | ❌ Needs rename |
| `bold_italic.rtf` | Tests bold and italic formatting | ❌ Needs rename |
| `empty.rtf` | Minimal valid RTF document | ❌ Needs rename |
| `mixed_formatting.rtf` | Combined bold/italic/underline in paragraphs | ❌ Needs rename |
| `multiple_paragraphs.rtf` | Sequential paragraph handling | ❌ Needs rename |
| `nested_styles.rtf` | Nested formatting spans | ❌ Needs rename |
| `simple_paragraph.rtf` | Basic single paragraph | ❌ Needs rename |
| `underline.rtf` | Underline formatting | ❌ Needs rename |
| `unicode.rtf` | Unicode character handling | ❌ Needs rename |

#### List-related Fixtures (6 files)
| Current Name | Purpose | Taxonomy Match |
|--------------|---------|----------------|
| `list_bullet_simple.rtf` | Simple bullet list | ✅ Correct |
| `list_decimal_simple.rtf` | Simple numbered list | ✅ Correct |
| `list_mixed_kinds.rtf` | Mixed bullet and numbered lists | ✅ Correct |
| `list_nested_two_levels.rtf` | Two-level nested list | ✅ Correct |
| `list_malformed_fallback.rtf` | Invalid list references with fallback | ⚠️ Consider move |
| `list_unresolved_ls_strict_fail.rtf` | Strict mode failure for unresolved ls | ⚠️ Consider move |

#### Table-related Fixtures (15 files)
| Current Name | Purpose | Taxonomy Match |
|--------------|---------|----------------|
| `table_simple_2x2.rtf` | Basic 2x2 table | ✅ Correct |
| `table_multirow_uneven_content.rtf` | Variable content length cells | ✅ Correct |
| `table_with_list_in_cell.rtf` | List nested inside table cell | ✅ Correct |
| `table_missing_cell_terminator.rtf` | Missing `\cell` control | ⚠️ Consider move |
| `table_missing_row_terminator.rtf` | Missing `\row` control | ⚠️ Consider move |
| `table_orphan_controls.rtf` | Orphan `\cell`/`\row` outside table | ⚠️ Consider move |
| `table_merge_controls_degraded.rtf` | Merge controls with degradation | ⚠️ Consider move |
| `table_conflicting_merge.rtf` | Conflicting merge controls | ⚠️ Consider move |
| `table_horizontal_merge_valid.rtf` | Valid horizontal merge | ✅ Correct |
| `table_large_stress.rtf` | Large table stress test (20x5) | ⚠️ Consider move |
| `table_mixed_merge.rtf` | Mixed merge scenarios | ✅ Correct |
| `table_non_monotonic_cellx.rtf` | Non-monotonic `\cellx` values | ⚠️ Consider move |
| `table_orphan_merge_continuation.rtf` | Orphan `\clmrg` control | ⚠️ Consider move |
| `table_prose_interleave.rtf` | Prose before/after table | ✅ Correct |
| `table_vertical_merge_valid.rtf` | Valid vertical merge | ✅ Correct |

#### Mixed Content Fixtures (1 file)
| Current Name | Purpose | Taxonomy Match |
|--------------|---------|----------------|
| `complex.rtf` | Multi-feature document | ❌ Needs rename |

#### Missing Categories
| Category | Current Count | Required |
|----------|---------------|----------|
| `malformed_*` | 0 explicit | Need to create/identify |
| `limits_*` | 0 explicit | Need to create |

---

## 2. Proposed Renames

### 2.1 Text Fixtures → `text_*` Prefix

| Current Name | Proposed Name | Rationale |
|--------------|---------------|-----------|
| `alignment.rtf` | `text_alignment.rtf` | Tests text alignment |
| `bold_italic.rtf` | `text_bold_italic.rtf` | Tests character formatting |
| `empty.rtf` | `text_empty.rtf` | Minimal document |
| `mixed_formatting.rtf` | `text_mixed_formatting.rtf` | Combined formatting |
| `multiple_paragraphs.rtf` | `text_multiple_paragraphs.rtf` | Multiple paragraphs |
| `nested_styles.rtf` | `text_nested_styles.rtf` | Nested formatting |
| `simple_paragraph.rtf` | `text_simple_paragraph.rtf` | Basic paragraph |
| `underline.rtf` | `text_underline.rtf` | Underline formatting |
| `unicode.rtf` | `text_unicode.rtf` | Unicode handling |

### 2.2 Mixed Content → `mixed_*` Prefix

| Current Name | Proposed Name | Rationale |
|--------------|---------------|-----------|
| `complex.rtf` | `mixed_complex_document.rtf` | Multi-feature document |

### 2.3 Malformed Input → `malformed_*` Prefix

These fixtures test malformed input with graceful degradation:

| Current Name | Proposed Name | Rationale |
|--------------|---------------|-----------|
| `list_malformed_fallback.rtf` | `malformed_list_invalid_ls.rtf` | Invalid list references |
| `list_unresolved_ls_strict_fail.rtf` | `malformed_list_unresolved_ls_strict.rtf` | Strict mode failure |
| `table_missing_cell_terminator.rtf` | `malformed_table_missing_cell.rtf` | Missing cell terminator |
| `table_missing_row_terminator.rtf` | `malformed_table_missing_row.rtf` | Missing row terminator |
| `table_orphan_controls.rtf` | `malformed_table_orphan_controls.rtf` | Orphan table controls |
| `table_merge_controls_degraded.rtf` | `malformed_table_merge_degraded.rtf` | Degraded merge |
| `table_conflicting_merge.rtf` | `malformed_table_conflicting_merge.rtf` | Conflicting merge |
| `table_non_monotonic_cellx.rtf` | `malformed_table_non_monotonic_cellx.rtf` | Non-monotonic cellx |
| `table_orphan_merge_continuation.rtf` | `malformed_table_orphan_merge.rtf` | Orphan merge control |

### 2.4 Limits Testing → `limits_*` Prefix

| Current Name | Proposed Name | Rationale |
|--------------|---------------|-----------|
| `table_large_stress.rtf` | `limits_table_large_stress.rtf` | Large table near limits |

---

## 3. Missing Fixtures to Add

### 3.1 Mixed Content Fixtures

| Proposed Name | Purpose | Priority |
|---------------|---------|----------|
| `mixed_prose_list_table.rtf` | Interleaved prose, list, and table content | High |
| `mixed_table_list_nested.rtf` | Table with nested lists in multiple cells | Medium |

### 3.2 Malformed Input Fixtures

| Proposed Name | Purpose | Priority |
|---------------|---------|----------|
| `malformed_repeated_bad_controls.rtf` | Repeated malformed controls with preserved text | High |
| `malformed_unclosed_groups.rtf` | Unclosed RTF groups | Medium |
| `malformed_invalid_control_words.rtf` | Unknown control words | Low |

### 3.3 Limits Testing Fixtures

Based on [`limits.rs`](crates/rtfkit-core/src/limits.rs:44-82) defaults:
- `max_rows_per_table`: 10,000
- `max_cells_per_row`: 1,000
- `max_merge_span`: 1,000
- `max_input_bytes`: 10 MB
- `max_group_depth`: 256

| Proposed Name | Purpose | Priority |
|---------------|---------|----------|
| `limits_rows_near_max.rtf` | Table with rows near 10,000 limit | High |
| `limits_rows_exceed_max.rtf` | Table exceeding row limit (should fail) | High |
| `limits_cells_near_max.rtf` | Row with cells near 1,000 limit | High |
| `limits_cells_exceed_max.rtf` | Row exceeding cell limit (should fail) | High |
| `limits_merge_span_near_max.rtf` | Merge near 1,000 cell span | Medium |
| `limits_merge_span_exceed_max.rtf` | Merge exceeding limit (should fail) | Medium |
| `limits_depth_near_max.rtf` | Group nesting near 256 levels | Medium |
| `limits_depth_exceed_max.rtf` | Group nesting exceeding limit (should fail) | Medium |

---

## 4. Golden File Updates

### 4.1 Files Requiring Rename (to match RTF renames)

All golden JSON files must be renamed to match their corresponding RTF fixture renames:

| Current Golden | Proposed Golden |
|----------------|-----------------|
| `alignment.json` | `text_alignment.json` |
| `bold_italic.json` | `text_bold_italic.json` |
| `empty.json` | `text_empty.json` |
| `mixed_formatting.json` | `text_mixed_formatting.json` |
| `multiple_paragraphs.json` | `text_multiple_paragraphs.json` |
| `nested_styles.json` | `text_nested_styles.json` |
| `simple_paragraph.json` | `text_simple_paragraph.json` |
| `underline.json` | `text_underline.json` |
| `unicode.json` | `text_unicode.json` |
| `complex.json` | `mixed_complex_document.json` |
| `list_malformed_fallback.json` | `malformed_list_invalid_ls.json` |
| `list_unresolved_ls_strict_fail.json` | `malformed_list_unresolved_ls_strict.json` |
| `table_missing_cell_terminator.json` | `malformed_table_missing_cell.json` |
| `table_missing_row_terminator.json` | `malformed_table_missing_row.json` |
| `table_orphan_controls.json` | `malformed_table_orphan_controls.json` |
| `table_merge_controls_degraded.json` | `malformed_table_merge_degraded.json` |
| `table_conflicting_merge.json` | `malformed_table_conflicting_merge.json` |
| `table_non_monotonic_cellx.json` | `malformed_table_non_monotonic_cellx.json` |
| `table_orphan_merge_continuation.json` | `malformed_table_orphan_merge.json` |
| `table_large_stress.json` | `limits_table_large_stress.json` |

### 4.2 New Golden Files to Create

New golden JSON files will be needed for the new fixtures:

| New Golden File | Source Fixture |
|-----------------|----------------|
| `mixed_prose_list_table.json` | `mixed_prose_list_table.rtf` |
| `mixed_table_list_nested.json` | `mixed_table_list_nested.rtf` |
| `malformed_repeated_bad_controls.json` | `malformed_repeated_bad_controls.rtf` |
| `malformed_unclosed_groups.json` | `malformed_unclosed_groups.rtf` |
| `malformed_invalid_control_words.json` | `malformed_invalid_control_words.rtf` |
| `limits_rows_near_max.json` | `limits_rows_near_max.rtf` |
| `limits_cells_near_max.json` | `limits_cells_near_max.rtf` |
| `limits_merge_span_near_max.json` | `limits_merge_span_near_max.rtf` |
| `limits_depth_near_max.json` | `limits_depth_near_max.rtf` |

Note: `*_exceed_max.*` fixtures should fail parsing and will not have golden IR files.

---

## 5. Implementation Checklist

### Phase 1: Rename Existing Fixtures (No Behavior Change)

- [ ] Create rename mapping table for reference
- [ ] Rename text fixtures to `text_*` prefix (9 files)
- [ ] Rename `complex.rtf` to `mixed_complex_document.rtf`
- [ ] Rename malformed table fixtures to `malformed_table_*` prefix (9 files)
- [ ] Rename malformed list fixtures to `malformed_list_*` prefix (2 files)
- [ ] Rename `table_large_stress.rtf` to `limits_table_large_stress.rtf`
- [ ] Rename all corresponding golden JSON files (20 files)
- [ ] Update any hardcoded fixture paths in test files

### Phase 2: Update Test References

- [ ] Search for fixture path references in test files
- [ ] Update [`golden_tests.rs`](crates/rtfkit-cli/tests/golden_tests.rs) fixture paths
- [ ] Update [`cli_contract_tests.rs`](crates/rtfkit-cli/tests/cli_contract_tests.rs) fixture paths
- [ ] Update [`docx_integration_tests.rs`](crates/rtfkit-cli/tests/docx_integration_tests.rs) fixture paths
- [ ] Run tests to verify no regressions

### Phase 3: Add New Fixtures

- [ ] Create `mixed_prose_list_table.rtf` with interleaved content
- [ ] Create `mixed_table_list_nested.rtf` with nested structures
- [ ] Create `malformed_repeated_bad_controls.rtf` with repeated issues
- [ ] Create `malformed_unclosed_groups.rtf` with unclosed braces
- [ ] Create `malformed_invalid_control_words.rtf` with unknown controls
- [ ] Generate golden JSON files for new fixtures

### Phase 4: Add Limits Testing Fixtures

- [ ] Create `limits_rows_near_max.rtf` (e.g., 9,999 rows)
- [ ] Create `limits_rows_exceed_max.rtf` (10,001 rows - should fail)
- [ ] Create `limits_cells_near_max.rtf` (e.g., 999 cells)
- [ ] Create `limits_cells_exceed_max.rtf` (1,001 cells - should fail)
- [ ] Create `limits_merge_span_near_max.rtf` (e.g., 999 cell merge)
- [ ] Create `limits_merge_span_exceed_max.rtf` (1,001 cell merge - should fail)
- [ ] Create `limits_depth_near_max.rtf` (e.g., 255 levels)
- [ ] Create `limits_depth_exceed_max.rtf` (257 levels - should fail)
- [ ] Generate golden JSON for near-limit fixtures
- [ ] Add tests verifying exceed-limit fixtures fail with exit code 2

### Phase 5: Documentation

- [ ] Add metadata comments to each fixture file explaining purpose
- [ ] Update README or create FIXTURES.md documenting the taxonomy
- [ ] Document expected behavior for each fixture category
- [ ] Document exit code expectations for limit-violating fixtures

---

## 6. Fixture Taxonomy Summary

```
fixtures/
├── text_*                    # Text and formatting fixtures
│   ├── text_alignment.rtf
│   ├── text_bold_italic.rtf
│   ├── text_empty.rtf
│   ├── text_mixed_formatting.rtf
│   ├── text_multiple_paragraphs.rtf
│   ├── text_nested_styles.rtf
│   ├── text_simple_paragraph.rtf
│   ├── text_underline.rtf
│   └── text_unicode.rtf
├── list_*                    # List fixtures (valid)
│   ├── list_bullet_simple.rtf
│   ├── list_decimal_simple.rtf
│   ├── list_mixed_kinds.rtf
│   └── list_nested_two_levels.rtf
├── table_*                   # Table fixtures (valid)
│   ├── table_simple_2x2.rtf
│   ├── table_multirow_uneven_content.rtf
│   ├── table_with_list_in_cell.rtf
│   ├── table_horizontal_merge_valid.rtf
│   ├── table_mixed_merge.rtf
│   ├── table_prose_interleave.rtf
│   └── table_vertical_merge_valid.rtf
├── mixed_*                   # Mixed content fixtures
│   ├── mixed_complex_document.rtf
│   ├── mixed_prose_list_table.rtf      # NEW
│   └── mixed_table_list_nested.rtf     # NEW
├── malformed_*               # Malformed input fixtures
│   ├── malformed_list_invalid_ls.rtf
│   ├── malformed_list_unresolved_ls_strict.rtf
│   ├── malformed_table_missing_cell.rtf
│   ├── malformed_table_missing_row.rtf
│   ├── malformed_table_orphan_controls.rtf
│   ├── malformed_table_merge_degraded.rtf
│   ├── malformed_table_conflicting_merge.rtf
│   ├── malformed_table_non_monotonic_cellx.rtf
│   ├── malformed_table_orphan_merge.rtf
│   ├── malformed_repeated_bad_controls.rtf  # NEW
│   ├── malformed_unclosed_groups.rtf        # NEW
│   └── malformed_invalid_control_words.rtf  # NEW
└── limits_*                  # Limit testing fixtures
    ├── limits_table_large_stress.rtf
    ├── limits_rows_near_max.rtf             # NEW
    ├── limits_rows_exceed_max.rtf           # NEW (should fail)
    ├── limits_cells_near_max.rtf            # NEW
    ├── limits_cells_exceed_max.rtf          # NEW (should fail)
    ├── limits_merge_span_near_max.rtf       # NEW
    ├── limits_merge_span_exceed_max.rtf     # NEW (should fail)
    ├── limits_depth_near_max.rtf            # NEW
    └── limits_depth_exceed_max.rtf          # NEW (should fail)
```

---

## 7. Risk Assessment

### Low Risk
- Renaming existing fixtures (purely mechanical)
- Updating test file references

### Medium Risk
- Creating new malformed fixtures (need to verify expected behavior)
- Creating near-limit fixtures (may reveal edge cases)

### High Risk
- Creating exceed-limit fixtures (must verify fail-closed behavior)
- Large fixtures may impact test performance

### Mitigations
- Run full test suite after each phase
- Verify exit codes for limit-violating fixtures
- Keep golden files in sync with fixture changes

---

## 8. Dependencies

This plan depends on:
- Current limits defined in [`limits.rs`](crates/rtfkit-core/src/limits.rs:96-104)
- Test infrastructure in [`golden_tests.rs`](crates/rtfkit-cli/tests/golden_tests.rs)
- Exit code contracts defined in [`PHASE6.md`](docs/specs/PHASE6.md:93-100)

---

## 9. Next Steps

1. Review this plan with stakeholders
2. Switch to Code mode for implementation
3. Execute phases sequentially with test verification between each
4. Update this document if requirements change during implementation
