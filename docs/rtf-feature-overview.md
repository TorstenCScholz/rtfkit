# RTF Feature Overview

This document summarizes the current RTF feature support in `rtfkit` (Phase 6).

For a detailed feature support matrix, see [Feature Support Matrix](feature-support.md).

## Supported

- **Core text extraction** into IR and DOCX output
- **Paragraph alignment**: `\ql`, `\qc`, `\qr`, `\qj`
- **Inline text styles**:
  - bold (`\b`)
  - italic (`\i`)
  - underline (`\ul`)
- **Unicode escape handling** (`\uN` with `\ucN`)
- **Lists**:
  - list references (`\lsN`)
  - nesting levels (`\ilvlN`, clamped to 0..8)
  - bullet and decimal ordered output
- **Tables**:
  - row/cell structure (`\trowd`, `\cellxN`, `\intbl`, `\cell`, `\row`)
  - horizontal merges (`\clmgf`, `\clmrg`)
  - vertical merges (`\clvmgf`, `\clvmrg`)
  - cell vertical alignment (`\clvertalt`, `\clvertalc`, `\clvertalb`)
  - deterministic recovery for malformed merge/table structures

## Partially supported / degraded

- **Row-level table properties** are parsed but only partially emitted:
  - parsed: `\trql`, `\trqc`, `\trqr`, `\trleft`
  - currently not fully mapped by `docx-rs` writer
- **Some malformed table/list inputs** are repaired with warnings (and `DroppedContent` when semantics are lost)
- **Warning-cap behavior** preserves strict-mode signal (`DroppedContent`)

## Not yet supported

- **Hyperlinks/fields** as first-class output:
  - `\field`, `\fldinst`, `\fldrslt` currently treated as unsupported destination content
- **Images/embedded objects** as first-class output:
  - `\pict`, `\obj`, related object destinations are currently dropped with warnings
- **Full font/style fidelity** from RTF controls (currently ignored in parser):
  - font family (`\fN`)
  - font size (`\fsN`)
  - color/highlight (`\cfN`, `\cbN`, `\highlightN`)
- **Full RTF table styling parity** (complex borders/layout behavior)

## Notes

- In `--strict` mode, any `DroppedContent` warning fails conversion with exit code `4`.
- Parser safety limits are enforced (input size, group depth, warnings, and table-specific hard limits).
- See [Limits Policy](limits-policy.md) for details on safety limits.
- See [Warning Reference](warning-reference.md) for warning documentation.

## Related Documentation

- [Feature Support Matrix](feature-support.md) - Detailed feature support
- [Warning Reference](warning-reference.md) - Warning documentation
- [Limits Policy](limits-policy.md) - Parser limits
- [Architecture Overview](arch/README.md) - System design
