# Warning Reference

This document provides comprehensive documentation for all warning types in rtfkit.

## Overview

Warnings are emitted during RTF interpretation to indicate issues that don't prevent parsing but may affect output quality or completeness. Warnings are included in the conversion report (JSON or text format).

**Note on Font and Color Handling**: Font family, font size, and foreground color are now fully supported features. Unresolved font or color indexes (e.g., `\f999` referencing a non-existent font) degrade gracefully without generating warnings - the text is rendered with default formatting. This is not considered a semantic loss since the text content is preserved.

## Warning Severity Levels

| Level | Description |
|-------|-------------|
| `info` | Informational message, no impact on output |
| `warning` | May affect output quality |
| `error` | Significantly affects output |

## Strict Mode Behavior

In strict mode (`--strict` flag), certain warnings cause conversion to fail with exit code 4:

- **Always fails**: `DroppedContent` (any severity)
- **May fail**: Warnings accompanied by `DroppedContent`

See [Strict Mode](#strict-mode) for details.

## Warning Types

### `unsupported_control_word`

Emitted when an RTF control word is recognized but not implemented.

**Severity**: `warning`

**Strict Mode**: Does NOT cause failure (cosmetic loss only)

**JSON Format**:
```json
{
  "type": "unsupported_control_word",
  "word": "strike",
  "parameter": null,
  "severity": "warning"
}
```

**Common Examples**:
- `\strike` - Strikethrough
- `\scaps` - Small caps
- `\caps` - All caps
- `\outl` - Outline

**Impact**: Text content is preserved; formatting is not applied.

---

### `unknown_destination`

Emitted when an RTF destination is not recognized.

**Severity**: `info`

**Strict Mode**: Does NOT cause failure on its own, but typically accompanied by `DroppedContent` which will fail.

**JSON Format**:
```json
{
  "type": "unknown_destination",
  "destination": "header",
  "severity": "info"
}
```

**Common Examples**:
- `\header` - Page header
- `\footer` - Page footer
- Custom application destinations

**Impact**: Destination content is typically dropped (see `DroppedContent`).

---

### `dropped_content`

Emitted when content could not be represented in the output.

**Severity**: `warning`

**Strict Mode**: **ALWAYS causes failure** (exit code 4)

**JSON Format**:
```json
{
  "type": "dropped_content",
  "reason": "Dropped unsupported RTF destination content",
  "size_hint": 42,
  "severity": "warning"
}
```

**Stable Reason Strings**:

The following reason strings are part of the stable API contract and should not change between versions:

| Reason String | Meaning |
|---------------|---------|
| `"merge_semantics"` | Merge semantics were lost or degraded |
| `"Dropped unsupported RTF destination content"` | Unknown destination content |
| `"Dropped unsupported binary RTF data"` | Binary data |
| `"Dropped legacy paragraph numbering content"` | Legacy `\pn` controls |
| `"Unresolved list override ls_id=N"` | List reference could not be resolved |

**Warning Cap Behavior**:

When the warning count limit is reached, `DroppedContent` warnings are specially preserved to ensure the strict-mode signal is not lost. If a `DroppedContent` warning arrives after the cap, it will replace the last non-`DroppedContent` warning.

**Impact**: Content is lost in output. Use strict mode to detect semantic loss.

---

### `unsupported_list_control`

Emitted when a list-related control word is not fully supported.

**Severity**: `warning`

**Strict Mode**: Does NOT cause failure on its own. If content is dropped, a separate `DroppedContent` warning will be emitted.

**JSON Format**:
```json
{
  "type": "unsupported_list_control",
  "control_word": "pnlvlbody",
  "severity": "warning"
}
```

**Common Examples**:
- `\pnlvlbody` - List body level
- `\pnlvlblt` - Bullet level
- `\pntext` - List text

**Impact**: List structure is preserved; some formatting may be lost.

---

### `unresolved_list_override`

Emitted when a list reference (`\lsN`) cannot be resolved to a list definition.

**Severity**: `warning`

**Strict Mode**: **Causes failure** - always accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "unresolved_list_override",
  "ls_id": 999,
  "severity": "warning"
}
```

**Cause**: The `\lsN` index references a list override that doesn't exist in `\listoverridetable`.

**Impact**: List item is rendered as plain paragraph. `DroppedContent` warning is also emitted.

---

### `unsupported_nesting_level`

Emitted when list nesting level exceeds the supported range (0-8).

**Severity**: `info`

**Strict Mode**: Does NOT cause failure (level is clamped).

**JSON Format**:
```json
{
  "type": "unsupported_nesting_level",
  "level": 10,
  "max": 8,
  "severity": "info"
}
```

**Cause**: DOCX supports a maximum of 9 list levels (0-8). Levels beyond this are clamped.

**Impact**: List item is rendered at the maximum supported level (8).

---

### `unsupported_table_control`

Emitted when a table-related control word is not mapped to output.

**Severity**: `warning`

**Strict Mode**: Does NOT cause failure (cosmetic loss only).

**JSON Format**:
```json
{
  "type": "unsupported_table_control",
  "control_word": "trbrdrt",
  "severity": "warning"
}
```

**Common Examples**:
- `\trbrdrt`, `\trbrdrl`, `\trbrdrb`, `\trbrdrr` - Table borders
- `\clbrdrt`, `\clbrdrl`, `\clbrdrb`, `\clbrdrr` - Cell borders
- `\clcbpat` - Cell shading

**Impact**: Table structure is preserved; formatting is not applied.

---

### `malformed_table_structure`

Emitted when table structure is malformed or incomplete.

**Severity**: `warning`

**Strict Mode**: May cause failure if accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "malformed_table_structure",
  "reason": "Cell count mismatch between rows",
  "severity": "warning"
}
```

**Common Causes**:
- Mismatched cell counts between rows
- Invalid nesting
- Missing row/cell definitions

**Impact**: Table is reconstructed with best-effort recovery. May emit `DroppedContent` if content is lost.

---

### `unclosed_table_cell`

Emitted when a table cell is not properly closed with `\cell`.

**Severity**: `warning`

**Strict Mode**: May cause failure if accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "unclosed_table_cell",
  "severity": "warning"
}
```

**Cause**: Missing `\cell` control before row end or document end.

**Impact**: Cell is auto-closed to preserve content. May emit `DroppedContent` for potential content reordering.

---

### `unclosed_table_row`

Emitted when a table row is not properly closed with `\row`.

**Severity**: `warning`

**Strict Mode**: May cause failure if accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "unclosed_table_row",
  "severity": "warning"
}
```

**Cause**: Missing `\row` control before next row or document end.

**Impact**: Row is auto-closed to preserve content. May emit `DroppedContent` for potential content reordering.

---

### `merge_conflict`

Emitted when merge semantics conflict or merge structure is invalid.

**Severity**: `warning`

**Strict Mode**: **Causes failure** - always accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "merge_conflict",
  "reason": "Orphan merge continuation without start - treating as standalone cell",
  "severity": "warning"
}
```

**Stable Reason Strings**:

| Reason String | Meaning |
|---------------|---------|
| `"Orphan merge continuation without start - treating as standalone cell"` | `\clmrg` or `\clvmrg` without preceding start |

**Common Causes**:
- Orphan merge continuation (no merge start)
- Conflicting horizontal and vertical merge directions
- Invalid merge chain

**Impact**: Merge is resolved deterministically. `DroppedContent` warning is also emitted.

---

### `table_geometry_conflict`

Emitted when table geometry is invalid (e.g., non-monotonic cell positions, impossible spans).

**Severity**: `warning`

**Strict Mode**: **Causes failure** if accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "table_geometry_conflict",
  "reason": "Merge span 5 exceeds available cells 3 - clamped",
  "severity": "warning"
}
```

**Stable Reason Strings**:

| Reason String | Meaning |
|---------------|---------|
| `"Merge span N exceeds available cells M - clamped"` | Horizontal merge span exceeds row width |

**Common Causes**:
- Merge span exceeding available cells
- Non-monotonic `\cellxN` positions
- Invalid cell widths

**Impact**: Geometry is adjusted or clamped. May emit `DroppedContent` for semantic loss.

---

### `pattern_degraded`

Emitted when a shading pattern cannot be fully represented in the output format.

**Severity**: `info`

**Strict Mode**: Does NOT cause failure (cosmetic loss only).

**JSON Format**:
```json
{
  "kind": "partial_support",
  "message": "pattern_degraded_paragraph_shading"
}
```

Common message values include:
- `pattern_degraded_paragraph_shading`
- `pattern_degraded_cell_shading`

**Common Causes**:
- Non-percent hatch/stripe/cross patterns in HTML or Typst output

**Impact**: Pattern is rendered as solid fill using the fill color. DOCX output preserves patterns fully.

---

### `unsupported_image_format`

Emitted when an RTF image is in an unsupported format.

**Severity**: `warning`

**Strict Mode**: **Causes failure** - always accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "unsupported_image_format",
  "format": "wmetafile",
  "severity": "warning"
}
```

**Common Examples**:
- `\wmetafile` - Windows Metafile (WMF)
- `\emfblip` - Enhanced Metafile (EMF)

**Behavior**:
- Default mode: Image is dropped, conversion continues with warning
- Strict mode: Conversion fails with exit code 4

**Recommendation**: Convert images to PNG or JPEG format before embedding in RTF.

---

### `malformed_image_hex_payload`

Emitted when the hex data for an embedded image is invalid.

**Severity**: `warning`

**Strict Mode**: **Causes failure** - always accompanied by `DroppedContent`.

**JSON Format**:
```json
{
  "type": "malformed_image_hex_payload",
  "reason": "odd-length hex string",
  "severity": "warning"
}
```

**Stable Reason Strings**:

| Reason String | Meaning |
|---------------|---------|
| `"odd-length hex string"` | Hex data has odd number of characters |
| `"invalid hex characters"` | Hex data contains non-hex characters |

**Common Causes**:
- Corrupted RTF file
- Invalid characters in hex data (e.g., `ZZ` instead of valid hex)
- Truncated image data

**Behavior**:
- Default mode: Image is dropped, conversion continues with warning
- Strict mode: Conversion fails with exit code 4

**Recommendation**: Ensure the RTF file is not corrupted and the image data is properly encoded.

## Strict Mode

When running with `--strict`, the conversion fails (exit code 4) if any `DroppedContent` warnings are present.

### Behavior

1. **Normal mode**: Warnings are collected and reported, conversion succeeds
2. **Strict mode**: Any `DroppedContent` warning causes immediate failure

### Warning Cap Preservation

When the warning count limit (default: 1000) is reached:

1. Normal warnings are no longer collected
2. `DroppedContent` warnings are specially preserved
3. If a `DroppedContent` arrives after the cap, it replaces the last non-`DroppedContent` warning

This ensures strict mode always detects semantic loss, even with pathological inputs.

### Example

```sh
# Normal mode - succeeds with warnings
rtfkit convert input.rtf --format json
# Exit code: 0

# Strict mode - fails on dropped content
rtfkit convert input.rtf --strict --format json
# Exit code: 4
# Stderr: Strict mode violated: DroppedContent warnings detected
```

## Reason String Stability

### Stable Reason Strings

The following reason strings are part of the stable API contract:

**`DroppedContent` reasons**:
- `"merge_semantics"`
- `"Dropped unsupported RTF destination content"`
- `"Dropped unsupported binary RTF data"`
- `"Dropped legacy paragraph numbering content"`
- `"Unresolved list override ls_id=N"`
- `"Dropped unsupported field type"`
- `"Field with no result text"`
- `"Field with no instruction text"`
- `"Field with no instruction and no result"`
- `"Malformed or unsupported hyperlink URL"`
- `"Unsupported hyperlink URL scheme"`
- `"Nested fields are not supported"`
- `"unsupported image format"`
- `"malformed image hex payload"`

**`MergeConflict` reasons**:
- `"Orphan merge continuation without start - treating as standalone cell"`

**`TableGeometryConflict` reasons**:
- `"Merge span N exceeds available cells M - clamped"`

### Guidance for Future Changes

When adding new warning reasons:

1. **Use descriptive, stable strings** - Avoid implementation details
2. **Document in code** - Add to the warning type's doc comment
3. **Add tests** - Verify reason strings in contract tests
4. **Update this document** - Add to the stable reason strings table

When modifying existing reasons:

1. **Avoid breaking changes** - Existing reason strings should not change
2. **Add new reasons** - If semantics change, add a new reason string
3. **Deprecate, don't remove** - Keep old reasons for compatibility

## Related Documentation

- [Architecture Overview](arch/README.md) - Warning system design
- [Feature Support Matrix](feature-support.md) - Supported features
- [Limits Policy](limits-policy.md) - Parser limits
- [CHANGELOG.md](../CHANGELOG.md) - Version history
